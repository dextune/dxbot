//! Acceptance coverage for `AT-CLI-010`: live directive controls.
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use cli::directive::{DirectiveController, DirectiveError, WaitResult};
use dxbot_core::types::{CanonicalTarget, ContentSource};

const POLL_GRACE_MILLIS: u64 = 5_000;

/// All directive payloads share a Task canonical target with the requested CAS.
fn assert_task_target(
    target: &CanonicalTarget,
    expected_id: &str,
    expected_revision: Option<i64>,
    expected_generation: Option<i64>,
) {
    match target {
        CanonicalTarget::Task {
            id,
            revision,
            execution_generation,
        } => {
            assert_eq!(id.0, expected_id);
            assert_eq!(Some(*revision), expected_revision);
            assert_eq!(*execution_generation, expected_generation);
        }
        other => panic!("expected Task target, got {other:?}"),
    }
}

#[test]
fn directive_cancel_creates_valid_payload() {
    let controller = DirectiveController::new();
    let payload = controller
        .cancel_task("task:alpha-13", Some(7), Some(3), Some("duplicate"))
        .unwrap();
    assert_eq!(payload.command_key, "task-cancel");
    assert!(payload.content.is_none());
    assert_task_target(&payload.canonical_target, "alpha-13", Some(7), Some(3));
    let cas = payload.cas.unwrap();
    assert_eq!(cas.if_revision, Some(7));
    assert_eq!(cas.if_execution_generation, Some(3));
    assert_eq!(payload.semantic_options["task"], "task:alpha-13");
    assert_eq!(payload.semantic_options["reason"], "duplicate");
}

#[test]
fn directive_suspend_creates_valid_payload() {
    let controller = DirectiveController::new();
    let payload = controller
        .suspend_task("task:beta-9", Some(2), Some(1), Some("waiting-safe-point"))
        .unwrap();
    assert_eq!(payload.command_key, "task-suspend");
    assert!(payload.content.is_none());
    assert_task_target(&payload.canonical_target, "beta-9", Some(2), Some(1));
    let cas = payload.cas.unwrap();
    assert_eq!(cas.if_revision, Some(2));
    assert_eq!(cas.if_execution_generation, Some(1));
    assert_eq!(payload.semantic_options["reason"], "waiting-safe-point");
}

#[test]
fn directive_resume_creates_valid_payload() {
    let controller = DirectiveController::new();
    let payload = controller.resume_task("task:beta-9", Some(2)).unwrap();
    assert_eq!(payload.command_key, "task-resume");
    assert!(payload.content.is_none());
    // Resume targets revision only; no execution-generation CAS is carried.
    assert_task_target(&payload.canonical_target, "beta-9", Some(2), None);
    let cas = payload.cas.unwrap();
    assert_eq!(cas.if_revision, Some(2));
    assert_eq!(cas.if_execution_generation, None);
}

#[test]
fn directive_wait_for_applied_returns_on_observation() {
    let controller = DirectiveController::new();
    let observer = controller.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(30));
        observer.observe_applied(12345);
    });
    let result = controller
        .wait_for_applied(Duration::from_millis(POLL_GRACE_MILLIS))
        .unwrap();
    match result {
        WaitResult::Applied { at } => assert_eq!(at, 12345),
        other => panic!("expected Applied, got {other:?}"),
    }
}

#[test]
fn directive_timeout_does_not_implicitly_cancel() {
    let controller = DirectiveController::new();
    // No observation is ever recorded. wait_for_applied must time out locally
    // and MUST NOT issue an implicit cancel as a side effect.
    let result = controller
        .wait_for_applied(Duration::from_millis(40))
        .unwrap();
    match result {
        WaitResult::Timeout { elapsed } => assert!(elapsed.as_millis() >= 1),
        WaitResult::Applied { .. } => panic!("expected Timeout, got Applied"),
        WaitResult::Interrupted => panic!("expected Timeout, got Interrupted"),
    }
    // The wait is observation-only: a second wait also times out rather than
    // exiting early with an implicit-cancelled state.
    let again = controller
        .wait_for_applied(Duration::from_millis(40))
        .unwrap();
    assert!(matches!(again, WaitResult::Timeout { .. }));
}

#[test]
fn directive_redirect_requires_replacement_content() {
    let controller = DirectiveController::new();
    // Empty text replacement is rejected: a redirect must carry content.
    let empty = controller.redirect_task(
        "task:alpha-13",
        Some(7),
        &ContentSource::Text {
            value: String::new(),
        },
    );
    assert_eq!(empty, Err(DirectiveError::ReplacementRequired));

    // Non-empty replacement projects a valid redirect payload.
    let ok = controller.redirect_task(
        "task:alpha-13",
        Some(7),
        &ContentSource::Text {
            value: "new specification".to_string(),
        },
    );
    let payload = ok.unwrap();
    assert_eq!(payload.command_key, "task-redirect");
    assert_eq!(
        payload.content,
        Some(ContentSource::Text {
            value: "new specification".to_string(),
        })
    );
    assert_task_target(&payload.canonical_target, "alpha-13", Some(7), None);
}
