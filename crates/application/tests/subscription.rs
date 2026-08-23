//! Acceptance tests for AT-APP-007: cursor-based application subscription.
//!
//! Covered in-process with the in-memory [`DomainState`] fixture (no external
//! I/O): event creation/receipt, cursor resume, reconnect-without-gap, gap
//! detection with resync hint, explicit resync from current state, and bounded
//! buffer pruning.
//!
//! `cargo test -p application subscription` runs every test below.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use application::state::DomainState;
use application::subscription::{StreamEvent, SubscriptionManager, TaskResult};
use application::AppError;
use dxbot_core::types::{ProcessId, TaskId};

/// A short poll window sufficient for drained-stream Timeout signalling.
const POLL: Duration = Duration::from_millis(150);

/// A manager over a fresh, empty domain state.
fn manager_over(state: DomainState) -> SubscriptionManager {
    SubscriptionManager::new(Arc::new(Mutex::new(state)))
}

#[test]
fn subscription_creates_and_receives_events() {
    let mut manager = manager_over(DomainState::new());
    let task_id = TaskId("task-1".to_owned());

    let sub = manager.subscribe_task(&task_id, None).unwrap();
    assert_eq!(sub.target_id, "task-1");
    assert_eq!(sub.cursor, None);

    // Emit events after subscribing; the subscription receives them in order.
    manager
        .emit_task(
            &task_id,
            StreamEvent::Progress {
                message: "starting".into(),
                percent: Some(0),
            },
        )
        .unwrap();
    manager
        .emit_task(
            &task_id,
            StreamEvent::Output {
                data: "hello".into(),
            },
        )
        .unwrap();

    assert_eq!(
        manager.next_event(&sub, POLL).unwrap(),
        StreamEvent::Progress {
            message: "starting".into(),
            percent: Some(0),
        }
    );
    assert_eq!(
        manager.next_event(&sub, POLL).unwrap(),
        StreamEvent::Output { data: "hello".into() }
    );
    assert_eq!(manager.last_delivered_cursor(&sub), Some("2".into()));
}

#[test]
fn subscription_cursor_resumes_from_last_position() {
    let mut manager = manager_over(DomainState::new());
    let task_id = TaskId("task-1".to_owned());

    // Seed three events so cursors 1, 2, 3 exist.
    for n in 1..=3 {
        manager
            .emit_task(&task_id, StreamEvent::Output { data: format!("m{n}") })
            .unwrap();
    }

    // A new subscriber resuming from cursor "2" only sees events after it,
    // i.e. cursor 3 — never a replay of 1 or 2.
    let sub = manager.subscribe_task(&task_id, Some("2".into())).unwrap();
    assert_eq!(sub.cursor, Some("2".into()));
    assert_eq!(
        manager.next_event(&sub, POLL).unwrap(),
        StreamEvent::Output { data: "m3".into() }
    );
    // The resumed subscription is now caught up.
    assert!(matches!(
        manager.next_event(&sub, POLL),
        Err(AppError::Timeout(_))
    ));
}

#[test]
fn subscription_reconnect_with_cursor_no_gap() {
    let mut manager = manager_over(DomainState::new());
    let task_id = TaskId("task-1".to_owned());

    let sub = manager.subscribe_task(&task_id, None).unwrap();

    // Emit five events and consume the first two (delivered cursor -> 2).
    for n in 1..=5 {
        manager
            .emit_task(&task_id, StreamEvent::Output { data: format!("m{n}") })
            .unwrap();
    }
    for _ in 0..2 {
        manager.next_event(&sub, POLL).unwrap();
    }

    // More events arrive after a "disconnect".
    manager
        .emit_task(
            &task_id,
            StreamEvent::StatusChange {
                from: "running".into(),
                to: "succeeded".into(),
            },
        )
        .unwrap();

    // Reconnect from the last delivered cursor: delivers 3, 4, 5 then the
    // new event — no gap, no replay of 1 or 2.
    let resumed = manager.reconnect(&sub.id, "2").unwrap();
    assert_eq!(resumed.cursor, Some("2".into()));

    assert_eq!(
        manager.next_event(&resumed, POLL).unwrap(),
        StreamEvent::Output { data: "m3".into() }
    );
    assert_eq!(
        manager.next_event(&resumed, POLL).unwrap(),
        StreamEvent::Output { data: "m4".into() }
    );
    assert_eq!(
        manager.next_event(&resumed, POLL).unwrap(),
        StreamEvent::Output { data: "m5".into() }
    );
    assert_eq!(
        manager.next_event(&resumed, POLL).unwrap(),
        StreamEvent::StatusChange {
            from: "running".into(),
            to: "succeeded".into(),
        }
    );
}

