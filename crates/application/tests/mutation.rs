//! Acceptance tests for AT-APP-005: mutation materialization, complete binding
//! identity, receipt, and fail-closed domain outcome semantics.

#![allow(clippy::expect_used)]

use application::mutation::AppError;
use application::{ApplicationMutator, DomainOutcome, cas_if_revision, resolve_outcome};
use dxbot_core::receipt::ReceiptDisposition;
use dxbot_core::types::{
    CanonicalTarget, CommandId, CommandPayload, ConversationId, IdempotencyKey, InstanceId,
    OperationId, OperationRequest, PrincipalRef, RequestDigest, ThreadId,
};

fn principal() -> PrincipalRef {
    PrincipalRef("http:principal-a".to_owned())
}

fn create_bot_request(
    command: &str,
    key: &str,
    digest: &str,
    operation: &str,
    name: &str,
) -> OperationRequest {
    OperationRequest {
        command_id: CommandId(command.to_owned()),
        idempotency_key: IdempotencyKey {
            principal_ref: principal(),
            key_digest: key.to_owned(),
            expires_at: i64::MAX,
        },
        request_digest: RequestDigest(digest.to_owned()),
        new_operation_id: OperationId(operation.to_owned()),
        payload: CommandPayload {
            command_key: "bot-create".to_owned(),
            principal_ref: principal(),
            instance_id: InstanceId("instance-1".to_owned()),
            canonical_target: CanonicalTarget::Instance(InstanceId("instance-1".to_owned())),
            cas: None,
            content: None,
            semantic_options: serde_json::json!({"name": name}),
        },
    }
}

fn bot_update(
    command: &str,
    operation: &str,
    revision: i64,
    command_key: &str,
) -> OperationRequest {
    OperationRequest {
        command_id: CommandId(command.to_owned()),
        idempotency_key: IdempotencyKey {
            principal_ref: principal(),
            key_digest: format!("key-{command}"),
            expires_at: i64::MAX,
        },
        request_digest: RequestDigest(format!("digest-{command}")),
        new_operation_id: OperationId(operation.to_owned()),
        payload: CommandPayload {
            command_key: command_key.to_owned(),
            principal_ref: principal(),
            instance_id: InstanceId("instance-1".to_owned()),
            canonical_target: CanonicalTarget::Bot {
                id: dxbot_core::types::BotId("alpha-bot".to_owned()),
                revision,
            },
            cas: Some(cas_if_revision(revision)),
            content: None,
            semantic_options: serde_json::Value::Null,
        },
    }
}

fn relation_request(command: &str, target: CanonicalTarget) -> OperationRequest {
    OperationRequest {
        command_id: CommandId(command.to_owned()),
        idempotency_key: IdempotencyKey {
            principal_ref: principal(),
            key_digest: format!("key-{command}"),
            expires_at: i64::MAX,
        },
        request_digest: RequestDigest(format!("digest-{command}")),
        new_operation_id: OperationId(format!("operation-{command}")),
        payload: CommandPayload {
            command_key: "create".to_owned(),
            principal_ref: principal(),
            instance_id: InstanceId("instance-1".to_owned()),
            canonical_target: target,
            cas: None,
            content: None,
            semantic_options: serde_json::Value::Null,
        },
    }
}

#[test]
fn mutation_create_bot_produces_main_conversation_and_receipt() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    let request = create_bot_request("command-1", "key-1", "digest-1", "operation-1", "alpha-bot");
    let result = mutator.mutate(&request)?;
    assert_eq!(result.command_id, request.command_id);
    assert_eq!(result.operation_id, request.new_operation_id);
    assert_eq!(result.receipt.disposition, ReceiptDisposition::Committed);
    assert_eq!(result.receipt.resolved_binding_digest, "digest-1");
    assert!(!result.operation_may_continue);
    let committed = result
        .committed_payload
        .as_ref()
        .ok_or_else(|| AppError::Internal("missing committed payload".to_owned()))?;
    assert_eq!(committed["bot_ref"], "bot:alpha-bot");
    assert_eq!(
        committed["main_conversation_ref"],
        "conversation:alpha-bot:main"
    );
    let state = mutator.snapshot().expect("snapshot must succeed");
    assert_eq!(state.bot_count(), 1);
    assert_eq!(state.conversation_count(), 1);
    Ok(())
}

#[test]
fn mutation_exact_retry_returns_existing_without_duplicate_effect() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    let request = create_bot_request("command-1", "key-1", "digest-1", "operation-1", "alpha-bot");
    let first = mutator.mutate(&request)?;
    assert_eq!(mutator.mutate(&request)?, first);
    assert_eq!(mutator.snapshot().expect("snapshot").bot_count(), 1);
    Ok(())
}

