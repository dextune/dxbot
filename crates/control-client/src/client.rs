//! Submission client: durable `Prepared`/`Dispatching` before send, binding
//! lookup recovery, and replay from an in-memory journal.
//!
//! CommandId, OperationId, IdempotencyKey, RequestDigest, and InstanceId are a
//! single replay identity. No component is regenerated or silently replaced
//! after the Prepared boundary.

use std::cell::RefCell;
use std::fmt;

use dxbot_core::receipt::{ReceiptDisposition, ReceiptRecord};
use dxbot_core::types::{
    CommandId, IdempotencyKey, InstanceId, JournalRecord, JournalState, OperationId,
    OperationRequest, OperationResult, VersionInfo,
};

use crate::crash::CrashPoint;
use crate::journal::JournalStore;

pub type Transport = Box<dyn Fn(&OperationRequest) -> OperationResult>;

#[derive(Debug, Clone, PartialEq)]
pub enum ClientError {
    SimulatedCrash(CrashPoint),
    Storage(String),
    AlreadyExists(CommandId),
    NotCommitted(CommandId),
    RecoveryRequired(CommandId),
    IdempotencyKeyConflict(CommandId),
    RequestDigestConflict(CommandId),
    OperationIdConflict(CommandId),
    TransportIdentityConflict(CommandId),
    InstanceMismatch {
        client: InstanceId,
        request: InstanceId,
    },
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
            Self::IdempotencyKeyConflict(command_id) => write!(
                formatter,
                "command id {} is already bound to a different idempotency key",
                command_id.0
            ),
            Self::RequestDigestConflict(command_id) => write!(
                formatter,
                "command id {} is already bound to a different request digest",
                command_id.0
            ),
            Self::OperationIdConflict(command_id) => write!(
                formatter,
                "command id {} is already bound to a different operation id",
                command_id.0
            ),
            Self::TransportIdentityConflict(command_id) => write!(
                formatter,
                "transport returned an operation identity inconsistent with command {}",
                command_id.0
            ),
            Self::InstanceMismatch { client, request } => write!(
                formatter,
                "client instance {} does not match request instance {}",
                client.0, request.0
            ),
        }
    }
}

impl std::error::Error for ClientError {}

pub struct SubmissionClient {
    pub instance_id: InstanceId,
    hard_crash: bool,
    journal: RefCell<JournalStore>,
    transport: RefCell<Transport>,
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
    pub fn with_hard_crash(mut self, hard_crash: bool) -> Self {
        self.hard_crash = hard_crash;
        self
    }

    pub fn with_transport(mut self, transport: Transport) -> Self {
        self.transport = Some(transport);
        self
    }

    pub fn with_journal(mut self, journal: JournalStore) -> Self {
        self.journal = journal;
        self
    }

    pub fn with_version(mut self, version: VersionInfo) -> Self {
        self.version = version;
        self
    }

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
    pub fn new(instance_id: InstanceId) -> Self {
        Self::builder(instance_id).build()
    }

    pub fn builder(instance_id: InstanceId) -> SubmissionClientBuilder {
        SubmissionClientBuilder {
            instance_id,
            hard_crash: false,
            journal: JournalStore::new(),
            transport: None,
            version: default_version(),
        }
    }

    pub fn get_version_compatibility(&self) -> Result<VersionInfo, ClientError> {
        Ok(self.version.clone())
    }

    pub fn hard_crash_enabled(&self) -> bool {
        self.hard_crash
    }

    pub fn journal_len(&self) -> usize {
        self.journal.borrow().len()
    }

    pub fn submit(&self, request: &OperationRequest) -> Result<OperationResult, ClientError> {
        self.submit_internal(request, None)
    }

    pub fn submit_with_crash_point(
        &self,
        request: &OperationRequest,
        crash_point: CrashPoint,
    ) -> Result<OperationResult, ClientError> {
        self.submit_internal(request, Some(crash_point))
    }

    /// Binding lookup is read-only discovery. It returns the original durable
    /// OperationId even when a terminal result has not yet been observed.
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

