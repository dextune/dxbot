//! Acceptance tests for AT-DELEGATE-001: typed multi-bot delegation with
//! membership and scope-boundary enforcement.
//!
//! `cargo test -p application delegation` runs every test below.

use std::sync::{Arc, Mutex};

use application::delegation::{
    DelegationManager, DelegationStatus, DelegationSummary,
};
use application::membership::MembershipManager;
use application::mutation::AppError;
use application::query::Page;
use application::state::DomainState;
use dxbot_core::types::{
    BotId, BotSelector, ProjectId, ProjectSelector, ScopeSelector, TaskId,
};

/// A stable Project scope used by every test.
fn project_scope(id: &str) -> ScopeSelector {
    ScopeSelector::Project(ProjectSelector::CanonicalId(ProjectId(id.to_owned())))
}

/// A bot selector for a given id.
fn bot(id: &str) -> BotSelector {
    BotSelector::CanonicalId(BotId(id.to_owned()))
}

/// Builds state where `manager-1` (role `manager`) and `worker-1` (role
/// `member`) are members of `project-a`, plus a `DelegationManager` over it.
fn delegation_manager() -> (DelegationManager, ScopeSelector, BotSelector, BotSelector) {
    let state = Arc::new(Mutex::new(DomainState::new()));
    let mut members = MembershipManager::new(state.clone());
    let scope = project_scope("project-a");
    let manager = bot("manager-1");
    let worker = bot("worker-1");
    members
        .set_membership(&scope, &manager, "manager", None)
        .unwrap();
    members
        .set_membership(&scope, &worker, "member", None)
        .unwrap();
    let dm = DelegationManager::new(state);
    (dm, scope, manager, worker)
}

#[test]
fn delegation_creates_typed_delegation_between_members() {
    let (mut dm, scope, manager, worker) = delegation_manager();

    let record = dm
        .delegate_task(&TaskId("task-1".to_owned()), &manager, &worker, &scope)
        .unwrap();

    // The record is fully typed and starts pending.
    assert_eq!(record.task_id, TaskId("task-1".to_owned()));
    assert_eq!(record.from_bot, manager);
    assert_eq!(record.to_bot, worker);
    assert_eq!(record.scope, scope);
    assert_eq!(record.role, "manager");
    assert_eq!(record.status, DelegationStatus::Pending);
    assert!(record.created_at > 0);
    assert!(record.resolved_at.is_none());
    assert!(!record.id.is_empty());

    // The delegation is durable in state and listable within the scope.
    let fetched = dm.get_delegation(&record.id).unwrap();
    assert_eq!(fetched, record);

    let page: Page<DelegationSummary> = dm.list_delegations(&scope, 10, None).unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].task_id, record.task_id);
    assert_eq!(page.items[0].status, DelegationStatus::Pending);
    assert!(!page.has_more);
}

#[test]
fn delegation_requires_membership_in_scope() {
    let (mut dm, scope, _, worker) = delegation_manager();
    // An outsider who is not a member of the scope cannot delegate within it.
    let outsider = bot("outsider");

    let err = dm
        .delegate_task(&TaskId("task-1".to_owned()), &outsider, &worker, &scope)
        .unwrap_err();
    assert!(matches!(err, AppError::PermissionDenied(_)));

    // The target bot must also be a member of the scope.
    let err = dm
        .delegate_task(
            &TaskId("task-1".to_owned()),
            &outsider,
            &outsider,
            &scope,
        )
        .unwrap_err();
    assert!(matches!(err, AppError::PermissionDenied(_)));
}

#[test]
fn delegation_only_target_can_accept() {
    let (mut dm, scope, manager, worker) = delegation_manager();
    let record = dm
        .delegate_task(&TaskId("task-1".to_owned()), &manager, &worker, &scope)
        .unwrap();

    // A bot that is not the target cannot accept.
    let intruder = bot("intruder");
    let err = dm.accept_delegation(&record.id, &intruder).unwrap_err();
    assert!(matches!(err, AppError::PermissionDenied(_)));

    // Even the delegating bot cannot accept on behalf of the target.
    let err = dm.accept_delegation(&record.id, &manager).unwrap_err();
    assert!(matches!(err, AppError::PermissionDenied(_)));

    // The record remains pending after the failed attempts.
    assert_eq!(dm.get_delegation(&record.id).unwrap().status, DelegationStatus::Pending);

    // The target bot can accept.
    let accepted = dm.accept_delegation(&record.id, &worker).unwrap();
    assert_eq!(accepted.status, DelegationStatus::Accepted);
    assert!(accepted.resolved_at.is_some());
}

#[test]
fn delegation_only_target_can_reject() {
    let (mut dm, scope, manager, worker) = delegation_manager();
    let record = dm
        .delegate_task(&TaskId("task-1".to_owned()), &manager, &worker, &scope)
        .unwrap();

    // A non-target bot cannot reject.
    let err = dm.reject_delegation(&record.id, &manager).unwrap_err();
    assert!(matches!(err, AppError::PermissionDenied(_)));

    // Still pending after the failed reject attempt.
    assert_eq!(dm.get_delegation(&record.id).unwrap().status, DelegationStatus::Pending);

    // The target bot can reject.
    let rejected = dm.reject_delegation(&record.id, &worker).unwrap();
    assert_eq!(rejected.status, DelegationStatus::Rejected);
    assert!(rejected.resolved_at.is_some());
}

#[test]
fn delegation_scope_boundary_is_enforced() {
    let (mut dm, _scope_a, manager, worker) = delegation_manager();

    // Both bots are members of `project-a`, so a delegation scoped to a
    // *different* project (`project-b`) crosses the scope boundary and is
    // rejected — a delegator cannot reach outside its membership scope.
    let scope_b = project_scope("project-b");

    let err = dm
        .delegate_task(&TaskId("task-1".to_owned()), &manager, &worker, &scope_b)
        .unwrap_err();
    assert!(matches!(err, AppError::PermissionDenied(_)));

    // No cross-scope delegation was recorded.
    let page: Page<DelegationSummary> = dm.list_delegations(&scope_b, 10, None).unwrap();
    assert!(page.items.is_empty());
}