//! Provider admission coverage for Bot activation and Task submission.

#![allow(clippy::expect_used)]

use std::sync::{Arc, Mutex};

use application::ApplicationMutator;
use control_server::{ControlServer, SecurityState, ServerError};
use dxbot_core::types::{
    BotId, CanonicalTarget, CommandId, CommandPayload, IdempotencyKey, InstanceId, OperationId,
    OperationRequest, PrincipalRef, RequestDigest,
};
use provider_host::{ProviderHost, ReferenceProvider};
use serde_json::json;

fn principal() -> PrincipalRef {
    PrincipalRef("local:instance-a:uid:1000".to_owned())
}

fn request(command: &str, sequence: &str, target: CanonicalTarget, fields: serde_json::Value) -> OperationRequest {
    OperationRequest {
        command_id: CommandId(format!("command-{sequence}")),
        idempotency_key: IdempotencyKey {
            principal_ref: principal(),
            key_digest: format!("key-{sequence}"),
            expires_at: i64::MAX,
        },
        request_digest: RequestDigest(format!("digest-{sequence}")),
        new_operation_id: OperationId(format!("operation-{sequence}")),
        payload: CommandPayload {
            command_key: command.to_owned(),
            principal_ref: principal(),
            instance_id: InstanceId("instance-a".to_owned()),
            canonical_target: target,
            cas: None,
            content: None,
            semantic_options: fields,
        },
    }
}

fn register_operator(server: &ControlServer) {
    server
        .register_local_operator(&principal())
        .expect("operator registration");
}

#[test]
fn bot_activation_fails_closed_without_ready_llm_provider() {
    let application = Arc::new(ApplicationMutator::new());
    let security = Arc::new(Mutex::new(SecurityState::new()));
    let server = ControlServer::new(security, application);
    register_operator(&server);
    let instance = CanonicalTarget::Instance(InstanceId("instance-a".to_owned()));
    server
        .handle_request(
            &principal(),
            &request("bot-create", "create", instance, json!({"name": "alpha"})),
        )
        .expect("identity creation does not require provider");

    let error = server
        .handle_request(
            &principal(),
            &request(
                "bot-activate",
                "activate",
                CanonicalTarget::Bot {
                    id: BotId("alpha".to_owned()),
                    revision: 1,
                },
                json!({}),
            ),
        )
        .expect_err("activation must require provider readiness");
    assert!(matches!(error, ServerError::ProviderUnavailable(_)));
}

#[test]
fn ready_llm_provider_allows_activation_owner_path() {
    let application = Arc::new(ApplicationMutator::new());
    let security = Arc::new(Mutex::new(SecurityState::new()));
    let mut providers = ProviderHost::new();
    providers
        .register_reference_provider(ReferenceProvider::new(
            dxbot_core::types::ProviderId("reference".to_owned()),
            "llm-chat",
            1,
        ))
        .expect("provider registration");
    let server = ControlServer::with_provider_host(security, application, providers);
    register_operator(&server);
    let instance = CanonicalTarget::Instance(InstanceId("instance-a".to_owned()));
    server
        .handle_request(
            &principal(),
            &request("bot-create", "create-ready", instance, json!({"name": "alpha"})),
        )
        .expect("create bot");

    let result = server
        .handle_request(
            &principal(),
            &request(
                "bot-activate",
                "activate-ready",
                CanonicalTarget::Bot {
                    id: BotId("alpha".to_owned()),
                    revision: 1,
                },
                json!({}),
            ),
        )
        .expect("ready provider admits activation");
    assert_eq!(result.status, "committed");
}
