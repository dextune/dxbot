//! BF-CLI-028 / BF-CLI-029 (Application-owned): deterministic tests for the
//! internal durable Process producer tied to task submission/control, and for
//! the Application-owned frozen semantic fields.
//!
//! These tests exercise the public `ApplicationMutator` mutation/query/watch
//! surface plus durable restart. They assert:
//! - task submit produces a linked Process aggregate and revisions advance;
//! - task control transitions the linked Process without copying task state;
//! - process show/watch observe a real producer, and restart preserves both;
//! - duplicate delivery / exact retry does not double-advance the Process;
//! - Bot policy bindings, task constraints, control reason/audit, task-result
//!   artifact selection, memory-get scope validation, memory-promote
//!   declassification provenance, and side-effect operation linkage close
//!   without silently ignoring any field.

#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use application::state::{SideEffectState, SideEffectStatus};
use application::{AppError, ApplicationMutator};
use dxbot_core::types::{
    BotId, CanonicalTarget, CommandId, CommandPayload, IdempotencyKey, InstanceId, OperationId,
    OperationRequest, PrincipalRef, ProcessId, ProjectId, RequestDigest, TaskId,
};
use serde_json::{Value, json};

fn principal() -> PrincipalRef {
    PrincipalRef("http:principal-a".to_owned())
}

fn instance() -> InstanceId {
    InstanceId("instance-1".to_owned())
}

fn request(
    command_id: &str,
    command_key: &str,
    target: CanonicalTarget,
    semantic_options: Value,
    content: Option<dxbot_core::types::ContentSource>,
) -> OperationRequest {
    OperationRequest {
        command_id: CommandId(command_id.to_owned()),
        idempotency_key: IdempotencyKey {
            principal_ref: principal(),
            key_digest: format!("key-{command_id}"),
            expires_at: i64::MAX,
        },
        request_digest: RequestDigest(format!("digest-{command_id}")),
        new_operation_id: OperationId(format!("op-{command_id}")),
        payload: CommandPayload {
            command_key: command_key.to_owned(),
            principal_ref: principal(),
            instance_id: instance(),
            canonical_target: target,
            cas: None,
            content,
            semantic_options,
        },
    }
}

fn text(value: &str) -> dxbot_core::types::ContentSource {
    dxbot_core::types::ContentSource::Text {
        value: value.to_owned(),
    }
}

fn seed_bot(mutator: &ApplicationMutator, name: &str) {
    mutator
        .mutate(&request(
            &format!("bot-{name}"),
            "bot-create",
            CanonicalTarget::Instance(instance()),
            json!({ "name": name }),
            None,
        ))
        .expect("bot create");
}

/// Submit a task against a bot scope and return the committing operation id.
fn submit_task(
    mutator: &ApplicationMutator,
    command_id: &str,
    bot: &str,
    semantic_options: Value,
) -> String {
    mutator
        .mutate(&request(
            command_id,
            "task-submit",
            CanonicalTarget::Bot {
                id: BotId(bot.to_owned()),
                revision: 0,
            },
            semantic_options,
            Some(text("do the thing")),
        ))
        .expect("task submit");
    format!("op-{command_id}")
}

fn task_target(operation_id: &str, execution_generation: Option<i64>) -> CanonicalTarget {
    CanonicalTarget::Task {
        id: TaskId(format!("task:{operation_id}")),
        revision: 0,
        execution_generation,
    }
}

fn process_query(mutator: &ApplicationMutator, operation_id: &str) -> Result<Value, AppError> {
    mutator.query(&CommandPayload {
        command_key: "process-show".to_owned(),
        principal_ref: principal(),
        instance_id: instance(),
        canonical_target: CanonicalTarget::Process {
            id: ProcessId(format!("process:{operation_id}")),
        },
        cas: None,
        content: None,
        semantic_options: json!({}),
    })
}

// ── BF-CLI-028: Process producer ──────────────────────────────────────────

