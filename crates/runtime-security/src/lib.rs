#![forbid(unsafe_code)]

//! DXBOT runtime security: Principal, Approval, Authority, pending operation
//! continuation, membership-subject bindings, and durable security state.

pub mod approval;
pub mod authority;
pub mod parking;
pub mod principal;
pub mod sandbox;
pub mod state;

pub use approval::{
    ApprovalBinding, ApprovalDecision, ApprovalDecisionRecord, ApprovalManager, ApprovalRecord,
    ApprovalState,
};
pub use authority::{AuthorityManager, MembershipAuthorityBinding};
pub use parking::{ParkedGateBinding, ParkedGateRecord, ParkedGateState, ParkingManager};
pub use principal::{PrincipalManager, PrincipalState, PrincipalStatus};
pub use sandbox::{
    LocalSubprocessSandbox, MountClass, MountMode, MountSpec, NetworkPolicy, RuntimeArtifact,
    SandboxError, SandboxHandle, SandboxRunResult, SandboxSpec, SandboxState, SandboxTerminal,
};
pub use state::{
    ApprovalDecisionDelta, ApprovalWakeup, MembershipBindingDelta, ParkingDelta,
    SecurityAuditIntent, SecurityDelta, SecurityState, SecurityStateStore,
};

use dxbot_core::types::{ApprovalId, OperationId, PrincipalRef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    DuplicatePrincipal(PrincipalRef),
    UnknownPrincipal(PrincipalRef),
    UnknownApproval(ApprovalId),
    ApprovalAlreadyDecided(ApprovalId),
    InvalidApprover(PrincipalRef),
    InvalidApprovalBinding,
    ApprovalIdSpaceExhausted,
    ApprovalRevisionExhausted(ApprovalId),
    UnknownAuthorityBinding(String),
    StaleAuthorityBinding(String),
    AuditIntentConflict(String),
    InvalidParkedGate(OperationId),
    ParkedGateConflict(OperationId),
    UnknownParkedGate(ApprovalId),
    ParkedGateDenied(ApprovalId),
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "runtime-security error: {self:?}")
    }
}

impl std::error::Error for Error {}
