use serde::{Deserialize, Serialize};

/// An operation receipt disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReceiptDisposition {
    Accepted,
    Committed,
    Rejected,
    Superseded,
    RecoveryRequired,
}

impl ReceiptDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Committed => "committed",
            Self::Rejected => "rejected",
            Self::Superseded => "superseded",
            Self::RecoveryRequired => "recovery-required",
        }
    }

    pub fn parse(value: &str) -> Result<Self, crate::DxbotError> {
        match value {
            "accepted" => Ok(Self::Accepted),
            "committed" => Ok(Self::Committed),
            "rejected" => Ok(Self::Rejected),
            "superseded" => Ok(Self::Superseded),
            "recovery-required" => Ok(Self::RecoveryRequired),
            _ => Err(crate::DxbotError {
                code: crate::error::ErrorCode::InvalidInput,
                category: crate::error::ErrorCategory::Input,
                message: format!("unknown receipt disposition: {value}"),
                retryable: false,
                operation_ref: None,
                target_refs: vec![],
                field_violations: vec![],
                current_revision: None,
                current_generation: None,
                resume_cursor: None,
                next_actions: vec![],
            }),
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Committed | Self::Rejected | Self::Superseded)
    }
}

/// A full receipt record returned from the control endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptRecord {
    pub operation_id: String,
    pub disposition: ReceiptDisposition,
    pub result_ref: String,
    pub resolved_binding_digest: String,
    pub owner_kind: String,
    pub lease_until: Option<i64>,
    pub last_progress: i64,
    pub reconciliation_policy: String,
}