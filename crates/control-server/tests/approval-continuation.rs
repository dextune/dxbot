//! BF-CLI-027: high-risk operation parking and approval continuation.

#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use application::ApplicationMutator;
use control_server::{ControlServer, SecurityCoordinationStore, SecurityState, ServerError};
use dxbot_core::types::{
    ApprovalId, BotId, BotSelector, CanonicalTarget, CasConditions, CommandId, CommandPayload,
    IdempotencyKey, InstanceId, OperationId, OperationRequest, PrincipalRef, ProjectId,
    ProjectSelector, RequestDigest, ScopeSelector,
};
use provider_host::ProviderHost;
use runtime_security::{
    ApprovalDecision, ApprovalDecisionDelta, ParkedGateState, SecurityDelta, SecurityStateStore,
};
use serde_json::{Value, json};

fn principal() -> PrincipalRef {
    PrincipalRef("local:instance-a:uid:1000".to_owned())
}

fn empty_cas() -> CasConditions {
    CasConditions {
        if_revision: None,
        if_generation: None,
        if_host_generation: None,
        if_execution_generation: None,
        if_source_revision: None,
        if_scope_revision: None,
        if_project_revision: None,
        if_channel_revision: None,
        if_membership_generation: None,
        if_proposal_revision: None,
        if_target_scope_revision: None,
        if_receipt_revision: None,
    }
}

fn request(
    command_key: &str,
    sequence: &str,
    target: CanonicalTarget,
    cas: Option<CasConditions>,
    fields: Value,
) -> OperationRequest {
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
            command_key: command_key.to_owned(),
            principal_ref: principal(),
            instance_id: InstanceId("instance-a".to_owned()),
            canonical_target: target,
            cas,
            content: None,
            semantic_options: fields,
        },
    }
}

fn instance_target() -> CanonicalTarget {
    CanonicalTarget::Instance(InstanceId("instance-a".to_owned()))
}

fn project_scope() -> ScopeSelector {
    ScopeSelector::Project(ProjectSelector::CanonicalId(ProjectId(
        "project-a".to_owned(),
    )))
}

fn membership_target() -> CanonicalTarget {
    CanonicalTarget::Membership {
        scope: project_scope(),
        member_bot: BotSelector::CanonicalId(BotId("member".to_owned())),
    }
}

fn member_set(sequence: &str, project_revision: i64) -> OperationRequest {
    let mut cas = empty_cas();
    cas.if_project_revision = Some(project_revision);
    request(
        "project-member-set",
        sequence,
        membership_target(),
        Some(cas),
        json!({"role_ref": "member"}),
    )
}

fn approval_decision(
    approval_id: ApprovalId,
    sequence: &str,
    approve: bool,
    reason: Option<&str>,
) -> OperationRequest {
    let mut cas = empty_cas();
    cas.if_revision = Some(1);
    request(
        if approve {
            "approval-approve"
        } else {
            "approval-deny"
        },
        sequence,
        CanonicalTarget::Approval {
            id: approval_id,
            revision: 1,
        },
        Some(cas),
        reason.map_or_else(|| json!({}), |reason| json!({"reason": reason})),
    )
}

fn seed(server: &ControlServer) {
    for name in ["owner", "member"] {
        server
            .handle_request(
                &principal(),
                &request(
                    "bot-create",
                    &format!("bot-{name}"),
                    instance_target(),
                    None,
                    json!({"name": name}),
                ),
            )
            .expect("bot create");
    }
    server
        .handle_request(
            &principal(),
            &request(
                "project-create",
                "project",
                instance_target(),
                None,
                json!({"name": "project-a", "owner_bot": "bot:owner"}),
            ),
        )
        .expect("project create");
}

fn memory_server() -> (
    ControlServer,
    Arc<Mutex<SecurityState>>,
    Arc<ApplicationMutator>,
) {
    let security = Arc::new(Mutex::new(SecurityState::new()));
    let application = Arc::new(ApplicationMutator::new());
    let server = ControlServer::new(security.clone(), application.clone());
    server
        .register_local_operator(&principal())
        .expect("register operator");
    seed(&server);
    (server, security, application)
}

fn expect_pending(result: Result<dxbot_core::types::OperationResult, ServerError>) -> ApprovalId {
    match result {
        Err(ServerError::ApprovalRequired(pending)) => pending.approval_id,
        other => panic!("expected approval-required, got {other:?}"),
    }
}

#[test]
fn high_risk_exact_retry_reuses_approval_and_approve_continues_once() {
    let (server, security, application) = memory_server();
    let original = member_set("set", 1);

    let first = expect_pending(server.handle_request(&principal(), &original));
    let retry = expect_pending(server.handle_request(&principal(), &original));
    assert_eq!(retry, first, "exact retry must reuse the bound approval");
    assert!(
        application
            .lookup_binding(&original.command_id, &original.idempotency_key)
            .expect("binding lookup")
            .is_none(),
        "parking must not mutate Application"
    );

    server
        .handle_request(
            &principal(),
            &approval_decision(first.clone(), "approve", true, None),
        )
        .expect("approve and continue");
    assert!(
        application
            .lookup_binding(&original.command_id, &original.idempotency_key)
            .expect("binding lookup")
            .is_some()
    );
    let gate = security
        .lock()
        .expect("security")
        .parking
        .gate_for_approval(&first)
        .expect("gate");
    assert_eq!(gate.state, ParkedGateState::Continued);

    let row = application
        .snapshot()
        .expect("snapshot")
        .memberships
        .values()
        .find(|row| {
            row.scope == project_scope()
                && row.member_bot == BotSelector::CanonicalId(BotId("member".to_owned()))
        })
        .cloned()
        .expect("continued membership");
    assert_eq!(row.generation, 1);
}

