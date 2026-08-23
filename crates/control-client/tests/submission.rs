//! Acceptance tests for `AT-SUBMIT-001` (durable submission + crash fixture).
//!
//! `cargo test -p control-client submission` runs every test below.

use std::error::Error;

use control_client::{ClientError, CrashPoint, SubmissionClient};
use dxbot_core::types::{
    CanonicalTarget, CommandId, CommandPayload, IdempotencyKey, InstanceId, OperationRequest,
    OperationId, PrincipalRef, RequestDigest,
};

// ── fixtures ──

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

/// A fresh client in *simulate* mode: crashes return `SimulatedCrash` and
/// leave the in-memory journal intact for inspection, so crash-boundary
/// invariants can be asserted deterministically in-process.
fn client() -> SubmissionClient {
    SubmissionClient::builder(InstanceId("instance-1".to_owned())).build()
}

// ── normal submission ──

#[test]
fn submission_normal_submits_and_returns_result() -> Result<(), Box<dyn Error>> {
    let client = client();
    let result = client.submit(&request())?;
    assert_eq!(result.command_id, CommandId("command-a".to_owned()));
    assert_eq!(result.operation_id, OperationId("operation-a".to_owned()));
    assert_eq!(result.status, "committed");
    // Prepared + Dispatching + Observed + Terminal were all recorded atomically.
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

    // No durable `Prepared` record escaped the crash boundary, so no
    // operation exists in the binding index.
    let found = client
        .lookup_operation(&request.command_id, &request.idempotency_key)
        .unwrap();
    assert!(found.is_none());
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

    // The operation was committed (durable `Prepared` + `Dispatching`) before
    // the crash, so the binding is recoverable via lookup rather than lost.
    let recovered = client
        .lookup_operation(&request.command_id, &request.idempotency_key)
        .unwrap()
        .expect("committed binding must be recoverable");
    assert_eq!(recovered.command_id, request.command_id);
    assert_eq!(recovered.status, "recovery-required");
    assert_eq!(client.journal_len(), 1);

    // Replay resolves the committed-but-unresolved binding to a terminal
    // result without needing a fresh submit.
    let resolved = client.replay_operation(&request).unwrap();
    assert_eq!(resolved.status, "committed");
    assert_eq!(resolved.operation_id, request.new_operation_id);
}

#[test]
fn submission_idempotent_retry_returns_existing() {
    let client = client();
    let request = request();

    let first = client.submit(&request).unwrap();
    // Same CommandId + IdempotencyKey is served from the existing binding.
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

    // Submit once with the original key.
    client.submit(&request).unwrap();

    // Submit again with the same CommandId but a different IdempotencyKey.
    let mut conflicting = request.clone();
    conflicting.idempotency_key.key_digest = "different-key".to_owned();

    let error = client.submit(&conflicting).unwrap_err();
    assert_eq!(
        error,
        ClientError::IdempotencyKeyConflict(request.command_id.clone())
    );
}

// ── crash fixture (real `std::process::exit`) via the probe binary ──

#[test]
fn submission_crash_probe_before_commit_exits_during_submit() {
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_submission-crash-probe"))
        .arg("before-commit")
        .status()
        .unwrap();
    assert!(!status.success());
    assert_eq!(
        status.code(),
        Some(CrashPoint::BeforeCommit.exit_code()),
        "probe must terminate at the configured crash boundary"
    );
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
        Some(CrashPoint::AfterCommitBeforeResponse.exit_code()),
        "probe must survive the commit and terminate at the response boundary"
    );
}