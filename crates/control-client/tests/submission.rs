//! Acceptance tests for `AT-SUBMIT-001` (durable submission + crash fixture).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::error::Error;

use control_client::{ClientError, CrashPoint, SubmissionClient};
use dxbot_core::types::{
    CanonicalTarget, CommandId, CommandPayload, IdempotencyKey, InstanceId, OperationRequest,
    OperationId, PrincipalRef, RequestDigest,
};

fn request() -> OperationRequest {
    OperationRequest {
        command_id: CommandId("command-a".to_owned()),
        idempotency_key: IdempotencyKey {
            principal_ref: PrincipalRef("principal-a".to_owned()),
            key_digest: "key-a".to_owned(),
            expires_at: 100,
        },
        request_digest: RequestDigest("request-a".to_owned()),
        new_operation_id: OperationId("operation-a".to_owned()),
        payload: CommandPayload {
            command_key: "cmd".to_owned(),
            principal_ref: PrincipalRef("principal-a".to_owned()),
            instance_id: InstanceId("instance-1".to_owned()),
            canonical_target: CanonicalTarget::Instance(InstanceId("instance-1".to_owned())),
            cas: None,
            content: None,
            semantic_options: serde_json::json!({}),
        },
    }
}

fn client() -> SubmissionClient {
    SubmissionClient::builder(InstanceId("instance-1".to_owned())).build()
}

#[test]
fn submission_normal_submits_and_returns_result() -> Result<(), Box<dyn Error>> {
    let client = client();
    let request = request();
    let result = client.submit(&request)?;
    assert_eq!(result.command_id, request.command_id);
    assert_eq!(result.operation_id, request.new_operation_id);
    assert_eq!(result.receipt.resolved_binding_digest, request.request_digest.0);
    assert_eq!(result.status, "committed");
    assert_eq!(client.journal_len(), 1);
    Ok(())
}

#[test]
fn submission_crash_before_commit_leaves_no_operation() {
    let client = client();
    let request = request();
    let error = client
        .submit_with_crash_point(&request, CrashPoint::BeforeCommit)
        .unwrap_err();
    assert_eq!(error, ClientError::SimulatedCrash(CrashPoint::BeforeCommit));
    assert!(client
        .lookup_operation(&request.command_id, &request.idempotency_key)
        .unwrap()
        .is_none());
    assert_eq!(client.journal_len(), 0);
}

#[test]
fn submission_crash_after_commit_is_recoverable() {
    let client = client();
    let request = request();
    let error = client
        .submit_with_crash_point(&request, CrashPoint::AfterCommitBeforeResponse)
        .unwrap_err();
    assert_eq!(
        error,
        ClientError::SimulatedCrash(CrashPoint::AfterCommitBeforeResponse)
    );

    let recovered = client
        .lookup_operation(&request.command_id, &request.idempotency_key)
        .unwrap()
        .expect("committed binding must be recoverable");
    assert_eq!(recovered.command_id, request.command_id);
    assert_eq!(recovered.status, "recovery-required");
    assert_eq!(
        recovered.receipt.resolved_binding_digest,
        request.request_digest.0
    );
    assert_eq!(client.journal_len(), 1);

    let resolved = client.replay_operation(&request).unwrap();
    assert_eq!(resolved.status, "committed");
    assert_eq!(resolved.operation_id, request.new_operation_id);
}

#[test]
fn submission_idempotent_retry_returns_existing() {
    let client = client();
    let request = request();
    let first = client.submit(&request).unwrap();
    let second = client.submit(&request).unwrap();
    assert_eq!(first, second);

    let looked_up = client
        .lookup_operation(&request.command_id, &request.idempotency_key)
        .unwrap()
        .expect("committed operation is visible to lookup");
    assert_eq!(looked_up, first);
}

#[test]
fn submission_same_command_different_key_is_conflict() {
    let client = client();
    let request = request();
    client.submit(&request).unwrap();

    let mut conflicting = request.clone();
    conflicting.idempotency_key.key_digest = "different-key".to_owned();
    let error = client.submit(&conflicting).unwrap_err();
    assert_eq!(
        error,
        ClientError::IdempotencyKeyConflict(request.command_id.clone())
    );
}

#[test]
fn submission_same_binding_different_request_digest_is_conflict() {
    let client = client();
    let request = request();
    client.submit(&request).unwrap();

    let mut conflicting = request.clone();
    conflicting.request_digest = RequestDigest("changed-request".to_owned());
    conflicting.new_operation_id = OperationId("changed-operation".to_owned());
    let error = client.submit(&conflicting).unwrap_err();
    assert_eq!(
        error,
        ClientError::RequestDigestConflict(request.command_id.clone())
    );
}

#[test]
fn submission_instance_mismatch_is_rejected_before_prepare() {
    let client = client();
    let mut request = request();
    request.payload.instance_id = InstanceId("instance-other".to_owned());

    let error = client.submit(&request).unwrap_err();
    assert_eq!(
        error,
        ClientError::InstanceMismatch {
            client: InstanceId("instance-1".to_owned()),
            request: InstanceId("instance-other".to_owned()),
        }
    );
    assert_eq!(client.journal_len(), 0);
}

#[test]
fn submission_crash_probe_before_commit_exits_during_submit() {
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_submission-crash-probe"))
        .arg("before-commit")
        .status()
        .unwrap();
    assert!(!status.success());
    assert_eq!(status.code(), Some(CrashPoint::BeforeCommit.exit_code()));
}

#[test]
fn submission_crash_probe_after_commit_exits_during_submit() {
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_submission-crash-probe"))
        .arg("after-commit")
        .status()
        .unwrap();
    assert!(!status.success());
    assert_eq!(
        status.code(),
        Some(CrashPoint::AfterCommitBeforeResponse.exit_code())
    );
}
