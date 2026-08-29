//! Stable machine interface for non-interactive automation (AT-CLI-AUTOMATION-001).
//!
//! Contract:
//!
//! - `--format json` emits exactly one complete, parseable result/error
//!   document on stdout — never partial, never interleaved.
//! - `--format jsonl` emits one JSON line per stream event, with a terminal
//!   record last.
//! - stdout carries the schema-bearing payload; stderr carries only
//!   diagnostic/progress. Nothing schema-bearing ever goes to stderr.
//! - TTY presence, color, pager, and locale never change machine field names
//!   or shapes.
//! - Next actions are typed: `action_code`, `command_key`, and typed `args`.
//!   A raw shell command string is never the stable schema.

#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions)]

use dxbot_core::error::{DxbotError, ErrorCategory, ErrorCode, TypedNextAction};
use dxbot_core::types::{CommandId, InstanceId, OperationResult, OutputFormat};
use dxbot_core::{ReceiptDisposition, ReceiptRecord};
use serde::{Deserialize, Serialize};

/// One event in a JSONL stream. Rendered one JSON line per event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "kebab-case")]
pub enum StreamEvent {
    /// Optional human-oriented progress hint (diagnostic channel only).
    Progress {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        percent: Option<u8>,
    },
    /// A diagnostic message with a machine-understandable level.
    Diagnostic { level: String, message: String },
    /// The terminal OperationResult record completing the stream.
    Terminal(Box<OperationResult>),
}

/// Errors produced by the machine interface renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The input is not a single complete JSON document.
    InvalidJson(String),
    /// The parsed document is not a valid OperationResult.
    InvalidResult(String),
}

