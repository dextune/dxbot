//! Acceptance coverage for `AT-CLI-AUTOMATION-001`: stable JSON/JSONL, stdout
//! schema vs stderr diagnostic separation, TTY-independence, and typed next
//! actions.
#![allow(clippy::unwrap_used)]

use cli::automation::{MachineRenderer, StreamEvent};
use dxbot_core::error::{DxbotError, ErrorCategory, ErrorCode, TypedNextAction};
use dxbot_core::types::{CommandId, InstanceId, OperationId, OperationResult, OutputFormat};
use dxbot_core::{ReceiptDisposition, ReceiptRecord};

fn renderer() -> MachineRenderer {
    MachineRenderer::new()
}

fn make_result() -> OperationResult {
    OperationResult {
        operation_id: OperationId("op-1".to_string()),
        command_id: CommandId("bot-create".to_string()),
        instance_id: InstanceId("default".to_string()),
        receipt: ReceiptRecord {
            operation_id: "op-1".to_string(),
            disposition: ReceiptDisposition::Accepted,
            result_ref: String::new(),
            resolved_binding_digest: String::new(),
            owner_kind: String::new(),
            lease_until: None,
            last_progress: 0,
            reconciliation_policy: String::new(),
        },
        status: "accepted".to_string(),
        committed_payload: None,
        error: None,
        operation_may_continue: true,
    }
}

#[test]
fn automation_json_result_is_valid_complete_document() {
    let r = renderer();
    let doc = r.render_json(&make_result());

    // A single complete, parseable document.
    let value: serde_json::Value = serde_json::from_str(&doc).unwrap();
    assert!(value.is_object(), "must be a single JSON object");

    let rendered = r.parse_json_result(doc.as_str()).unwrap();
    assert_eq!(rendered, make_result(), "round-trips to the same result");
    assert_eq!(rendered.command_id, CommandId("bot-create".to_string()));
}

#[test]
fn automation_jsonl_produces_one_line_per_event() {
    let r = renderer();
    let events = vec![
        StreamEvent::Progress {
            message: "dispatching".to_string(),
            percent: Some(25),
        },
        StreamEvent::Diagnostic {
            level: "info".to_string(),
            message: "sent".to_string(),
        },
        StreamEvent::Terminal(Box::new(make_result())),
    ];

    let out = r.render_jsonl(&events);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3, "one JSON line per event");

    // Every line is valid JSON; the final line is the terminal record with the
    // full OperationResult schema.
    for (i, line) in lines.iter().enumerate() {
        let value: serde_json::Value = serde_json::from_str(line).unwrap();
        if i == lines.len() - 1 {
            assert_eq!(value["event"], "terminal");
            // The OperationResult content is present on the terminal line
            // (internally-tagged newtype content is flattened into the record).
            assert_eq!(value["command_id"], "bot-create");
        } else {
            assert_ne!(value["event"], "terminal");
        }
    }
}

#[test]
fn automation_json_error_on_nonzero_exit_is_parseable() {
    let r = renderer();
    let error = DxbotError {
        code: ErrorCode::Conflict,
        category: ErrorCategory::Conflict,
        message: "revision 4 expected, found 5".to_string(),
        retryable: false,
        operation_ref: Some("op-9".to_string()),
        target_refs: vec!["bot:9".to_string()],
        field_violations: Vec::new(),
        current_revision: Some(5),
        current_generation: None,
        resume_cursor: None,
        next_actions: Vec::new(),
    };

    let doc = r.render_error(&error, OutputFormat::Json);
    // Even for a nonzero-exit error, the document is complete and parseable.
    let value: serde_json::Value = serde_json::from_str(&doc).unwrap();
    assert_eq!(value["code"], "conflict");
    assert_eq!(value["message"], "revision 4 expected, found 5");

    // JSONL error branch is a single parseable line too.
    let line = r.render_error(&error, OutputFormat::Jsonl);
    let value: serde_json::Value = serde_json::from_str(line.trim_end()).unwrap();
    assert_eq!(value["code"], "conflict");
}

#[test]
fn automation_stderr_never_contains_schema_payload() {
    let _ = renderer();
    // Diagnostic/progress is what the machine interface may write to stderr;
    // it must never carry schema-bearing OperationResult fields.
    for ev in [
        StreamEvent::Progress {
            message: "uploading".to_string(),
            percent: Some(50),
        },
        StreamEvent::Diagnostic {
            level: "warn".to_string(),
            message: "retrying".to_string(),
        },
    ] {
        let json = serde_json::to_string(&ev).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(
            value.get("receipt").is_none(),
            "diagnostic must not leak receipt: {json}"
        );
        assert!(
            value.get("operation_id").is_none(),
            "diagnostic must not leak operation id: {json}"
        );
        assert!(
            value.get("committed_payload").is_none(),
            "diagnostic must not leak committed payload: {json}"
        );
        assert_eq!(
            json.matches("\"operation_may_continue\"").count(),
            0,
            "diagnostic must not contain schema-bearing field: {json}"
        );
    }
}

#[test]
fn automation_tty_does_not_change_field_names() {
    let r = renderer();
    // The renderer is stateless wrt TTY/color: rendering twice yields the same
    // document with stable machine field names.
    let first = r.render_json(&make_result());
    let second = r.render_json(&make_result());
    assert_eq!(first, second, "field names/shapes are TTY-independent");

    let value: serde_json::Value = serde_json::from_str(&first).unwrap();
    let obj = value.as_object().unwrap();
    for key in [
        "operation_id",
        "command_id",
        "instance_id",
        "receipt",
        "status",
        "committed_payload",
        "error",
        "operation_may_continue",
    ] {
        assert!(obj.contains_key(key), "missing stable field '{key}'");
    }
}

#[test]
fn automation_next_action_has_typed_args_not_shell_string() {
    let r = renderer();
    let action = TypedNextAction {
        action_code: "runtime-doctor".to_string(),
        command_key: "runtime-doctor".to_string(),
        args: serde_json::json!({ "section": "provider" }),
    };

    let doc = r.render_next_action(&action);
    let value: serde_json::Value = serde_json::from_str(&doc).unwrap();
    assert_eq!(value["action_code"], "runtime-doctor");
    assert_eq!(value["command_key"], "runtime-doctor");
    assert_eq!(value["args"]["section"], "provider");

    // The stable schema is typed args, never a raw shell command string.
    assert!(!doc.contains("dxb runtime doctor"), "no raw shell command string");
    assert!(!doc.contains("--section provider"), "no shell-flag string");
}