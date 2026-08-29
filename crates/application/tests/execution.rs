//! Canonical Execution/Context Plan/result UoW acceptance coverage.

#![allow(clippy::expect_used)]

use application::{
    ApplicationMutator, BotState, ContextPlan, ConversationOwner, ConversationState, DomainState,
    ExecutionEvidenceInput, ExecutionResultInput, ExecutionStatus, LifecycleState,
    MemoryAssertionStatus, MemoryState, ProviderBinding, TaskStatus,
};
use dxbot_core::types::{
    BotId, BotSelector, CanonicalTarget, CasConditions, CommandId, CommandPayload, ContentSource,
    ConversationId, IdempotencyKey, InstanceId, MemoryId, OperationId, OperationRequest,
    PrincipalRef, ProjectId, ProjectSelector, ProviderId, RequestDigest, ScopeSelector, TaskId,
};

fn seeded_state() -> DomainState {
    let mut state = DomainState::new();
    let bot_id = BotId("alpha".to_owned());
    state.bots.insert(
        bot_id.clone(),
        BotState {
            id: bot_id.clone(),
            name: "alpha".to_owned(),
            revision: 4,
            lifecycle: LifecycleState::Active,
            policy_bindings: application::BotPolicyBindings {
                permission_policy: Some("permission:default@1".to_owned()),
                ..Default::default()
            },
        },
    );
    let conversation_id = ConversationId("alpha:main".to_owned());
    state.conversations.insert(
        conversation_id.clone(),
        ConversationState {
            id: conversation_id,
            owner: ConversationOwner::Bot {
                bot_id: bot_id.clone(),
            },
            revision: 2,
            messages: Vec::new(),
        },
    );
    state.memories.insert(
        MemoryId("accepted".to_owned()),
        MemoryState {
            id: MemoryId("accepted".to_owned()),
            scope_key: "bot:alpha".to_owned(),
            revision: 3,
            status: MemoryAssertionStatus::Accepted,
            statement: ContentSource::Text {
                value: "canonical accepted memory".to_owned(),
            },
            evidence: vec!["evidence:a".to_owned()],
            history: Vec::new(),
            declassifications: Vec::new(),
        },
    );
    state.memories.insert(
        MemoryId("retracted".to_owned()),
        MemoryState {
            id: MemoryId("retracted".to_owned()),
            scope_key: "bot:alpha".to_owned(),
            revision: 8,
            status: MemoryAssertionStatus::Retracted,
            statement: ContentSource::Text {
                value: "must not enter context".to_owned(),
            },
            evidence: Vec::new(),
            history: Vec::new(),
            declassifications: Vec::new(),
        },
    );
    state
}

fn request() -> OperationRequest {
    OperationRequest {
        command_id: CommandId("command-task".to_owned()),
        idempotency_key: IdempotencyKey {
            principal_ref: PrincipalRef("principal".to_owned()),
            key_digest: "key".to_owned(),
            expires_at: i64::MAX,
        },
        request_digest: RequestDigest("digest".to_owned()),
        new_operation_id: OperationId("operation-task".to_owned()),
        payload: CommandPayload {
            command_key: "task-submit".to_owned(),
            principal_ref: PrincipalRef("principal".to_owned()),
            instance_id: InstanceId("instance".to_owned()),
            canonical_target: CanonicalTarget::Bot {
                id: BotId("alpha".to_owned()),
                revision: 4,
            },
            cas: None,
            content: Some(ContentSource::Text {
                value: "answer the task".to_owned(),
            }),
            semantic_options: serde_json::json!({"owner": "bot:alpha", "budget": "32"}),
        },
    }
}

fn binding() -> ProviderBinding {
    ProviderBinding {
        provider_id: ProviderId("real-provider".to_owned()),
        capability: "llm-chat".to_owned(),
        generation: 7,
    }
}

