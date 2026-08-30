//! Standalone fake ACP agent for `DeepSeekHarnessAcpProvider` conformance.
//!
//! Compiled with `rustc` (std only, no dependencies) at test time and spawned
//! as a real child. It speaks the agent side of ACP v1 newline-delimited
//! JSON-RPC 2.0 over stdin/stdout and is fully scripted by `--mode <name>`.
//! stdout carries protocol frames only; diagnostics go to stderr.
//!
//! Modes:
//! - success            : normal initialize/new/prompt → one message chunk → end_turn
//! - route-drift         : session/new advertises a non-MiniMax model currentValue (fail closed)
//! - version-drift       : initialize advertises protocolVersion 2 (must fail closed)
//! - auth-methods        : initialize advertises a non-empty authMethods (fail closed)
//! - contaminated        : print a non-JSON line to stdout before the initialize reply
//! - malformed           : send a session/update whose JSON is structurally wrong
//! - oversized           : send a message chunk far larger than max_line_bytes
//! - unknown-id          : reply to initialize with an id the client never issued
//! - permission          : request permission before answering; honour the decision
//! - crash               : exit hard after session/new, before any prompt result
//! - startup-crash       : exit during startup BEFORE answering initialize (never-started server)
//! - handshake-hang      : record initialize receipt, then never answer it
//! - hang                : never resolve the prompt (wait for session/cancel)
//! - grandchild-leak     : fork a long-lived grandchild, record its PID, then hang
//! - echo-secret-stderr  : write the MINIMAX_API_KEY to stderr, then succeed
//! - echo-env-names      : stream the received env var NAMES as the message text

use std::env;
use std::io::{BufRead, Write};

fn main() {
    let mode = parse_mode();
    // startup-crash: model the real DSH launcher exiting during startup BEFORE
    // it ever answers `initialize` (e.g. an unresolved runtime/module tree). The
    // client reads a clean EOF during the handshake with no protocol frame at
    // all — the provider must classify this as a transport/startup failure, not
    // a mid-turn truncation or a generic execution failure.
    if mode == "startup-crash" {
        std::process::exit(3);
    }
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    let mut lines = stdin.lock().lines();
    let mut session_id = String::new();

    while let Some(Ok(line)) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let id = extract_number(trimmed, "\"id\":");
        let method = extract_string(trimmed, "\"method\":");
        match method.as_deref() {
            Some("initialize") => {
                if mode == "handshake-hang" {
                    if let Ok(home) = env::var("DSH_HOME") {
                        let marker = std::path::Path::new(&home).join("initialize.received");
                        let _ = std::fs::write(marker, b"received");
                    }
                    // Keep stdin open and wait for cancellation-driven teardown;
                    // never emit an initialize response.
                    continue;
                }
                if mode == "contaminated" {
                    // Non-JSON contamination on the protocol stream.
                    writeln!(out, "this is not json rpc").ok();
                    out.flush().ok();
                }
                let reply_id = if mode == "unknown-id" {
                    99_999
                } else {
                    id.unwrap_or(0)
                };
                let version = if mode == "version-drift" { 2 } else { 1 };
                let auth = if mode == "auth-methods" {
                    "[{\"id\":\"token\"}]"
                } else {
                    "[]"
                };
                let frame = format!(
                    "{{\"jsonrpc\":\"2.0\",\"id\":{reply_id},\"result\":{{\"protocolVersion\":{version},\"authMethods\":{auth},\"agentCapabilities\":{{\"promptCapabilities\":{{\"image\":false}}}}}}}}"
                );
                send(&mut out, &frame);
            }
            Some("session/new") => {
                session_id = "sess-fake-1".to_owned();
                if mode == "crash" {
                    std::process::exit(17);
                }
                if mode == "grandchild-leak" || mode == "grandchild-leak-crash" {
                    spawn_grandchild(&mut out);
                }
                if mode == "grandchild-leak-crash" {
                    // Crash hard after the grandchild exists but before any
                    // prompt result, exercising the crash teardown branch.
                    std::process::exit(19);
                }
                // ACP `NewSessionResponse.configOptions` advertises the model
                // selector. The pinned production route encodes its
                // `currentValue` as `JSON.stringify([provider, model])`. The
                // route-drift mode advertises a different route so the client's
                // pre-publication verification must fail closed.
                let current_value = if mode == "route-drift" {
                    "[\\\"deepseek-official\\\",\\\"deepseek-v4-pro\\\"]"
                } else {
                    "[\\\"minimax\\\",\\\"MiniMax-M3\\\"]"
                };
                let config_options = format!(
                    "[{{\"id\":\"model\",\"name\":\"Model\",\"category\":\"model\",\"type\":\"select\",\"currentValue\":\"{current_value}\",\"options\":[]}}]"
                );
                let frame = format!(
                    "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{{\"sessionId\":\"{session_id}\",\"configOptions\":{config_options}}}}}",
                    id.unwrap_or(0)
                );
                send(&mut out, &frame);
            }
            Some("session/prompt") => {
                handle_prompt(&mode, id.unwrap_or(0), &session_id, &mut out, &mut lines);
            }
            Some("session/cancel") => {
                // For the hang mode a cancel resolves nothing here; the client's
                // own deadline/cancel path is authoritative. Acknowledge silently.
            }
            _ => {}
        }
    }
}

