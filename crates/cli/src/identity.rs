//! Replay identity projection for one materialized Application Command.
//!
//! CommandId, OperationId, IdempotencyKey and RequestDigest are generated once
//! before Prepared and then reused unchanged by the local journal and control
//! transport. IDs are uniqueness tokens, not secrets.
//!
//! RequestDigest deliberately excludes local discovery/render/wait state, but
//! binds the raw typed selector independently from the materialized payload so
//! selector intent cannot drift while resolving to the same canonical target.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use application_contract::{CliInput, LOCAL_CONTROL_PROTOCOL_VERSION, LOCAL_CONTROL_SCHEMA_VERSION};
use dxbot_core::DxbotError;
use dxbot_core::error::{ErrorCategory, ErrorCode};
use dxbot_core::types::{
    CommandId, CommandPayload, IdempotencyKey, JournalRecord, OperationId, OperationRequest,
    RequestDigest,
};
use serde::Serialize;
use serde_json::Value;

use crate::sha256::sha256_hex;

const IDEMPOTENCY_TTL_SECONDS: i64 = 24 * 60 * 60;
static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Serialize)]
struct DigestEnvelope<'a> {
    protocol_version: &'static str,
    schema_version: &'static str,
    raw_selector: &'a Option<Value>,
    payload: &'a CommandPayload,
}

pub fn build_operation_request(
    input: &CliInput,
    payload: CommandPayload,
) -> Result<OperationRequest, DxbotError> {
    let seed = unique_seed()?;
    let command_id = CommandId(format!(
        "cmd-{}",
        sha256_hex(format!("command:{seed}").as_bytes())
    ));
    let operation_id = OperationId(format!(
        "op-{}",
        sha256_hex(format!("operation:{seed}").as_bytes())
    ));
    let request_digest = request_digest_for_input(input, &payload)?;
    let key_digest = sha256_hex(
        format!(
            "idempotency:{}:{}:{}:{seed}",
            payload.principal_ref.0, command_id.0, request_digest.0
        )
        .as_bytes(),
    );
    let now = unix_seconds()?;
    let expires_at = now
        .checked_add(IDEMPOTENCY_TTL_SECONDS)
        .ok_or_else(|| local_error("system clock cannot represent idempotency expiry"))?;

    Ok(OperationRequest {
        command_id,
        idempotency_key: IdempotencyKey {
            principal_ref: payload.principal_ref.clone(),
            key_digest,
            expires_at,
        },
        request_digest,
        new_operation_id: operation_id,
        payload,
    })
}

/// Reconstruct the exact operation request represented by a surviving journal
/// record using a fresh invocation only as payload material. The fresh digest,
/// Instance and Principal must all match before durable IDs are reused.
pub fn build_replay_request(
    input: &CliInput,
    payload: CommandPayload,
    record: &JournalRecord,
) -> Result<OperationRequest, DxbotError> {
    let request_digest = request_digest_for_input(input, &payload)?;
    if request_digest != record.request_digest {
        return Err(recovery_error(format!(
            "fresh invocation does not match durable request digest for {}",
            record.command_id.0
        )));
    }
    if payload.instance_id != record.instance_id {
        return Err(recovery_error(format!(
            "fresh invocation Instance does not match durable command {}",
            record.command_id.0
        )));
    }
    if payload.principal_ref != record.idempotency_key.principal_ref {
        return Err(recovery_error(format!(
            "fresh invocation Principal does not match durable command {}",
            record.command_id.0
        )));
    }
    Ok(OperationRequest {
        command_id: record.command_id.clone(),
        idempotency_key: record.idempotency_key.clone(),
        request_digest,
        new_operation_id: record.operation_id.clone(),
        payload,
    })
}

pub fn request_digest_for_input(
    input: &CliInput,
    payload: &CommandPayload,
) -> Result<RequestDigest, DxbotError> {
    let envelope = DigestEnvelope {
        protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION,
        schema_version: LOCAL_CONTROL_SCHEMA_VERSION,
        raw_selector: &input.selector,
        payload,
    };
    let canonical = serde_json::to_vec(&envelope)
        .map_err(|error| local_error(format!("cannot canonicalize request payload: {error}")))?;
    Ok(RequestDigest(sha256_hex(&canonical)))
}