#[test]
fn task_submit_produces_linked_process_aggregate() {
    let mutator = ApplicationMutator::new();
    seed_bot(&mutator, "alpha");
    let op = submit_task(&mutator, "cmd-submit", "alpha", json!({}));

    // The producer exists and is observable via process-show.
    let process = process_query(&mutator, &op).expect("process visible");
    assert_eq!(process["process_ref"], format!("process:process:{op}"));
    assert_eq!(process["definition_ref"], "task-execution");
    assert_eq!(process["state"], "running");
    assert_eq!(process["revision"], 1);

    // The Process references the child task but does not copy task state.
    let child_refs = process["child_refs"].as_array().expect("child refs array");
    assert_eq!(child_refs.len(), 1);
    assert_eq!(child_refs[0], format!("task:task:{op}"));
    assert!(
        process.get("status").is_none(),
        "process must not copy task status"
    );
    assert!(
        process.get("intent").is_none(),
        "process must not copy task intent"
    );

    // The task carries a back-reference to the process, not an embedded copy.
    let state = mutator.snapshot().expect("snapshot");
    let task = state
        .tasks
        .get(&TaskId(format!("task:{op}")))
        .expect("task exists");
    assert_eq!(
        task.process_ref.as_deref(),
        Some(format!("process:process:{op}").as_str())
    );
}

#[test]
fn task_cancel_transitions_linked_process_with_reason() {
    let mutator = ApplicationMutator::new();
    seed_bot(&mutator, "alpha");
    let op = submit_task(&mutator, "cmd-submit", "alpha", json!({}));

    mutator
        .mutate(&request(
            "cmd-cancel",
            "task-cancel",
            task_target(&op, Some(1)),
            json!({ "reason": "operator abort" }),
            None,
        ))
        .expect("task cancel");

    let process = process_query(&mutator, &op).expect("process visible");
    assert_eq!(process["state"], "cancelled");
    assert_eq!(
        process["revision"], 2,
        "control advances the process revision"
    );
    assert_eq!(process["progress"], 1);
    assert_eq!(process["terminal_reason"], "operator abort");

    // The directive reason is durably recorded on the task audit trail.
    let state = mutator.snapshot().expect("snapshot");
    let task = state
        .tasks
        .get(&TaskId(format!("task:{op}")))
        .expect("task");
    assert_eq!(task.control_history.len(), 1);
    assert_eq!(task.control_history[0].command_key, "task-cancel");
    assert_eq!(
        task.control_history[0].reason.as_deref(),
        Some("operator abort")
    );
    assert_eq!(task.control_history[0].applied_revision, task.revision);
}

#[test]
fn process_watch_observes_producer_and_monotonic_cursor() {
    let mutator = ApplicationMutator::new();
    seed_bot(&mutator, "alpha");
    let op = submit_task(&mutator, "cmd-submit", "alpha", json!({}));

    let watch_payload = CommandPayload {
        command_key: "process-watch".to_owned(),
        principal_ref: principal(),
        instance_id: instance(),
        canonical_target: CanonicalTarget::Process {
            id: ProcessId(format!("process:{op}")),
        },
        cas: None,
        content: None,
        semantic_options: json!({}),
    };

    let first = mutator
        .watch_next(&watch_payload, None, Duration::from_millis(5))
        .expect("initial watch");
    assert_eq!(first["state"], "running");
    assert_eq!(first["terminal"], false);
    let cursor = first["cursor"].as_str().expect("cursor").to_owned();

    // Advance the process by cancelling the task, then observe the new revision.
    mutator
        .mutate(&request(
            "cmd-cancel",
            "task-cancel",
            task_target(&op, Some(1)),
            json!({ "reason": "abort" }),
            None,
        ))
        .expect("cancel");
    let second = mutator
        .watch_next(&watch_payload, Some(&cursor), Duration::from_millis(50))
        .expect("watch after advance");
    assert_eq!(second["state"], "cancelled");
    assert_eq!(second["terminal"], true);
    assert!(
        second["cursor"].as_str().expect("cursor2") > cursor.as_str(),
        "watch cursor is monotonic across the transition"
    );
}

