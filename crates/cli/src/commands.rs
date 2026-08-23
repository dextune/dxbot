//! CLI core command projection: `CoreCommands` owns the boundary-local
//! `operation → CommandPayload` projection for the core Bot path
//! (`AT-CLI-CORE-001`).
//!
//! This module only depends on `dxbot-core` and standard types. It never
//! imports application or control-client crates; the CLI sits at the boundary
//! and only projects into the canonical [`CommandPayload`] surface. All values
//! are deterministic and testable without a live Runtime.
//!
//! [`CommandPayload`]: dxbot_core::types::CommandPayload

#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;

use dxbot_core::error::{DxbotError, ErrorCategory, ErrorCode};
use dxbot_core::types::{
    BotId, BotSelector, CanonicalTarget, CasConditions, CommandPayload, ContentSource,
    ConversationId, InstanceId, PrincipalRef, TaskId, TaskSelector,
};
use serde_json::json;

/// The canonical instance id this projection targets by default.
const DEFAULT_INSTANCE_ID: &str = "default";
/// The canonical local principal used by the CLI projection.
const DEFAULT_PRINCIPAL: &str = "cli";

/// Command keys produced by this module.
pub const CMD_BOT_CREATE: &str = "bot-create";
pub const CMD_CONVERSATION_SEND: &str = "conversation-send";
pub const CMD_TASK_SUBMIT: &str = "task-submit";
pub const CMD_TASK_SHOW: &str = "task-show";
pub const CMD_TASK_RESULT: &str = "task-result";

/// A well-known reference to a bot's main conversation.
const MAIN_CONV_SUFFIX: &str = ":main";

/// Errors produced by the CLI core command projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandsError {
    /// The supplied name/owner is empty and cannot form a canonical id.
    InvalidName { name: String },
}

impl CommandsError {
    /// Projects this error onto the canonical [`DxbotError`] surface.
    pub fn to_dxbot_error(&self) -> DxbotError {
        match self {
            Self::InvalidName { name } => DxbotError {
                code: ErrorCode::InvalidInput,
                category: ErrorCategory::Input,
                message: format!("name/owner must not be empty: {name:?}"),
                retryable: false,
                operation_ref: None,
                target_refs: Vec::new(),
                field_violations: Vec::new(),
                current_revision: None,
                current_generation: None,
                resume_cursor: None,
                next_actions: Vec::new(),
            },
        }
    }
}

/// Options controlling how a task is submitted.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskOptions {
    /// Optional bot the task is delegated to. When present it is carried in
    /// the payload's semantic options so the server can re-resolve it.
    pub delegate_to_bot: Option<BotSelector>,
    /// Optional wall-clock deadline (unix seconds).
    pub deadline: Option<i64>,
    /// Optional budget constraint.
    pub budget: Option<i64>,
}

/// Projection entry point for the core Bot path.
///
/// Holds the boundary identity (`InstanceId` + `PrincipalRef`) applied to every
/// command payload it produces. All methods are pure and deterministic: they
/// build a valid [`CommandPayload`] without touching a Runtime or journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreCommands {
    instance_id: InstanceId,
    principal_ref: PrincipalRef,
}

impl Default for CoreCommands {
    fn default() -> Self {
        Self::at(
            InstanceId(DEFAULT_INSTANCE_ID.to_string()),
            PrincipalRef(DEFAULT_PRINCIPAL.to_string()),
        )
    }
}

impl CoreCommands {
    /// Creates a projection bound to the given boundary identity.
    pub fn at(instance_id: InstanceId, principal_ref: PrincipalRef) -> Self {
        Self {
            instance_id,
            principal_ref,
        }
    }

    /// Creates a bot-create command payload for a new bot named `name`.
    ///
    /// The payload's canonical target is the created [`BotId`], and the bot's
    /// main conversation is recorded as a [`ConversationSelector::BotMain`]
    /// reference in the semantic options so the server re-resolves it.
    pub fn bot_create(
        &self,
        name: &str,
        policies: &HashMap<String, String>,
    ) -> Result<CommandPayload, CommandsError> {
        if name.is_empty() {
            return Err(CommandsError::InvalidName { name: name.into() });
        }
        let bot_id = BotId(name.to_string());
        let target = CanonicalTarget::Bot {
            id: bot_id.clone(),
            revision: 0,
        };
        // The bot's main conversation is recorded as a derived conversation-id
        // reference (`<bot-name>:main`) in the semantic options. The typed
        // [`BotSelector`]/[`ConversationSelector`] enums carry `serde(tag)`
        // newtype variants that cannot be (de)serialized as standalone JSON, so
        // the projection encodes references canonically as strings.
        let opts = json!({
            "name": name,
            "policies": policies,
            "main_conversation": format!("{name}{MAIN_CONV_SUFFIX}"),
        });
        Ok(self.payload(CMD_BOT_CREATE, target, None, None, opts))
    }