fn handle_prompt(
    mode: &str,
    prompt_id: i64,
    session_id: &str,
    out: &mut std::io::Stdout,
    lines: &mut std::io::Lines<std::io::StdinLock<'_>>,
) {
    match mode {
        "malformed" => {
            // A session/update missing the required nested shape.
            let frame = format!(
                "{{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{{\"sessionId\":\"{session_id}\",\"update\":{{\"sessionUpdate\":123}}}}}}"
            );
            send(out, &frame);
        }
        "oversized" => {
            let huge = "x".repeat(200 * 1024);
            let frame = format!(
                "{{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{{\"sessionId\":\"{session_id}\",\"update\":{{\"sessionUpdate\":\"agent_message_chunk\",\"content\":{{\"type\":\"text\",\"text\":\"{huge}\"}}}}}}}}"
            );
            send(out, &frame);
        }
        "hang" | "grandchild-leak" => {
            // Never resolve; wait until stdin closes.
            for line in lines.by_ref() {
                if line.is_err() {
                    break;
                }
            }
        }
        "tool-lifecycle" => {
            // A well-formed tool lifecycle: tool_call (registers correlation
            // `t1`), then a tool_call_update completing it, then assistant text
            // and a clean end_turn. The provider maps these to bounded neutral
            // effects and completes.
            let start = format!(
                "{{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{{\"sessionId\":\"{session_id}\",\"update\":{{\"sessionUpdate\":\"tool_call\",\"toolCallId\":\"t1\",\"title\":\"fetch\",\"kind\":\"read\"}}}}}}"
            );
            send(out, &start);
            let update = format!(
                "{{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{{\"sessionId\":\"{session_id}\",\"update\":{{\"sessionUpdate\":\"tool_call_update\",\"toolCallId\":\"t1\",\"status\":\"completed\"}}}}}}"
            );
            send(out, &update);
            message_chunk(out, session_id, "DXBOT_ACP_OK");
            end_turn(out, prompt_id, "end_turn");
        }
        "unattributed-tool" => {
            // A tool_call_update for a correlation that was never opened by a
            // prior tool_call. This is unattributed and must fail protocol; the
            // provider never guesses an association or grants an effect.
            let update = format!(
                "{{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{{\"sessionId\":\"{session_id}\",\"update\":{{\"sessionUpdate\":\"tool_call_update\",\"toolCallId\":\"ghost\",\"status\":\"completed\"}}}}}}"
            );
            send(out, &update);
            // If the provider (incorrectly) tolerated it, it would wait for a
            // terminal; send one so a bug would surface as a false success.
            message_chunk(out, session_id, "DXBOT_ACP_OK");
            end_turn(out, prompt_id, "end_turn");
        }
        "permission" => {
            // Ask for permission before answering.
            let frame = format!(
                "{{\"jsonrpc\":\"2.0\",\"id\":9000,\"method\":\"session/request_permission\",\"params\":{{\"sessionId\":\"{session_id}\",\"toolCall\":{{\"toolCallId\":\"c1\",\"title\":\"side effect\",\"kind\":\"execute\"}},\"options\":[{{\"optionId\":\"yes\",\"name\":\"Allow\",\"kind\":\"allow_once\"}},{{\"optionId\":\"no\",\"name\":\"Reject\",\"kind\":\"reject_once\"}}]}}}}"
            );
            send(out, &frame);
            // Read the client's permission response.
            let decision = next_line(lines);
            let selected = decision
                .as_deref()
                .map(|line| line.contains("\"outcome\":\"selected\"") || line.contains("\"outcome\": \"selected\""))
                .unwrap_or(false);
            if selected {
                message_chunk(out, session_id, "DXBOT_ACP_OK");
                end_turn(out, prompt_id, "end_turn");
            } else {
                end_turn(out, prompt_id, "cancelled");
            }
        }
        "echo-secret-stderr" => {
            if let Ok(secret) = env::var("MINIMAX_API_KEY") {
                eprintln!("child saw key len {}", secret.len());
            }
            message_chunk(out, session_id, "DXBOT_ACP_OK");
            end_turn(out, prompt_id, "end_turn");
        }
        "echo-env-names" => {
            let mut names: Vec<String> = env::vars().map(|(name, _)| name).collect();
            names.sort();
            let joined = names
                .iter()
                .map(|name| format!("|{name}|"))
                .collect::<Vec<_>>()
                .join(" ");
            message_chunk(out, session_id, &joined);
            end_turn(out, prompt_id, "end_turn");
        }
        // success and any other mode: one chunk + end_turn.
        _ => {
            message_chunk(out, session_id, "DXBOT_ACP_OK");
            end_turn(out, prompt_id, "end_turn");
        }
    }
}

