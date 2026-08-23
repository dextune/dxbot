//! Acceptance tests for AT-MEMBER-001: application membership, generation CAS,
//! atomic authority binding, and paginated listing over the in-memory
//! [`DomainState`] fixture.
//!
//! `cargo test -p application membership` runs every test below.

use std::sync::{Arc, Mutex};

use application::membership::MembershipManager;
use application::mutation::AppError;
use application::state::DomainState;
use dxbot_core::types::{BotId, BotSelector, ProjectId, ProjectSelector, ScopeSelector};

/// A stable Project scope used by every test.
fn project_scope() -> ScopeSelector {
    ScopeSelector::Project(ProjectSelector::CanonicalId(ProjectId("proj-1".to_owned())))
}

/// A manager over a freshly created in-memory domain state.
fn membership_manager() -> MembershipManager {
    MembershipManager::new(Arc::new(Mutex::new(DomainState::new())))
}

/// A member bot selector for a given id.
fn bot(id: &str) -> BotSelector {
    BotSelector::CanonicalId(BotId(id.to_owned()))
}

#[test]
fn membership_set_creates_binding_and_authority() {
    let mut mgr = membership_manager();
    let scope = project_scope();
    let alice = bot("bot-alice");

    let record = mgr
        .set_membership(&scope, &alice, "admin", None)
        .unwrap();

    // The membership record carries the scope, member, role and a fresh generation.
    assert_eq!(record.role, "admin");
    assert_eq!(record.generation, 1);
    assert_eq!(record.member_bot, alice);
    assert_eq!(record.scope, scope);
    assert_eq!(record.id, record.id); // stable id is present
    assert!(record.created_at > 0);

    // The membership is persisted.
    let snapshot = mgr.snapshot().expect("snapshot must succeed");
    assert_eq!(snapshot.memberships.len(), 1);

    // The authority binding was created atomically alongside the membership.
    assert_eq!(snapshot.authority_bindings.len(), 1);
    let binding = snapshot.authority_bindings.values().next().unwrap();
    assert_eq!(binding.principal, alice);
    assert_eq!(binding.scope, scope);
    assert_eq!(binding.role, "admin");
    assert!(binding.bound_at > 0);
}

#[test]
fn membership_remove_requires_exact_generation() {
    let mut mgr = membership_manager();
    let scope = project_scope();
    let alice = bot("bot-alice");
    mgr.set_membership(&scope, &alice, "member", None).unwrap(); // generation 1

    // Removing with a stale generation is a CAS conflict.
    assert!(matches!(
        mgr.remove_membership(&scope, &alice, 99),
        Err(AppError::Conflict(_))
    ));
    // The membership survives an unsuccessful removal.
    assert_eq!(mgr.snapshot().expect("snapshot must succeed").memberships.len(), 1);

    // Removing with the exact generation succeeds and clears both stores.
    mgr.remove_membership(&scope, &alice, 1).unwrap();
    let snapshot = mgr.snapshot().expect("snapshot must succeed");
    assert!(snapshot.memberships.is_empty());
    assert!(snapshot.authority_bindings.is_empty());

    // A repeated removal now targets an absent membership.
    assert!(matches!(
        mgr.remove_membership(&scope, &alice, 1),
        Err(AppError::NotFound(_))
    ));
}

#[test]
fn membership_list_returns_paginated_members() {
    let mut mgr = membership_manager();
    let scope = project_scope();
    let total = 5;
    for i in 0..total {
        mgr.set_membership(
            &scope,
            &bot(&format!("bot-{i:02}")),
            "member",
            None,
        )
        .unwrap();
    }

    // First page is bounded and signals continuation.
    let page1 = mgr.list_memberships(&scope, 2, None).unwrap();
    assert_eq!(page1.items.len(), 2);
    assert!(page1.has_more);
    assert!(page1.next_cursor.is_some());
    assert_eq!(page1.items[0].member_bot, bot("bot-00"));

    // Cursor continuation resumes exactly after the previous page — no overlap.
    let page2 = mgr.list_memberships(&scope, 2, page1.next_cursor.clone()).unwrap();
    assert_eq!(page2.items.len(), 2);
    assert_eq!(page2.items[0].member_bot, bot("bot-02"));
    assert_eq!(page2.items[1].member_bot, bot("bot-03"));

    // The final page is short and reports exhaustion.
    let page3 = mgr.list_memberships(&scope, 2, page2.next_cursor.clone()).unwrap();
    assert_eq!(page3.items.len(), 1);
    assert_eq!(page3.items[0].member_bot, bot("bot-04"));
    assert!(!page3.has_more);
    assert_eq!(page3.next_cursor, None);

    // Every summary exposes member, role and generation.
    assert_eq!(page3.items[0].role, "member");
    assert_eq!(page3.items[0].generation, 1);
}

#[test]
fn membership_conflict_on_stale_generation() {
    let mut mgr = membership_manager();
    let scope = project_scope();
    let alice = bot("bot-alice");
    mgr.set_membership(&scope, &alice, "member", None).unwrap(); // generation 1

    // A stale generation precondition is a conflict and does not mutate state.
    assert!(matches!(
        mgr.set_membership(&scope, &alice, "admin", Some(0)),
        Err(AppError::Conflict(_))
    ));
    let after_conflict = mgr.snapshot().expect("snapshot must succeed");
    assert_eq!(after_conflict.memberships.len(), 1);
    assert_eq!(after_conflict.memberships.values().next().unwrap().generation, 1);

    // Matching the exact generation allows the update and bumps generation.
    let updated = mgr.set_membership(&scope, &alice, "owner", Some(1)).unwrap();
    assert_eq!(updated.role, "owner");
    assert_eq!(updated.generation, 2);
    assert_eq!(mgr.snapshot().expect("snapshot must succeed").memberships.values().next().unwrap().generation, 2);
}

#[test]
fn membership_authority_is_revoked_on_removal() {
    let mut mgr = membership_manager();
    let scope = project_scope();
    let alice = bot("bot-alice");

    mgr.set_membership(&scope, &alice, "admin", None).unwrap();
    assert_eq!(mgr.snapshot().expect("snapshot must succeed").authority_bindings.len(), 1);

    mgr.remove_membership(&scope, &alice, 1).unwrap();

    // Removal revokes the authority binding in the same operation.
    let snapshot = mgr.snapshot().expect("snapshot must succeed");
    assert!(snapshot.authority_bindings.is_empty());
    assert!(snapshot.memberships.is_empty());
}