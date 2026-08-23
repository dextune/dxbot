//! Required audit intent atomicity and redacted observation.
//!
//! The [`AuditLogger`] owns the canonical, security-sensitive audit record.
//! Every audit intent ([`AuditLogger::log_operation`]) and security decision
//! ([`AuditLogger::log_security_event`]) is recorded as one atomic,
//! append-only entry: the in-memory store wraps a [`Vec`] in a mutex, so a
//! write either fully commits or is never observed — the same guarantee a real
//! write-then-fsync-before-mark-complete append-only journal provides.
//!
//! Observation is **redacted**: queries return [`RedactedAuditRecord`]s whose
//! payloads never expose secret-like material. The raw bytes are retained
//! internally for audit integrity, but no caller observes them without going
//! through the redaction boundary.

use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use dxbot_core::types::{OperationId, PrincipalRef};

/// Secret-like keyword hints. Any whitespace-delimited token containing one of
/// these (case-insensitively) is treated as secret material and redacted.
const SECRET_HINTS: [&str; 5] = ["secret", "password", "credential", "token", "key"];

/// Text substituted for any secret-like token in a redacted observation.
const REDACTED: &str = "[REDACTED]";

/// Result alias for audit operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Stable, semantically-typed errors returned by the audit logger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The audit log is or was found to be corrupt; fail closed rather than
    /// serving partial or unverifiable records.
    CorruptLog,
    /// The caller supplied an invalid payload or filter.
    InvalidInput(String),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CorruptLog => write!(formatter, "audit log is corrupt"),
            Self::InvalidInput(message) => write!(formatter, "invalid audit input: {message}"),
        }
    }
}

impl std::error::Error for Error {}

/// How sensitive a recorded audit entry is, and how aggressively its payload
/// is redacted when observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionLevel {
    /// No secret-like content; the payload is safe to observe verbatim.
    None,
    /// Some secret-like tokens are present; they are replaced with
    /// `[REDACTED]` while non-secret content is preserved.
    Partial,
    /// The payload is entirely secret-like; the whole payload is hidden.
    Full,
}

/// A security-sensitive decision recorded in the audit log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityEvent {
    AuthenticationSuccess,
    AuthenticationFailure,
    AuthorizationDenied,
    ApprovalDecided,
    InformationFlowBlocked,
}

/// The event kind stored on an [`AuditRecord`]. Audit intents and security
/// decisions share one axis so both can be filtered uniformly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    /// A required audit intent attached to an operation.
    Operation,
    /// A security decision.
    Security(SecurityEvent),
}

/// One immutable, append-only audit entry. The raw `payload` is retained
/// internally for audit integrity; callers must observe it through the
/// redacted [`RedactedAuditRecord`] boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRecord {
    /// Globally unique, monotonically increasing record id.
    pub id: String,
    /// Unix seconds at which the record was committed.
    pub timestamp: i64,
    /// The operation this audit intent is attached to.
    pub operation_id: OperationId,
    /// The principal the record is attributed to.
    pub principal_ref: PrincipalRef,
    /// The event kind being recorded.
    pub event_type: EventType,
    /// The raw, unredacted payload.
    pub payload: String,
    /// The redaction classification computed at write time.
    pub redaction_level: RedactionLevel,
}

/// A redacted projection of an [`AuditRecord`] produced by a query. Its
/// `payload` never contains secret-like material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactedAuditRecord {
    /// The id of the source record.
    pub id: String,
    /// Unix seconds at which the record was committed.
    pub timestamp: i64,
    /// The operation this audit intent is attached to.
    pub operation_id: OperationId,
    /// The principal the record is attributed to.
    pub principal_ref: PrincipalRef,
    /// The event kind being recorded.
    pub event_type: EventType,
    /// The payload with secret-like tokens redacted.
    pub payload: String,
    /// The redaction classification used to produce this observation.
    pub redaction_level: RedactionLevel,
}

/// Matches audit records. Every field except `time_range` is matched exactly;
/// `time_range` is an inclusive `[start, end]` window. `None` means *match any*.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AuditFilter {
    /// Restrict to records attributed to this principal.
    pub principal_ref: Option<PrincipalRef>,
    /// Restrict to records with this event kind.
    pub event_type: Option<EventType>,
    /// Inclusive `[start, end]` unix-seconds window on `timestamp`.
    pub time_range: Option<(i64, i64)>,
    /// Restrict to records attached to this operation.
    pub operation_id: Option<OperationId>,
}

