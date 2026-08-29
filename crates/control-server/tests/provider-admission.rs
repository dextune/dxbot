//! Provider admission coverage for Bot activation and Task submission.

#![allow(clippy::expect_used)]

use std::sync::{Arc, Mutex};

use application::ApplicationMutator;
use control_server::{ControlServer, SecurityState, ServerError};
use dxbot_core::types::{
    BotId, BotSelector, CanonicalTarget, CasConditions, CommandId, CommandPayload, ContentSource,
    IdempotencyKey, InstanceId, OperationId, OperationRequest, PrincipalRef, ProjectId,
    ProjectSelector, RequestDigest, ScopeSelector,
};
use provider_host::{ProviderHost, ReferenceProvider};
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
            cas,
            content: None,
            semantic_options: fields,
        },
    }
}

fn revision_cas(revision: i64) -> CasConditions {
    CasConditions {
        if_revision: Some(revision),
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

fn project_revision_cas(revision: i64) -> CasConditions {
    let mut cas = revision_cas(0);
    cas.if_revision = None;
    cas.if_project_revision = Some(revision);
    cas
}

fn scope_revision_cas(revision: i64) -> CasConditions {
    let mut cas = revision_cas(0);
    cas.if_revision = None;
    cas.if_scope_revision = Some(revision);
    cas
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
            &request(
                "bot-create",
                "create",
                instance,
                None,
                json!({"name": "alpha"}),
            ),
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
                Some(revision_cas(1)),
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
            &request(
                "bot-create",
                "create-ready",
                instance,
                None,
                json!({"name": "alpha"}),
            ),
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
                Some(revision_cas(1)),
                json!({}),
            ),
        )
        .expect("ready provider admits activation");
    assert_eq!(result.status, "committed");
}

#[test]
fn operator_delegation_is_authorized_server_side_and_creates_recipient_execution() {
    let application = Arc::new(ApplicationMutator::new());
    for (sequence, name) in [("create-alpha", "alpha"), ("create-beta", "beta")] {
        application
            .mutate(&request(
                "bot-create",
                sequence,
                CanonicalTarget::Instance(InstanceId("instance-a".to_owned())),
                None,
                json!({"name": name}),
            ))
            .expect("bot create");
    }
    application
        .mutate(&request(
            "project-create",
            "project",
            CanonicalTarget::Instance(InstanceId("instance-a".to_owned())),
            None,
            json!({"name": "collaboration", "owner_bot": "bot:alpha"}),
        ))
        .expect("project create");
    let scope = ScopeSelector::Project(ProjectSelector::CanonicalId(ProjectId(
        "collaboration".to_owned(),
    )));
    application
        .mutate(&request(
            "project-member-set",
            "member-beta",
            CanonicalTarget::Membership {
                scope: scope.clone(),
                member_bot: BotSelector::CanonicalId(BotId("beta".to_owned())),
            },
            Some(project_revision_cas(1)),
            json!({"role_ref": "member"}),
        ))
        .expect("recipient membership");

    let security = Arc::new(Mutex::new(SecurityState::new()));
    let mut providers = ProviderHost::new();
    providers
        .register_reference_provider(ReferenceProvider::new(
            dxbot_core::types::ProviderId("reference".to_owned()),
            "llm-chat",
            4,
        ))
        .expect("test provider registration");
    let server = ControlServer::with_provider_host(security, Arc::clone(&application), providers);
    register_operator(&server);
    let mut delegated = request(
        "task-submit",
        "delegate-beta",
        CanonicalTarget::Project {
            id: ProjectId("collaboration".to_owned()),
            revision: 2,
        },
        Some(scope_revision_cas(2)),
        json!({
            "delegate_to_bot": "bot:beta",
            "requested_sender_bot": "bot:alpha",
            "budget": "32"
        }),
    );
    delegated.payload.content = Some(ContentSource::Text {
        value: "delegated work".to_owned(),
    });
    let result = server
        .handle_request(&principal(), &delegated)
        .expect("server-authorized delegation");
    assert_eq!(result.status, "committed");

    let state = application.snapshot().expect("snapshot");
    assert_eq!(state.delegations.len(), 1);
    assert_eq!(state.tasks.len(), 1);
    assert_eq!(state.executions.len(), 1);
    let task = state.tasks.values().next().expect("recipient task");
    assert_eq!(task.owner, "bot:beta");
    let execution = state.executions.values().next().expect("execution");
    assert_eq!(execution.context_plan.bot_ref.as_deref(), Some("bot:beta"));
    assert_eq!(execution.context_plan.scope_ref, "project:collaboration");
    assert_eq!(execution.context_plan.provider_binding.generation, 4);
}
