use serde::{Deserialize, Serialize};

/// Core error types for DXBOT. Stable semantic meaning, not raw I/O errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DxbotError {
    pub code: ErrorCode,
    pub category: ErrorCategory,
    pub message: String,
    pub retryable: bool,
    pub operation_ref: Option<String>,
    pub target_refs: Vec<String>,
    pub field_violations: Vec<FieldViolation>,
    pub current_revision: Option<i64>,
    pub current_generation: Option<i64>,
    pub resume_cursor: Option<String>,
    pub next_actions: Vec<TypedNextAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCode {
    Success,
    Usage,
    InvalidInput,
    NotFound,
    AmbiguousTarget,
    Conflict,
    PermissionDenied,
    ApprovalRequired,
    ResourceExhausted,
    RuntimeUnavailable,
    Incompatible,
    Timeout,
    Interrupted,
    PartialOrResync,
    RecoveryRequired,
    ProviderUnavailable,
    StorageOrCorruption,
    InternalInvariant,
}

impl ErrorCode {
    pub fn exit_code(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::Usage => 2,
            Self::InvalidInput => 3,
            Self::NotFound => 4,
            Self::AmbiguousTarget => 5,
            Self::Conflict => 6,
            Self::PermissionDenied => 7,
            Self::ApprovalRequired => 8,
            Self::ResourceExhausted => 9,
            Self::RuntimeUnavailable => 10,
            Self::Incompatible => 11,
            Self::Timeout => 12,
            Self::Interrupted => 13,
            Self::PartialOrResync => 14,
            Self::RecoveryRequired => 15,
            Self::ProviderUnavailable => 16,
            Self::StorageOrCorruption => 17,
            Self::InternalInvariant => 18,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCategory {
    Integrity,
    Internal,
    Recovery,
    Approval,
    Permission,
    Conflict,
    Input,
    Availability,
    Resource,
    Local,
    Success,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldViolation {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedNextAction {
    pub action_code: String,
    pub command_key: String,
    pub args: serde_json::Value,
}