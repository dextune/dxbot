//! Executable crash fixture for the submission protocol.
//!
//! Runs a genuine submission in a child process and provokes a real
//! `std::process::exit` at the requested [`CrashPoint`] boundary. The exit
//! code lets the parent harness verify exactly where the process terminated.
//!
//! Usage: `submission-crash-probe <before-commit|after-commit|before-dispatching|after-dispatching>`

use std::process::ExitCode;

use control_client::{CrashPoint, SubmissionClient};
use dxbot_core::types::{
    CanonicalTarget, CommandId, CommandPayload, IdempotencyKey, InstanceId, OperationRequest,
    OperationId, PrincipalRef, RequestDigest,
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
        .build();
    // The submitted request intentionally carries an operation id that the
    // caller overrides on replay, mirroring the M1A storage fixture.
    let _ = client.submit_with_crash_point(&request, crash_point);

    // If we reach here, the configured crash point did not terminate the
    // process. Return a non-zero code that never collides with the documented
    // crash exit codes so the parent harness reports a missing crash.
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