#[test]
fn task_submit_exact_retry_does_not_duplicate_process() {
    let mutator = ApplicationMutator::new();
    seed_bot(&mutator, "alpha");
    let submit = request(
        "cmd-submit",
        "task-submit",
        CanonicalTarget::Bot {
            id: BotId("alpha".to_owned()),
            revision: 0,
        },
        json!({}),
        Some(text("do the thing")),
    );
    let first = mutator.mutate(&submit).expect("submit");
    let retry = mutator.mutate(&submit).expect("exact retry");
    assert_eq!(first, retry, "exact retry returns the committed result");

    let state = mutator.snapshot().expect("snapshot");
    assert_eq!(
        state.processes.len(),
        1,
        "retry must not create a second process"
    );
    assert_eq!(state.tasks.len(), 1);
}

#[test]
fn duplicate_terminal_control_does_not_double_advance_process() {
    let mutator = ApplicationMutator::new();
    seed_bot(&mutator, "alpha");
    let op = submit_task(&mutator, "cmd-submit", "alpha", json!({}));

    let cancel = request(
        "cmd-cancel",
        "task-cancel",
        task_target(&op, Some(1)),
        json!({ "reason": "abort" }),
        None,
    );
    mutator.mutate(&cancel).expect("cancel");
    // Exact-retry the same cancel command: idempotent, returns the committed
    // result and does not advance the process a second time.
    mutator.mutate(&cancel).expect("duplicate cancel");

    let process = process_query(&mutator, &op).expect("process");
    assert_eq!(
        process["revision"], 2,
        "process revision advances exactly once"
    );
    assert_eq!(process["progress"], 1);
}

#[test]
fn terminal_process_cannot_be_resurrected_by_new_control_operation() {
    let mutator = ApplicationMutator::new();
    seed_bot(&mutator, "alpha");
    let op = submit_task(&mutator, "cmd-submit", "alpha", json!({}));
    mutator
        .mutate(&request(
            "cmd-cancel",
            "task-cancel",
            task_target(&op, Some(1)),
            json!({ "reason": "done" }),
            None,
        ))
        .expect("cancel");

    let error = mutator
        .mutate(&request(
            "cmd-resume-after-cancel",
            "task-resume",
            task_target(&op, None),
            json!({}),
            None,
        ))
        .expect_err("terminal task/process must not resume");
    assert!(matches!(error, AppError::Conflict(_)));
    let process = process_query(&mutator, &op).expect("process");
    assert_eq!(process["state"], "cancelled");
    assert_eq!(process["revision"], 2);
}

#[test]
fn missing_linked_process_rolls_back_task_control_atomically() {
    let task_id = TaskId("task:orphan".to_owned());
    let mut state = application::state::DomainState::new();
    state.tasks.insert(
        task_id.clone(),
        application::state::TaskState {
            id: task_id.clone(),
            owner: "bot:alpha".to_owned(),
            revision: 1,
            execution_generation: 1,
            status: application::state::TaskStatus::Pending,
            intent: None,
            result: None,
            constraints: Default::default(),
            control_history: Vec::new(),
            process_ref: Some("process:missing".to_owned()),
        },
    );
    let mutator = ApplicationMutator::with_state(state);
    let error = mutator
        .mutate(&request(
            "cmd-orphan-cancel",
            "task-cancel",
            CanonicalTarget::Task {
                id: task_id.clone(),
                revision: 1,
                execution_generation: Some(1),
            },
            json!({ "reason": "abort" }),
            None,
        ))
        .expect_err("missing Process ref must fail the unit of work");
    assert!(matches!(error, AppError::Internal(_)));

    let snapshot = mutator.snapshot().expect("snapshot");
    let task = snapshot.tasks.get(&task_id).expect("task");
    assert_eq!(task.status, application::state::TaskStatus::Pending);
    assert_eq!(task.revision, 1);
    assert!(task.control_history.is_empty());
    assert!(snapshot.command_bindings.is_empty());
}

