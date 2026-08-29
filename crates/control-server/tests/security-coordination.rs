//! Cross-owner Application + Security coordination coverage.

#![allow(clippy::expect_used)]

use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use application::ApplicationMutator;
use control_server::{ControlServer, SecurityCoordinationStore, SecurityState, ServerError};
use dxbot_core::types::{
    BotId, BotSelector, CanonicalTarget, CasConditions, CommandId, CommandPayload, IdempotencyKey,
    InstanceId, OperationId, OperationRequest, PrincipalRef, ProjectId, ProjectSelector,
    RequestDigest, ScopeSelector,
};
use provider_host::ProviderHost;
use runtime_security::{
    ApprovalBinding, ApprovalDecision, ApprovalState, SecurityAuditIntent, SecurityDelta,
    SecurityStateStore,
};
use serde_json::json;

fn principal() -> PrincipalRef {
    PrincipalRef("local:instance-a:uid:1000".to_owned())
}

fn request(
    command: &str,
    sequence: &str,
    target: CanonicalTarget,
    cas: Option<CasConditions>,
    fields: serde_json::Value,
) -> OperationRequest {
    let principal = principal();
    OperationRequest {
        command_id: CommandId(format!("command-{sequence}")),
        idempotency_key: IdempotencyKey {
            principal_ref: principal.clone(),
            key_digest: format!("key-{sequence}"),
            expires_at: i64::MAX,
        },
        request_digest: RequestDigest(format!("digest-{sequence}")),
        new_operation_id: OperationId(format!("operation-{sequence}")),
        payload: CommandPayload {
            command_key: command.to_owned(),
            principal_ref: principal,
            instance_id: InstanceId("instance-a".to_owned()),
            canonical_target: target,
            cas,
            content: None,
            semantic_options: fields,
        },
    }
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

fn operator_server(
    security: Arc<Mutex<SecurityState>>,
    application: Arc<ApplicationMutator>,
) -> ControlServer {
    let server = ControlServer::new(security, application);
    server
        .register_local_operator(&principal())
        .expect("register operator");
    server
}

fn approve_high_risk(server: &ControlServer, original: OperationRequest, decision_sequence: &str) {
    let pending = match server.handle_request(&principal(), &original) {
        Err(ServerError::ApprovalRequired(pending)) => pending,
        other => panic!("expected approval-required, got {other:?}"),
    };
    let mut cas = empty_cas();
    cas.if_revision = Some(1);
    server
        .handle_request(
            &principal(),
            &request(
                "approval-approve",
                decision_sequence,
                CanonicalTarget::Approval {
                    id: pending.approval_id,
                    revision: 1,
                },
                Some(cas),
                json!({}),
            ),
        )
        .expect("approval decision and continuation");
    assert!(
        server
            .lookup_binding(
                &principal(),
                &original.command_id,
                &original.idempotency_key,
            )
            .expect("original binding lookup")
            .is_some(),
        "approved high-risk operation must commit"
    );
}

fn membership_binding_id(scope: &ScopeSelector, member: &BotSelector) -> String {
    let encoded = serde_json::to_string(&(scope, member)).expect("subject");
    format!("membership-subject:{encoded}")
}

#[test]
fn project_create_commits_membership_and_security_binding_for_bot_subject() {
    let application = Arc::new(ApplicationMutator::new());
    let security = Arc::new(Mutex::new(SecurityState::new()));
    let server = operator_server(security.clone(), application.clone());

    let instance = CanonicalTarget::Instance(InstanceId("instance-a".to_owned()));
    server
        .handle_request(
            &principal(),
            &request(
                "bot-create",
                "bot",
                instance.clone(),
                None,
                json!({"name": "bot-a"}),
            ),
        )
        .expect("bot create");
    server
        .handle_request(
            &principal(),
            &request(
                "project-create",
                "project",
                instance,
                None,
                json!({"name": "project-a", "owner_bot": "bot:bot-a"}),
            ),
        )
        .expect("project create");

    let scope = ScopeSelector::Project(ProjectSelector::CanonicalId(ProjectId(
        "project-a".to_owned(),
    )));
    let member = BotSelector::CanonicalId(BotId("bot-a".to_owned()));
    let snapshot = application.snapshot().expect("application snapshot");
    assert!(snapshot.memberships.values().any(|row| {
        row.scope == scope && row.member_bot == member && row.role == "owner" && row.active
    }));

    let binding_id = membership_binding_id(&scope, &member);
    let security = security.lock().expect("security");
    let binding = security
        .authority
        .membership_binding(&binding_id)
        .expect("security binding");
    assert_eq!(binding.scope, scope);
    assert_eq!(binding.member_bot, member);
    assert_eq!(binding.role, "owner");
    assert_eq!(binding.generation, 1);
    assert!(binding.active);
    assert!(
        security
            .principals
            .resolve_principal(&PrincipalRef("bot:bot-a".to_owned()))
            .is_err()
    );
}

#[test]
fn membership_remove_and_readd_keep_application_and_security_generations_aligned() {
    let application = Arc::new(ApplicationMutator::new());
    let security = Arc::new(Mutex::new(SecurityState::new()));
    let server = operator_server(security.clone(), application.clone());
    let instance = CanonicalTarget::Instance(InstanceId("instance-a".to_owned()));

    for name in ["owner", "member"] {
        server
            .handle_request(
                &principal(),
                &request(
                    "bot-create",
                    &format!("bot-{name}"),
                    instance.clone(),
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
                "project-membership",
                instance,
                None,
                json!({"name": "project-a", "owner_bot": "bot:owner"}),
            ),
        )
        .expect("project create");

    let scope = ScopeSelector::Project(ProjectSelector::CanonicalId(ProjectId(
        "project-a".to_owned(),
    )));
    let member = BotSelector::CanonicalId(BotId("member".to_owned()));
    let target = CanonicalTarget::Membership {
        scope: scope.clone(),
        member_bot: member.clone(),
    };
    let mut set_cas = empty_cas();
    set_cas.if_project_revision = Some(1);
    approve_high_risk(
        &server,
        request(
            "project-member-set",
            "member-set-1",
            target.clone(),
            Some(set_cas),
            json!({"role_ref": "member"}),
        ),
        "approve-member-set-1",
    );

    let mut remove_cas = empty_cas();
    remove_cas.if_project_revision = Some(2);
    remove_cas.if_membership_generation = Some(1);
    approve_high_risk(
        &server,
        request(
            "project-member-remove",
            "member-remove",
            target.clone(),
            Some(remove_cas),
            json!({}),
        ),
        "approve-member-remove",
    );

    let tombstone = application
        .snapshot()
        .expect("application snapshot")
        .memberships
        .values()
        .find(|row| row.scope == scope && row.member_bot == member)
        .cloned()
        .expect("application tombstone");
    assert!(!tombstone.active);
    assert_eq!(tombstone.generation, 2);
    let binding_id = membership_binding_id(&scope, &member);
    let security_tombstone = security
        .lock()
        .expect("security")
        .authority
        .membership_binding(&binding_id)
        .expect("security tombstone");
    assert!(!security_tombstone.active);
    assert_eq!(security_tombstone.generation, 2);

    let mut readd_cas = empty_cas();
    readd_cas.if_project_revision = Some(3);
    readd_cas.if_membership_generation = Some(2);
    approve_high_risk(
        &server,
        request(
            "project-member-set",
            "member-set-2",
            target,
            Some(readd_cas),
            json!({"role_ref": "admin"}),
        ),
        "approve-member-set-2",
    );

    let restored = application
        .snapshot()
        .expect("application snapshot")
        .memberships
        .values()
        .find(|row| row.scope == scope && row.member_bot == member)
        .cloned()
        .expect("application membership");
    assert!(restored.active);
    assert_eq!(restored.generation, 3);
    assert_eq!(restored.role, "admin");
    let security_restored = security
        .lock()
        .expect("security")
        .authority
        .membership_binding(&binding_id)
        .expect("security membership");
    assert!(security_restored.active);
    assert_eq!(security_restored.generation, 3);
    assert_eq!(security_restored.role, "admin");
}

#[test]
fn approval_decision_is_revision_fenced_durable_operation_and_idempotent_retry() {
    let application = Arc::new(ApplicationMutator::new());
    let security = Arc::new(Mutex::new(SecurityState::new()));
    let server = operator_server(security.clone(), application.clone());
    let approval_id = {
        let mut security = security.lock().expect("security");
        security
            .approvals
            .create_bound_approval(
                OperationId("pending-high-risk".to_owned()),
                ApprovalBinding {
                    action: "delete".to_owned(),
                    target: "project:project-a".to_owned(),
                    policy_generation: 7,
                },
                vec![principal()],
            )
            .expect("approval")
    };
    let mut cas = empty_cas();
    cas.if_revision = Some(1);
    let decision = request(
        "approval-approve",
        "approval",
        CanonicalTarget::Approval {
            id: approval_id.clone(),
            revision: 1,
        },
        Some(cas),
        json!({}),
    );

    let first = server
        .handle_request(&principal(), &decision)
        .expect("first decision");
    let retry = server
        .handle_request(&principal(), &decision)
        .expect("idempotent retry");
    assert_eq!(first, retry);
    let security = security.lock().expect("security");
    let approval = security
        .approvals
        .get_approval(&approval_id)
        .expect("approval state");
    assert_eq!(approval.state, ApprovalState::Approved);
    assert_eq!(approval.revision, 2);
    assert_eq!(approval.decisions.len(), 1);
    assert_eq!(approval.decisions[0].decision, ApprovalDecision::Approve);
    assert_eq!(security.approval_wakeups().len(), 1);
    assert_eq!(security.audit_intents().len(), 1);
}

#[test]
fn restart_recovers_security_delta_only_when_application_binding_committed() {
    let root = std::env::temp_dir().join(format!(
        "dxbot-control-recovery-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("temp root");
    let application_path = root.join("application-state.json");
    let security_path = root.join("security-state.json");
    let marker_path = root.join("coordination.json");

    let application = Arc::new(
        ApplicationMutator::with_persistent_state(application_path.clone()).expect("application"),
    );
    let (security_store, mut security_state) =
        SecurityStateStore::open(security_path.clone()).expect("security store");
    security_state
        .principals
        .register_principal(principal())
        .expect("principal");
    security_state
        .authority
        .bind_global_authority(&principal(), "operator")
        .expect("operator");
    security_store
        .persist(&security_state)
        .expect("security seed");

    let committed_request = request(
        "bot-create",
        "recovery",
        CanonicalTarget::Instance(InstanceId("instance-a".to_owned())),
        None,
        json!({"name": "recovered-bot"}),
    );
    application
        .mutate(&committed_request)
        .expect("application commit");
    let marker = SecurityCoordinationStore::new(marker_path.clone());
    marker
        .prepare(&control_server::coordination::SecurityCoordinationRecord {
            request: committed_request.clone(),
            delta: SecurityDelta::audit_only(SecurityAuditIntent {
                operation_id: committed_request.new_operation_id.clone(),
                principal_ref: principal(),
                action: "recovery-test".to_owned(),
                target: "bot:recovered-bot".to_owned(),
                created_at: 1,
            }),
        })
        .expect("prepare simulated crash marker");

    let reopened_application = Arc::new(
        ApplicationMutator::with_persistent_state(application_path).expect("reopen application"),
    );
    let (reopened_store, reopened_state) =
        SecurityStateStore::open(security_path).expect("reopen security");
    let reopened_security = Arc::new(Mutex::new(reopened_state));
    ControlServer::with_persistence(
        reopened_security.clone(),
        reopened_application,
        ProviderHost::new(),
        Arc::new(reopened_store),
        Arc::new(SecurityCoordinationStore::new(marker_path)),
    )
    .expect("recovery");

    assert_eq!(
        reopened_security
            .lock()
            .expect("security")
            .audit_intents()
            .len(),
        1
    );
    assert!(marker.load().expect("marker load").is_none());
    fs::remove_dir_all(root).expect("cleanup");
}
