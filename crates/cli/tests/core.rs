//! Acceptance coverage for `AT-CLI-CORE-001`.
//!
//! The convenience path must use `application-contract` as the canonical owner
//! of target/CAS materialization rather than redefining that policy in CLI.

#![allow(clippy::unwrap_used, clippy::panic)]

use std::collections::HashMap;

use cli::commands::{CoreCommands, TaskOptions};
use dxbot_core::types::{
    BotId, BotSelector, CanonicalTarget, ContentSource, ConversationId, InstanceId, PrincipalRef,
    TaskId,
};

fn commands() -> CoreCommands {
    CoreCommands::default()
}

fn text(value: &str) -> ContentSource {
    ContentSource::Text {
        value: value.to_string(),
    }
}

#[test]
fn core_bot_create_uses_canonical_instance_target() {
    let mut policies = HashMap::new();
    policies.insert("brain_policy".to_string(), "default".to_string());
    policies.insert("provider_policy".to_string(), "text".to_string());

    let payload = commands().bot_create("alpha", &policies).unwrap();
    assert_eq!(payload.command_key, "bot-create");
    assert_eq!(
        payload.canonical_target,
        CanonicalTarget::Instance(InstanceId("default".to_string()))
    );
    assert_eq!(payload.semantic_options["name"], "alpha");
    assert_eq!(payload.semantic_options["brain_policy"], "default");
    assert_eq!(payload.semantic_options["provider_policy"], "text");
    assert!(payload.cas.is_some());
}

#[test]
fn core_conversation_send_resolves_bot_main_conversation() {
    let payload = commands()
        .conversation_send("alpha", &text("hello there"))
        .unwrap();
    assert_eq!(payload.command_key, "conversation-send");
    match &payload.canonical_target {
        CanonicalTarget::Conversation { id, revision } => {
            assert_eq!(id, &ConversationId("alpha:main".to_string()));
            assert_eq!(*revision, 0);
        }
        other => panic!("expected Conversation target, got {other:?}"),
    }
    assert_eq!(payload.semantic_options["bot"], "alpha");
    assert_eq!(payload.content, Some(text("hello there")));
}

#[test]
fn core_task_submit_show_result_path() {
    let intent = text("implement the acceptance harness");
    let submit = commands()
        .task_submit("alpha", &intent, &TaskOptions::default())
        .unwrap();
    assert_eq!(submit.command_key, "task-submit");
    match &submit.canonical_target {
        CanonicalTarget::Task {
            id,
            revision: 0,
            execution_generation,
        } => {
            assert_eq!(id, &TaskId("alpha".to_string()));
            assert_eq!(*execution_generation, None);
        }
        other => panic!("expected Task target, got {other:?}"),
    }
    assert_eq!(submit.content, Some(intent));

    let show = commands().task_show("task:alpha").unwrap();
    assert_eq!(show.command_key, "task-show");
    assert!(matches!(
        show.canonical_target,
        CanonicalTarget::Task { ref id, .. } if id == &TaskId("alpha".to_string())
    ));

    let result = commands().task_result("task:alpha").unwrap();
    assert_eq!(result.command_key, "task-result");
    assert!(matches!(
        result.canonical_target,
        CanonicalTarget::Task { ref id, .. } if id == &TaskId("alpha".to_string())
    ));
    assert_eq!(result.semantic_options["task"], "task:alpha");
}

#[test]
fn core_task_submit_with_delegate_option() {
    let options = TaskOptions {
        delegate_to_bot: Some(BotSelector::CanonicalId(BotId("worker".to_string()))),
        deadline: Some(100),
        budget: Some(50),
    };
    let payload = commands()
        .task_submit("alpha", &text("delegated work"), &options)
        .unwrap();
    assert_eq!(payload.semantic_options["delegate_to_bot"], "bot:worker");
    assert_eq!(payload.semantic_options["deadline"], 100);
    assert_eq!(payload.semantic_options["budget"], 50);
}

#[test]
fn core_projection_preserves_boundary_identity_without_owning_target_policy() {
    let commands = CoreCommands::at(
        InstanceId("instance-a".to_string()),
        PrincipalRef("principal-a".to_string()),
    );
    let payload = commands.bot_create("alpha", &HashMap::new()).unwrap();
    assert_eq!(payload.instance_id, InstanceId("instance-a".to_string()));
    assert_eq!(payload.principal_ref, PrincipalRef("principal-a".to_string()));
    assert_eq!(
        payload.canonical_target,
        CanonicalTarget::Instance(InstanceId("instance-a".to_string()))
    );
}
