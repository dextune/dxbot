use std::fmt;

/// An operation receipt disposition used by the executable M1A storage proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptDisposition {
    Accepted,
    Committed,
    Rejected,
    Superseded,
    RecoveryRequired,
}

impl ReceiptDisposition {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Committed => "committed",
            Self::Rejected => "rejected",
            Self::Superseded => "superseded",
            Self::RecoveryRequired => "recovery-required",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, SpikeError> {
        match value {
            "accepted" => Ok(Self::Accepted),
            "committed" => Ok(Self::Committed),
            "rejected" => Ok(Self::Rejected),
            "superseded" => Ok(Self::Superseded),
            "recovery-required" => Ok(Self::RecoveryRequired),
            _ => Err(SpikeError::InvariantViolation("unknown receipt disposition")),
        }
    }

    pub(crate) const fn is_terminal(self) -> bool {
        matches!(self, Self::Committed | Self::Rejected | Self::Superseded)
    }
}

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

/// Errors with stable semantic meaning inside the isolated M1A storage spike.
#[derive(Debug)]
pub enum SpikeError {
    Sqlite(rusqlite::Error),
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
        Self::Sqlite(error)
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
