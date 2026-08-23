//! Acceptance tests for AT-SEC-005: endpoint security boundary, fail-closed
//! information flow, and non-disclosure of target existence.
//!
//! Covered in-process: authentication resolves a registered principal; any
//! security failure yields `PermissionDenied` (never an internal error); and an
//! unauthorized request never discloses whether its target exists.

use std::error::Error;
use std::sync::{Arc, Mutex};

use application::ApplicationMutator;
use control_server::{ControlServer, SecurityState, ServerError};
use dxbot_core::types::{
    BotId, BotSelector, CanonicalTarget, CommandId, CommandPayload, IdempotencyKey, InstanceId,
    OperationId, OperationRequest, PrincipalRef, RequestDigest, ScopeSelector,
};

/// Build a `CreateBot` operation request for `principal` targeting `bot_id`.
fn create_bot_request(principal: &str, bot_id: &str, command_id: &str) -> OperationRequest {
    let principal_ref = PrincipalRef(format!("http:{principal}"));
    OperationRequest {
        command_id: CommandId(command_id.to_string()),
        idempotency_key: IdempotencyKey {
            principal_ref: principal_ref.clone(),
            key_digest: "digest".to_string(),
            expires_at: i64::MAX,
        },
        request_digest: RequestDigest("request-digest".to_string()),
        new_operation_id: OperationId("op-new".to_string()),
        payload: CommandPayload {
            command_key: "CreateBot".to_string(),
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

/// The bot scope for a canonical bot id.
fn bot_scope(bot_id: &str) -> ScopeSelector {
    ScopeSelector::Bot(BotSelector::CanonicalId(BotId(bot_id.to_string())))
}

/// A control server with an empty security state and empty domain state.
fn server() -> (ControlServer, Arc<Mutex<SecurityState>>) {
    let security = Arc::new(Mutex::new(SecurityState::new()));
    let application = Arc::new(Mutex::new(ApplicationMutator::new()));
    let server = ControlServer::new(security.clone(), application);
    (server, security)
}

/// Register `principal` so it is authenticated at the boundary.
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

/// Grant `role` to `principal` on `scope`.
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

    // A registered principal with the needed authority on the target scope.
    let alice = PrincipalRef("http:alice".into());
    register(&security, &alice)?;
    grant(&security, &alice, &bot_scope("bot-1"))?;

    let result = server.handle_request(&create_bot_request("alice", "bot-1", "cmd-1"))?;
    assert_eq!(result.status, "committed");
    assert!(result.error.is_none());
    if let Some(payload) = result.committed_payload.as_ref() {
        assert!(payload.get("bot_ref").is_some());
    } else {
        panic!("authorized mutation must produce a committed payload");
    }
    Ok(())
}

#[test]
fn security_unauthorized_operation_is_denied() -> Result<(), Box<dyn Error>> {
    let (server, security) = server();

    // Registered principal but NO authority bound -> not authorized.
    register(&security, &PrincipalRef("http:eve".into()))?;

    assert!(matches!(
        server.handle_request(&create_bot_request("eve", "bot-1", "cmd-1")),
        Err(ServerError::PermissionDenied(_))
    ));
    Ok(())
}

#[test]
fn security_information_flow_is_fail_closed() -> Result<(), Box<dyn Error>> {
    let (server, security) = server();

    // The peer is not even registered (unauthenticated).
    let request = create_bot_request("mallory", "bot-1", "cmd-1");

    // Fail-closed: a security failure is always PermissionDenied, never an
    // internal invariant error.
    match server.handle_request(&request) {
        Err(ServerError::PermissionDenied(_)) => {}
        other => panic!("fail-closed: expected PermissionDenied, got {other:?}"),
    }

    // A registered but unauthorized principal is likewise fail-closed.
    register(&security, &PrincipalRef("http:eve".into()))?;
    match server.handle_request(&create_bot_request("eve", "bot-1", "cmd-1")) {
        Err(ServerError::PermissionDenied(_)) => {}
        other => panic!("fail-closed: expected PermissionDenied, got {other:?}"),
    }
    Ok(())
}

#[test]
fn security_nonexistent_target_does_not_disclose_existence() -> Result<(), Box<dyn Error>> {
    let (server, security) = server();

    // Authorized for the target's scope, so the mutator legitimately creates
    // the (previously nonexistent) target — committing reveals nothing.
    let alice = PrincipalRef("http:alice".into());
    register(&security, &alice)?;
    grant(&security, &alice, &bot_scope("ghost-bot"))?;
    assert!(
        server
            .handle_request(&create_bot_request("alice", "ghost-bot", "cmd-ghost"))
            .is_ok()
    );

    // An UNAUTHORIZED request to a (different) nonexistent target must yield
    // PermissionDenied and never an existence-disclosing NotFound.
    register(&security, &PrincipalRef("http:carol".into()))?;
    match server.handle_request(&create_bot_request("carol", "ghost-bot", "cmd-ghost2")) {
        Err(ServerError::PermissionDenied(_)) => {}
        Err(ServerError::NotFound(_)) => panic!(
            "non-disclosure: unauthorized request must not leak target existence via NotFound"
        ),
        other => panic!("non-disclosure: expected PermissionDenied, got {other:?}"),
    }
    Ok(())
}