    /// Sends `content` into the given bot's main conversation.
    ///
    /// Resolves the bot selector to the bot's main conversation: the target
    /// conversation id is derived as `<bot-name>:main`. The original
    /// [`ConversationSelector::BotMain`] reference is retained in the
    /// semantic options so the server re-resolves the authoritative id.
    pub fn conversation_send(
        &self,
        bot_selector: &str,
        content: &ContentSource,
    ) -> Result<CommandPayload, CommandsError> {
        if bot_selector.is_empty() {
            return Err(CommandsError::InvalidName {
                name: bot_selector.into(),
            });
        }
        let selector = parse_bot_selector(bot_selector);
        let name = bot_name(&selector).to_string();
        let main_id = ConversationId(format!("{name}{MAIN_CONV_SUFFIX}"));
        let target = CanonicalTarget::Conversation {
            id: main_id,
            revision: 0,
        };
        // The original selector is retained as a canonical string reference so
        // the server can authoritative re-resolve the main conversation id.
        let opts = json!({
            "bot": bot_selector,
        });
        Ok(self.payload(
            CMD_CONVERSATION_SEND,
            target,
            None,
            Some(content.clone()),
            opts,
        ))
    }

    /// Submits a task owned by `owner` with `intent` as its content source.
    ///
    /// The task id is derived from the owner so the submit → show → result
    /// path is deterministic and testable.
    pub fn task_submit(
        &self,
        owner: &str,
        intent: &ContentSource,
        options: &TaskOptions,
    ) -> Result<CommandPayload, CommandsError> {
        if owner.is_empty() {
            return Err(CommandsError::InvalidName { name: owner.into() });
        }
        let task_id = TaskId(owner.to_string());
        let target = CanonicalTarget::Task {
            id: task_id,
            revision: 0,
            execution_generation: None,
        };
        let opts = json!({
            "owner": owner,
            "delegate_to_bot": options.delegate_to_bot.as_ref().map(encode_bot),
            "deadline": options.deadline,
            "budget": options.budget,
        });
        Ok(self.payload(
            CMD_TASK_SUBMIT,
            target,
            None,
            Some(intent.clone()),
            opts,
        ))
    }

    /// Shows the task identified by `task_selector`.
    pub fn task_show(
        &self,
        task_selector: &str,
    ) -> Result<CommandPayload, CommandsError> {
        let task_id = task_name(&parse_task_selector(task_selector));
        let target = CanonicalTarget::Task {
            id: TaskId(task_id),
            revision: 0,
            execution_generation: None,
        };
        Ok(self.payload(
            CMD_TASK_SHOW,
            target,
            None,
            None,
            json!({ "task": task_selector }),
        ))
    }

    /// Gets the result of the task identified by `task_selector`.
    pub fn task_result(
        &self,
        task_selector: &str,
    ) -> Result<CommandPayload, CommandsError> {
        let task_id = task_name(&parse_task_selector(task_selector));
        let target = CanonicalTarget::Task {
            id: TaskId(task_id),
            revision: 0,
            execution_generation: None,
        };
        Ok(self.payload(
            CMD_TASK_RESULT,
            target,
            None,
            None,
            json!({ "task": task_selector }),
        ))
    }

    fn payload(
        &self,
        command_key: &str,
        canonical_target: CanonicalTarget,
        cas: Option<CasConditions>,
        content: Option<ContentSource>,
        semantic_options: serde_json::Value,
    ) -> CommandPayload {
        CommandPayload {
            command_key: command_key.to_string(),
            principal_ref: self.principal_ref.clone(),
            instance_id: self.instance_id.clone(),
            canonical_target,
            cas,
            content,
            semantic_options,
        }
    }
}

/// Parses a bot selector string into a [`BotSelector`]. A `bot:`-prefixed value
/// is treated as a canonical id; anything else is a scoped-exact name.
fn parse_bot_selector(selector: &str) -> BotSelector {
    if let Some(rest) = selector.strip_prefix("bot:") {
        BotSelector::CanonicalId(BotId(rest.to_string()))
    } else {
        BotSelector::ScopedExact(selector.to_string())
    }
}

/// Parses a task selector string into a [`TaskSelector`]. A `task:`-prefixed
/// value is treated as a canonical id; anything else is a scoped-exact name.
fn parse_task_selector(selector: &str) -> TaskSelector {
    if let Some(rest) = selector.strip_prefix("task:") {
        TaskSelector::CanonicalId(TaskId(rest.to_string()))
    } else {
        TaskSelector::ScopedExact(selector.to_string())
    }
}

/// The underlying name of a bot selector (the canonical id value or the
/// scoped-exact name).
fn bot_name(selector: &BotSelector) -> &str {
    match selector {
        BotSelector::CanonicalId(id) => &id.0,
        BotSelector::ScopedExact(name) => name,
    }
}

/// Canonical string encoding of a bot selector, safe for JSON projection. The
/// typed enum uses `serde(tag)` newtype variants that cannot be (de)serialized
/// standalone, so references are projected as canonical strings instead.
fn encode_bot(selector: &BotSelector) -> String {
    match selector {
        BotSelector::CanonicalId(id) => format!("bot:{}", id.0),
        BotSelector::ScopedExact(name) => name.clone(),
    }
}

/// The underlying name of a task selector.
fn task_name(selector: &TaskSelector) -> String {
    match selector {
        TaskSelector::CanonicalId(id) => id.0.clone(),
        TaskSelector::ScopedExact(name) => name.clone(),
    }
}