//! Submission client: durable `Prepared`/`Dispatching` before send, binding
//! lookup recovery, and replay from an in-memory journal.
//!
//! The client is deliberately testable without network I/O: the actual send
//! is a call through an injectable [`Transport`] closure, and durability is
//! modelled by an in-memory [`JournalStore`]. The crash fixture
//! ([`SubmissionClient::submit_with_crash_point`]) can either simulate a
//! crash in-process (leaving the journal at the pre-crash boundary, for
//! deterministic unit tests) or genuinely terminate the process with
//! [`std::process::exit`] via the executable probe binary.

use std::cell::RefCell;
use std::fmt;

use dxbot_core::receipt::{ReceiptDisposition, ReceiptRecord};
use dxbot_core::types::{
    CommandId, IdempotencyKey, InstanceId, JournalRecord, JournalState, OperationId,
    OperationRequest, OperationResult, VersionInfo,
};

use crate::crash::CrashPoint;
use crate::journal::JournalStore;

/// Injectable transport used to simulate the control-endpoint round trip.
pub type Transport = Box<dyn Fn(&OperationRequest) -> OperationResult>;

/// Stable, semantically-typed errors returned by the submission client.
#[derive(Debug, Clone, PartialEq)]
pub enum ClientError {
    /// The requested crash point was reached in simulate mode (no process exit).
    SimulatedCrash(CrashPoint),
    /// A storage/journal write rejected the mutation.
    Storage(String),
    /// A record for this command id was already committed.
    AlreadyExists(CommandId),
    /// The requested command has no journal entry to operate on.
    NotCommitted(CommandId),
    /// Recovery must happen before the operation can be replayed.
    RecoveryRequired(CommandId),
    /// The command id is already bound to a different idempotency key.
    IdempotencyKeyConflict(CommandId),
}

impl fmt::Display for ClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SimulatedCrash(point) => write!(formatter, "simulated crash at {point:?}"),
            Self::Storage(message) => write!(formatter, "storage error: {message}"),
            Self::AlreadyExists(command_id) => {
                write!(formatter, "command already committed: {}", command_id.0)
            }
            Self::NotCommitted(command_id) => {
                write!(formatter, "record not committed: {}", command_id.0)
            }
            Self::RecoveryRequired(command_id) => {
                write!(formatter, "recovery required before reissue: {}", command_id.0)
            }
            Self::IdempotencyKeyConflict(command_id) => {
                write!(
                    formatter,
                    "command id {} is already bound to a different idempotency key",
                    command_id.0
                )
            }
        }
    }
}

impl std::error::Error for ClientError {}

/// A durable, recoverable submission client for one runtime instance.
///
/// Created via [`SubmissionClient::new`], or via [`SubmissionClient::builder`]
/// when the in-memory journal or the transport needs to be injected (which is
/// how tests exercise the crash fixture without network I/O).
pub struct SubmissionClient {
    /// The runtime instance this client submits on behalf of.
    pub instance_id: InstanceId,
    hard_crash: bool,
    journal: RefCell<JournalStore>,
    transport: RefCell<Transport>,
    /// The version compatibility matrix this client advertises.
    version: VersionInfo,
}

impl fmt::Debug for SubmissionClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SubmissionClient")
            .field("instance_id", &self.instance_id)
            .field("hard_crash", &self.hard_crash)
            .field("journal_len", &self.journal.borrow().len())
            .field("client_version", &self.version.client_version)
            .finish_non_exhaustive()
    }
}

/// Builder for a [`SubmissionClient`] with injectable journal and transport.
pub struct SubmissionClientBuilder {
    instance_id: InstanceId,
    hard_crash: bool,
    journal: JournalStore,
    transport: Option<Transport>,
    version: VersionInfo,
}

impl fmt::Debug for SubmissionClientBuilder {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SubmissionClientBuilder")
            .field("instance_id", &self.instance_id)
            .field("hard_crash", &self.hard_crash)
            .field("journal_len", &self.journal.len())
            .field("transport_configured", &self.transport.is_some())
            .finish()
    }
}

impl SubmissionClientBuilder {
    /// Configures whether the crash fixture performs a genuine process exit.
    pub fn with_hard_crash(mut self, hard_crash: bool) -> Self {
        self.hard_crash = hard_crash;
        self
    }

    /// Replaces the transport used to simulate the control-endpoint round trip.
    pub fn with_transport(mut self, transport: Transport) -> Self {
        self.transport = Some(transport);
        self
    }

    /// Seeds the client with an existing journal (used for recovery tests).
    pub fn with_journal(mut self, journal: JournalStore) -> Self {
        self.journal = journal;
        self
    }