fn operation_request(
    suffix: &str,
    command_key: &str,
    target: CanonicalTarget,
    semantic_options: serde_json::Value,
    content: Option<ContentSource>,
) -> OperationRequest {
    OperationRequest {
        command_id: CommandId(format!("command-{suffix}")),
        idempotency_key: IdempotencyKey {
            principal_ref: PrincipalRef("principal".to_owned()),
            key_digest: format!("key-{suffix}"),
            expires_at: i64::MAX,
        },
        request_digest: RequestDigest(format!("digest-{suffix}")),
        new_operation_id: OperationId(format!("operation-{suffix}")),
        payload: CommandPayload {
            command_key: command_key.to_owned(),
            principal_ref: PrincipalRef("principal".to_owned()),
            instance_id: InstanceId("instance".to_owned()),
            canonical_target: target,
            cas: None,
            content,
            semantic_options,
        },
    }
}

fn project_revision_cas(revision: i64) -> CasConditions {
    CasConditions {
        if_revision: None,
        if_generation: None,
        if_host_generation: None,
        if_execution_generation: None,
        if_source_revision: None,
        if_scope_revision: None,
        if_project_revision: Some(revision),
        if_channel_revision: None,
        if_membership_generation: None,
        if_proposal_revision: None,
        if_target_scope_revision: None,
        if_receipt_revision: None,
    }
}

fn only_plan(state: &DomainState) -> &ContextPlan {
    &state
        .executions
        .values()
        .next()
        .expect("execution")
        .context_plan
}

#[test]
fn task_admission_atomically_creates_execution_and_bounded_context_plan() {
    let application = ApplicationMutator::with_state(seeded_state());
    let result = application
        .mutate_with_provider_binding(&request(), &binding())
        .expect("task admitted");
    let payload = result.committed_payload.expect("payload");
    assert_eq!(payload["state"], "admitted");
    assert!(payload["execution_ref"].as_str().is_some());
    assert!(payload["context_plan_ref"].as_str().is_some());

    let state = application.snapshot().expect("snapshot");
    let task = state.tasks.values().next().expect("task");
    assert_eq!(task.status, TaskStatus::Admitted);
    assert_eq!(task.execution_refs.len(), 1);
    let execution = state.executions.values().next().expect("execution");
    assert_eq!(execution.status, ExecutionStatus::Admitted);
    assert_eq!(execution.context_plan.provider_binding, binding());
    let plan = only_plan(&state);
    assert_eq!(plan.identity_revision, Some(4));
    assert_eq!(plan.memory_refs.len(), 1);
    assert_eq!(plan.memory_refs[0].memory_id.0, "accepted");
    assert!(plan.bounded_context.contains("canonical accepted memory"));
    assert!(!plan.bounded_context.contains("must not enter context"));
}

#[test]
fn result_commit_fences_all_generations_and_is_not_duplicated() {
    let application = ApplicationMutator::with_state(seeded_state());
    application
        .mutate_with_provider_binding(&request(), &binding())
        .expect("task admitted");
    let candidate = application
        .admitted_executions(1)
        .expect("candidates")
        .pop()
        .expect("candidate");
    let fence = application
        .begin_execution(&candidate, 11)
        .expect("execution starts");
    let side_effect = application
        .prepare_provider_side_effect(&fence)
        .expect("effect prepared");
    application
        .mark_provider_side_effect_dispatched(&fence, &side_effect)
        .expect("effect dispatched");
    let result = ExecutionResultInput {
        output: "provider answer".to_owned(),
        evidence: vec![ExecutionEvidenceInput {
            provider_id: ProviderId("real-provider".to_owned()),
            observation: "real-provider:llm-chat".to_owned(),
        }],
    };
    application
        .complete_execution_with_side_effect(&fence, &result, Some(&side_effect))
        .expect("result commits");
    assert!(application.complete_execution(&fence, &result).is_err());

    let state = application.snapshot().expect("snapshot");
    let task = state.tasks.values().next().expect("task");
    assert_eq!(task.status, TaskStatus::Succeeded);
    assert_eq!(
        task.result.as_ref().expect("result")["output"],
        "provider answer"
    );
    let execution = state.executions.values().next().expect("execution");
    assert_eq!(execution.status, ExecutionStatus::Succeeded);
    let process = state.processes.values().next().expect("process");
    assert_eq!(process.lifecycle, application::ProcessLifecycle::Completed);
    assert_eq!(process.outcome_refs.len(), 1);
    let effect = state.side_effects.get(&side_effect).expect("side effect");
    assert_eq!(effect.status, application::SideEffectStatus::Confirmed);
    assert_eq!(effect.execution_ref.as_ref(), Some(&fence.execution_id));
    let phases = state
        .execution_audit_intents
        .values()
        .map(|intent| intent.phase)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        phases,
        std::collections::BTreeSet::from([
            application::ExecutionAuditPhase::Admitted,
            application::ExecutionAuditPhase::Running,
            application::ExecutionAuditPhase::Dispatched,
            application::ExecutionAuditPhase::Succeeded,
        ])
    );
}

