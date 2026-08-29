//! Stateless bounded watch projection over canonical Task/Process revisions.
//!
//! The cursor encodes the last observed canonical revision as an opaque,
//! lexicographically monotonic fixed-width token. Legacy decimal revision
//! cursors remain accepted. A jump larger than one is reported as a gap rather
//! than silently dropping intermediate progress. No watcher state is canonical
//! or retained server-side, so reconnect/restart only requires the caller's
//! cursor.

use std::time::{Duration, Instant};

use dxbot_core::types::{CanonicalTarget, CommandPayload};
use serde_json::{Value, json};

use crate::mutation::{AppError, ApplicationMutator};
use crate::state::{ProcessLifecycle, TaskStatus};

const CURSOR_PREFIX: &str = "r";

impl ApplicationMutator {
    pub fn watch_next(
        &self,
        payload: &CommandPayload,
        cursor: Option<&str>,
        timeout: Duration,
    ) -> Result<Value, AppError> {
        if timeout.is_zero() {
            return Err(AppError::Conflict("watch timeout must be non-zero".to_owned()));
        }
        let requested = parse_cursor(cursor)?;
        let deadline = Instant::now().checked_add(timeout).ok_or_else(|| {
            AppError::Internal("watch timeout exceeds monotonic clock range".to_owned())
        })?;
        loop {
            let state = self.snapshot()?;
            let observation = match (&payload.canonical_target, payload.command_key.as_str()) {
                (CanonicalTarget::Task { id, .. }, "task-watch") => {
                    let row = state
                        .tasks
                        .get(id)
                        .ok_or_else(|| AppError::NotFound(format!("task {} does not exist", id.0)))?;
                    Observation {
                        revision: row.revision,
                        terminal: task_terminal(row.status),
                        value: json!({
                            "event": "task-state",
                            "task_ref": format!("task:{}", row.id.0),
                            "revision": row.revision,
                            "execution_generation": row.execution_generation,
                            "state": format!("{:?}", row.status).to_ascii_lowercase(),
                            "result": row.result,
                        }),
                    }
                }
                (CanonicalTarget::Process { id }, "process-watch") => {
                    let row = state.processes.get(id).ok_or_else(|| {
                        AppError::NotFound(format!("process {} does not exist", id.0))
                    })?;
                    Observation {
                        revision: row.revision,
                        terminal: process_terminal(row.lifecycle),
                        value: json!({
                            "event": "process-state",
                            "process_ref": format!("process:{}", row.id.0),
                            "revision": row.revision,
                            "state": format!("{:?}", row.lifecycle).to_ascii_lowercase(),
                            "current_step_ref": row.current_step_ref,
                            "waiting_condition_ref": row.waiting_condition_ref,
                            "child_refs": row.child_refs,
                            "progress": row.progress,
                            "terminal_reason": row.terminal_reason,
                        }),
                    }
                }
                _ => {
                    return Err(AppError::Conflict(format!(
                        "{} is not a Task/Process watch target",
                        payload.command_key
                    )));
                }
            };

            match requested {
                None => return observation.into_value(),
                Some(cursor) if observation.revision < cursor => {
                    return Err(AppError::Conflict(format!(
                        "watch cursor {cursor} is ahead of current revision {}",
                        observation.revision
                    )));
                }
                Some(cursor) if observation.revision > cursor.saturating_add(1) => {
                    return Err(AppError::GapDetected(format!(
                        "watch cursor gap: requested {cursor}, current {}; resync without a cursor",
                        observation.revision
                    )));
                }
                Some(cursor) if observation.revision > cursor => {
                    return observation.into_value();
                }
                Some(_) if observation.terminal => {
                    // A reconnect at the terminal revision still receives one
                    // idempotent terminal observation and may then exit.
                    return observation.into_value();
                }
                Some(_) => {}
            }

            if Instant::now() >= deadline {
                return Err(AppError::Timeout("watch observation timed out".to_owned()));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

struct Observation {
    revision: i64,
    terminal: bool,
    value: Value,
}

impl Observation {
    fn into_value(mut self) -> Result<Value, AppError> {
        let cursor = format_cursor(self.revision)?;
        if let Some(object) = self.value.as_object_mut() {
            object.insert("cursor".to_owned(), json!(cursor));
            object.insert("terminal".to_owned(), json!(self.terminal));
        }
        Ok(self.value)
    }
}

fn format_cursor(revision: i64) -> Result<String, AppError> {
    if revision < 0 {
        return Err(AppError::Internal(
            "canonical watch revision must be non-negative".to_owned(),
        ));
    }
    Ok(format!("{CURSOR_PREFIX}{revision:019}"))
}

fn parse_cursor(cursor: Option<&str>) -> Result<Option<i64>, AppError> {
    cursor
        .map(|value| {
            let encoded = value.strip_prefix(CURSOR_PREFIX).unwrap_or(value);
            encoded
                .parse::<i64>()
                .map_err(|_| AppError::Conflict(format!("invalid watch cursor: {value}")))
                .and_then(|revision| {
                    if revision < 0 {
                        Err(AppError::Conflict("watch cursor must be non-negative".to_owned()))
                    } else {
                        Ok(revision)
                    }
                })
        })
        .transpose()
}

fn task_terminal(status: TaskStatus) -> bool {
    matches!(
        status,
        TaskStatus::Succeeded
            | TaskStatus::Failed
            | TaskStatus::Rejected
            | TaskStatus::Deferred
            | TaskStatus::Cancelled
            | TaskStatus::RecoveryRequired
    )
}

fn process_terminal(status: ProcessLifecycle) -> bool {
    matches!(
        status,
        ProcessLifecycle::Completed
            | ProcessLifecycle::Cancelled
            | ProcessLifecycle::Failed
            | ProcessLifecycle::RecoveryRequired
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use dxbot_core::types::{InstanceId, PrincipalRef, ProcessId, TaskId};

    use super::*;
    use crate::{DomainState, ProcessState, TaskState};

    fn payload(command_key: &str, target: CanonicalTarget) -> CommandPayload {
        CommandPayload {
            command_key: command_key.to_owned(),
            principal_ref: PrincipalRef("principal".to_owned()),
            instance_id: InstanceId("instance".to_owned()),
            canonical_target: target,
            cas: None,
            content: None,
            semantic_options: json!({}),
        }
    }

    #[test]
    fn task_watch_returns_current_revision_without_server_handle() {
        let mut state = DomainState::new();
        let id = TaskId("task-a".to_owned());
        state.tasks.insert(
            id.clone(),
            TaskState {
                id: id.clone(),
                owner: "bot:a".to_owned(),
                revision: 3,
                execution_generation: 1,
                status: TaskStatus::Running,
                intent: None,
                result: None,
            },
        );
        let app = ApplicationMutator::with_state(state);
        let value = app
            .watch_next(
                &payload(
                    "task-watch",
                    CanonicalTarget::Task {
                        id,
                        revision: 0,
                        execution_generation: None,
                    },
                ),
                None,
                Duration::from_millis(1),
            )
            .expect("watch succeeds");
        assert_eq!(value["cursor"], "r0000000000000000003");
        assert_eq!(value["terminal"], false);
    }

    #[test]
    fn process_watch_rejects_legacy_cursor_gap() {
        let mut state = DomainState::new();
        let id = ProcessId("process-a".to_owned());
        state.processes.insert(
            id.clone(),
            ProcessState {
                id: id.clone(),
                definition_id: "definition".to_owned(),
                definition_version: "1".to_owned(),
                revision: 5,
                scope_ref: "project:a".to_owned(),
                initiator_ref: "principal:a".to_owned(),
                lifecycle: ProcessLifecycle::Running,
                current_step_ref: None,
                waiting_condition_ref: None,
                child_refs: Vec::new(),
                progress: 1,
                terminal_reason: None,
            },
        );
        let app = ApplicationMutator::with_state(state);
        let error = app
            .watch_next(
                &payload("process-watch", CanonicalTarget::Process { id }),
                Some("2"),
                Duration::from_millis(1),
            )
            .expect_err("gap must fail");
        assert!(matches!(error, AppError::GapDetected(_)));
    }

    #[test]
    fn fixed_width_cursor_is_lexicographically_monotonic_across_digit_boundary() {
        let nine = format_cursor(9).expect("nine");
        let ten = format_cursor(10).expect("ten");
        assert!(nine < ten);
        assert_eq!(parse_cursor(Some(&ten)).expect("parse"), Some(10));
        assert_eq!(parse_cursor(Some("10")).expect("legacy parse"), Some(10));
    }
}