fn message_chunk(out: &mut std::io::Stdout, session_id: &str, text: &str) {
    let frame = format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{{\"sessionId\":\"{session_id}\",\"update\":{{\"sessionUpdate\":\"agent_message_chunk\",\"content\":{{\"type\":\"text\",\"text\":\"{}\"}}}}}}}}",
        escape(text)
    );
    send(out, &frame);
}

fn end_turn(out: &mut std::io::Stdout, prompt_id: i64, reason: &str) {
    let frame = format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{prompt_id},\"result\":{{\"stopReason\":\"{reason}\"}}}}"
    );
    send(out, &frame);
}

fn send(out: &mut std::io::Stdout, frame: &str) {
    out.write_all(frame.as_bytes()).ok();
    out.write_all(b"\n").ok();
    out.flush().ok();
}

fn next_line(lines: &mut std::io::Lines<std::io::StdinLock<'_>>) -> Option<String> {
    match lines.next() {
        Some(Ok(line)) => Some(line),
        _ => None,
    }
}

/// The provider always invokes the pinned exact argv
/// `--profile acp --patch <patch_path>`. To stay faithful to that contract the
/// test harness encodes the scripted behaviour into the patch filename as
/// `dsh-acp.patch.<mode>.yaml`; this fake agent recovers `<mode>` from the
/// `--patch` value. A patch value without an encoded mode defaults to
/// `success`.
fn parse_mode() -> String {
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--patch" {
            if let Some(patch) = args.next() {
                return mode_from_patch(&patch);
            }
        }
    }
    "success".to_owned()
}

/// Recover `<mode>` from a `dsh-acp.patch.<mode>.yaml` file name.
fn mode_from_patch(patch: &str) -> String {
    let name = patch.rsplit('/').next().unwrap_or(patch);
    let stem = name
        .strip_prefix("dsh-acp.patch.")
        .and_then(|rest| rest.strip_suffix(".yaml"));
    match stem {
        Some(mode) if !mode.is_empty() => mode.to_owned(),
        _ => "success".to_owned(),
    }
}

/// Extract a JSON number field like `"id":N` from a compact line.
fn extract_number(line: &str, key: &str) -> Option<i64> {
    let start = line.find(key)? + key.len();
    let rest = line[start..].trim_start();
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '-')
        .unwrap_or(rest.len());
    rest[..end].parse::<i64>().ok()
}

/// Extract a JSON string field like `"method":"x"` from a compact line.
fn extract_string(line: &str, key: &str) -> Option<String> {
    let start = line.find(key)? + key.len();
    let rest = line[start..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Spawn a long-lived grandchild that outlives this agent, and record its PID
/// to `${DSH_HOME}/grandchild.pid` so the test can assert the whole process
/// group is reaped. The grandchild sleeps far longer than any test bound; only
/// process-group teardown (killpg on the negative pgid) can reap it.
fn spawn_grandchild(_out: &mut std::io::Stdout) {
    use std::process::{Command, Stdio};
    // `sleep 600` is a distinct grandchild in this agent's process group.
    if let Ok(child) = Command::new("sleep")
        .arg("600")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        if let Ok(home) = env::var("DSH_HOME") {
            let path = std::path::Path::new(&home).join("grandchild.pid");
            let _ = std::fs::write(path, child.id().to_string());
        }
    }
}
