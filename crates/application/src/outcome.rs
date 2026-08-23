//! Domain-outcome semantics.
//!
//! These semantics resolve what would happen to a canonical target given the
//! current in-memory domain state and optional CAS conditions. They are kept
//! separate from the receipt disposition: a receipt records what the operation
//! *did*, a [`DomainOutcome`] records the *reason* a mutation is allowed or
//! rejected at the domain layer.

use dxbot_core::types::{CanonicalTarget, CasConditions};

use crate::state::DomainState;

/// The resolved outcome of attempting a mutation against a canonical target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainOutcome {
    /// The target does not exist yet and can be materialized.
    Created,
    /// The target exists and the CAS conditions are satisfied.
    Updated,
    /// The target currently holds a different revision than the request expects.
    Conflict,
    /// The target does not exist but a precondition requires it to exist.
    NotFound,
    /// The principal is not permitted to mutate the target.
    PermissionDenied,
}

/// The exact object revision a target refers to, if it exists in state.
fn current_revision(current_state: &DomainState, target: &CanonicalTarget) -> Option<i64> {
    match target {
        CanonicalTarget::Instance(_) => Some(0),
        CanonicalTarget::Bot { id, .. } => current_state.bots.get(id).map(|b| b.revision),
        CanonicalTarget::Conversation { id, .. } => {
            current_state.conversations.get(id).map(|c| c.revision)
        }
        CanonicalTarget::Thread { id, .. } => current_state.threads.get(id).map(|t| t.revision),
        CanonicalTarget::Task { id, .. } => current_state.tasks.get(id).map(|t| t.revision),
        // Targets not materialized in the in-memory fixture are treated as absent.
        _ => None,
    }
}

/// Resolve the domain outcome of a mutation attempt without applying it.
///
/// * Absent target without a revision precondition → [`Created`](DomainOutcome::Created).
/// * Absent target with a revision precondition → [`NotFound`](DomainOutcome::NotFound).
/// * Present target whose revision matches (or has no) precondition → [`Updated`](DomainOutcome::Updated).
/// * Present target whose revision differs from the precondition → [`Conflict`](DomainOutcome::Conflict).
pub fn resolve_outcome(
    current_state: &DomainState,
    target: &CanonicalTarget,
    cas: &Option<CasConditions>,
) -> DomainOutcome {
    let current = current_revision(current_state, target);
    let expected = cas.as_ref().and_then(|c| c.if_revision);
    match (current, expected) {
        (None, None) => DomainOutcome::Created,
        (None, Some(_)) => DomainOutcome::NotFound,
        (Some(_), None) => DomainOutcome::Updated,
        (Some(cur), Some(want)) if cur == want => DomainOutcome::Updated,
        (Some(_), Some(_)) => DomainOutcome::Conflict,
    }
}

/// Build [`CasConditions`] expressing a single `if_revision` precondition.
///
/// Convenience for tests and callers that only care about the revision guard.
pub fn cas_if_revision(revision: i64) -> CasConditions {
    CasConditions {
        if_revision: Some(revision),
        if_generation: None,
        if_host_generation: None,
        if_execution_generation: None,
        if_source_revision: None,
        if_scope_revision: None,
        if_project_revision: None,
        if_channel_revision: None,
        if_membership_generation: None,
        if_proposal_revision: None,
        if_target_scope_revision: None,
        if_receipt_revision: None,
    }
}