impl Error {
    /// Projects onto the canonical [`DxbotError`] surface so a machine consumer
    /// still receives a complete, parseable error document.
    pub fn to_dxbot_error(&self) -> DxbotError {
        let (code, message) = match self {
            Self::InvalidJson(m) => (
                ErrorCode::StorageOrCorruption,
                format!("cannot parse machine result payload: {m}"),
            ),
            Self::InvalidResult(m) => (
                ErrorCode::InvalidInput,
                format!("machine result is not a valid OperationResult: {m}"),
            ),
        };
        DxbotError {
            code,
            category: ErrorCategory::Input,
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

/// Renders operation results, stream events, and errors to the machine
/// contract. Stateless: field names and shapes never depend on TTY/color.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MachineRenderer;

impl MachineRenderer {
    pub fn new() -> Self {
        Self
    }

    /// Renders a single complete JSON result document (one valid JSON value).
    pub fn render_json(&self, result: &OperationResult) -> String {
        serde_json::to_string(result)
            .unwrap_or_else(|e| self.render_serialize_failure(e.to_string()))
    }

    /// Renders a JSONL stream: one JSON line per event, terminal record last.
    ///
    /// If the supplied events do not already end with a `Terminal` record, a
    /// terminal record is appended so the final line is always the terminal.
    pub fn render_jsonl(&self, events: &[StreamEvent]) -> String {
        let mut events: Vec<StreamEvent> = events.to_vec();
        if !matches!(events.last(), Some(StreamEvent::Terminal(_))) {
            let terminal = events
                .iter()
                .rev()
                .find(|e| matches!(e, StreamEvent::Terminal(_)))
                .cloned()
                .unwrap_or_else(synthetic_terminal);
            events.push(terminal);
        }

        let mut out = String::new();
        for ev in &events {
            let line = serde_json::to_string(ev)
                .unwrap_or_else(|e| self.render_serialize_failure(e.to_string()));
            out.push_str(&line);
            out.push('\n');
        }
        out
    }

    /// Parses a complete JSON result document back into an [`OperationResult`].
    pub fn parse_json_result(&self, input: &str) -> Result<OperationResult, Error> {
        let value: serde_json::Value =
            serde_json::from_str(input).map_err(|e| Error::InvalidJson(e.to_string()))?;
        serde_json::from_value(value).map_err(|e| Error::InvalidResult(e.to_string()))
    }

    /// Renders a [`DxbotError`] in the requested output format.
    ///
    /// Machine rendering supplies registry-defined safe recovery actions when
    /// the owner did not already provide a more specific action set. `json`
    /// yields one complete error document; `jsonl` yields one error document
    /// line; `human` remains a concise diagnostic.
    pub fn render_error(&self, error: &DxbotError, format: OutputFormat) -> String {
        let projected = project_cli_error(error);
        match format {
            OutputFormat::Json => serde_json::to_string(&projected)
                .unwrap_or_else(|e| self.render_serialize_failure(e.to_string())),
            OutputFormat::Jsonl => {
                let mut s = serde_json::to_string(&projected)
                    .unwrap_or_else(|e| self.render_serialize_failure(e.to_string()));
                s.push('\n');
                s
            }
            OutputFormat::Human => {
                let mut rendered = format!("error {:?}: {}", projected.code, projected.message);
                for action in &projected.next_actions {
                    let args =
                        serde_json::to_string(&action.args).unwrap_or_else(|_| "{}".to_owned());
                    rendered.push_str(&format!(
                        "\nnext action: {} {} ({})",
                        action.command_key, args, action.action_code
                    ));
                }
                rendered
            }
        }
    }

    /// Renders a typed next action as a single machine document: `action_code`,
    /// `command_key`, and typed `args`. The stable schema is never a raw shell
    /// command string.
    pub fn render_next_action(&self, action: &TypedNextAction) -> String {
        serde_json::to_string(action)
            .unwrap_or_else(|e| self.render_serialize_failure(e.to_string()))
    }

    /// Renders the full typed next-action set as a JSON array of documents.
    pub fn render_next_actions(&self, actions: &[TypedNextAction]) -> String {
        serde_json::to_string(actions)
            .unwrap_or_else(|e| self.render_serialize_failure(e.to_string()))
    }

    fn render_serialize_failure(&self, message: String) -> String {
        // A serialization failure must still produce a complete, parseable
        // document.
        serde_json::json!({
            "error": {
                "code": "internal-invariant",
                "category": "internal",
                "message": message,
                "retryable": false
            }
        })
        .to_string()
    }
}

fn project_cli_error(error: &DxbotError) -> DxbotError {
    let mut projected = error.clone();
    if !projected.next_actions.is_empty() {
        return projected;
    }
    projected.next_actions = match projected.code {
        ErrorCode::RuntimeUnavailable => vec![
            action("inspect-runtime", "runtime-status", serde_json::json!({})),
            action("start-runtime", "runtime-start", serde_json::json!({})),
            action("diagnose-runtime", "runtime-doctor", serde_json::json!({})),
        ],
        ErrorCode::ProviderUnavailable => vec![action(
            "diagnose-provider",
            "runtime-doctor",
            serde_json::json!({"section": "provider"}),
        )],
        ErrorCode::ApprovalRequired => approval_action(&projected).into_iter().collect(),
        ErrorCode::RecoveryRequired => recovery_actions(&projected),
        ErrorCode::Incompatible => {
            vec![action("inspect-version", "version", serde_json::json!({}))]
        }
        ErrorCode::PartialOrResync => projected
            .resume_cursor
            .as_ref()
            .map(|cursor| {
                vec![action(
                    "resume-or-restart",
                    "runtime-doctor",
                    serde_json::json!({"resume_cursor": cursor}),
                )]
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    projected
}

fn approval_action(error: &DxbotError) -> Option<TypedNextAction> {
    let approval = error
        .target_refs
        .iter()
        .find(|target| target.starts_with("approval:"))?;
    Some(action(
        "show-approval",
        "approval-show",
        serde_json::json!({"approval": approval}),
    ))
}

fn recovery_actions(error: &DxbotError) -> Vec<TypedNextAction> {
    let mut actions = Vec::new();
    if let Some(operation) = error.operation_ref.as_ref() {
        actions.push(action(
            "show-operation",
            "operation-show",
            serde_json::json!({"operation": operation}),
        ));
    }
    actions.push(action(
        "diagnose-journal",
        "runtime-doctor",
        serde_json::json!({"section": "journal"}),
    ));
    actions
}

fn action(action_code: &str, command_key: &str, args: serde_json::Value) -> TypedNextAction {
    TypedNextAction {
        action_code: action_code.to_owned(),
        command_key: command_key.to_owned(),
        args,
    }
}

/// Builds a minimal, complete terminal record used only when a caller omits the
/// terminal `StreamEvent::Terminal` from a JSONL stream.
fn synthetic_terminal() -> StreamEvent {
    let op = OperationResult {
        operation_id: dxbot_core::types::OperationId(String::new()),
        command_id: CommandId(String::new()),
        instance_id: InstanceId(String::new()),
        receipt: ReceiptRecord {
            operation_id: String::new(),
            disposition: ReceiptDisposition::Rejected,
            result_ref: String::new(),
            resolved_binding_digest: String::new(),
            owner_kind: String::new(),
            lease_until: None,
            last_progress: 0,
            reconciliation_policy: String::new(),
        },
        status: "unknown".to_string(),
        committed_payload: None,
        error: None,
        operation_may_continue: false,
    };
    StreamEvent::Terminal(Box::new(op))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    fn error(code: ErrorCode) -> DxbotError {
        DxbotError {
            code,
            category: ErrorCategory::Availability,
            message: "test".to_owned(),
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

    #[test]
    fn provider_failure_gets_typed_doctor_action() {
        let renderer = MachineRenderer::new();
        let rendered =
            renderer.render_error(&error(ErrorCode::ProviderUnavailable), OutputFormat::Json);
        let value: serde_json::Value = serde_json::from_str(&rendered).expect("json");
        assert_eq!(value["next_actions"][0]["command_key"], "runtime-doctor");
        assert_eq!(value["next_actions"][0]["args"]["section"], "provider");
    }

    #[test]
    fn owner_supplied_actions_are_not_replaced() {
        let mut input = error(ErrorCode::RuntimeUnavailable);
        input.next_actions.push(action(
            "custom",
            "runtime-status",
            serde_json::json!({"detail": true}),
        ));
        let rendered = MachineRenderer::new().render_error(&input, OutputFormat::Json);
        let value: serde_json::Value = serde_json::from_str(&rendered).expect("json");
        assert_eq!(value["next_actions"].as_array().expect("array").len(), 1);
        assert_eq!(value["next_actions"][0]["action_code"], "custom");
    }

    #[test]
    fn human_failures_include_typed_next_actions() {
        let renderer = MachineRenderer::new();
        for (code, expected_command) in [
            (ErrorCode::RuntimeUnavailable, "runtime-status"),
            (ErrorCode::ProviderUnavailable, "runtime-doctor"),
            (ErrorCode::RecoveryRequired, "runtime-doctor"),
        ] {
            let rendered = renderer.render_error(&error(code), OutputFormat::Human);
            assert!(rendered.contains("next action:"));
            assert!(rendered.contains(expected_command));
            assert!(
                !rendered.contains("dxb "),
                "human action is not a stable shell string"
            );
        }

        let mut approval = error(ErrorCode::ApprovalRequired);
        approval.target_refs.push("approval:approval-1".to_owned());
        let rendered = renderer.render_error(&approval, OutputFormat::Human);
        assert!(rendered.contains("approval-show"));
        assert!(rendered.contains("approval:approval-1"));
    }
}
