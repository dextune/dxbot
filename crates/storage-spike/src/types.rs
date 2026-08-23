use std::fmt;

use dxbot_core::receipt::ReceiptDisposition;

/// Deliberate process-termination boundary used only by the executable crash fixture.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrashPoint {
    BeforeCommit,
    AfterCommitBeforeResponse,
}

/// Borrowed operation identity used only by the internal storage spike.
#[derive(Debug, Clone, Copy)]
pub struct OperationRequest<'a> {
    pub principal_ref: &'a str,
    pub idempotency_key_principal_ref: &'a str,
    pub idempotency_key_digest: &'a str,
    pub command_id: &'a str,
    pub request_digest: &'a str,
    pub new_operation_id: &'a str,
    pub key_expires_at: i64,
    pub now: i64,
}

/// Borrowed atomic state/event/outbox/audit payload used by the storage spike.
#[derive(Debug, Clone, Copy)]
pub struct OperationEffect<'a> {
    pub aggregate_id: &'a str,
    pub state_value: &'a str,
    pub event_payload: &'a str,
    pub outbox_payload: &'a str,
    pub audit_payload: &'a str,
    pub resolved_binding_digest: &'a str,
    pub result_ref: &'a str,
}

/// Result of accepting or replaying an operation in the M1A reference store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmitOutcome {
    Created { operation_id: String },
    Existing {
        operation_id: String,
        disposition: ReceiptDisposition,
        compacted: bool,
    },
    IdempotencyExpired,
}

/// Hard request budget for a bounded materialized-keyset snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotBudget {
    pub max_items: usize,
    pub max_retained_bytes: usize,
}

/// One bounded page from a durable snapshot keyset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotPage {
    pub projection_watermark: i64,
    pub items: Vec<String>,
    pub next_cursor: Option<i64>,
}

/// Effective SQLite settings that bound noncanonical storage growth in the M1A proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageRuntimeProfile {
    pub journal_mode: String,
    pub synchronous: i64,
    pub wal_autocheckpoint_pages: i64,
    pub journal_size_limit_bytes: i64,
    pub temp_store_memory: bool,
}

/// Errors with stable semantic meaning inside the isolated M1A storage spike.
#[derive(Debug)]
pub enum SpikeError {
    Sqlite(rusqlite::Error),
    StorageFull,
    IdempotencyConflict,
    StaleWriter,
    UnsupportedSchema(i64),
    SnapshotLimitExceeded,
    SnapshotExpired,
    SnapshotNotFound,
    InvariantViolation(&'static str),
}

impl fmt::Display for SpikeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(error) => write!(formatter, "sqlite error: {error}"),
            Self::StorageFull => formatter.write_str("storage capacity exhausted"),
            Self::IdempotencyConflict => formatter.write_str("idempotency conflict"),
            Self::StaleWriter => formatter.write_str("stale writer fence"),
            Self::UnsupportedSchema(version) => {
                write!(formatter, "unsupported storage spike schema version: {version}")
            }
            Self::SnapshotLimitExceeded => formatter.write_str("snapshot limit exceeded"),
            Self::SnapshotExpired => formatter.write_str("snapshot expired"),
            Self::SnapshotNotFound => formatter.write_str("snapshot not found"),
            Self::InvariantViolation(message) => {
                write!(formatter, "storage spike invariant violation: {message}")
            }
        }
    }
}

impl std::error::Error for SpikeError {}

impl From<rusqlite::Error> for SpikeError {
    fn from(error: rusqlite::Error) -> Self {
        if error.sqlite_error_code() == Some(rusqlite::ErrorCode::DiskFull) {
            Self::StorageFull
        } else {
            Self::Sqlite(error)
        }
    }
}

impl From<dxbot_core::DxbotError> for SpikeError {
    fn from(error: dxbot_core::DxbotError) -> Self {
        Self::InvariantViolation(
            Box::leak(
                format!("core domain error in storage spike: {}", error.message).into_boxed_str()
            )
        )
    }
}

/// Artifact counts used to prove the atomic commit boundary in tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationArtifactCounts {
    pub aggregate_state: i64,
    pub event: i64,
    pub outbox: i64,
    pub receipt: i64,
    pub command_binding: i64,
    pub principal_binding: i64,
    pub audit_intent: i64,
}