    pub fn replay_operation(
        &self,
        request: &OperationRequest,
    ) -> Result<OperationResult, ClientError> {
        self.validate_request(request)?;
        let command_id = request.command_id.clone();
        match self.bind(&command_id, &request.idempotency_key) {
            Some((record, Some(result))) => {
                ensure_replay_identity(&record, request)?;
                validate_result_identity(request, &result)?;
                Ok(result)
            }
            Some((record, None)) => {
                ensure_replay_identity(&record, request)?;
                let result = self.send(request);
                validate_result_identity(request, &result)?;
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
        self.validate_request(request)?;
        let command_id = request.command_id.clone();

        if let Some((record, result)) = self.bind(&command_id, &request.idempotency_key) {
            ensure_replay_identity(&record, request)?;
            return match result {
                Some(result) => {
                    validate_result_identity(request, &result)?;
                    Ok(result)
                }
                None => Ok(recoverable_result(&command_id, &record)),
            };
        }

        if self.journal.borrow().lookup(&command_id).is_some() {
            return Err(ClientError::IdempotencyKeyConflict(command_id));
        }

        self.crash_at(crash, CrashPoint::BeforeCommit)?;
        self.journal.borrow_mut().prepare(self.prepared_base(request))?;

        self.crash_at(crash, CrashPoint::BeforeDispatching)?;
        self.journal.borrow_mut().dispatch(&command_id)?;

        self.crash_at(crash, CrashPoint::AfterDispatchingBeforeSend)?;
        let result = self.send(request);
        validate_result_identity(request, &result)?;

        self.crash_at(crash, CrashPoint::AfterCommitBeforeResponse)?;
        let mut store = self.journal.borrow_mut();
        store.observe(&command_id, &result)?;
        store.terminate(&command_id, &result)?;
        Ok(result)
    }

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
            operation_id: request.new_operation_id.clone(),
            idempotency_key: request.idempotency_key.clone(),
            request_digest: request.request_digest.clone(),
            sequence: 0,
            previous_digest: String::new(),
            record_digest: String::new(),
        }
    }

    fn validate_request(&self, request: &OperationRequest) -> Result<(), ClientError> {
        if request.payload.instance_id != self.instance_id {
            return Err(ClientError::InstanceMismatch {
                client: self.instance_id.clone(),
                request: request.payload.instance_id.clone(),
            });
        }
        Ok(())
    }
}

fn ensure_replay_identity(
    record: &JournalRecord,
    request: &OperationRequest,
) -> Result<(), ClientError> {
    if record.request_digest != request.request_digest {
        return Err(ClientError::RequestDigestConflict(request.command_id.clone()));
    }
    if record.operation_id != request.new_operation_id {
        return Err(ClientError::OperationIdConflict(request.command_id.clone()));
    }
    Ok(())
}

fn validate_result_identity(
    request: &OperationRequest,
    result: &OperationResult,
) -> Result<(), ClientError> {
    if result.command_id != request.command_id
        || result.operation_id != request.new_operation_id
        || result.instance_id != request.payload.instance_id
        || result.receipt.operation_id != request.new_operation_id.0
        || result.receipt.resolved_binding_digest != request.request_digest.0
    {
        return Err(ClientError::TransportIdentityConflict(
            request.command_id.clone(),
        ));
    }
    Ok(())
}

fn default_version() -> VersionInfo {
    VersionInfo {
        client_version: env!("CARGO_PKG_VERSION").to_owned(),
        supported_protocol_versions: vec!["1.0".to_owned()],
        supported_schema_versions: vec!["1.0".to_owned()],
        remote_compatibility: None,
    }
}

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
            resolved_binding_digest: request.request_digest.0.clone(),
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

fn recoverable_result(command_id: &CommandId, record: &JournalRecord) -> OperationResult {
    OperationResult {
        operation_id: record.operation_id.clone(),
        command_id: command_id.clone(),
        instance_id: record.instance_id.clone(),
        receipt: ReceiptRecord {
            operation_id: record.operation_id.0.clone(),
            disposition: ReceiptDisposition::RecoveryRequired,
            result_ref: format!("operation:{}", record.operation_id.0),
            resolved_binding_digest: record.request_digest.0.clone(),
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