#[test]
fn restart_marks_running_provider_activity_recovery_required_without_redispatch() {
    let application = ApplicationMutator::with_state(seeded_state());
    application
        .mutate_with_provider_binding(&request(), &binding())
        .expect("task admitted");
    let candidate = application
        .admitted_executions(1)
        .expect("candidates")
        .pop()
        .expect("candidate");
    let fence = application
        .begin_execution(&candidate, 12)
        .expect("execution starts");
    let side_effect = application
        .prepare_provider_side_effect(&fence)
        .expect("effect prepared");
    application
        .mark_provider_side_effect_dispatched(&fence, &side_effect)
        .expect("effect dispatched");
    assert_eq!(
        application
            .recover_inflight_executions()
            .expect("reconciles"),
        1
    );
    assert!(
        application
            .admitted_executions(1)
            .expect("no redispatch")
            .is_empty()
    );
    let late_result = ExecutionResultInput {
        output: "late provider response".to_owned(),
        evidence: Vec::new(),
    };
    assert!(
        application
            .complete_execution_with_side_effect(&fence, &late_result, Some(&side_effect))
            .is_err(),
        "late result after uncertain restart must have effect 0"
    );
    let state = application.snapshot().expect("snapshot");
    assert_eq!(
        state.executions.values().next().expect("execution").status,
        ExecutionStatus::RecoveryRequired
    );
    assert_eq!(
        state.tasks.values().next().expect("task").status,
        TaskStatus::RecoveryRequired
    );
    assert_eq!(
        state
            .side_effects
            .get(&side_effect)
            .expect("side effect")
            .status,
        application::SideEffectStatus::Unknown
    );
    drop(state);

    let resume = operation_request(
        "recovery-resume",
        "task-resume",
        CanonicalTarget::Task {
            id: fence.task_id.clone(),
            revision: 3,
            execution_generation: Some(1),
        },
        serde_json::json!({"reason": "operator chose a fresh attempt"}),
        None,
    );
    let fresh_binding = ProviderBinding {
        generation: 8,
        ..binding()
    };
    application
        .mutate_with_provider_binding(&resume, &fresh_binding)
        .expect("explicit recovery resume");
    let resumed = application.snapshot().expect("resumed snapshot");
    let task = resumed.tasks.get(&fence.task_id).expect("task");
    assert_eq!(task.status, TaskStatus::Admitted);
    assert_eq!(task.execution_refs.len(), 2);
    let next = resumed
        .executions
        .get(&task.execution_refs[1])
        .expect("attempt");
    assert_eq!(next.status, ExecutionStatus::Admitted);
    assert_eq!(next.context_plan.provider_binding.generation, 8);
    assert_eq!(
        resumed
            .processes
            .values()
            .next()
            .expect("process")
            .lifecycle,
        application::ProcessLifecycle::Running
    );
}

