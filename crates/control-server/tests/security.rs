//! Acceptance tests for AT-SEC-005: endpoint security boundary, fail-closed
//! information flow, non-disclosure, and request-identity preservation.

#![allow(clippy::panic)]

use std::error::Error;
use std::sync::{Arc, Mutex};

use application::ApplicationMutator;
use control_server::{ControlServer, SecurityState, ServerError};
use dxbot_core::types::{
    BotId, BotSelector, CanonicalTarget, CommandId, CommandPayload, IdempotencyKey, InstanceId,
    OperationId, OperationRequest, PrincipalRef, RequestDigest, ScopeSelector,
};

fn create_bot_request(principal: &str, bot_id: &str, command_id: &str) -> OperationRequest {
    let principal_ref = PrincipalRef(format!("http:{principal}"));
    OperationRequest {
        command_id: CommandId(command_id.to_string()),
        idempotency_key: IdempotencyKey {
            principal_ref: principal_ref.clone(),
            key_digest: format!("key-{command_id}"),
            expires_at: i64::MAX,
        },
        request_digest: RequestDigest(format!("request-{command_id}")),
        new_operation_id: OperationId(format!("operation-{command_id}")),
        payload: CommandPayload {
            command_key: "bot-create".to_string(),
            principal_ref,
            instance_id: InstanceId("instance-1".to_string()),
            canonical_target: CanonicalTarget::Bot {
                id: BotId(bot_id.to_string()),
                revision: 0,
            },
            cas: None,
            content: None,
            semantic_options: serde_json::json!({ "name": bot_id }),
        },
    }
}

fn bot_scope(bot_id: &str) -> ScopeSelector {
    ScopeSelector::Bot(BotSelector::CanonicalId(BotId(bot_id.to_string())))
}

fn server() -> (ControlServer, Arc<Mutex<SecurityState>>) {
    let security = Arc::new(Mutex::new(SecurityState::new()));
    let application = Arc::new(ApplicationMutator::new());
    let server = ControlServer::new(security.clone(), application);
    (server, security)
}

fn register(
    security: &Arc<Mutex<SecurityState>>,
    principal: &PrincipalRef,
) -> Result<(), Box<dyn Error>> {
    let mut guard = security
        .lock()
        .map_err(|_| "security state lock unavailable")?;
    guard.principals.register_principal(principal.clone())?;
    Ok(())
}

fn grant(
    security: &Arc<Mutex<SecurityState>>,
    principal: &PrincipalRef,
    scope: &ScopeSelector,
) -> Result<(), Box<dyn Error>> {
    let mut guard = security
        .lock()
        .map_err(|_| "security state lock unavailable")?;
    guard.authority.bind_authority(principal, scope, "mutator")?;
    Ok(())
}

#[test]
fn security_authenticate_resolves_principal() -> Result<(), Box<dyn Error>> {
    let (server, security) = server();
    let alice = PrincipalRef("http:alice".to_string());
    register(&security, &alice)?;
    grant(&security, &alice, &bot_scope("bot-1"))?;

    let request = create_bot_request("alice", "bot-1", "cmd-1");
    let result = server.handle_request(&request)?;
    assert_eq!(result.status, "committed");
    assert_eq!(result.command_id, request.command_id);
    assert_eq!(result.operation_id, request.new_operation_id);
    assert!(result.error.is_none());
    assert!(result
        .committed_payload
        .as_ref()
        .is_some_and(|payload| payload.get("bot_ref").is_some()));
    Ok(())
}

#[test]
fn security_unauthorized_operation_is_denied() -> Result<(), Box<dyn Error>> {
    let (server, security) = server();
    register(&security, &PrincipalRef("http:eve".to_string()))?;

    assert!(matches!(
        server.handle_request(&create_bot_request("eve", "bot-1", "cmd-1")),
        Err(ServerError::PermissionDenied(_))
    ));
    Ok(())
}

#[test]
fn security_information_flow_is_fail_closed() -> Result<(), Box<dyn Error>> {
    let (server, security) = server();
    let request = create_bot_request("mallory", "bot-1", "cmd-1");
    assert!(matches!(
        server.handle_request(&request),
        Err(ServerError::PermissionDenied(_))
    ));

    register(&security, &PrincipalRef("http:eve".to_string()))?;
    assert!(matches!(
        server.handle_request(&create_bot_request("eve", "bot-1", "cmd-2")),
        Err(ServerError::PermissionDenied(_))
    ));
    Ok(())
}

#[test]
fn security_nonexistent_target_does_not_disclose_existence() -> Result<(), Box<dyn Error>> {
    let (server, security) = server();
    let alice = PrincipalRef("http:alice".to_string());
    register(&security, &alice)?;
    grant(&security, &alice, &bot_scope("ghost-bot"))?;
    assert!(
        server
            .handle_request(&create_bot_request("alice", "ghost-bot", "cmd-ghost"))
            .is_ok()
    );

    register(&security, &PrincipalRef("http:carol".to_string()))?;
    assert!(matches!(
        server.handle_request(&create_bot_request("carol", "other-ghost", "cmd-ghost2")),
        Err(ServerError::PermissionDenied(_))
    ));
    Ok(())
}

#[test]
fn security_same_command_id_with_changed_digest_conflicts_after_authorization(
) -> Result<(), Box<dyn Error>> {
    let (server, security) = server();
    let alice = PrincipalRef("http:alice".to_string());
    register(&security, &alice)?;
    grant(&security, &alice, &bot_scope("bot-1"))?;

    let request = create_bot_request("alice", "bot-1", "cmd-1");
    server.handle_request(&request)?;
    let mut conflicting = request.clone();
    conflicting.request_digest = RequestDigest("changed-digest".to_string());
    conflicting.new_operation_id = OperationId("changed-operation".to_string());
    assert!(matches!(
        server.handle_request(&conflicting),
        Err(ServerError::Conflict(_))
    ));
    Ok(())
}
