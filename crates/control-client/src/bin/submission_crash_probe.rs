//! Executable crash fixture for the submission protocol.
//!
//! Runs a genuine submission in a child process with an explicit deterministic
//! transport and provokes a real `std::process::exit` at the requested
//! [`CrashPoint`] boundary.

use std::process::ExitCode;

use control_client::{CrashPoint, SubmissionClient};
use dxbot_core::receipt::{ReceiptDisposition, ReceiptRecord};
use dxbot_core::types::{
    CanonicalTarget, CommandId, CommandPayload, IdempotencyKey, InstanceId, OperationRequest,
    OperationResult, PrincipalRef, RequestDigest,
};

fn main() -> ExitCode {
    let Some(mode) = std::env::args().nth(1) else {
        return ExitCode::FAILURE;
    };
    let crash_point = match mode.as_str() {
        "before-commit" => CrashPoint::BeforeCommit,
        "after-commit" => CrashPoint::AfterCommitBeforeResponse,
        "before-dispatching" => CrashPoint::BeforeDispatching,
        "after-dispatching" => CrashPoint::AfterDispatchingBeforeSend,
        _ => return ExitCode::FAILURE,
    };

    let request = request();
    let client = SubmissionClient::builder(InstanceId("instance-1".to_owned()))
        .with_hard_crash(true)
        .with_transport(Box::new(committed_result))
        .build();
    let _ = client.submit_with_crash_point(&request, crash_point);

    ExitCode::from(99)
}

fn request() -> OperationRequest {
    OperationRequest {
        command_id: CommandId("command-a".to_owned()),
        idempotency_key: IdempotencyKey {
            principal_ref: PrincipalRef("principal-a".to_owned()),
            key_digest: "key-a".to_owned(),
            expires_at: 100,
        },
        request_digest: RequestDigest("request-a".to_owned()),
        new_operation_id: dxbot_core::types::OperationId("operation-a".to_owned()),
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

fn committed_result(request: &OperationRequest) -> OperationResult {
    let operation_id = request.new_operation_id.clone();
    OperationResult {
        operation_id: operation_id.clone(),
        command_id: request.command_id.clone(),
        instance_id: request.payload.instance_id.clone(),
        receipt: ReceiptRecord {
            operation_id: operation_id.0.clone(),
            disposition: ReceiptDisposition::Committed,
            result_ref: format!("result:{}", operation_id.0),
            resolved_binding_digest: request.request_digest.0.clone(),
            owner_kind: "crash-fixture".to_owned(),
            lease_until: None,
            last_progress: 0,
            reconciliation_policy: "at-least-once".to_owned(),
        },
        status: "committed".to_owned(),
        committed_payload: None,
        error: None,
        operation_may_continue: false,
    }
}
