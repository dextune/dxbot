//! Required audit intent atomicity with write-time redaction.
//!
//! The logger never persists secret-like raw payload bytes. Redaction happens
//! before the append boundary, and record IDs are allocated while holding the
//! same mutex that defines append order.

use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use dxbot_core::types::{OperationId, PrincipalRef};

const SECRET_HINTS: [&str; 5] = ["secret", "password", "credential", "token", "key"];
const REDACTED: &str = "[REDACTED]";

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    CorruptLog,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionLevel {
    None,
    Partial,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityEvent {
    AuthenticationSuccess,
    AuthenticationFailure,
    AuthorizationDenied,
    ApprovalDecided,
    InformationFlowBlocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    Operation,
    Security(SecurityEvent),
}

/// Immutable append-only audit record. `payload` is already redacted when the
/// record is created; raw secret-like payload bytes are never stored here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRecord {
    pub id: String,
    pub timestamp: i64,
    pub operation_id: OperationId,
    pub principal_ref: PrincipalRef,
    pub event_type: EventType,
    pub payload: String,
    pub redaction_level: RedactionLevel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactedAuditRecord {
    pub id: String,
    pub timestamp: i64,
    pub operation_id: OperationId,
    pub principal_ref: PrincipalRef,
    pub event_type: EventType,
    pub payload: String,
    pub redaction_level: RedactionLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AuditFilter {
    pub principal_ref: Option<PrincipalRef>,
    pub event_type: Option<EventType>,
    pub time_range: Option<(i64, i64)>,
    pub operation_id: Option<OperationId>,
}

pub struct AuditLogger {
    store: Arc<Mutex<Vec<AuditRecord>>>,
    sequence: Arc<AtomicU64>,
    corrupt: Arc<AtomicBool>,
}

impl fmt::Debug for AuditLogger {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let len = match self.store.lock() {
            Ok(store) => store.len(),
            Err(_) => {
                self.corrupt.store(true, Ordering::SeqCst);
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
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(Vec::new())),
            sequence: Arc::new(AtomicU64::new(0)),
            corrupt: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn log_operation(
        &self,
        operation_id: &OperationId,
        payload: &str,
    ) -> Result<AuditRecord> {
        self.log_operation_as(&PrincipalRef(String::new()), operation_id, payload)
    }

    pub fn log_operation_as(
        &self,
        principal_ref: &PrincipalRef,
        operation_id: &OperationId,
        payload: &str,
    ) -> Result<AuditRecord> {
        let redaction_level = classify(payload);
        let payload = redact_payload(payload, redaction_level);
        self.append(AuditRecord {
            id: String::new(),
            timestamp: now_unix_secs(),
            operation_id: operation_id.clone(),
            principal_ref: principal_ref.clone(),
            event_type: EventType::Operation,
            payload,
            redaction_level,
        })
    }

    pub fn log_security_event(&self, event: SecurityEvent) -> Result<AuditRecord> {
        self.log_security_event_as(&PrincipalRef(String::new()), event)
    }

    pub fn log_security_event_as(
        &self,
        principal_ref: &PrincipalRef,
        event: SecurityEvent,
    ) -> Result<AuditRecord> {
        self.append(AuditRecord {
            id: String::new(),
            timestamp: now_unix_secs(),
            operation_id: OperationId(String::new()),
            principal_ref: principal_ref.clone(),
            event_type: EventType::Security(event),
            payload: format!("security-event: {event:?}"),
            redaction_level: RedactionLevel::None,
        })
    }

    pub fn query_audit_log(&self, filter: &AuditFilter) -> Result<Vec<RedactedAuditRecord>> {
        validate_filter(filter)?;
        let store = self.lock_store()?;
        if self.corrupt.load(Ordering::SeqCst) {
            return Err(Error::CorruptLog);
        }
        Ok(store
            .iter()
            .filter(|record| matches_filter(record, filter))
            .map(redacted_projection)
            .collect())
    }

    pub fn record_count(&self) -> Result<usize> {
        let store = self.lock_store()?;
        if self.corrupt.load(Ordering::SeqCst) {
            return Err(Error::CorruptLog);
        }
        Ok(store.len())
    }

    /// Once this method returns, every subsequent query and append fails
    /// closed. Taking the append mutex first orders corruption after any append
    /// already in progress and before every append that follows.
    pub fn mark_corrupt(&self) {
        match self.store.lock() {
            Ok(_guard) => self.corrupt.store(true, Ordering::SeqCst),
            Err(_) => self.corrupt.store(true, Ordering::SeqCst),
        }
    }

    fn append(&self, mut record: AuditRecord) -> Result<AuditRecord> {
        let mut store = self.lock_store()?;
        if self.corrupt.load(Ordering::SeqCst) {
            return Err(Error::CorruptLog);
        }

        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
        record.id = format!("audit-{sequence:016x}");
        store.push(record.clone());
        Ok(record)
    }

    fn lock_store(&self) -> Result<MutexGuard<'_, Vec<AuditRecord>>> {
        self.store.lock().map_err(|_| {
            self.corrupt.store(true, Ordering::SeqCst);
            Error::CorruptLog
        })
    }
}

fn validate_filter(filter: &AuditFilter) -> Result<()> {
    if let Some((start, end)) = filter.time_range {
        if start > end {
            return Err(Error::InvalidInput(
                "audit time range start must not exceed end".to_owned(),
            ));
        }
    }
    Ok(())
}

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

fn redacted_projection(record: &AuditRecord) -> RedactedAuditRecord {
    RedactedAuditRecord {
        id: record.id.clone(),
        timestamp: record.timestamp,
        operation_id: record.operation_id.clone(),
        principal_ref: record.principal_ref.clone(),
        event_type: record.event_type,
        payload: record.payload.clone(),
        redaction_level: record.redaction_level,
    }
}

fn redact_payload(payload: &str, level: RedactionLevel) -> String {
    match level {
        RedactionLevel::None => payload.to_owned(),
        RedactionLevel::Full => REDACTED.to_owned(),
        RedactionLevel::Partial => payload
            .split_whitespace()
            .map(|token| if is_secret_like(token) { REDACTED } else { token })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

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

fn is_secret_like(token: &str) -> bool {
    let lower = token.to_lowercase();
    SECRET_HINTS.iter().any(|hint| lower.contains(hint))
}

pub(crate) fn redact_diagnostic_value(value: &str) -> String {
    let level = classify(value);
    redact_payload(value, level)
}

fn now_unix_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}
