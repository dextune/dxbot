//! End-to-end local control framing/authentication test.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use application::ApplicationMutator;
use application_contract::{
    LocalControlHello, LocalControlRequest, LocalControlResponse, read_local_control_frame,
    write_local_control_frame,
};
use control_server::{ControlServer, LocalControlServer, SecurityState};
use dxbot_core::types::{
    CanonicalTarget, CommandId, CommandPayload, IdempotencyKey, InstanceId, OperationId,
    OperationRequest, PrincipalRef, RequestDigest,
};

fn socket_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "dxbot-local-control-{}-{}.sock",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ))
}

#[test]
fn local_control_derives_principal_and_dispatches_after_handshake() {
    let path = socket_path();
    let _ = fs::remove_file(&path);
    let application = Arc::new(ApplicationMutator::new());
    let security = Arc::new(Mutex::new(SecurityState::new()));
    let control = Arc::new(ControlServer::new(security, application));
    let instance = InstanceId("instance-local".to_owned());
    let server =
        LocalControlServer::bind(path.clone(), instance.clone(), 7, control).expect("server binds");
    let expected_principal = server.principal_ref().clone();
    assert!(
        expected_principal
            .0
            .starts_with("local:instance-local:uid:")
    );
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );

    let worker = std::thread::spawn(move || server.serve_one().expect("serve one connection"));
    let mut stream = UnixStream::connect(&path).expect("connect");

    write_local_control_frame(
        &mut stream,
        &LocalControlRequest::Hello {
            hello: LocalControlHello::new(instance.clone(), 7),
        },
    )
    .unwrap();
    let handshake = match read_local_control_frame::<_, LocalControlResponse>(&mut stream).unwrap()
    {
        LocalControlResponse::Handshake { handshake } => handshake,
        other => panic!("unexpected handshake response: {other:?}"),
    };
    assert_eq!(handshake.principal_ref, expected_principal);

    let request = request(instance, handshake.principal_ref.clone());
    write_local_control_frame(
        &mut stream,
        &LocalControlRequest::Submit {
            request: request.clone(),
        },
    )
    .unwrap();
    match read_local_control_frame::<_, LocalControlResponse>(&mut stream).unwrap() {
        LocalControlResponse::Operation { result } => {
            assert_eq!(result.status, "committed");
            assert_eq!(result.command_id, request.command_id);
            assert_eq!(result.operation_id, request.new_operation_id);
        }
        other => panic!("unexpected operation response: {other:?}"),
    }

    drop(stream);
    worker.join().unwrap();
    let _ = fs::remove_file(path);
}

fn request(instance: InstanceId, principal: PrincipalRef) -> OperationRequest {
    OperationRequest {
        command_id: CommandId("command-local".to_owned()),
        idempotency_key: IdempotencyKey {
            principal_ref: principal.clone(),
            key_digest: "key-local".to_owned(),
            expires_at: i64::MAX,
        },
        request_digest: RequestDigest("digest-local".to_owned()),
        new_operation_id: OperationId("operation-local".to_owned()),
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