#[test]
fn deadline_and_budget_are_normalized_or_fail_closed_before_execution() {
    let application = ApplicationMutator::with_state(seeded_state());
    let mut valid = request();
    valid.payload.semantic_options = serde_json::json!({
        "owner": "bot:alpha",
        "budget": "32",
        "deadline": "2026-09-01T00:00:00Z"
    });
    application
        .mutate_with_provider_binding(&valid, &binding())
        .expect("valid limits");
    let state = application.snapshot().expect("snapshot");
    let plan = only_plan(&state);
    assert_eq!(plan.max_output_tokens, Some(32));
    assert!(
        plan.deadline_unix_seconds
            .is_some_and(|value| value > 1_700_000_000)
    );

    let application = ApplicationMutator::with_state(seeded_state());
    let mut invalid = request();
    invalid.payload.semantic_options = serde_json::json!({
        "owner": "bot:alpha",
        "budget": "unresolved-policy-ref",
        "deadline": "tomorrow"
    });
    assert!(
        application
            .mutate_with_provider_binding(&invalid, &binding())
            .is_err()
    );
    let state = application.snapshot().expect("snapshot");
    assert!(state.tasks.is_empty());
    assert!(state.executions.is_empty());
}

#[test]
fn resume_creates_a_new_immutable_execution_attempt_with_fresh_provider_binding() {
    let application = ApplicationMutator::with_state(seeded_state());
    application
        .mutate_with_provider_binding(&request(), &binding())
        .expect("task admitted");
    let task_id = TaskId("task:operation-task".to_owned());

    let mut suspend = request();
    suspend.command_id = CommandId("command-suspend".to_owned());
    suspend.idempotency_key.key_digest = "key-suspend".to_owned();
    suspend.request_digest = RequestDigest("digest-suspend".to_owned());
    suspend.new_operation_id = OperationId("operation-suspend".to_owned());
    suspend.payload.command_key = "task-suspend".to_owned();
    suspend.payload.canonical_target = CanonicalTarget::Task {
        id: task_id.clone(),
        revision: 1,
        execution_generation: Some(1),
    };
    suspend.payload.content = None;
    suspend.payload.semantic_options = serde_json::json!({"reason": "checkpoint"});
    application.mutate(&suspend).expect("task suspended");

    let mut resume = request();
    resume.command_id = CommandId("command-resume".to_owned());
    resume.idempotency_key.key_digest = "key-resume".to_owned();
    resume.request_digest = RequestDigest("digest-resume".to_owned());
    resume.new_operation_id = OperationId("operation-resume".to_owned());
    resume.payload.command_key = "task-resume".to_owned();
    resume.payload.canonical_target = CanonicalTarget::Task {
        id: task_id.clone(),
        revision: 2,
        execution_generation: Some(1),
    };
    resume.payload.content = None;
    resume.payload.semantic_options = serde_json::json!({"reason": "continue"});
    let fresh_binding = ProviderBinding {
        provider_id: ProviderId("real-provider".to_owned()),
        capability: "llm-chat".to_owned(),
        generation: 8,
    };
    let result = application
        .mutate_with_provider_binding(&resume, &fresh_binding)
        .expect("task resumed as a fresh attempt");
    let payload = result.committed_payload.expect("resume payload");
    assert_eq!(payload["state"], "admitted");
    assert_eq!(payload["execution_generation"], 2);
    assert!(payload["execution_ref"].as_str().is_some());

    let state = application.snapshot().expect("snapshot");
    let task = state.tasks.get(&task_id).expect("task");
    assert_eq!(task.status, TaskStatus::Admitted);
    assert_eq!(task.execution_generation, 2);
    assert_eq!(task.execution_refs.len(), 2);
    let first = state
        .executions
        .get(&task.execution_refs[0])
        .expect("first");
    let second = state
        .executions
        .get(&task.execution_refs[1])
        .expect("second");
    assert_eq!(first.status, ExecutionStatus::Cancelled);
    assert_eq!(first.context_plan.provider_binding.generation, 7);
    assert_eq!(second.status, ExecutionStatus::Admitted);
    assert_eq!(second.attempt, 2);
    assert_eq!(second.context_plan.task_revision, 3);
    assert_eq!(second.context_plan.provider_binding, fresh_binding);
    assert_ne!(first.context_plan.plan_ref, second.context_plan.plan_ref);
    let process = state.processes.values().next().expect("process");
    assert_eq!(
        process.current_activity_ref.as_deref(),
        Some(format!("execution:{}", second.id.0).as_str())
    );
    assert_eq!(process.child_refs.len(), 3);
}

