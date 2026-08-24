//! Acceptance tests for application membership ownership and generation CAS.

#![allow(clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use application::membership::MembershipManager;
use application::mutation::AppError;
use application::state::DomainState;
use dxbot_core::types::{BotId, BotSelector, ProjectId, ProjectSelector, ScopeSelector};

fn project_scope() -> ScopeSelector {
    ScopeSelector::Project(ProjectSelector::CanonicalId(ProjectId("proj-1".to_owned())))
}
fn membership_manager() -> MembershipManager {
    MembershipManager::new(Arc::new(Mutex::new(DomainState::new())))
}
fn bot(id: &str) -> BotSelector {
    BotSelector::CanonicalId(BotId(id.to_owned()))
}

#[test]
fn membership_set_creates_membership_fact_only() {
    let mut manager = membership_manager();
    let scope = project_scope();
    let alice = bot("bot-alice");
    let record = manager
        .set_membership(&scope, &alice, "admin", None)
        .unwrap();
    assert_eq!(record.role, "admin");
    assert_eq!(record.generation, 1);
    let snapshot = manager.snapshot().unwrap();
    assert_eq!(snapshot.memberships.len(), 1);
}

#[test]
fn membership_remove_requires_exact_generation() {
    let mut manager = membership_manager();
    let scope = project_scope();
    let alice = bot("bot-alice");
    manager
        .set_membership(&scope, &alice, "member", None)
        .unwrap();
    assert!(matches!(
        manager.remove_membership(&scope, &alice, 99),
        Err(AppError::Conflict(_))
    ));
    manager.remove_membership(&scope, &alice, 1).unwrap();
    assert!(manager.snapshot().unwrap().memberships.is_empty());
}

#[test]
fn membership_list_returns_paginated_members() {
    let mut manager = membership_manager();
    let scope = project_scope();
    for index in 0..5 {
        manager
            .set_membership(
                &scope,
                &bot(&format!("bot-{index:02}")),
                "member",
                None,
            )
            .unwrap();
    }
    let page1 = manager.list_memberships(&scope, 2, None).unwrap();
    let page2 = manager
        .list_memberships(&scope, 2, page1.next_cursor.clone())
        .unwrap();
    let page3 = manager
        .list_memberships(&scope, 2, page2.next_cursor.clone())
        .unwrap();
    assert_eq!(page1.items.len(), 2);
    assert_eq!(page2.items[0].member_bot, bot("bot-02"));
    assert_eq!(page3.items[0].member_bot, bot("bot-04"));
    assert!(!page3.has_more);
}

#[test]
fn membership_conflict_on_stale_generation() {
    let mut manager = membership_manager();
    let scope = project_scope();
    let alice = bot("bot-alice");
    manager
        .set_membership(&scope, &alice, "member", None)
        .unwrap();
    assert!(matches!(
        manager.set_membership(&scope, &alice, "admin", Some(0)),
        Err(AppError::Conflict(_))
    ));
    let updated = manager
        .set_membership(&scope, &alice, "owner", Some(1))
        .unwrap();
    assert_eq!(updated.generation, 2);
}

#[test]
fn delegation_role_matching_is_exact_not_substring_based() {
    assert!(MembershipManager::can_delegate("admin"));
    assert!(MembershipManager::can_delegate(" Coordinator "));
    assert!(!MembershipManager::can_delegate("not-admin"));
    assert!(!MembershipManager::can_delegate("unowned"));
}

#[test]
fn membership_keys_do_not_collide_on_delimiters() {
    let mut manager = membership_manager();
    let scope_a = ScopeSelector::Project(ProjectSelector::VisibleExact(
        "x|exact:y".to_owned(),
    ));
    let scope_b = ScopeSelector::Project(ProjectSelector::VisibleExact("x".to_owned()));
    let bot_a = BotSelector::ScopedExact("z".to_owned());
    let bot_b = BotSelector::ScopedExact("y|exact:z".to_owned());

    manager
        .set_membership(&scope_a, &bot_a, "member", None)
        .unwrap();
    manager
        .set_membership(&scope_b, &bot_b, "member", None)
        .unwrap();
    assert_eq!(manager.snapshot().unwrap().memberships.len(), 2);
}