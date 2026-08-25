#![forbid(unsafe_code)]

//! DXBOT runtime security: Principal, Approval, Authority, pending operation
//! continuation, and Docker-independent sandbox ownership fencing.

pub mod approval;
pub mod authority;
pub mod principal;
pub mod sandbox;

pub use approval::{
    ApprovalDecision, ApprovalDecisionRecord, ApprovalManager, ApprovalRecord, ApprovalState,
};
pub use authority::AuthorityManager;
pub use principal::{PrincipalManager, PrincipalState, PrincipalStatus};
pub use sandbox::{
    LocalSubprocessSandbox, MountClass, MountMode, MountSpec, NetworkPolicy, RuntimeArtifact,
    SandboxError, SandboxHandle, SandboxRunResult, SandboxSpec, SandboxState, SandboxTerminal,
};

use dxbot_core::types::{ApprovalId, PrincipalRef};

/// Error surfaced by the runtime-security managers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A principal was already registered and cannot be re-registered.
    DuplicatePrincipal(PrincipalRef),
    /// A principal could not be resolved.
    UnknownPrincipal(PrincipalRef),
    /// An approval with the given id does not exist.
    UnknownApproval(ApprovalId),
    /// The approval has already been decided (is no longer pending).
    ApprovalAlreadyDecided(ApprovalId),
    /// The casting principal is not a required approver for the approval.
    InvalidApprover(PrincipalRef),
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "runtime-security error: {self:?}")
    }
}

impl std::error::Error for Error {}