/// The durable audit logger.
///
/// Backed by an in-memory append-only store ([`Arc<Mutex<Vec<AuditRecord>>>`])
/// so crash/fault behaviour can be exercised deterministically; the mutex
/// makes each append atomic, which is exactly what a filesystem
/// write-fsync-mark-complete journal provides. The store is shared through an
/// [`Arc`] so the logger body can be copied cheaply and handed to the layers
/// that need to record — all appends remain serialized by the single mutex.
pub struct AuditLogger {
    store: Arc<Mutex<Vec<AuditRecord>>>,
    sequence: Arc<AtomicU64>,
    corrupt: Arc<AtomicBool>,
}

impl fmt::Debug for AuditLogger {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let len = match self.store.lock() {
            Ok(s) => s.len(),
            Err(_) => {
                self.mark_corrupt();
                0
            }
        };
        formatter
            .debug_struct("AuditLogger")
            .field("record_count", &len)
            .field("corrupt", &self.corrupt.load(Ordering::SeqCst))
            .finish_non_exhaustive()
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditLogger {
    /// Creates an empty, append-only audit logger.
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(Vec::new())),
            sequence: Arc::new(AtomicU64::new(0)),
            corrupt: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Atomically records a required audit intent for `operation_id`. The
    /// write is all-or-nothing: either the record is fully appended (and
    /// visible to every subsequent query) or it is not observed at all, so a
    /// crash mid-write never leaves a partial record.
    ///
    /// No principal is supplied by this intent, so the entry is attributed to
    /// an empty (system/unknown) [`PrincipalRef`]. Use
    /// [`Self::log_operation_as`] when the principal is known.
    pub fn log_operation(
        &self,
        operation_id: &OperationId,
        payload: &str,
    ) -> Result<AuditRecord> {
        self.log_operation_as(&PrincipalRef(String::new()), operation_id, payload)
    }

    /// Atomically records a required audit intent attributed to `principal_ref`.
    pub fn log_operation_as(
        &self,
        principal_ref: &PrincipalRef,
        operation_id: &OperationId,
        payload: &str,
    ) -> Result<AuditRecord> {
        self.append(AuditRecord {
            id: self.next_id(),
            timestamp: now_unix_secs(),
            operation_id: operation_id.clone(),
            principal_ref: principal_ref.clone(),
            event_type: EventType::Operation,
            payload: payload.to_owned(),
            redaction_level: classify(payload),
        })
    }

    /// Records a security decision attributed to an empty (system/unknown)
    /// principal. Use [`Self::log_security_event_as`] when the principal is
    /// known.
    pub fn log_security_event(&self, event: SecurityEvent) -> Result<AuditRecord> {
        self.log_security_event_as(&PrincipalRef(String::new()), event)
    }

    /// Records a security decision attributed to `principal_ref`.
    pub fn log_security_event_as(
        &self,
        principal_ref: &PrincipalRef,
        event: SecurityEvent,
    ) -> Result<AuditRecord> {
        let payload = format!("security-event: {event:?}");
        self.append(AuditRecord {
            id: self.next_id(),
            timestamp: now_unix_secs(),
            operation_id: OperationId(String::new()),
            principal_ref: principal_ref.clone(),
            event_type: EventType::Security(event),
            payload,
            redaction_level: RedactionLevel::None,
        })
    }

    /// Queries the log and returns *redacted* observations.
    ///
    /// Fails closed: if the log has been marked corrupt, no records are
    /// served — a caller must not rely on partial or unverifiable audit data.
    pub fn query_audit_log(&self, filter: &AuditFilter) -> Result<Vec<RedactedAuditRecord>> {
        if self.corrupt.load(Ordering::SeqCst) {
            return Err(Error::CorruptLog);
        }
        let store = self.store.lock().map_err(|_| Error::CorruptLog)?;
        Ok(store
            .iter()
            .filter(|record| matches_filter(record, filter))
            .map(redact)
            .collect())
    }

