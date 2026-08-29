//! Adversarial coverage for the owner-verified local Unix control client.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::time::Duration;

use application_contract::{
    LOCAL_CONTROL_PROTOCOL_VERSION, LOCAL_CONTROL_SCHEMA_VERSION, LocalControlHandshake,
    LocalControlRequest, LocalControlResponse, read_local_control_frame, write_local_control_frame,
};
use control_client::{ClientError, LocalControlClient};
use dxbot_core::receipt::{ReceiptDisposition, ReceiptRecord};
use dxbot_core::types::{
    CanonicalTarget, CommandId, CommandPayload, IdempotencyKey, InstanceId, OperationId,
    OperationRequest, OperationResult, PrincipalRef, RequestDigest,
};

fn socket_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "dxbot-control-client-{}-{name}.sock",
        std::process::id()
    ))
}

#[test]
fn local_client_validates_owner_handshake_and_submit_identity() {
    let path = socket_path("roundtrip");
    let _ = fs::remove_file(&path);
    let listener = UnixListener::bind(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let uid = fs::metadata(&path).unwrap().uid();
    let instance = InstanceId("instance-a".to_owned());
    let principal = PrincipalRef(format!("local:{}:uid:{uid}", instance.0));
    let worker_instance = instance.clone();
    let worker_principal = principal.clone();

    let worker = std::thread::spawn(move || {
        for index in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_local_control_frame::<_, LocalControlRequest>(&mut stream).unwrap();
            match request {
                LocalControlRequest::Hello { hello } => {
                    assert_eq!(hello.instance_id, worker_instance);
                    assert_eq!(hello.host_generation, 9);
                }
                other => panic!("expected hello, got {other:?}"),
            }
            write_local_control_frame(
                &mut stream,
                &LocalControlResponse::Handshake {
                    handshake: LocalControlHandshake {
                        instance_id: worker_instance.clone(),
                        host_generation: 9,
                        principal_ref: worker_principal.clone(),
                        protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_owned(),
                        schema_version: LOCAL_CONTROL_SCHEMA_VERSION.to_owned(),
                    },
                },
            )
            .unwrap();

            if index == 1 {
                let request =
                    read_local_control_frame::<_, LocalControlRequest>(&mut stream).unwrap();
                let LocalControlRequest::Submit { request } = request else {
                    panic!("expected submit");
                };
                write_local_control_frame(
                    &mut stream,
                    &LocalControlResponse::Operation {
                        result: committed_result(&request),
                    },
                )
                .unwrap();
            }
        }
    });

    let client = LocalControlClient::new(
        path.clone(),
        instance.clone(),
        9,
        uid,
        Duration::from_secs(1),
    )
    .unwrap();
    let handshake = client.handshake().unwrap();
    assert_eq!(handshake.principal_ref, principal);

    let request = operation_request(instance, handshake.principal_ref);
    let result = client.submit(&request).unwrap();
    assert_eq!(result.command_id, request.command_id);
    assert_eq!(result.operation_id, request.new_operation_id);
    worker.join().unwrap();
    let _ = fs::remove_file(path);
}

#[test]
fn local_client_rejects_regular_file_and_owner_mismatch_before_connect() {
    let path = socket_path("invalid");
    let _ = fs::remove_file(&path);
    fs::write(&path, b"not a socket").unwrap();
    let error = LocalControlClient::new(
        path.clone(),
        InstanceId("instance-a".to_owned()),
        1,
        fs::metadata(&path).unwrap().uid(),
        Duration::from_secs(1),
    )
    .unwrap_err();
    assert!(matches!(error, ClientError::Transport(_)));
    let _ = fs::remove_file(path);
}

fn operation_request(instance: InstanceId, principal: PrincipalRef) -> OperationRequest {
    OperationRequest {
        command_id: CommandId("command-a".to_owned()),
        idempotency_key: IdempotencyKey {
            principal_ref: principal.clone(),
            key_digest: "key-a".to_owned(),
            expires_at: i64::MAX,
        },
        request_digest: RequestDigest("digest-a".to_owned()),
        new_operation_id: OperationId("operation-a".to_owned()),
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

fn committed_result(request: &OperationRequest) -> OperationResult {
    OperationResult {
        operation_id: request.new_operation_id.clone(),
        command_id: request.command_id.clone(),
        instance_id: request.payload.instance_id.clone(),
        receipt: ReceiptRecord {
            operation_id: request.new_operation_id.0.clone(),
            disposition: ReceiptDisposition::Committed,
            result_ref: format!("operation:{}", request.new_operation_id.0),
            resolved_binding_digest: request.request_digest.0.clone(),
            owner_kind: "test".to_owned(),
            lease_until: None,
            last_progress: 1,
            reconciliation_policy: "test".to_owned(),
        },
        status: "committed".to_owned(),
        committed_payload: None,
        error: None,
        operation_may_continue: false,
    }
}
