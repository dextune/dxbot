//! Acceptance tests for AT-APP-005: mutation materialization, complete binding
//! identity, receipt, and domain-outcome semantics.

#![allow(clippy::expect_used)]

use application::mutation::AppError;
use application::{ApplicationMutator, DomainOutcome, cas_if_revision, resolve_outcome};
use dxbot_core::receipt::ReceiptDisposition;
use dxbot_core::types::{
    CanonicalTarget, CommandId, CommandPayload, ConversationId, IdempotencyKey, InstanceId,
    OperationId, OperationRequest, PrincipalRef, RequestDigest, ThreadId,
};

fn principal() -> PrincipalRef {
    PrincipalRef("http:principal-a".to_string())
}

fn create_bot_request(
    command_id: &str,
    key: &str,
    digest: &str,
    operation_id: &str,
    name: &str,
) -> OperationRequest {
    OperationRequest {
        command_id: CommandId(command_id.to_string()),
        idempotency_key: IdempotencyKey {
            principal_ref: principal(),
            key_digest: key.to_string(),
            expires_at: i64::MAX,
        },
        request_digest: RequestDigest(digest.to_string()),
        new_operation_id: OperationId(operation_id.to_string()),
        payload: CommandPayload {
            command_key: "bot-create".to_string(),
            principal_ref: principal(),
            instance_id: InstanceId("instance-1".to_string()),
            canonical_target: CanonicalTarget::Instance(InstanceId("instance-1".to_string())),
            cas: None,
            content: None,
            semantic_options: serde_json::json!({ "name": name }),
        },
    }
}

fn update_request(command_id: &str, operation_id: &str, revision: i64) -> OperationRequest {
    OperationRequest {
        command_id: CommandId(command_id.to_string()),
        idempotency_key: IdempotencyKey {
            principal_ref: principal(),
            key_digest: format!("key-{command_id}"),
            expires_at: i64::MAX,
        },
        request_digest: RequestDigest(format!("digest-{command_id}")),
        new_operation_id: OperationId(operation_id.to_string()),
        payload: CommandPayload {
            command_key: "bot-deactivate".to_string(),
            principal_ref: principal(),
            instance_id: InstanceId("instance-1".to_string()),
            canonical_target: CanonicalTarget::Bot {
                id: dxbot_core::types::BotId("alpha-bot".to_string()),
                revision,
            },
            cas: Some(cas_if_revision(revision)),
            content: None,
            semantic_options: serde_json::Value::Null,
        },
    }
}

fn relation_request(command_id: &str, target: CanonicalTarget) -> OperationRequest {
    OperationRequest {
        command_id: CommandId(command_id.to_owned()),
        idempotency_key: IdempotencyKey {
            principal_ref: principal(),
            key_digest: format!("key-{command_id}"),
            expires_at: i64::MAX,
        },
        request_digest: RequestDigest(format!("digest-{command_id}")),
        new_operation_id: OperationId(format!("operation-{command_id}")),
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
fn mutation_create_bot_produces_bot_ref_and_main_conversation() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    let request = create_bot_request(
        "command-1",
        "key-1",
        "digest-1",
        "operation-1",
        "alpha-bot",
    );
    let result = mutator.mutate(&request)?;

    assert_eq!(result.command_id, request.command_id);
    assert_eq!(result.operation_id, request.new_operation_id);
    assert_eq!(result.status, "committed");
    assert_eq!(result.error, None);
    assert_eq!(result.receipt.disposition, ReceiptDisposition::Committed);
    assert!(!result.operation_may_continue);

    let committed = result
        .committed_payload
        .as_ref()
        .ok_or_else(|| AppError::Internal("missing committed payload".to_string()))?;
    assert_eq!(committed["bot_ref"], "bot:alpha-bot");
    assert_eq!(
        committed["main_conversation_ref"],
        "conversation:alpha-bot:main"
    );
    assert_eq!(committed["bot_revision"], 1);

    let state = mutator.snapshot().expect("snapshot must succeed");
    assert_eq!(state.bot_count(), 1);
    assert_eq!(state.conversation_count(), 1);
    Ok(())
}

#[test]
fn mutation_stale_revision_is_conflict() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    mutator.mutate(&create_bot_request(
        "command-1",
        "key-1",
        "digest-1",
        "operation-1",
        "alpha-bot",
    ))?;

    let stale = update_request("command-2", "operation-2", 0);
    assert!(matches!(
        mutator.mutate(&stale),
        Err(AppError::Conflict(_))
    ));
    Ok(())
}

#[test]
fn mutation_receipt_is_retrievable() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    let result = mutator.mutate(&create_bot_request(
        "command-1",
        "key-1",
        "digest-1",
        "operation-1",
        "alpha-bot",
    ))?;
    let receipt = mutator
        .get_receipt(&result.operation_id)?
        .ok_or_else(|| AppError::Internal("receipt must be retrievable".to_string()))?;

    assert_eq!(receipt.operation_id, "operation-1");
    assert_eq!(receipt.disposition, ReceiptDisposition::Committed);
    assert_eq!(receipt.resolved_binding_digest, "digest-1");
    assert!(
        mutator
            .get_receipt(&OperationId("op-missing".to_string()))?
            .is_none()
    );
    Ok(())
}