fn unique_seed() -> Result<String, DxbotError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| local_error(format!("system clock precedes Unix epoch: {error}")))?
        .as_nanos();
    let counter = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    Ok(format!("{}:{nanos}:{counter}", std::process::id()))
}

fn unix_seconds() -> Result<i64, DxbotError> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| local_error(format!("system clock precedes Unix epoch: {error}")))?
        .as_secs();
    i64::try_from(seconds).map_err(|_| local_error("system time exceeds i64 range"))
}

fn recovery_error(message: impl Into<String>) -> DxbotError {
    DxbotError {
        code: ErrorCode::RecoveryRequired,
        category: ErrorCategory::Recovery,
        message: message.into(),
        retryable: false,
        operation_ref: None,
        target_refs: Vec::new(),
        field_violations: Vec::new(),
        current_revision: None,
        current_generation: None,
        resume_cursor: None,
        next_actions: Vec::new(),
    }
}

fn local_error(message: impl Into<String>) -> DxbotError {
    DxbotError {
        code: ErrorCode::InternalInvariant,
        category: ErrorCategory::Local,
        message: message.into(),
        retryable: false,
        operation_ref: None,
        target_refs: Vec::new(),
        field_violations: Vec::new(),
        current_revision: None,
        current_generation: None,
        resume_cursor: None,
        next_actions: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use dxbot_core::types::{CanonicalTarget, InstanceId, JournalState, PrincipalRef};
    use serde_json::json;

    use super::*;

    fn input(selector: &str) -> CliInput {
        CliInput {
            command_key: "bot-show".to_owned(),
            global_options: Default::default(),
            selector: Some(json!({"kind": "bot", "value": selector})),
            cas: None,
            content: None,
            fields: json!({}),
        }
    }

    fn payload() -> CommandPayload {
        let instance = InstanceId("instance-a".to_owned());
        CommandPayload {
            command_key: "bot-create".to_owned(),
            principal_ref: PrincipalRef("local:instance-a:uid:1000".to_owned()),
            instance_id: instance.clone(),
            canonical_target: CanonicalTarget::Instance(instance),
            cas: None,
            content: None,
            semantic_options: json!({"name": "alpha"}),
        }
    }

    #[test]
    fn request_digest_is_stable_for_same_materialized_input() {
        let input = input("alpha");
        let payload = payload();
        let first = request_digest_for_input(&input, &payload).expect("digest computes");
        let second = request_digest_for_input(&input, &payload).expect("digest computes");
        assert_eq!(first, second);
    }

    #[test]
    fn request_digest_binds_raw_selector_even_when_payload_is_same() {
        let payload = payload();
        let first = request_digest_for_input(&input("alpha"), &payload).expect("digest computes");
        let second = request_digest_for_input(&input("bot:alpha"), &payload).expect("digest computes");
        assert_ne!(first, second);
    }

    #[test]
    fn generated_operation_identity_is_unique_and_principal_bound() {
        let input = input("alpha");
        let first = build_operation_request(&input, payload()).expect("request builds");
        let second = build_operation_request(&input, payload()).expect("request builds");
        assert_ne!(first.command_id, second.command_id);
        assert_ne!(first.new_operation_id, second.new_operation_id);
        assert_ne!(first.idempotency_key.key_digest, second.idempotency_key.key_digest);
        assert_eq!(
            first.idempotency_key.principal_ref,
            first.payload.principal_ref
        );
        assert_eq!(first.request_digest, second.request_digest);
    }

    #[test]
    fn replay_request_reuses_all_durable_identity() {
        let input = input("alpha");
        let payload = payload();
        let original = build_operation_request(&input, payload.clone()).expect("request builds");
        let record = JournalRecord {
            state: JournalState::Dispatching,
            instance_id: original.payload.instance_id.clone(),
            command_id: original.command_id.clone(),
            operation_id: original.new_operation_id.clone(),
            idempotency_key: original.idempotency_key.clone(),
            request_digest: original.request_digest.clone(),
            sequence: 2,
            previous_digest: "previous".to_owned(),
            record_digest: "record".to_owned(),
        };
        let replay = build_replay_request(&input, payload, &record).expect("replay builds");
        assert_eq!(replay.command_id, original.command_id);
        assert_eq!(replay.new_operation_id, original.new_operation_id);
        assert_eq!(replay.idempotency_key, original.idempotency_key);
        assert_eq!(replay.request_digest, original.request_digest);
    }
}