#[test]
fn mutation_replay_identity_is_exact() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    let original = create_bot_request("command-1", "key-1", "digest-1", "operation-1", "alpha-bot");
    mutator.mutate(&original)?;

    let mut digest = original.clone();
    digest.request_digest = RequestDigest("other".to_owned());
    assert!(matches!(
        mutator.mutate(&digest),
        Err(AppError::Conflict(_))
    ));

    let mut operation = original.clone();
    operation.new_operation_id = OperationId("other-operation".to_owned());
    assert!(matches!(
        mutator.mutate(&operation),
        Err(AppError::Conflict(_))
    ));

    let mut expiry = original.clone();
    expiry.idempotency_key.expires_at -= 1;
    assert!(matches!(
        mutator.mutate(&expiry),
        Err(AppError::Conflict(_))
    ));
    Ok(())
}

#[test]
fn mutation_single_sided_bindings_conflict() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    mutator.mutate(&create_bot_request(
        "command-1",
        "key-1",
        "digest-1",
        "operation-1",
        "alpha-bot",
    ))?;
    assert!(matches!(
        mutator.mutate(&create_bot_request(
            "command-1",
            "key-2",
            "digest-1",
            "operation-2",
            "beta-bot"
        )),
        Err(AppError::Conflict(_))
    ));
    assert!(matches!(
        mutator.mutate(&create_bot_request(
            "command-2",
            "key-1",
            "digest-2",
            "operation-2",
            "beta-bot"
        )),
        Err(AppError::Conflict(_))
    ));
    Ok(())
}

#[test]
fn mutation_stale_revision_and_missing_non_create_bot_fail_closed() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    assert!(matches!(
        mutator.mutate(&bot_update("missing", "op-missing", 0, "bot-deactivate")),
        Err(AppError::NotFound(_))
    ));
    assert!(mutator.snapshot().expect("snapshot").bots.is_empty());

    mutator.mutate(&create_bot_request(
        "create",
        "create-key",
        "create-digest",
        "create-op",
        "alpha-bot",
    ))?;
    assert!(matches!(
        mutator.mutate(&bot_update("stale", "stale-op", 0, "bot-deactivate")),
        Err(AppError::Conflict(_))
    ));
    Ok(())
}

#[test]
fn mutation_unknown_bot_command_does_not_advance_revision() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    mutator.mutate(&create_bot_request(
        "create",
        "create-key",
        "create-digest",
        "create-op",
        "alpha-bot",
    ))?;
    assert!(matches!(
        mutator.mutate(&bot_update("unknown", "unknown-op", 1, "bot-unknown")),
        Err(AppError::Conflict(_))
    ));
    let state = mutator.snapshot().expect("snapshot");
    let bot = state
        .bots
        .get(&dxbot_core::types::BotId("alpha-bot".to_owned()))
        .expect("bot exists");
    assert_eq!(bot.revision, 1);
    Ok(())
}

#[test]
fn mutation_does_not_fabricate_missing_relationships() {
    let mutator = ApplicationMutator::new();
    let conversation = relation_request(
        "conversation-create",
        CanonicalTarget::Conversation {
            id: ConversationId("orphan".to_owned()),
            revision: 0,
        },
    );
    assert!(matches!(
        mutator.mutate(&conversation),
        Err(AppError::NotFound(_))
    ));
    let thread = relation_request(
        "thread-create",
        CanonicalTarget::Thread {
            id: ThreadId("orphan-thread".to_owned()),
            parent_id: None,
            revision: 0,
        },
    );
    assert!(matches!(
        mutator.mutate(&thread),
        Err(AppError::NotFound(_))
    ));
}

#[test]
fn resolve_outcome_semantics_remain_stable() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    mutator.mutate(&create_bot_request(
        "create",
        "key",
        "digest",
        "operation",
        "alpha-bot",
    ))?;
    let state = mutator.snapshot().expect("snapshot");
    let target = CanonicalTarget::Bot {
        id: dxbot_core::types::BotId("alpha-bot".to_owned()),
        revision: 1,
    };
    assert_eq!(
        resolve_outcome(&state, &target, &None),
        DomainOutcome::Updated
    );
    assert_eq!(
        resolve_outcome(&state, &target, &Some(cas_if_revision(0))),
        DomainOutcome::Conflict
    );
    let absent = CanonicalTarget::Bot {
        id: dxbot_core::types::BotId("missing".to_owned()),
        revision: 0,
    };
    assert_eq!(
        resolve_outcome(&state, &absent, &None),
        DomainOutcome::Created
    );
    assert_eq!(
        resolve_outcome(&state, &absent, &Some(cas_if_revision(1))),
        DomainOutcome::NotFound
    );
    Ok(())
}