    /// Injects the version compatibility matrix this client advertises.
    ///
    /// Because the matrix lives in-memory, this lets callers exercise
    /// [`SubmissionClient::get_version_compatibility`] without network I/O.
    pub fn with_version(mut self, version: VersionInfo) -> Self {
        self.version = version;
        self
    }

    /// Builds the configured client.
    pub fn build(self) -> SubmissionClient {
        let default_instance = self.instance_id.clone();
        let transport = self.transport.unwrap_or_else(|| {
            Box::new(move |request| committed_result(request, &default_instance))
        });
        SubmissionClient {
            instance_id: self.instance_id,
            hard_crash: self.hard_crash,
            journal: RefCell::new(self.journal),
            transport: RefCell::new(transport),
            version: self.version,
        }
    }
}

impl SubmissionClient {
    /// Creates a submission client with a fresh in-memory journal and a
    /// default transport that returns a committed result for any request.
    pub fn new(instance_id: InstanceId) -> Self {
        Self::builder(instance_id).build()
    }

    /// Starts a builder for configuring journal/transport/hard-crash behavior.
    pub fn builder(instance_id: InstanceId) -> SubmissionClientBuilder {
        SubmissionClientBuilder {
            instance_id,
            hard_crash: false,
            journal: JournalStore::new(),
            transport: None,
            version: default_version(),
        }
    }

    /// Returns the version compatibility matrix for this runtime's client.
    ///
    /// In-memory: the matrix is injected through
    /// [`SubmissionClientBuilder::with_version`] (defaulting to the local
    /// package version and protocol/schema v1), so the query needs no network
    /// I/O. Returns the client_version, the supported protocol and schema
    /// versions, and — when a remote handshake was observed — the remote
    /// compatibility verdict.
    pub fn get_version_compatibility(&self) -> Result<VersionInfo, ClientError> {
        Ok(self.version.clone())
    }

    /// True when the client is configured to genuinely terminate the process.
    pub fn hard_crash_enabled(&self) -> bool {
        self.hard_crash
    }

    /// Number of committed journal entries (read-only diagnostics).
    pub fn journal_len(&self) -> usize {
        self.journal.borrow().len()
    }

    /// Durable submit: journal `Prepared` + `Dispatching` before send, then
    /// observe/terminate after the response. Idempotent retries return the
    /// existing binding result.
    pub fn submit(&self, request: &OperationRequest) -> Result<OperationResult, ClientError> {
        self.submit_internal(request, None)
    }

    /// Runs the same submission path but terminates at the requested crash
    /// boundary (hard mode) or returns [`ClientError::SimulatedCrash`]
    /// (simulate mode), leaving the journal at the pre-crash state.
    pub fn submit_with_crash_point(
        &self,
        request: &OperationRequest,
        crash_point: CrashPoint,
    ) -> Result<OperationResult, ClientError> {
        self.submit_internal(request, Some(crash_point))
    }

    /// Binding lookup recovery: returns the resolved result for a committed
    /// operation, or `None` when no binding exists. A committed-but-unresolved
    /// binding (crash after commit before response) is reported as a
    /// recoverable result rather than being lost.
    pub fn lookup_operation(
        &self,
        command_id: &CommandId,
        key: &IdempotencyKey,
    ) -> Result<Option<OperationResult>, ClientError> {
        match self.bind(command_id, key) {
            None => Ok(None),
            Some((_record, Some(result))) => Ok(Some(result)),
            Some((record, None)) => Ok(Some(recoverable_result(command_id, &record))),
        }
    }

    /// Replays an operation from the journal. A committed-but-unresolved
    /// binding is re-sent to obtain the terminal result; a fully resolved
    /// binding returns its existing result; an unknown command is an error.
    pub fn replay_operation(
        &self,
        request: &OperationRequest,
    ) -> Result<OperationResult, ClientError> {
        let command_id = request.command_id.clone();
        match self.bind(&command_id, &request.idempotency_key) {
            Some((_record, Some(result))) => Ok(result),
            Some((_record, None)) => {
                let result = self.send(request);
                let mut store = self.journal.borrow_mut();
                store.observe(&command_id, &result)?;
                store.terminate(&command_id, &result)?;
                Ok(result)
            }
            None => Err(ClientError::RecoveryRequired(command_id)),
        }
    }

