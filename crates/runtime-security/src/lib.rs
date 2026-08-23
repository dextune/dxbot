#![forbid(unsafe_code)]

//! DXBOT runtime security: Principal, Approval, Authority, and pending
//! operation continuation.
//!
//! This crate provides three in-memory managers that own the security state of
//! the local boundary:
//!
//! - [`PrincipalManager`] owns the canonical registry of authenticated peers
//!   and their lifecycle status.
//! - [`ApprovalManager`] owns high-risk operation approvals and the
//!   continuation gate for pending operations.
//! - [`AuthorityManager`] owns principal-to-scope role bindings (least
//!   privilege, deny-unknown).
//!
//! Managers are plain `&mut self` state holders; the caller bundles them (for
//! example in [`SecurityState`]-style composition under an `Arc<Mutex<_>>`)
//! to provide thread safety.

pub mod principal;
pub mod approval;
pub mod authority;

pub use approval::{
    ApprovalDecision, ApprovalDecisionRecord, ApprovalManager, ApprovalRecord, ApprovalState,
};
pub use authority::AuthorityManager;
pub use principal::{PrincipalManager, PrincipalState, PrincipalStatus};

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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "runtime-security error: {self:?}")
    }
}

impl std::error::Error for Error {}