//! Acceptance tests for AT-APP-006: bounded page, cursor, all-loop and resync
//! semantics over the in-memory [`DomainState`] fixture.
//!
//! `cargo test -p application query` runs every test below.

use std::sync::{Arc, Mutex};

use application::{
    AllLoopResult, ApplicationQuery, BotSummary, Page, all_loop, resync,
};
use application::state::{BotState, DomainState, LifecycleState};
use dxbot_core::types::BotId;

/// Build a bot state with a lexicographically stable id for cursor ordering.
fn bot_state(id: &str, revision: i64) -> BotState {
    BotState {
        id: BotId(id.to_owned()),
        name: format!("name-{id}"),
        revision,
        lifecycle: LifecycleState::Active,
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

    // The page respects the requested page_size and signals continuation.
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

    // Each page resumes exactly after the previous one — no overlap, no gap.
    assert_eq!(
        page2.items
            .iter()
            .map(|b| b.id.clone())
            .collect::<Vec<_>>(),
        vec![BotId("bot-02".to_owned()), BotId("bot-03".to_owned())]
    );
    // The final page is short and reports exhaustion.
    assert_eq!(page3.items.len(), 1);
    assert_eq!(page3.items[0].id, BotId("bot-04".to_owned()));
    assert!(!page3.has_more);
    assert_eq!(page3.next_cursor, None);
}

#[test]
fn query_all_loop_returns_partial_when_ceiling_reached() {
    let query = query_over(state_with_bots(8));

    // Local ceiling of 3 is hit before all 8 bots are drained -> Partial.
    let result = all_loop(
        |page_size, cursor| query.list_bots(page_size, cursor),
        2,
        3,
    )
    .unwrap();

    match result {
        AllLoopResult::Partial {
            items,
            next_cursor,
        } => {
            assert!(items.len() >= 3, "ceiling reached with partial page");
            assert!(next_cursor.is_some(), "partial exposes a resync cursor");
        }
        AllLoopResult::Complete { .. } => {
            panic!("local ceiling must be reached before the source is exhausted")
        }
    }
}

#[test]
fn query_resync_from_cursor_restarts() {
    let query = query_over(state_with_bots(5));

    // Capture a cursor mid-stream.
    let first: Page<BotSummary> = query.list_bots(2, None).unwrap();
    assert!(first.next_cursor.is_some());

    // Resync restarts from that cursor and drains the rest to completion.
    let result = resync(
        |page_size, cursor| query.list_bots(page_size, cursor),
        first.next_cursor,
        2,
        100,
    )
    .unwrap();

    match result {
        AllLoopResult::Complete { items } => {
            // Remaining bots bot-02, bot-03, bot-04 — no overlap with the
            // items already covered by the earlier page.
            assert_eq!(items.len(), 3);
            assert_eq!(items[0].id, BotId("bot-02".to_owned()));
        }
        AllLoopResult::Partial { .. } => {
            panic!("ceiling high enough that resync should complete")
        }
    }
}