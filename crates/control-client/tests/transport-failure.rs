//! Adversarial transport-error coverage for the durable submission boundary.

#![allow(clippy::unwrap_used)]

use control_client::{ClientError, SubmissionClient};
use dxbot_core::types::{
    CanonicalTarget, CommandId, CommandPayload, IdempotencyKey, InstanceId, OperationId,
    OperationRequest, PrincipalRef, RequestDigest,
};

fn request() -> OperationRequest {
    let principal = PrincipalRef("principal-a".to_owned());
    let instance = InstanceId("instance-1".to_owned());
    OperationRequest {
        command_id: CommandId("command-transport-failure".to_owned()),
        idempotency_key: IdempotencyKey {
            principal_ref: principal.clone(),
            key_digest: "key-transport-failure".to_owned(),
            expires_at: 100,
        },
        request_digest: RequestDigest("digest-transport-failure".to_owned()),
        new_operation_id: OperationId("operation-transport-failure".to_owned()),
        payload: CommandPayload {
            command_key: "bot-create".to_owned(),
            principal_ref: principal,
            instance_id: instance.clone(),
            canonical_target: CanonicalTarget::Instance(instance),
            cas: None,
            content: None,
            semantic_options: serde_json::json!({"name": "alpha"}),
        },
    }
}

#[test]
fn transport_error_never_becomes_a_success_result() {
    let request = request();
    let client = SubmissionClient::builder(InstanceId("instance-1".to_owned()))
        .with_fallible_transport(Box::new(|_| {
            Err(ClientError::Transport("connection reset".to_owned()))
        }))
        .build();

    let error = client.submit(&request).unwrap_err();
    assert_eq!(error, ClientError::Transport("connection reset".to_owned()));

    let recovery = client
        .lookup_operation(&request.command_id, &request.idempotency_key)
        .unwrap()
        .expect("dispatch identity must remain recoverable after transport loss");
    assert_eq!(recovery.operation_id, request.new_operation_id);
    assert_eq!(recovery.status, "recovery-required");
    assert!(recovery.operation_may_continue);
}