#[test]
fn process_producer_survives_restart() {
    let dir = std::env::temp_dir().join(format!(
        "dxbot-process-restart-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    let path = dir.join("application-state.json");

    let op = {
        let mutator = ApplicationMutator::with_persistent_state(path.clone()).expect("open");
        seed_bot(&mutator, "alpha");
        let op = submit_task(&mutator, "cmd-submit", "alpha", json!({}));
        mutator
            .mutate(&request(
                "cmd-suspend",
                "task-suspend",
                task_target(&op, Some(1)),
                json!({ "reason": "pause" }),
                None,
            ))
            .expect("suspend");
        op
    };

    // Reopen from durable state: the Process and its refs are restored, and the
    // task retains its process back-reference and control audit.
    let restarted = ApplicationMutator::with_persistent_state(path.clone()).expect("reopen");
    let process = process_query(&restarted, &op).expect("process restored");
    assert_eq!(process["state"], "suspended");
    assert_eq!(process["revision"], 2);

    let state = restarted.snapshot().expect("snapshot");
    let task = state
        .tasks
        .get(&TaskId(format!("task:{op}")))
        .expect("task restored");
    assert_eq!(
        task.process_ref.as_deref(),
        Some(format!("process:process:{op}").as_str())
    );
    assert_eq!(task.control_history.len(), 1);
    assert_eq!(task.control_history[0].reason.as_deref(), Some("pause"));

    let _ = std::fs::remove_dir_all(&dir);
}

// ── BF-CLI-029: Application-owned semantic fields ───────────────────────────

#[test]
fn bot_create_persists_explicit_policy_bindings() {
    let mutator = ApplicationMutator::new();
    let result = mutator
        .mutate(&request(
            "cmd-bot",
            "bot-create",
            CanonicalTarget::Instance(instance()),
            json!({
                "name": "alpha",
                "brain_policy": "brain:default",
                "permission_policy": "perm:default",
                "resource_policy": "res:default",
                "provider_policy": "prov:default"
            }),
            None,
        ))
        .expect("bot create");
    let committed = result.committed_payload.expect("committed payload");
    assert_eq!(
        committed["policy_bindings"]["brain_policy"],
        "brain:default"
    );

    let state = mutator.snapshot().expect("snapshot");
    let bot = state.bots.get(&BotId("alpha".to_owned())).expect("bot");
    assert_eq!(
        bot.policy_bindings.brain_policy.as_deref(),
        Some("brain:default")
    );
    assert_eq!(
        bot.policy_bindings.permission_policy.as_deref(),
        Some("perm:default")
    );
    assert_eq!(
        bot.policy_bindings.resource_policy.as_deref(),
        Some("res:default")
    );
    assert_eq!(
        bot.policy_bindings.provider_policy.as_deref(),
        Some("prov:default")
    );
}

#[test]
fn task_submit_persists_execution_constraints() {
    let mutator = ApplicationMutator::new();
    seed_bot(&mutator, "alpha");
    seed_bot(&mutator, "beta");
    let op = submit_task(
        &mutator,
        "cmd-submit",
        "alpha",
        json!({
            "delegate_to_bot": "bot:beta",
            "requested_sender_bot": "bot:alpha",
            "deadline": "2026-09-01T00:00:00Z",
            "budget": "1000"
        }),
    );
    let state = mutator.snapshot().expect("snapshot");
    let task = state
        .tasks
        .get(&TaskId(format!("task:{op}")))
        .expect("task");
    assert_eq!(
        task.constraints.delegate_to_bot.as_deref(),
        Some("bot:beta")
    );
    assert_eq!(
        task.constraints.requested_sender_bot.as_deref(),
        Some("bot:alpha")
    );
    assert_eq!(
        task.constraints.deadline.as_deref(),
        Some("2026-09-01T00:00:00Z")
    );
    assert_eq!(task.constraints.budget.as_deref(), Some("1000"));
}

#[test]
fn task_submit_rejects_unknown_delegate_target() {
    let mutator = ApplicationMutator::new();
    seed_bot(&mutator, "alpha");
    let error = mutator
        .mutate(&request(
            "cmd-submit",
            "task-submit",
            CanonicalTarget::Bot {
                id: BotId("alpha".to_owned()),
                revision: 0,
            },
            json!({ "delegate_to_bot": "bot:ghost" }),
            Some(text("x")),
        ))
        .expect_err("unknown delegate must fail closed");
    assert!(matches!(error, AppError::NotFound(_)));
}

#[test]
fn task_submit_rejects_unknown_requested_sender() {
    let mutator = ApplicationMutator::new();
    seed_bot(&mutator, "alpha");
    let error = mutator
        .mutate(&request(
            "cmd-submit-sender",
            "task-submit",
            CanonicalTarget::Bot {
                id: BotId("alpha".to_owned()),
                revision: 0,
            },
            json!({ "requested_sender_bot": "bot:ghost" }),
            Some(text("x")),
        ))
        .expect_err("unknown requested sender must fail closed");
    assert!(matches!(error, AppError::NotFound(_)));
}

#[test]
fn memory_promote_persists_declassification_provenance() {
    let mutator = ApplicationMutator::new();
    seed_bot(&mutator, "alpha");
    mutator
        .mutate(&request(
            "cmd-project",
            "project-create",
            CanonicalTarget::Instance(instance()),
            json!({ "name": "proj", "owner_bot": "bot:alpha" }),
            None,
        ))
        .expect("project create");

    // Propose against the bot scope.
    let propose_op = "op-cmd-memory";
    mutator
        .mutate(&request(
            "cmd-memory",
            "memory-propose",
            CanonicalTarget::Bot {
                id: BotId("alpha".to_owned()),
                revision: 0,
            },
            json!({ "evidence": ["e1"] }),
            Some(text("the sky is blue")),
        ))
        .expect("memory propose");
    let memory_id = format!("memory:{propose_op}");

    mutator
        .mutate(&request(
            "cmd-promote",
            "memory-promote",
            CanonicalTarget::Memory {
                id: dxbot_core::types::MemoryId(memory_id.clone()),
                revision: 0,
            },
            json!({
                "target_scope": "project:proj",
                "evidence": ["e2"],
                "declassification_ref": "declass:ticket-42"
            }),
            None,
        ))
        .expect("memory promote");

    let state = mutator.snapshot().expect("snapshot");
    let memory = state
        .memories
        .get(&dxbot_core::types::MemoryId(memory_id))
        .expect("memory");
    assert_eq!(memory.declassifications.len(), 1);
    assert_eq!(
        memory.declassifications[0].declassification_ref,
        "declass:ticket-42"
    );
    assert_eq!(
        memory.declassifications[0].applied_revision,
        memory.revision
    );
}

#[test]
fn side_effect_reconcile_resolves_by_operation_linkage() {
    let mutator = ApplicationMutator::new();
    // Seed a side effect linked to an operation via the durable linkage index.
    {
        let handle = mutator.snapshot().expect("snapshot");
        drop(handle);
    }
    // Insert directly through a seeded operation-linked side effect. We use the
    // public mutate path to create an operation binding, then seed the linked
    // side effect into canonical state via a fresh mutator state clone is not
    // possible; instead we build a mutator with pre-seeded state.
    let mut seeded = application::state::DomainState::new();
    seeded.side_effects.insert(
        "se-1".to_owned(),
        SideEffectState {
            id: "se-1".to_owned(),
            revision: 1,
            status: SideEffectStatus::Dispatched,
            evidence: Vec::new(),
            operation_ref: Some(OperationId("op-xyz".to_owned())),
        },
    );
    let mutator = ApplicationMutator::with_state(seeded);

    // Reconcile selecting by operation reference rather than canonical id.
    let (target, _cas) = mutator
        .preflight(
            &CommandPayload {
                command_key: "side-effect-reconcile".to_owned(),
                principal_ref: principal(),
                instance_id: instance(),
                canonical_target: CanonicalTarget::SideEffect {
                    id: "operation:op-xyz".to_owned(),
                    revision: 0,
                },
                cas: None,
                content: None,
                semantic_options: json!({ "action": "confirm" }),
            },
            Some(&json!({ "value": "operation:op-xyz" })),
        )
        .expect("operation linkage resolves");
    match target {
        CanonicalTarget::SideEffect { id, .. } => assert_eq!(id, "se-1"),
        other => panic!("expected side effect target, got {other:?}"),
    }
}

#[test]
fn side_effect_operation_linkage_reports_ambiguity() {
    let mut seeded = application::state::DomainState::new();
    for id in ["se-1", "se-2"] {
        seeded.side_effects.insert(
            id.to_owned(),
            SideEffectState {
                id: id.to_owned(),
                revision: 1,
                status: SideEffectStatus::Dispatched,
                evidence: Vec::new(),
                operation_ref: Some(OperationId("op-dup".to_owned())),
            },
        );
    }
    let error = seeded
        .resolve_side_effect_id(&dxbot_core::types::SideEffectSelector::Operation(
            dxbot_core::types::OperationSelector::OperationId(OperationId("op-dup".to_owned())),
        ))
        .expect_err("ambiguous linkage must fail closed");
    assert!(matches!(
        error,
        application::state::SideEffectResolveError::Ambiguous(_)
    ));
    // Unused to satisfy the borrow of ProjectId import in other tests.
    let _ = ProjectId("proj".to_owned());
}

#[test]
fn task_result_selects_artifact_by_id() {
    let mut seeded = application::state::DomainState::new();
    seeded.tasks.insert(
        TaskId("task:op-1".to_owned()),
        application::state::TaskState {
            id: TaskId("task:op-1".to_owned()),
            owner: "bot:alpha".to_owned(),
            revision: 2,
            execution_generation: 1,
            status: application::state::TaskStatus::Succeeded,
            intent: None,
            result: Some(json!({
                "artifacts": [
                    { "artifact_id": "a1", "digest": "d1" },
                    { "artifact_id": "a2", "digest": "d2" }
                ]
            })),
            constraints: Default::default(),
            control_history: Vec::new(),
            process_ref: None,
        },
    );
    let mutator = ApplicationMutator::with_state(seeded);

    let selected = mutator
        .query(&CommandPayload {
            command_key: "task-result".to_owned(),
            principal_ref: principal(),
            instance_id: instance(),
            canonical_target: CanonicalTarget::Task {
                id: TaskId("task:op-1".to_owned()),
                revision: 0,
                execution_generation: None,
            },
            cas: None,
            content: None,
            semantic_options: json!({ "artifact_id": "a2" }),
        })
        .expect("artifact selection");
    assert_eq!(selected["artifact_id"], "a2");
    assert_eq!(selected["artifact"]["digest"], "d2");

    // Unknown artifact fails closed rather than returning the whole result.
    let error = mutator
        .query(&CommandPayload {
            command_key: "task-result".to_owned(),
            principal_ref: principal(),
            instance_id: instance(),
            canonical_target: CanonicalTarget::Task {
                id: TaskId("task:op-1".to_owned()),
                revision: 0,
                execution_generation: None,
            },
            cas: None,
            content: None,
            semantic_options: json!({ "artifact_id": "ghost" }),
        })
        .expect_err("unknown artifact must fail closed");
    assert!(matches!(error, AppError::NotFound(_)));
}

#[test]
fn memory_get_validates_optional_scope() {
    let mutator = ApplicationMutator::new();
    seed_bot(&mutator, "alpha");
    seed_bot(&mutator, "beta");
    mutator
        .mutate(&request(
            "cmd-memory",
            "memory-propose",
            CanonicalTarget::Bot {
                id: BotId("alpha".to_owned()),
                revision: 0,
            },
            json!({ "evidence": ["e1"] }),
            Some(text("scoped fact")),
        ))
        .expect("propose");
    let memory_id = "memory:op-cmd-memory".to_owned();

    // Correct scope resolves.
    let ok = mutator
        .query(&CommandPayload {
            command_key: "memory-get".to_owned(),
            principal_ref: principal(),
            instance_id: instance(),
            canonical_target: CanonicalTarget::Memory {
                id: dxbot_core::types::MemoryId(memory_id.clone()),
                revision: 0,
            },
            cas: None,
            content: None,
            semantic_options: json!({ "scope": "bot:alpha" }),
        })
        .expect("in-scope memory-get");
    assert_eq!(ok["scope"], "bot:alpha");

    // Wrong scope fails closed rather than silently returning the memory.
    let error = mutator
        .query(&CommandPayload {
            command_key: "memory-get".to_owned(),
            principal_ref: principal(),
            instance_id: instance(),
            canonical_target: CanonicalTarget::Memory {
                id: dxbot_core::types::MemoryId(memory_id),
                revision: 0,
            },
            cas: None,
            content: None,
            semantic_options: json!({ "scope": "bot:beta" }),
        })
        .expect_err("out-of-scope memory-get must fail closed");
    assert!(matches!(error, AppError::NotFound(_)));
}
