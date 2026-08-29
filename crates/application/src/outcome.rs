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
        CanonicalTarget::Project { .. } => cas
            .if_project_revision
            .or(cas.if_scope_revision)
            .or(cas.if_revision),
        CanonicalTarget::Channel { .. } => cas
            .if_channel_revision
            .or(cas.if_scope_revision)
            .or(cas.if_revision),
        CanonicalTarget::Thread { .. } => cas.if_source_revision.or(cas.if_revision),
        CanonicalTarget::Memory { .. } => cas.if_proposal_revision.or(cas.if_revision),
        CanonicalTarget::Operation { .. } => cas.if_receipt_revision,
        CanonicalTarget::Bot { .. }
        | CanonicalTarget::Conversation { .. }
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

#[cfg(test)]
mod tests {
    use dxbot_core::types::{ThreadId, ThreadSelector};

    use super::*;
    use crate::state::ThreadState;

    #[test]
    fn thread_branch_prefers_source_revision_cas() {
        let mut state = DomainState::new();
        let id = ThreadId("thread-a".to_owned());
        state.threads.insert(
            id.clone(),
            ThreadState {
                id: id.clone(),
                conversation_id: dxbot_core::types::ConversationId("conversation-a".to_owned()),
                revision: 7,
                parent_message_id: None,
                title: "a".to_owned(),
                messages: Vec::new(),
            },
        );
        let target = CanonicalTarget::Thread {
            id,
            parent_id: None,
            revision: 7,
        };
        let mut cas = cas_if_revision(99);
        cas.if_source_revision = Some(7);
        assert_eq!(resolve_outcome(&state, &target, &Some(cas)), DomainOutcome::Updated);
        let _ = ThreadSelector::CanonicalId(ThreadId("unused".to_owned()));
    }
}