#[test]
fn approved_operation_with_stale_original_cas_fails_without_mutation() {
    let (server, security, application) = memory_server();
    let original = member_set("stale-set", 1);
    let approval_id = expect_pending(server.handle_request(&principal(), &original));

    let mut channel_cas = empty_cas();
    channel_cas.if_project_revision = Some(1);
    server
        .handle_request(
            &principal(),
            &request(
                "channel-create",
                "advance-project",
                CanonicalTarget::Project {
                    id: ProjectId("project-a".to_owned()),
                    revision: 1,
                },
                Some(channel_cas),
                json!({"name": "general"}),
            ),
        )
        .expect("advance project revision");

    server
        .handle_request(
            &principal(),
            &approval_decision(approval_id.clone(), "approve-stale", true, None),
        )
        .expect("approval decision itself commits");

    assert!(
        application
            .lookup_binding(&original.command_id, &original.idempotency_key)
            .expect("binding lookup")
            .is_none(),
        "stale approved operation must not commit"
    );
    let gate = security
        .lock()
        .expect("security")
        .parking
        .gate_for_approval(&approval_id)
        .expect("gate");
    assert_eq!(gate.state, ParkedGateState::Failed);
    assert!(
        gate.failure_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("revision")),
        "terminal failure must retain stale-CAS evidence: {:?}",
        gate.failure_reason
    );
}

#[test]
fn denied_operation_never_continues_and_reason_is_durable() {
    let (server, security, application) = memory_server();
    let original = member_set("denied-set", 1);
    let approval_id = expect_pending(server.handle_request(&principal(), &original));

    server
        .handle_request(
            &principal(),
            &approval_decision(
                approval_id.clone(),
                "deny",
                false,
                Some("insufficient justification"),
            ),
        )
        .expect("deny");

    assert!(
        application
            .lookup_binding(&original.command_id, &original.idempotency_key)
            .expect("binding lookup")
            .is_none()
    );
    let gate = security
        .lock()
        .expect("security")
        .parking
        .gate_for_approval(&approval_id)
        .expect("gate");
    assert_eq!(gate.state, ParkedGateState::Denied);
    assert_eq!(
        gate.deny_reason.as_deref(),
        Some("insufficient justification")
    );
}

fn temp_root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "dxbot-approval-continuation-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ))
}

#[test]
fn parked_operation_survives_restart_and_continues_after_approval() {
    let root = temp_root();
    fs::create_dir_all(&root).expect("root");
    let application_path = root.join("application-state.json");
    let security_path = root.join("security-state.json");
    let coordination_path = root.join("coordination.json");

    let original = member_set("restart-set", 1);
    let approval_id = {
        let application = Arc::new(
            ApplicationMutator::with_persistent_state(application_path.clone())
                .expect("application"),
        );
        let (store, state) = SecurityStateStore::open(security_path.clone()).expect("security");
        let server = ControlServer::with_persistence(
            Arc::new(Mutex::new(state)),
            application,
            ProviderHost::new(),
            Arc::new(store),
            Arc::new(SecurityCoordinationStore::new(coordination_path.clone())),
        )
        .expect("server");
        server
            .register_local_operator(&principal())
            .expect("operator");
        seed(&server);
        expect_pending(server.handle_request(&principal(), &original))
    };

    // Simulate a crash/restart boundary after the approval decision became
    // durable and emitted its wakeup, but before a ControlServer could consume
    // that wakeup and continue the original operation.
    {
        let (store, mut state) =
            SecurityStateStore::open(security_path.clone()).expect("reopen for decision");
        state
            .apply_delta(&SecurityDelta {
                membership: None,
                approval: Some(ApprovalDecisionDelta {
                    approval_id: approval_id.clone(),
                    expected_revision: 1,
                    decision: ApprovalDecision::Approve,
                    by: principal(),
                }),
                parking: None,
                audit_intent: None,
            })
            .expect("apply durable approval wakeup");
        store.persist(&state).expect("persist approval wakeup");
    }

    let application = Arc::new(
        ApplicationMutator::with_persistent_state(application_path).expect("reopen application"),
    );
    let (store, state) = SecurityStateStore::open(security_path).expect("reopen security");
    let security = Arc::new(Mutex::new(state));
    let _server = ControlServer::with_persistence(
        security.clone(),
        application.clone(),
        ProviderHost::new(),
        Arc::new(store),
        Arc::new(SecurityCoordinationStore::new(coordination_path)),
    )
    .expect("restart server drives approved wakeup");
    assert!(
        application
            .lookup_binding(&original.command_id, &original.idempotency_key)
            .expect("binding lookup")
            .is_some()
    );
    assert_eq!(
        security
            .lock()
            .expect("security")
            .parking
            .gate_for_approval(&approval_id)
            .expect("gate")
            .state,
        ParkedGateState::Continued
    );
    assert!(
        !root.join("parked-operations.json").exists()
            || fs::read_to_string(root.join("parked-operations.json"))
                .expect("parking store")
                .contains("\"records\":{}")
    );

    fs::remove_dir_all(root).expect("cleanup");
}
