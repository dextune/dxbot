#![allow(clippy::unwrap_used)]
//! Acceptance tests for AT-APP-007: cursor-based application subscription.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use application::AppError;
use application::state::DomainState;
use application::subscription::{StreamEvent, SubscriptionManager, TaskResult};
use dxbot_core::types::{ProcessId, TaskId};

const POLL: Duration = Duration::from_millis(150);

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
        StreamEvent::Output {
            data: "hello".into()
        }
    );
    assert_eq!(manager.last_delivered_cursor(&sub), Some("2".into()));
}

#[test]
fn subscription_cursor_resumes_from_last_position() {
    let mut manager = manager_over(DomainState::new());
    let task_id = TaskId("task-1".to_owned());

    for n in 1..=3 {
        manager
            .emit_task(
                &task_id,
                StreamEvent::Output {
                    data: format!("m{n}"),
                },
            )
            .unwrap();
    }

    let sub = manager.subscribe_task(&task_id, Some("2".into())).unwrap();
    assert_eq!(sub.cursor, Some("2".into()));
    assert_eq!(
        manager.next_event(&sub, POLL).unwrap(),
        StreamEvent::Output { data: "m3".into() }
    );
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
    for n in 1..=5 {
        manager
            .emit_task(
                &task_id,
                StreamEvent::Output {
                    data: format!("m{n}"),
                },
            )
            .unwrap();
    }
    for _ in 0..2 {
        manager.next_event(&sub, POLL).unwrap();
    }

    manager
        .emit_task(
            &task_id,
            StreamEvent::StatusChange {
                from: "running".into(),
                to: "succeeded".into(),
            },
        )
        .unwrap();

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
fn subscription_gap_detection_distinguishes_missing_from_predecessor() {
    let mut manager = manager_over(DomainState::new());
    let task_id = TaskId("task-1".to_owned());

    for n in 1..=1005 {
        manager
            .emit_task(
                &task_id,
                StreamEvent::Progress {
                    message: format!("m{n}"),
                    percent: None,
                },
            )
            .unwrap();
    }

    let error = manager
        .subscribe_task(&task_id, Some("2".into()))
        .unwrap_err();
    match error {
        AppError::GapDetected(hint) => {
            assert!(hint.contains("oldest retained is 6"), "hint: {hint}");
            assert!(hint.contains("resync"), "hint: {hint}");
        }
        other => panic!("expected GapDetected, got {other:?}"),
    }

    // Cursor 5 is exactly the predecessor of oldest retained cursor 6, so no
    // event is missing and delivery may continue with event 6.
    let predecessor = manager.subscribe_task(&task_id, Some("5".into())).unwrap();
    assert_eq!(
        manager.next_event(&predecessor, POLL).unwrap(),
        StreamEvent::Progress {
            message: "m6".into(),
            percent: None,
        }
    );

    // A claimed cursor that has not been emitted yet is rejected rather than
    // silently suppressing future events until the stream catches up.
    assert!(matches!(
        manager.subscribe_task(&task_id, Some("1006".into())),
        Err(AppError::Conflict(_))
    ));
}

#[test]
fn subscription_explicit_resync_from_current_retained_state() {
    let mut manager = manager_over(DomainState::new());
    let task_id = TaskId("task-1".to_owned());

    for n in 1..=3 {
        manager
            .emit_task(
                &task_id,
                StreamEvent::Output {
                    data: format!("m{n}"),
                },
            )
            .unwrap();
    }

    let sub = manager.explicit_resync(&task_id.0).unwrap();
    assert_eq!(sub.cursor, None);
    for n in 1..=3 {
        assert_eq!(
            manager.next_event(&sub, POLL).unwrap(),
            StreamEvent::Output {
                data: format!("m{n}")
            }
        );
    }
    assert!(matches!(
        manager.next_event(&sub, POLL),
        Err(AppError::Timeout(_))
    ));
}

#[test]
fn subscription_active_reader_detects_pruning_gap_then_resyncs() {
    let mut manager = manager_over(DomainState::new());
    let process_id = ProcessId("proc-1".to_owned());

    // This subscription begins before any event. It must not silently jump
    // forward if the retained window later prunes events before it reads them.
    let lagging = manager.subscribe_process(&process_id, None).unwrap();

    for n in 0..1200 {
        manager
            .emit_process(
                &process_id,
                StreamEvent::Progress {
                    message: format!("step-{n}"),
                    percent: None,
                },
            )
            .unwrap();
    }

    assert!(matches!(
        manager.next_event(&lagging, POLL),
        Err(AppError::GapDetected(_))
    ));

    // Explicit resync is the only path that intentionally starts at the oldest
    // currently retained event.
    let resynced = manager.explicit_resync(&process_id.0).unwrap();
    let mut received = Vec::new();
    while let Ok(event) = manager.next_event(&resynced, POLL) {
        if let StreamEvent::Progress { message, .. } = event {
            received.push(message);
        }
    }

    assert_eq!(
        received.len(),
        application::subscription::MAX_EVENTS_PER_STREAM
    );
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