#[test]
fn subscription_gap_detection_returns_error() {
    let manager = manager_over(DomainState::new());
    let task_id = TaskId("task-1".to_owned());

    // Overflow the bounded buffer so only the newest events are retained:
    // cursors 1..=1000+5 -> oldest retained is 6.
    for n in 1..=1005 {
        manager
            .emit_task(&task_id, StreamEvent::Progress {
                message: format!("m{n}"),
                percent: None,
            })
            .unwrap();
    }

    // Resuming from a cursor behind the oldest retained event is a gap.
    let err = manager.subscribe_task(&task_id, Some("2".into())).unwrap_err();
    match err {
        AppError::GapDetected(hint) => {
            // The hint names the resync position.
            assert!(hint.contains("oldest retained is 6"), "hint: {hint}");
            assert!(hint.contains("resync"), "hint: {hint}");
        }
        other => panic!("expected GapDetected, got {other:?}"),
    }

    // The same gap is reported on reconnect.
    let sub = manager.subscribe_task(&task_id, None).unwrap();
    let err = manager.reconnect(&sub.id, "3").unwrap_err();
    assert!(matches!(err, AppError::GapDetected(_)));
}

#[test]
fn subscription_explicit_resync_from_current_state() {
    let mut manager = manager_over(DomainState::new());
    let task_id = TaskId("task-1".to_owned());

    // Seed the current state with some retained events.
    for n in 1..=3 {
        manager
            .emit_task(&task_id, StreamEvent::Output { data: format!("m{n}") })
            .unwrap();
    }

    // An explicit resync replays the entire current retained window.
    let sub = manager.explicit_resync(&task_id.0).unwrap();
    assert_eq!(sub.cursor, None);
    for n in 1..=3 {
        assert_eq!(
            manager.next_event(&sub, POLL).unwrap(),
            StreamEvent::Output { data: format!("m{n}") }
        );
    }
    // Caught up after a resync.
    assert!(matches!(
        manager.next_event(&sub, POLL),
        Err(AppError::Timeout(_))
    ));
}

#[test]
fn subscription_bounded_buffer_prunes_oldest_events() {
    let mut manager = manager_over(DomainState::new());
    let process_id = ProcessId("proc-1".to_owned());

    let sub = manager.subscribe_process(&process_id, None).unwrap();

    // Emit more events than the per-target bound allows.
    let excess = 1200;
    for n in 0..excess {
        manager
            .emit_process(
                &process_id,
                StreamEvent::Progress {
                    message: format!("step-{n}"), // distinct label = cursor + 1
                    percent: None,
                },
            )
            .unwrap();
    }

    // Drain the stream: exactly MAX events remain, oldest pruned.
    let mut received = Vec::new();
    while let Ok(event) = manager.next_event(&sub, POLL) {
        if let StreamEvent::Progress { message, .. } = event {
            received.push(message);
        }
    }

    assert_eq!(received.len(), application::subscription::MAX_EVENTS_PER_STREAM);
    // Cursors 1..=1000+200 -> retained are 201..=1200. The oldest (step-0)
    // was pruned; the first retained event is step-200.
    assert_eq!(received.first().map(String::as_str), Some("step-200"));
    assert_eq!(received.last().map(String::as_str), Some("step-1199"));
}

#[test]
fn subscription_complete_event_carries_task_result() {
    let mut manager = manager_over(DomainState::new());
    let task_id = TaskId("task-9".to_owned());

    let sub = manager.subscribe_task(&task_id, None).unwrap();
    manager
        .emit_task(
            &task_id,
            StreamEvent::Complete {
                result: TaskResult {
                    task_id: task_id.clone(),
                    status: "succeeded".into(),
                    output: Some("ok".into()),
                },
            },
        )
        .unwrap();

    match manager.next_event(&sub, POLL).unwrap() {
        StreamEvent::Complete { result } => {
            assert_eq!(result.task_id, task_id);
            assert_eq!(result.status, "succeeded");
            assert_eq!(result.output.as_deref(), Some("ok"));
        }
        other => panic!("expected Complete, got {other:?}"),
    }
}