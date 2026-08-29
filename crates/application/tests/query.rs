#![allow(clippy::unwrap_used)]
//! Acceptance tests for AT-APP-006: bounded page, cursor, all-loop and resync
//! semantics over the in-memory [`DomainState`] fixture.
//!
//! `cargo test -p application query` runs every test below.

use std::sync::{Arc, Mutex};

use application::state::{BotState, DomainState, LifecycleState};
use application::{AllLoopResult, AppError, ApplicationQuery, BotSummary, Page, all_loop, resync};
use dxbot_core::types::BotId;

/// Build a bot state with a lexicographically stable id for cursor ordering.
fn bot_state(id: &str, revision: i64) -> BotState {
    BotState {
        id: BotId(id.to_owned()),
        name: format!("name-{id}"),
        revision,
        lifecycle: LifecycleState::Active,
        policy_bindings: Default::default(),
    }
}

/// A domain state seeded with `count` bots `bot-00 .. bot-{count-1}`.
fn state_with_bots(count: usize) -> DomainState {
    let mut state = DomainState::new();
    for i in 0..count {
        let id = format!("bot-{i:02}");
        state.bots.insert(BotId(id.clone()), bot_state(&id, 1));
    }
    state
}

/// A query over a freshly seeded domain state.
fn query_over(state: DomainState) -> ApplicationQuery {
    ApplicationQuery::new(Arc::new(Mutex::new(state)))
}

#[test]
fn query_pagination_returns_bounded_page() {
    let query = query_over(state_with_bots(5));

    let page: Page<BotSummary> = query.list_bots(2, None).unwrap();

    assert_eq!(page.items.len(), 2);
    assert!(page.has_more);
    assert!(page.next_cursor.is_some());
    assert_eq!(page.items[0].id, BotId("bot-00".to_owned()));
    assert_eq!(page.items[1].id, BotId("bot-01".to_owned()));
}

#[test]
fn query_pagination_cursor_continues_from_previous() {
    let query = query_over(state_with_bots(5));

    let page1: Page<BotSummary> = query.list_bots(2, None).unwrap();
    let page2: Page<BotSummary> = query.list_bots(2, page1.next_cursor.clone()).unwrap();
    let page3: Page<BotSummary> = query.list_bots(2, page2.next_cursor.clone()).unwrap();

    assert_eq!(
        page2
            .items
            .iter()
            .map(|bot| bot.id.clone())
            .collect::<Vec<_>>(),
        vec![BotId("bot-02".to_owned()), BotId("bot-03".to_owned())]
    );
    assert_eq!(page3.items.len(), 1);
    assert_eq!(page3.items[0].id, BotId("bot-04".to_owned()));
    assert!(!page3.has_more);
    assert_eq!(page3.next_cursor, None);
}

#[test]
fn query_all_loop_never_exceeds_local_ceiling() {
    let query = query_over(state_with_bots(8));

    let result = all_loop(|page_size, cursor| query.list_bots(page_size, cursor), 2, 3).unwrap();

    match result {
        AllLoopResult::Partial { items, next_cursor } => {
            assert_eq!(items.len(), 3, "local ceiling is a hard upper bound");
            assert!(next_cursor.is_some(), "partial exposes a resume cursor");
        }
        AllLoopResult::Complete { .. } => {
            panic!("local ceiling must be reached before the source is exhausted")
        }
    }
}

#[test]
fn query_exact_ceiling_is_complete_when_source_is_exhausted() {
    let query = query_over(state_with_bots(4));

    let result = all_loop(|page_size, cursor| query.list_bots(page_size, cursor), 3, 4).unwrap();

    match result {
        AllLoopResult::Complete { items } => assert_eq!(items.len(), 4),
        AllLoopResult::Partial { .. } => {
            panic!("exhausting the source exactly at the ceiling is complete")
        }
    }
}

#[test]
fn query_all_loop_rejects_fetch_that_breaks_requested_bound() {
    let error = all_loop::<usize>(
        |requested, _| {
            Ok(Page {
                items: vec![0; requested + 1],
                next_cursor: Some("next".to_owned()),
                has_more: true,
            })
        },
        4,
        1,
    )
    .unwrap_err();

    assert!(matches!(error, AppError::Internal(_)));
}

#[test]
fn query_all_loop_rejects_non_progressing_page() {
    let error = all_loop::<usize>(
        |_, _| {
            Ok(Page {
                items: Vec::new(),
                next_cursor: Some("next".to_owned()),
                has_more: true,
            })
        },
        2,
        5,
    )
    .unwrap_err();

    assert!(matches!(error, AppError::Internal(_)));
}

#[test]
fn query_resync_from_cursor_restarts() {
    let query = query_over(state_with_bots(5));

    let first: Page<BotSummary> = query.list_bots(2, None).unwrap();
    assert!(first.next_cursor.is_some());

    let result = resync(
        |page_size, cursor| query.list_bots(page_size, cursor),
        first.next_cursor,
        2,
        100,
    )
    .unwrap();

    match result {
        AllLoopResult::Complete { items } => {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0].id, BotId("bot-02".to_owned()));
        }
        AllLoopResult::Partial { .. } => {
            panic!("ceiling high enough that resync should complete")
        }
    }
}
