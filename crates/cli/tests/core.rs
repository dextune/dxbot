//! Acceptance coverage for `AT-CLI-CORE-001`: the CLI core Bot-only path.
//!
//! Verifies the boundary-local `operation → CommandPayload` projection for
//! bot create, conversation send, and the task submit → show → result path.
#![allow(clippy::unwrap_used)]

use std::collections::HashMap;

use cli::commands::{CoreCommands, TaskOptions};
use dxbot_core::types::{
    BotId, BotSelector, CanonicalTarget, ContentSource, ConversationId,
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
fn core_bot_create_produces_valid_payload() {
    let mut policies = HashMap::new();
    policies.insert("approval".to_string(), "required".to_string());
    policies.insert("memory".to_string(), "shared".to_string());

    let payload = commands().bot_create("alpha", &policies).unwrap();

    assert_eq!(payload.command_key, "bot-create");
    // Correct BotRef: the canonical target is the created Bot id.
    match &payload.canonical_target {
        CanonicalTarget::Bot { id, revision } => {
            assert_eq!(id, &BotId("alpha".to_string()));
            assert_eq!(*revision, 0);
        }
        other => panic!("expected Bot canonical target, got {other:?}"),
    }
    // Correct MainConversationRef: the created bot's main conversation id.
    assert_eq!(payload.semantic_options["main_conversation"], "alpha:main");
    assert_eq!(payload.semantic_options["name"], "alpha");
    assert_eq!(payload.semantic_options["policies"]["approval"], "required");
}

#[test]
fn core_conversation_send_resolves_bot_main_conversation() {
    let payload = commands()
        .conversation_send("alpha", &text("hello there"))
        .unwrap();

    assert_eq!(payload.command_key, "conversation-send");
    // Send resolves to the bot's main conversation id.
    match &payload.canonical_target {
        CanonicalTarget::Conversation { id, revision } => {
            assert_eq!(id, &ConversationId("alpha:main".to_string()));
            assert_eq!(*revision, 0);
        }
        other => panic!("expected Conversation target, got {other:?}"),
    }
    // The bot selector is retained for authoritative re-resolution.
    assert_eq!(payload.semantic_options["bot"], "alpha");
    // Content is carried through.
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
    // Content intent is carried through.
    assert_eq!(submit.content, Some(intent.clone()));

    // Show resolves the same canonical task.
    let show = commands().task_show("task:alpha").unwrap();
    assert_eq!(show.command_key, "task-show");
    assert!(matches!(
        show.canonical_target,
        CanonicalTarget::Task { id: _, .. }
    ));

    // Result resolves the same canonical task, via the parsed selector.
    let result = commands().task_result("task:alpha").unwrap();
    assert_eq!(result.command_key, "task-result");
    assert!(matches!(
        result.canonical_target,
        CanonicalTarget::Task { id: _, .. }
    ));

    // The semantic option carries the selector used.
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

    // Delegation option is projected into the payload, canonically encoded.
    assert_eq!(payload.semantic_options["delegate_to_bot"], "bot:worker");
    assert_eq!(payload.semantic_options["deadline"], 100);
    assert_eq!(payload.semantic_options["budget"], 50);

    // Without delegation the option is absent from the payload.
    let plain = commands()
        .task_submit("alpha", &text("work"), &TaskOptions::default())
        .unwrap();
    assert!(plain.semantic_options["delegate_to_bot"].is_null());
}