#[test]
fn durable_task_cancel_is_observed_and_finalizes_inflight_execution() {
    let application = ApplicationMutator::with_state(seeded_state());
    application
        .mutate_with_provider_binding(&request(), &binding())
        .expect("task admitted");
    let candidate = application
        .admitted_executions(1)
        .expect("candidates")
        .pop()
        .expect("candidate");
    let fence = application
        .begin_execution(&candidate, 21)
        .expect("execution starts");

    let mut cancel = request();
    cancel.command_id = CommandId("command-cancel-running".to_owned());
    cancel.idempotency_key.key_digest = "key-cancel-running".to_owned();
    cancel.request_digest = RequestDigest("digest-cancel-running".to_owned());
    cancel.new_operation_id = OperationId("operation-cancel-running".to_owned());
    cancel.payload.command_key = "task-cancel".to_owned();
    cancel.payload.canonical_target = CanonicalTarget::Task {
        id: TaskId("task:operation-task".to_owned()),
        revision: 2,
        execution_generation: Some(1),
    };
    cancel.payload.content = None;
    cancel.payload.semantic_options = serde_json::json!({"reason": "operator abort"});
    application.mutate(&cancel).expect("cancel committed");

    assert!(
        application
            .execution_should_cancel(&fence.execution_id)
            .expect("cancel observation")
    );
    application
        .finalize_cancelled_execution(&fence, "Provider transport cancellation observed")
        .expect("execution cancelled");
    let state = application.snapshot().expect("snapshot");
    assert_eq!(
        state
            .executions
            .get(&fence.execution_id)
            .expect("execution")
            .status,
        ExecutionStatus::Cancelled
    );
    assert_eq!(
        state.tasks.values().next().expect("task").status,
        TaskStatus::Cancelled
    );
}