    fn submit_internal(
        &self,
        request: &OperationRequest,
        crash: Option<CrashPoint>,
    ) -> Result<OperationResult, ClientError> {
        let command_id = request.command_id.clone();

        // Fast path: an existing binding resolves to its prior terminal result
        // (idempotent retry) or to a recoverable marker.
        if let Some((record, result)) = self.bind(&command_id, &request.idempotency_key) {
            return match result {
                Some(result) => Ok(result),
                None => Ok(recoverable_result(&command_id, &record)),
            };
        }

        // The command id exists in the journal but with a different idempotency key.
        if self.journal.borrow().lookup(&command_id).is_some() {
            return Err(ClientError::IdempotencyKeyConflict(command_id));
        }

        // Durable boundary 1: Prepared (the commit point).
        self.crash_at(crash, CrashPoint::BeforeCommit)?;
        let base = self.prepared_base(request);
        self.journal.borrow_mut().prepare(base)?;

        // Durable boundary 2: Dispatching (fsync-before-send).
        self.crash_at(crash, CrashPoint::BeforeDispatching)?;
        self.journal.borrow_mut().dispatch(&command_id)?;

        // Durable boundary 3: after Dispatching is durable, before send.
        self.crash_at(crash, CrashPoint::AfterDispatchingBeforeSend)?;

        // Simulate the network round trip.
        let result = self.send(request);

        // Durable boundary 4: committed, but before the response is observed.
        self.crash_at(crash, CrashPoint::AfterCommitBeforeResponse)?;

        let mut store = self.journal.borrow_mut();
        store.observe(&command_id, &result)?;
        store.terminate(&command_id, &result)?;
        Ok(result)
    }

    /// Fires a crash at `at` if it equals the requested point: either a real
    /// process exit (hard mode) or a [`ClientError::SimulatedCrash`].
    fn crash_at(
        &self,
        requested: Option<CrashPoint>,
        at: CrashPoint,
    ) -> Result<(), ClientError> {
        if requested == Some(at) {
            if self.hard_crash {
                std::process::exit(at.exit_code());
            }
            return Err(ClientError::SimulatedCrash(at));
        }
        Ok(())
    }

    fn send(&self, request: &OperationRequest) -> OperationResult {
        let transport = self.transport.borrow();
        (transport)(request)
    }

    /// Returns the record + resolved result for a command/key binding, if any.
    fn bind(
        &self,
        command_id: &CommandId,
        key: &IdempotencyKey,
    ) -> Option<(JournalRecord, Option<OperationResult>)> {
        self.journal.borrow().find_binding(command_id, key)
    }

    fn prepared_base(&self, request: &OperationRequest) -> JournalRecord {
        JournalRecord {
            state: JournalState::Prepared,
            instance_id: self.instance_id.clone(),
            command_id: request.command_id.clone(),
            idempotency_key: request.idempotency_key.clone(),
            request_digest: request.request_digest.clone(),
            sequence: 0,
            previous_digest: String::new(),
            record_digest: String::new(),
        }
    }
}

/// Default version compatibility matrix advertised by a bare client before a
/// remote handshake is injected via the builder.
fn default_version() -> VersionInfo {
    VersionInfo {
        client_version: env!("CARGO_PKG_VERSION").to_owned(),
        supported_protocol_versions: vec!["1.0".to_owned()],
        supported_schema_versions: vec!["1.0".to_owned()],
        remote_compatibility: None,
    }
}

/// Builds the committed `OperationResult` the default transport returns.
fn committed_result(request: &OperationRequest, instance_id: &InstanceId) -> OperationResult {
    let operation_id = request.new_operation_id.clone();
    OperationResult {
        operation_id: operation_id.clone(),
        command_id: request.command_id.clone(),
        instance_id: instance_id.clone(),
        receipt: ReceiptRecord {
            operation_id: operation_id.0.clone(),
            disposition: ReceiptDisposition::Committed,
            result_ref: format!("result:{}", operation_id.0),
            resolved_binding_digest: String::new(),
            owner_kind: "instance".to_owned(),
            lease_until: None,
            last_progress: 0,
            reconciliation_policy: "at-least-once".to_owned(),
        },
        status: "committed".to_owned(),
        committed_payload: None,
        error: None,
        operation_may_continue: false,
    }
}

/// Builds a `recovery-required` marker for a committed-but-unresolved binding.
fn recoverable_result(command_id: &CommandId, record: &JournalRecord) -> OperationResult {
    OperationResult {
        operation_id: OperationId(format!("recovery:{}", command_id.0)),
        command_id: command_id.clone(),
        instance_id: record.instance_id.clone(),
        receipt: ReceiptRecord {
            operation_id: String::new(),
            disposition: ReceiptDisposition::RecoveryRequired,
            result_ref: String::new(),
            resolved_binding_digest: String::new(),
            owner_kind: "instance".to_owned(),
            lease_until: None,
            last_progress: 0,
            reconciliation_policy: "at-least-once".to_owned(),
        },
        status: "recovery-required".to_owned(),
        committed_payload: None,
        error: None,
        operation_may_continue: true,
    }
}