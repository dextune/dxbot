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
    Diagnostic {
        level: String,
        message: String,
    },
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
        serde_json::to_string(result).unwrap_or_else(|e| self.render_serialize_failure(e.to_string()))
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
    /// `json` yields one complete error document (parseable even on nonzero
    /// exit); `jsonl` yields one error document line; `human` yields a readable
    /// message. None of the machine branches touch stderr.
    pub fn render_error(&self, error: &DxbotError, format: OutputFormat) -> String {
        match format {
            OutputFormat::Json => serde_json::to_string(error)
                .unwrap_or_else(|e| self.render_serialize_failure(e.to_string())),
            OutputFormat::Jsonl => {
                let mut s = serde_json::to_string(error)
                    .unwrap_or_else(|e| self.render_serialize_failure(e.to_string()));
                s.push('\n');
                s
            }
            OutputFormat::Human => format!("error {:?}: {}", error.code, error.message),
        }
    }

    /// Renders a typed next action as a single machine document: `action_code`,
    /// `command_key`, and typed `args`. The stable schema is never a raw shell
    /// command string.
    pub fn render_next_action(&self, action: &TypedNextAction) -> String {
        serde_json::to_string(action).unwrap_or_else(|e| self.render_serialize_failure(e.to_string()))
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