#[test]
fn accepted_memory_survives_restart_and_enters_next_context_without_result_promotion() {
    let dir = std::env::temp_dir().join(format!(
        "dxbot-memory-continuity-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ));
    let path = dir.join("application-state.json");
    {
        let application =
            ApplicationMutator::with_persistent_state(path.clone()).expect("persistent open");
        application
            .mutate(&operation_request(
                "bot",
                "bot-create",
                CanonicalTarget::Instance(InstanceId("instance".to_owned())),
                serde_json::json!({"name": "alpha"}),
                None,
            ))
            .expect("bot create");
        application
            .mutate(&operation_request(
                "memory",
                "memory-propose",
                CanonicalTarget::Bot {
                    id: BotId("alpha".to_owned()),
                    revision: 1,
                },
                serde_json::json!({"evidence": ["observation:1"]}),
                Some(ContentSource::Text {
                    value: "remembered across restart".to_owned(),
                }),
            ))
            .expect("memory propose");
        application
            .mutate(&operation_request(
                "promote",
                "memory-promote",
                CanonicalTarget::Memory {
                    id: MemoryId("memory:operation-memory".to_owned()),
                    revision: 1,
                },
                serde_json::json!({
                    "target_scope": "bot:alpha",
                    "evidence": ["review:accepted"]
                }),
                None,
            ))
            .expect("memory accepted");
    }

    let restarted =
        ApplicationMutator::with_persistent_state(path.clone()).expect("persistent reopen");
    restarted
        .mutate_with_provider_binding(
            &operation_request(
                "after-restart",
                "task-submit",
                CanonicalTarget::Bot {
                    id: BotId("alpha".to_owned()),
                    revision: 1,
                },
                serde_json::json!({"owner": "bot:alpha"}),
                Some(ContentSource::Text {
                    value: "use prior memory".to_owned(),
                }),
            ),
            &binding(),
        )
        .expect("post-restart task admitted");
    let state = restarted.snapshot().expect("snapshot");
    let plan = state
        .executions
        .values()
        .next()
        .expect("execution")
        .context_plan
        .clone();
    assert_eq!(plan.memory_refs.len(), 1);
    assert_eq!(plan.memory_refs[0].memory_id.0, "memory:operation-memory");
    assert!(plan.bounded_context.contains("remembered across restart"));
    assert_eq!(
        state.memories.len(),
        1,
        "Task admission/result never auto-promotes Memory"
    );

    drop(restarted);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn three_bot_delegations_create_recipient_executions_once_and_survive_restart() {
    let dir = std::env::temp_dir().join(format!(
        "dxbot-three-bot-delegation-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ));
    let path = dir.join("application-state.json");
    {
        let application =
            ApplicationMutator::with_persistent_state(path.clone()).expect("persistent open");
        for bot in ["alpha", "beta", "gamma"] {
            application
                .mutate(&operation_request(
                    &format!("bot-{bot}"),
                    "bot-create",
                    CanonicalTarget::Instance(InstanceId("instance".to_owned())),
                    serde_json::json!({"name": bot}),
                    None,
                ))
                .expect("bot create");
        }
        application
            .mutate(&operation_request(
                "collaboration-project",
                "project-create",
                CanonicalTarget::Instance(InstanceId("instance".to_owned())),
                serde_json::json!({"name": "collaboration", "owner_bot": "bot:alpha"}),
                None,
            ))
            .expect("project create");
        let scope = ScopeSelector::Project(ProjectSelector::CanonicalId(ProjectId(
            "collaboration".to_owned(),
        )));
        for (index, bot) in ["beta", "gamma"].into_iter().enumerate() {
            let mut member = operation_request(
                &format!("member-{bot}"),
                "project-member-set",
                CanonicalTarget::Membership {
                    scope: scope.clone(),
                    member_bot: BotSelector::CanonicalId(BotId(bot.to_owned())),
                },
                serde_json::json!({"role_ref": "member"}),
                None,
            );
            member.payload.cas = Some(project_revision_cas(index as i64 + 1));
            application.mutate(&member).expect("member set");
        }
        for recipient in ["beta", "gamma"] {
            let delegated = operation_request(
                &format!("delegate-{recipient}"),
                "task-submit",
                CanonicalTarget::Project {
                    id: ProjectId("collaboration".to_owned()),
                    revision: 3,
                },
                serde_json::json!({
                    "delegate_to_bot": format!("bot:{recipient}"),
                    "requested_sender_bot": "bot:alpha",
                    "budget": "64"
                }),
                Some(ContentSource::Text {
                    value: format!("work for {recipient}"),
                }),
            );
            application
                .mutate_with_provider_binding(&delegated, &binding())
                .expect("recipient task admitted");
            if recipient == "beta" {
                application
                    .mutate_with_provider_binding(&delegated, &binding())
                    .expect("exact retry returns existing delegation");
            }
        }
        let state = application.snapshot().expect("snapshot");
        assert_eq!(state.tasks.len(), 2);
        assert_eq!(state.executions.len(), 2);
        assert_eq!(state.delegations.len(), 2);
        assert_eq!(state.processes.len(), 2);
    }

    let restarted =
        ApplicationMutator::with_persistent_state(path.clone()).expect("persistent reopen");
    let state = restarted.snapshot().expect("snapshot");
    assert_eq!(state.tasks.len(), 2);
    assert_eq!(state.executions.len(), 2);
    assert_eq!(state.delegations.len(), 2);
    let owners = state
        .tasks
        .values()
        .map(|task| task.owner.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        owners,
        std::collections::BTreeSet::from(["bot:beta", "bot:gamma"])
    );
    for delegation in &state.delegations {
        assert_eq!(delegation.status, application::DelegationStatus::Accepted);
        let task = state
            .tasks
            .get(&delegation.task_id)
            .expect("recipient task");
        assert_eq!(task.execution_refs.len(), 1);
        let execution = state
            .executions
            .get(&task.execution_refs[0])
            .expect("attempt");
        assert_eq!(execution.status, ExecutionStatus::Admitted);
        assert_eq!(execution.context_plan.scope_ref, "project:collaboration");
        assert_eq!(
            execution.context_plan.bot_ref.as_deref(),
            Some(task.owner.as_str())
        );
    }
    drop(restarted);
    let _ = std::fs::remove_dir_all(dir);
}
