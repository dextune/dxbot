//! Acceptance tests for AT-APP-005: mutation target materialization, receipt,
//! and domain-outcome semantics.
//!
//! Covered in-process with the in-memory [`DomainState`] fixture (no external
//! I/O): CreateBot materialization, stale-revision conflict, receipt retrieval,
//! and idempotent (same CommandId) playback.

use application::mutation::AppError;
use application::{ApplicationMutator, DomainOutcome, cas_if_revision, resolve_outcome};
use dxbot_core::receipt::ReceiptDisposition;
use dxbot_core::types::{CanonicalTarget, CommandPayload, InstanceId, PrincipalRef};

/// Build a `CreateBot` command payload for a given bot id/name.
fn create_bot_payload(bot_id: &str, name: &str) -> CommandPayload {
    CommandPayload {
        command_key: "bot-create".to_string(),
        principal_ref: PrincipalRef("http:principal-a".to_string()),
        instance_id: InstanceId("instance-1".to_string()),
        canonical_target: CanonicalTarget::Bot {
            id: dxbot_core::types::BotId(bot_id.to_string()),
            revision: 0,
        },
        cas: None,
        content: None,
        semantic_options: serde_json::json!({ "name": name }),
    }
}

#[test]
fn mutation_create_bot_produces_bot_ref_and_main_conversation() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    let result = mutator.mutate(&create_bot_payload("bot-1", "alpha-bot"))?;

    assert_eq!(result.status, "committed");
    assert_eq!(result.error, None);
    assert_eq!(result.receipt.disposition, ReceiptDisposition::Committed);
    assert!(!result.operation_may_continue);

    let committed = result
        .committed_payload
        .as_ref()
        .ok_or_else(|| AppError::Internal("missing committed payload".to_string()))?;
    let bot_ref = committed
        .get("bot_ref")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Internal("missing bot_ref".to_string()))?;
    let main_conv = committed
        .get("main_conversation_ref")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Internal("missing main_conversation_ref".to_string()))?;
    assert_eq!(bot_ref, "bot:bot-1");
    assert_eq!(main_conv, "conversation:main-bot-1");
    assert_eq!(
        committed.get("bot_revision").and_then(|v| v.as_i64()),
        Some(1)
    );

    // The domain state actually materialized both identities.
    let state = mutator.snapshot().expect("snapshot must succeed");
    assert_eq!(state.bot_count(), 1);
    assert!(
        state
            .bots
            .contains_key(&dxbot_core::types::BotId("bot-1".to_string()))
    );
    assert_eq!(state.conversation_count(), 1);
    Ok(())
}

#[test]
fn mutation_stale_revision_is_conflict() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    mutator.mutate(&create_bot_payload("bot-1", "alpha-bot"))?;

    // A stale update expecting revision 0 while the bot is at revision 1.
    let stale = CommandPayload {
        command_key: "bot-deactivate".to_string(),
        principal_ref: PrincipalRef("http:principal-a".to_string()),
        instance_id: InstanceId("instance-1".to_string()),
        canonical_target: CanonicalTarget::Bot {
            id: dxbot_core::types::BotId("bot-1".to_string()),
            revision: 0,
        },
        cas: Some(cas_if_revision(0)),
        content: None,
        semantic_options: serde_json::Value::Null,
    };

    assert!(matches!(
        mutator.mutate(&stale).err(),
        Some(AppError::Conflict(_))
    ));
    Ok(())
}

#[test]
fn mutation_receipt_is_retrievable() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    let result = mutator.mutate(&create_bot_payload("bot-1", "alpha-bot"))?;

    let receipt = mutator
        .get_receipt(&result.operation_id)?
        .ok_or_else(|| AppError::Internal("receipt must be retrievable".to_string()))?;

    assert_eq!(receipt.operation_id, result.operation_id.0);
    assert_eq!(receipt.disposition, ReceiptDisposition::Committed);
    assert!(!receipt.result_ref.is_empty());
    assert!(!receipt.resolved_binding_digest.is_empty());

    // Unknown operation ids yield None, not an error.
    let missing = mutator.get_receipt(&dxbot_core::types::OperationId("op-missing".to_string()))?;
    assert!(missing.is_none());
    Ok(())
}

#[test]
fn mutation_idempotent_returns_existing() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    let payload = create_bot_payload("bot-1", "alpha-bot");

    let first = mutator.mutate(&payload)?;

    // Same command identity -> replay the existing outcome, no new effect.
    let second = mutator.mutate(&payload)?;

    assert_eq!(second.operation_id, first.operation_id);
    assert_eq!(second.command_id, first.command_id);
    assert_eq!(second.receipt, first.receipt);

    // Exactly one bot and one main conversation were materialized.
    let state = mutator.snapshot().expect("snapshot must succeed");
    assert_eq!(state.bot_count(), 1);
    assert_eq!(state.conversation_count(), 1);
    Ok(())
}

#[test]
fn mutation_same_command_different_digest_conflicts() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    mutator.mutate(&create_bot_payload("bot-1", "alpha-bot"))?;

    // Same command key but different semantic content -> different binding.
    let different = create_bot_payload("bot-2", "beta-bot");
    assert!(matches!(
        mutator.mutate(&different).err(),
        Some(AppError::Conflict(_))
    ));
    Ok(())
}

#[test]
fn resolve_outcome_semantics() -> Result<(), AppError> {
    let mutator = ApplicationMutator::new();
    mutator.mutate(&create_bot_payload("bot-1", "alpha-bot"))?;
    let state = mutator.snapshot().expect("snapshot must succeed");
    let target = CanonicalTarget::Bot {
        id: dxbot_core::types::BotId("bot-1".to_string()),
        revision: 1,
    };

    assert_eq!(
        resolve_outcome(&state, &target, &None),
        DomainOutcome::Updated
    );
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