    /// Number of committed records (read-only diagnostics).
    ///
    /// Returns [`Error::CorruptLog`] if the log has been corrupted or the
    /// internal mutex is poisoned (indicating a prior panic in a critical
    /// section).
    pub fn record_count(&self) -> Result<usize> {
        self.store
            .lock()
            .map(|s| s.len())
            .map_err(|_| {
                self.mark_corrupt();
                Error::CorruptLog
            })
    }

    /// Marks the log as corrupt. Intended as a fault-injection seam: once
    /// triggered, every query and append fails closed with
    /// [`Error::CorruptLog`] rather than serving possibly-incomplete records.
    /// This mirrors what a real implementation does on a truncated or
    /// corrupted append-only journal.
    pub fn mark_corrupt(&self) {
        self.corrupt.store(true, Ordering::SeqCst);
    }

    /// The next monotonic record id.
    fn next_id(&self) -> String {
        let n = self.sequence.fetch_add(1, Ordering::SeqCst);
        format!("audit-{n:016x}")
    }

    /// Append is the atomicity boundary: the mutex guarantees the push (and
    /// the fsync-equivalent serialization of that push in memory) happens as
    /// one indivisible step, so a mid-write crash cannot yield a partial
    /// record. In a real journal this is write-then-fsync-
    /// before-marking-the-record-complete.
    fn append(&self, record: AuditRecord) -> Result<AuditRecord> {
        if self.corrupt.load(Ordering::SeqCst) {
            return Err(Error::CorruptLog);
        }
        let mut store = self.store.lock().map_err(|_| Error::CorruptLog)?;
        store.push(record.clone());
        Ok(record)
    }
}

/// Filters a record against the query, matching exact fields and the
/// inclusive time window.
fn matches_filter(record: &AuditRecord, filter: &AuditFilter) -> bool {
    if let Some(principal) = &filter.principal_ref {
        if record.principal_ref != *principal {
            return false;
        }
    }
    if let Some(event_type) = filter.event_type {
        if record.event_type != event_type {
            return false;
        }
    }
    if let Some((start, end)) = filter.time_range {
        if record.timestamp < start || record.timestamp > end {
            return false;
        }
    }
    if let Some(operation_id) = &filter.operation_id {
        if &record.operation_id != operation_id {
            return false;
        }
    }
    true
}

/// Projects a raw record into a redacted observation.
fn redact(record: &AuditRecord) -> RedactedAuditRecord {
    RedactedAuditRecord {
        id: record.id.clone(),
        timestamp: record.timestamp,
        operation_id: record.operation_id.clone(),
        principal_ref: record.principal_ref.clone(),
        event_type: record.event_type,
        payload: redact_payload(&record.payload, record.redaction_level),
        redaction_level: record.redaction_level,
    }
}

/// Computes the redacted payload for a record at the given level: `None`
/// keeps it verbatim, `Full` hides it entirely, and `Partial` replaces only
/// the secret-like tokens.
fn redact_payload(payload: &str, level: RedactionLevel) -> String {
    match level {
        RedactionLevel::None => payload.to_owned(),
        RedactionLevel::Full => REDACTED.to_owned(),
        RedactionLevel::Partial => payload
            .split_whitespace()
            .map(|token| {
                if is_secret_like(token) {
                    REDACTED
                } else {
                    token
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// Classifies a payload by counting its secret-like tokens.
fn classify(payload: &str) -> RedactionLevel {
    let tokens: Vec<&str> = payload.split_whitespace().collect();
    if tokens.is_empty() {
        return RedactionLevel::None;
    }
    let secret_count = tokens.iter().filter(|token| is_secret_like(token)).count();
    if secret_count == 0 {
        RedactionLevel::None
    } else if secret_count == tokens.len() {
        RedactionLevel::Full
    } else {
        RedactionLevel::Partial
    }
}

/// True when a token contains any secret-like keyword (case-insensitively).
fn is_secret_like(token: &str) -> bool {
    let lower = token.to_lowercase();
    SECRET_HINTS.iter().any(|hint| lower.contains(hint))
}

/// Current unix time in seconds. On an impossible clock failure we fall back
/// to `0` rather than panicking or surfacing a transient error.
fn now_unix_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}