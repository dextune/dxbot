//! Acceptance coverage for `AT-CLI-009`: exact, ambiguity-safe selector
//! resolution. A fuzzy match is a help suggestion only and is never a mutation
//! target.
#![allow(clippy::unwrap_used)]

use cli::selector::{
    BotSummary, ConversationSummary, Error, SelectorResolver, TaskSummary, ThreadSummary,
};
use dxbot_core::types::{BotId, ConversationId, TaskId, ThreadId};

fn resolver() -> SelectorResolver {
    SelectorResolver::new()
}

fn bot(id: &str, name: &str) -> BotSummary {
    BotSummary {
        id: BotId(format!("bot:{id}")),
        name: name.to_string(),
    }
}

fn thread(id: &str, name: &str) -> ThreadSummary {
    ThreadSummary {
        id: ThreadId(format!("thread:{id}")),
        name: name.to_string(),
    }
}

fn task(id: &str, name: &str) -> TaskSummary {
    TaskSummary {
        id: TaskId(format!("task:{id}")),
        name: name.to_string(),
    }
}

fn conversation(id: &str, name: &str) -> ConversationSummary {
    ConversationSummary {
        id: ConversationId(format!("conv:{id}")),
        name: name.to_string(),
        parent_bot: None,
    }
}

#[test]
fn selector_exact_canonical_id_resolves_immediately() {
    let r = resolver();
    let candidates = vec![bot("c-a", "alice"), bot("c-b", "bob")];

    // Canonical ID wins even when a different resource shares the scoped name.
    match r.resolve_bot("bot:c-a", &candidates).unwrap() {
        dxbot_core::types::BotSelector::CanonicalId(id) => {
            assert_eq!(id, BotId("bot:c-a".to_string()))
        }
        other => panic!("expected CanonicalId, got {other:?}"),
    }

    // resolve_target for threads/tasks identical semantics.
    match r
        .resolve_thread("thread:tx", &[thread("tx", "main")])
        .unwrap()
    {
        dxbot_core::types::ThreadSelector::CanonicalId(id) => {
            assert_eq!(id, ThreadId("thread:tx".to_string()))
        }
        other => panic!("expected ThreadSelector::CanonicalId, got {other:?}"),
    }
    match r.resolve_task("task:tk", &[task("tk", "do")]).unwrap() {
        dxbot_core::types::TaskSelector::CanonicalId(id) => {
            assert_eq!(id, TaskId("task:tk".to_string()))
        }
        other => panic!("expected TaskSelector::CanonicalId, got {other:?}"),
    }
}

#[test]
fn selector_single_scoped_match_resolves() {
    let r = resolver();
    let candidates = vec![bot("c-a", "alice"), bot("c-b", "bob"), bot("c-c", "carol")];

    match r.resolve_bot("bob", &candidates).unwrap() {
        dxbot_core::types::BotSelector::ScopedExact(name) => {
            assert_eq!(name, "bob");
        }
        other => panic!("expected ScopedExact, got {other:?}"),
    }

    let conversations = vec![
        conversation("1", "project-x/main"),
        conversation("2", "project-y/main"),
    ];
    match r
        .resolve_conversation("project-y/main", &conversations)
        .unwrap()
    {
        dxbot_core::types::ConversationSelector::ConversationId(id) => {
            assert_eq!(id, ConversationId("conv:2".to_string()))
        }
        other => panic!("expected ConversationSelector::ConversationId, got {other:?}"),
    }
}

#[test]
fn selector_multiple_matches_returns_ambiguous_with_candidates() {
    let r = resolver();
    // Two distinct resources share the same scoped name "shared".
    let candidates = vec![bot("bx-1", "shared"), bot("bx-2", "shared")];

    match r.resolve_bot("shared", &candidates) {
        Err(Error::Ambiguous { candidates }) => {
            // Visible canonical refs of every matched candidate, so the user
            // can disambiguate (never auto-selected).
            assert_eq!(
                candidates,
                vec!["bot:bx-1".to_string(), "bot:bx-2".to_string()]
            );
        }
        other => panic!("expected Ambiguous, got {other:?}"),
    }

    // Ambiguity projects to the canonical ambiguous-target surface (exit 5).
    let dx = Error::Ambiguous {
        candidates: vec!["bot:bx-1".to_string(), "bot:bx-2".to_string()],
    }
    .to_dxbot_error();
    assert_eq!(dx.code, dxbot_core::error::ErrorCode::AmbiguousTarget);
    assert_eq!(dx.code.exit_code(), 5);
    assert_eq!(dx.target_refs.len(), 2);
}

#[test]
fn selector_zero_matches_returns_not_found() {
    let r = resolver();
    let candidates = vec![bot("c-a", "alice"), bot("c-b", "bob")];

    match r.resolve_bot("nobody-here", &candidates) {
        Err(Error::NotFound) => {}
        other => panic!("expected NotFound, got {other:?}"),
    }
    // Empty candidate list is also NotFound, never a false resolve.
    match r.resolve_bot("alice", &[]) {
        Err(Error::NotFound) => {}
        other => panic!("expected NotFound for empty candidates, got {other:?}"),
    }
}

#[test]
fn selector_fuzzy_match_is_suggestion_only_not_mutation_target() {
    let r = resolver();
    let candidates = vec![bot("c-a", "bot-alice"), bot("c-b", "bot-bob")];

    // The misspelled input does NOT resolve (no mutation is executed).
    match r.resolve_bot("bot-alix", &candidates) {
        Err(Error::NotFound) => {}
        other => panic!("fuzzy input must not resolve; got {other:?}"),
    }

    // But it is surfaced as a distance-thresholded help suggestion.
    let suggestions = r.suggest_candidates(
        "bot-alix",
        &["bot-alice".to_string(), "bot-bob".to_string()],
    );
    assert!(
        suggestions.contains(&"bot-alice".to_string()),
        "expected fuzzy suggestion, got {suggestions:?}"
    );

    // Exact inputs are not re-suggested (they resolve, not suggest).
    let exact_suggestions = r.suggest_candidates("bot-alice", &["bot-alice".to_string()]);
    assert!(exact_suggestions.is_empty());
}
