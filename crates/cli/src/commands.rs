//! CLI core convenience projection for `AT-CLI-CORE-001`.
//!
//! Canonical target/CAS materialization is delegated to `application-contract`;
//! this module only assembles the already-resolved first-use-path inputs.

#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;

use application_contract::{CliInput, CommandPayload as ContractPayload};
use dxbot_core::error::{DxbotError, ErrorCategory, ErrorCode};
use dxbot_core::types::{
    BotId, BotSelector, CliGlobalOptions, CommandPayload, ContentSource, InstanceId, PrincipalRef,
};
use serde_json::{Map, Value, json};

const DEFAULT_INSTANCE_ID: &str = "default";
const DEFAULT_PRINCIPAL: &str = "cli";
const MAIN_CONV_SUFFIX: &str = ":main";
const BOT_CREATE_POLICY_KEYS: [&str; 4] = [
    "brain_policy",
    "permission_policy",
    "resource_policy",
    "provider_policy",
];

pub const CMD_BOT_CREATE: &str = "bot-create";
pub const CMD_CONVERSATION_SEND: &str = "conversation-send";
pub const CMD_TASK_SUBMIT: &str = "task-submit";
pub const CMD_TASK_SHOW: &str = "task-show";
pub const CMD_TASK_RESULT: &str = "task-result";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandsError {
    InvalidName { name: String },
    InvalidPolicyKey { key: String },
    Projection { message: String },
}

impl CommandsError {
    pub fn to_dxbot_error(&self) -> DxbotError {
        let (code, category, message) = match self {
            Self::InvalidName { name } => (
                ErrorCode::InvalidInput,
                ErrorCategory::Input,
                format!("name/owner must not be empty: {name:?}"),
            ),
            Self::InvalidPolicyKey { key } => (
                ErrorCode::InvalidInput,
                ErrorCategory::Input,
                format!("unsupported bot-create policy field: {key}"),
            ),
            Self::Projection { message } => (
                ErrorCode::InternalInvariant,
                ErrorCategory::Internal,
                message.clone(),
            ),
        };
        DxbotError {
            code,
            category,
            message,
            retryable: false,
            operation_ref: None,
            target_refs: Vec::new(),
            field_violations: Vec::new(),
            current_revision: None,
            current_generation: None,
            resume_cursor: None,
            next_actions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskOptions {
    pub delegate_to_bot: Option<BotSelector>,
    pub deadline: Option<i64>,
    pub budget: Option<i64>,
}

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
    pub fn at(instance_id: InstanceId, principal_ref: PrincipalRef) -> Self {
        Self {
            instance_id,
            principal_ref,
        }
    }

    pub fn bot_create(
        &self,
        name: &str,
        policies: &HashMap<String, String>,
    ) -> Result<CommandPayload, CommandsError> {
        validate_name(name)?;
        let mut fields = Map::new();
        fields.insert("name".to_string(), Value::String(name.to_string()));
        for (key, value) in policies {
            if !BOT_CREATE_POLICY_KEYS.contains(&key.as_str()) {
                return Err(CommandsError::InvalidPolicyKey { key: key.clone() });
            }
            fields.insert(key.clone(), Value::String(value.clone()));
        }
        self.project(CMD_BOT_CREATE, None, None, Value::Object(fields))
    }

    pub fn conversation_send(
        &self,
        bot_selector: &str,
        content: &ContentSource,
    ) -> Result<CommandPayload, CommandsError> {
        validate_name(bot_selector)?;
        let bot = parse_bot_selector(bot_selector);
        let conversation = format!("{}{}", bot_name(&bot), MAIN_CONV_SUFFIX);
        self.project(
            CMD_CONVERSATION_SEND,
            Some(selector("conversation", &conversation)),
            Some(content.clone()),
            json!({ "bot": bot_selector }),
        )
    }

    pub fn task_submit(
        &self,
        owner: &str,
        intent: &ContentSource,
        options: &TaskOptions,
    ) -> Result<CommandPayload, CommandsError> {
        validate_name(owner)?;
        self.project(
            CMD_TASK_SUBMIT,
            Some(selector("scope", owner)),
            Some(intent.clone()),
            json!({
                "delegate_to_bot": options.delegate_to_bot.as_ref().map(encode_bot),
                "deadline": options.deadline,
                "budget": options.budget,
            }),
        )
    }

    pub fn task_show(&self, task_selector: &str) -> Result<CommandPayload, CommandsError> {
        validate_name(task_selector)?;
        let task = task_selector.strip_prefix("task:").unwrap_or(task_selector);
        self.project(
            CMD_TASK_SHOW,
            Some(selector("task", task)),
            None,
            json!({ "task": task_selector }),
        )
    }

    pub fn task_result(&self, task_selector: &str) -> Result<CommandPayload, CommandsError> {
        validate_name(task_selector)?;
        let task = task_selector.strip_prefix("task:").unwrap_or(task_selector);
        self.project(
            CMD_TASK_RESULT,
            Some(selector("task", task)),
            None,
            json!({ "task": task_selector }),
        )
    }

    fn project(
        &self,
        command_key: &str,
        selector: Option<Value>,
        content: Option<ContentSource>,
        fields: Value,
    ) -> Result<CommandPayload, CommandsError> {
        let input = CliInput {
            command_key: command_key.to_string(),
            global_options: CliGlobalOptions {
                instance: Some(self.instance_id.0.clone()),
                ..CliGlobalOptions::default()
            },
            selector,
            cas: None,
            content,
            fields,
        };
        let mut payload = ContractPayload::from_cli_input(&input)
            .map_err(|error| CommandsError::Projection {
                message: error.message,
            })?
            .inner;
        payload.principal_ref = self.principal_ref.clone();
        Ok(payload)
    }
}

fn validate_name(value: &str) -> Result<(), CommandsError> {
    if value.trim().is_empty() {
        return Err(CommandsError::InvalidName {
            name: value.to_string(),
        });
    }
    Ok(())
}

fn selector(kind: &str, value: &str) -> Value {
    json!({ "kind": kind, "value": value })
}

fn parse_bot_selector(selector: &str) -> BotSelector {
    if let Some(rest) = selector.strip_prefix("bot:") {
        BotSelector::CanonicalId(BotId(rest.to_string()))
    } else {
        BotSelector::ScopedExact(selector.to_string())
    }
}

fn bot_name(selector: &BotSelector) -> &str {
    match selector {
        BotSelector::CanonicalId(id) => &id.0,
        BotSelector::ScopedExact(name) => name,
    }
}

fn encode_bot(selector: &BotSelector) -> String {
    match selector {
        BotSelector::CanonicalId(id) => format!("bot:{}", id.0),
        BotSelector::ScopedExact(name) => name.clone(),
    }
}