#[test]
fn mutation_exact_retry_returns_existing() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    let request = create_bot_request(
        "command-1",
        "key-1",
        "digest-1",
        "operation-1",
        "alpha-bot",
    );
    let first = mutator.mutate(&request)?;
    let second = mutator.mutate(&request)?;

    assert_eq!(second, first);
    let state = mutator.snapshot().expect("snapshot must succeed");
    assert_eq!(state.bot_count(), 1);
    assert_eq!(state.conversation_count(), 1);
    Ok(())
}

#[test]
fn mutation_distinct_commands_of_same_kind_are_independent() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    mutator.mutate(&create_bot_request(
        "command-1",
        "key-1",
        "digest-1",
        "operation-1",
        "alpha-bot",
    ))?;
    mutator.mutate(&create_bot_request(
        "command-2",
        "key-2",
        "digest-2",
        "operation-2",
        "beta-bot",
    ))?;

    let state = mutator.snapshot().expect("snapshot must succeed");
    assert_eq!(state.bot_count(), 2);
    assert_eq!(state.conversation_count(), 2);
    Ok(())
}

#[test]
fn mutation_same_command_different_digest_conflicts() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    let original = create_bot_request(
        "command-1",
        "key-1",
        "digest-1",
        "operation-1",
        "alpha-bot",
    );
    mutator.mutate(&original)?;

    let mut conflicting = original.clone();
    conflicting.request_digest = RequestDigest("digest-other".to_string());
    assert!(matches!(
        mutator.mutate(&conflicting),
        Err(AppError::Conflict(_))
    ));
    Ok(())
}

#[test]
fn mutation_same_binding_different_operation_id_conflicts() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    let original = create_bot_request(
        "command-1",
        "key-1",
        "digest-1",
        "operation-1",
        "alpha-bot",
    );
    mutator.mutate(&original)?;

    let mut conflicting = original.clone();
    conflicting.new_operation_id = OperationId("operation-other".to_owned());
    assert!(matches!(
        mutator.mutate(&conflicting),
        Err(AppError::Conflict(_))
    ));
    Ok(())
}

#[test]
fn mutation_same_key_digest_different_expiry_still_conflicts() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    mutator.mutate(&create_bot_request(
        "command-1",
        "key-1",
        "digest-1",
        "operation-1",
        "alpha-bot",
    ))?;

    let mut changed = create_bot_request(
        "command-2",
        "key-1",
        "digest-2",
        "operation-2",
        "beta-bot",
    );
    changed.idempotency_key.expires_at = i64::MAX - 1;
    assert!(matches!(
        mutator.mutate(&changed),
        Err(AppError::Conflict(_))
    ));

    let state = mutator.snapshot().expect("snapshot must succeed");
    assert_eq!(state.bot_count(), 1);
    Ok(())
}

#[test]
fn mutation_single_sided_binding_conflicts() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    let original = create_bot_request(
        "command-1",
        "key-1",
        "digest-1",
        "operation-1",
        "alpha-bot",
    );
    mutator.mutate(&original)?;

    let same_command_new_key = create_bot_request(
        "command-1",
        "key-2",
        "digest-1",
        "operation-2",
        "beta-bot",
    );
    assert!(matches!(
        mutator.mutate(&same_command_new_key),
        Err(AppError::Conflict(_))
    ));

    let new_command_same_key = create_bot_request(
        "command-2",
        "key-1",
        "digest-2",
        "operation-2",
        "beta-bot",
    );
    assert!(matches!(
        mutator.mutate(&new_command_same_key),
        Err(AppError::Conflict(_))
    ));
    Ok(())
}

#[test]
fn mutation_does_not_fabricate_missing_relationships() {
    let mutator = ApplicationMutator::new();

    let conversation = relation_request(
        "conversation-create",
        CanonicalTarget::Conversation {
            id: ConversationId("conversation-orphan".to_owned()),
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
            id: ThreadId("thread-orphan".to_owned()),
            parent_id: None,
            revision: 0,
        },
    );
    assert!(matches!(mutator.mutate(&thread), Err(AppError::NotFound(_))));
}

#[test]
fn resolve_outcome_semantics() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    mutator.mutate(&create_bot_request(
        "command-1",
        "key-1",
        "digest-1",
        "operation-1",
        "alpha-bot",
    ))?;
    let state = mutator.snapshot().expect("snapshot must succeed");
    let target = CanonicalTarget::Bot {
        id: dxbot_core::types::BotId("alpha-bot".to_string()),
        revision: 1,
    };

    assert_eq!(resolve_outcome(&state, &target, &None), DomainOutcome::Updated);
    assert_eq!(
        resolve_outcome(&state, &target, &Some(cas_if_revision(1))),
        DomainOutcome::Updated
    );
    assert_eq!(
        resolve_outcome(&state, &target, &Some(cas_if_revision(0))),
        DomainOutcome::Conflict
    );

    let absent = CanonicalTarget::Bot {
        id: dxbot_core::types::BotId("bot-nope".to_string()),
        revision: 0,
    };
    assert_eq!(resolve_outcome(&state, &absent, &None), DomainOutcome::Created);
    assert_eq!(
        resolve_outcome(&state, &absent, &Some(cas_if_revision(1))),
        DomainOutcome::NotFound
    );
    Ok(())
}
