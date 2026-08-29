//! Domain-outcome semantics separated from Operation receipt disposition.

use dxbot_core::types::{CanonicalTarget, CasConditions};

use crate::state::DomainState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainOutcome {
    Created,
    Updated,
    Conflict,
    NotFound,
    PermissionDenied,
}

fn current_revision(state: &DomainState, target: &CanonicalTarget) -> Option<i64> {
    match target {
        CanonicalTarget::Instance(_) => Some(0),
        CanonicalTarget::Bot { id, .. } => state.bots.get(id).map(|row| row.revision),
        CanonicalTarget::Conversation { id, .. } => {
            state.conversations.get(id).map(|row| row.revision)
        }
        CanonicalTarget::Thread { id, .. } => state.threads.get(id).map(|row| row.revision),
        CanonicalTarget::Task { id, .. } => state.tasks.get(id).map(|row| row.revision),
        CanonicalTarget::Project { id, .. } => state.projects.get(id).map(|row| row.revision),
        CanonicalTarget::Channel { id, .. } => state.channels.get(id).map(|row| row.revision),
        CanonicalTarget::Memory { id, .. } => state.memories.get(id).map(|row| row.revision),
        CanonicalTarget::Operation { operation_id, .. } => state
            .results
            .get(operation_id)
            .map(|result| result.receipt.last_progress),
        CanonicalTarget::SideEffect { id, .. } => {
            state.side_effects.get(id).map(|row| row.revision)
        }
        CanonicalTarget::Membership { .. }
        | CanonicalTarget::Approval { .. }
        | CanonicalTarget::Provider { .. }
        | CanonicalTarget::Process { .. } => None,
    }
}

fn expected_revision(target: &CanonicalTarget, cas: &Option<CasConditions>) -> Option<i64> {
    let cas = cas.as_ref()?;
    match target {
        CanonicalTarget::Project { .. } => cas.if_project_revision.or(cas.if_scope_revision).or(cas.if_revision),
        CanonicalTarget::Channel { .. } => cas.if_channel_revision.or(cas.if_scope_revision).or(cas.if_revision),
        CanonicalTarget::Memory { .. } => cas.if_proposal_revision.or(cas.if_revision),
        CanonicalTarget::Operation { .. } => cas.if_receipt_revision,
        CanonicalTarget::Bot { .. }
        | CanonicalTarget::Conversation { .. }
        | CanonicalTarget::Thread { .. }
        | CanonicalTarget::Task { .. }
        | CanonicalTarget::SideEffect { .. } => cas.if_revision,
        _ => None,
    }
}

pub fn resolve_outcome(
    state: &DomainState,
    target: &CanonicalTarget,
    cas: &Option<CasConditions>,
) -> DomainOutcome {
    let current = current_revision(state, target);
    let expected = expected_revision(target, cas);
    match (current, expected) {
        (None, None) => DomainOutcome::Created,
        (None, Some(_)) => DomainOutcome::NotFound,
        (Some(_), None) => DomainOutcome::Updated,
        (Some(current), Some(expected)) if current == expected => DomainOutcome::Updated,
        (Some(_), Some(_)) => DomainOutcome::Conflict,
    }
}

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
