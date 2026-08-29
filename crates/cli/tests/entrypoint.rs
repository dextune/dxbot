//! Binary-level regression tests proving that unwired CLI paths never report
//! success or print projected command payloads as committed results.

#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Per-invocation isolated state root. Binary tests must never touch the
/// caller's real `$HOME/.local/state/dxbot` tree, otherwise a `runtime start`
/// in one test leaks a live Runtime Host into every subsequent invocation and
/// corrupts exit-code assertions across the whole suite.
struct StateDir(PathBuf);

impl StateDir {
    fn new() -> Self {
        // Keep this path short: the Runtime Host binds a Unix-domain socket at
        // `<state>/dxbot/runtime/endpoints/<instance>.sock`, and the full path
        // must stay within the platform `sun_path` limit (108 bytes on Linux).
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("dxb-e{nanos:x}"));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for StateDir {
    fn drop(&mut self) {
        let pid_path = self.0.join("dxbot/runtime/.runtime-host.pid");
        if let Ok(pid) = fs::read_to_string(pid_path) {
            if pid.trim().bytes().all(|byte| byte.is_ascii_digit()) {
                let _ = Command::new("kill").args(["-KILL", pid.trim()]).status();
            }
        }
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn dxb(args: &[&str]) -> std::process::Output {
    let state = StateDir::new();
    Command::new(env!("CARGO_BIN_EXE_dxb"))
        .args(args)
        .env("XDG_STATE_HOME", state.path())
        .env_remove("HOME")
        .output()
        .unwrap()
}

/// Runs a sequence of `dxb` invocations against one shared isolated state root
/// so a real Runtime Host started by an earlier step is observable by later
/// steps. The state directory (and any endpoint sockets under it) is removed on
/// drop.
fn dxb_session(steps: &[&[&str]]) -> Vec<std::process::Output> {
    let state = StateDir::new();
    steps
        .iter()
        .map(|args| {
            Command::new(env!("CARGO_BIN_EXE_dxb"))
                .args(*args)
                .env("XDG_STATE_HOME", state.path())
                .env_remove("HOME")
                .output()
                .unwrap()
        })
        .collect()
}

#[cfg(unix)]
fn dxb_provider_session(steps: &[&[&str]]) -> Vec<std::process::Output> {
    let state = StateDir::new();
    let runtime_root = state.path().join("dxbot/runtime");
    fs::create_dir_all(&runtime_root).unwrap();
    let config_path = runtime_root.join("provider-config.json");
    fs::write(
        &config_path,
        serde_json::json!({
            "schema_version": 1,
            "adapter": "deepseek-flash",
            "provider_id": "deepseek-production",
            "capability": "llm-chat",
            "generation": 7,
            "endpoint": "http://127.0.0.1:9",
            "model": "test-model",
            "credential_ref": "env:DXBOT_TEST_PROVIDER_TOKEN",
            "timeout_ms": 1000
        })
        .to_string(),
    )
    .unwrap();
    fs::set_permissions(&config_path, fs::Permissions::from_mode(0o600)).unwrap();

    steps
        .iter()
        .map(|args| {
            Command::new(env!("CARGO_BIN_EXE_dxb"))
                .args(*args)
                .env("XDG_STATE_HOME", state.path())
                .env("DXBOT_TEST_PROVIDER_TOKEN", "secret-canary-not-for-output")
                .env_remove("HOME")
                .output()
                .unwrap()
        })
        .collect()
}

#[test]
fn help_and_version_remain_local_successes() {
    let help = dxb(&["--help"]);
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("DXBOT command line interface"));

    let version = dxb(&["--version", "--format", "json"]);
    assert!(version.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&version.stdout).expect("offline version JSON");
    assert!(value.get("client_version").is_some());
    assert!(value["remote_compatibility"].is_null());
}

/// A mutating command with no verified local Instance must fail closed with a
/// runtime-unavailable exit (10) and must never print a fabricated committed
/// result. This is the false-success guard from BF-CLI-024/007.
#[test]
fn bot_create_without_runtime_fails_closed_and_does_not_fabricate_commit() {
    let output = dxb(&["bot", "create", "alpha"]);
    assert_eq!(output.status.code(), Some(10));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("committed"));
    assert!(!stdout.contains("bot_ref"));
    assert!(!stdout.contains("operation"));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("RuntimeUnavailable"));
}

/// A directive against a non-connected runtime must not be reported as applied
/// and must fail closed rather than fabricating an operation.
#[test]
fn task_directive_without_runtime_is_not_reported_as_applied() {
    let output = dxb(&["task", "cancel", "task-1"]);
    assert_eq!(output.status.code(), Some(10));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("applied"));
    assert!(!stdout.contains("committed"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("RuntimeUnavailable"));
}

/// Global option values must not be misparsed as command groups. `--profile
/// work` must bind the profile hint, not treat `work` as a group; with no
/// runtime present this still fails closed as runtime-unavailable.
#[test]
fn global_option_values_are_not_misparsed_as_command_groups() {
    let output = dxb(&["--profile", "work", "runtime", "status"]);
    assert_eq!(output.status.code(), Some(10));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("unknown command group: work"));
}

/// An invalid enumerated global option value must fail closed as a usage error
/// (exit 2) even when the following token is also a command group name.
#[test]
fn invalid_global_format_is_usage_error() {
    let output = dxb(&["--format", "yaml", "version"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid --format"));
}

/// End-to-end binary journey (L4/L5 evidence, plan §11 item 2 and item 10):
/// first `runtime start` genuinely bootstraps a local Runtime Host, `runtime
/// status` observes it as running, a mutating `bot create` commits a real
/// operation against it, and `runtime stop --host-stop` tears the host down
/// under an authenticated host-generation fence. The host must not survive the
/// session.
#[test]
fn runtime_start_status_commit_and_host_stop_journey() {
    let outputs = dxb_session(&[
        &["runtime", "start", "--format", "json"],
        &["version", "--format", "json"],
        &["runtime", "status", "--format", "json"],
        &["bot", "create", "alpha"],
        &["runtime", "stop", "--host-stop", "-y", "--format", "json"],
    ]);

    let start = &outputs[0];
    assert_eq!(start.status.code(), Some(0), "runtime start should succeed");
    let start_stdout = String::from_utf8_lossy(&start.stdout);
    assert!(start_stdout.contains("\"status\":\"started\""));
    assert!(start_stdout.contains("\"ready_at\":\"control\""));

    let version = &outputs[1];
    assert_eq!(
        version.status.code(),
        Some(0),
        "live version should succeed"
    );
    let version_value: serde_json::Value =
        serde_json::from_slice(&version.stdout).expect("live version JSON");
    let remote = version_value["remote_compatibility"]
        .as_object()
        .expect("live endpoint adds remote compatibility");
    assert_eq!(remote["compatible"], true);
    assert_eq!(remote["protocol_version"], "1");
    assert_eq!(remote["schema_version"], "v3");

    let status = &outputs[2];
    assert_eq!(
        status.status.code(),
        Some(0),
        "runtime status should succeed"
    );
    assert!(String::from_utf8_lossy(&status.stdout).contains("\"status\":\"running\""));

    let create = &outputs[3];
    assert_eq!(create.status.code(), Some(0), "bot create should commit");
    let create_stdout = String::from_utf8_lossy(&create.stdout);
    for expected in [
        "Instance:",
        "Operation:",
        "Receipt: committed",
        "bot_ref:",
        "main_conversation_ref:",
        "Next action:",
    ] {
        assert!(
            create_stdout.contains(expected),
            "missing {expected}: {create_stdout}"
        );
    }

    let stop = &outputs[4];
    assert_eq!(stop.status.code(), Some(0), "host stop should succeed");
    assert!(String::from_utf8_lossy(&stop.stdout).contains("\"status\":\"stopped\""));
}

#[test]
fn runtime_start_ready_at_control_is_real_and_unobservable_stages_fail_closed() {
    for ready_at in ["process", "storage", "runtime"] {
        let output = dxb(&[
            "runtime",
            "start",
            "--ready-at",
            ready_at,
            "--format",
            "json",
        ]);
        assert_eq!(output.status.code(), Some(11), "ready_at={ready_at}");
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("error JSON");
        assert_eq!(value["code"], "incompatible");
        assert!(
            value["message"]
                .as_str()
                .is_some_and(|message| message.contains("not independently observable"))
        );
    }

    let outputs = dxb_session(&[
        &[
            "runtime",
            "start",
            "--ready-at",
            "control",
            "--format",
            "json",
        ],
        &["runtime", "stop", "--host-stop", "-y", "--format", "json"],
    ]);
    assert_eq!(outputs[0].status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_slice(&outputs[0].stdout).expect("start JSON");
    assert_eq!(value["ready_at"], "control");
    assert_eq!(outputs[1].status.code(), Some(0));
}

#[test]
fn runtime_doctor_reports_live_operational_owner_sections() {
    let outputs = dxb_session(&[
        &["runtime", "start", "--format", "json"],
        &["runtime", "doctor", "--format", "json"],
        &["runtime", "stop", "--host-stop", "-y", "--format", "json"],
    ]);
    assert_eq!(outputs[0].status.code(), Some(0));
    assert_eq!(outputs[1].status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_slice(&outputs[1].stdout).expect("doctor JSON");
    let items = value["items"].as_array().expect("diagnostic items");
    for section in [
        "control", "provider", "storage", "audit", "resource", "recovery",
    ] {
        assert!(
            items.iter().any(|item| item["section"] == section),
            "missing doctor section {section}: {value}"
        );
    }
    for section in ["storage", "audit", "resource", "recovery"] {
        let item = items
            .iter()
            .find(|item| item["section"] == section)
            .expect("section");
        assert_eq!(item["available"], true, "live {section} owner unavailable");
    }
    let resource = items
        .iter()
        .find(|item| item["section"] == "resource")
        .expect("resource");
    assert_eq!(resource["active_permits"], 0);
    assert_eq!(resource["capacity"], 4);
    assert_eq!(resource["scheduler_generation"], 1);
    let audit = items
        .iter()
        .find(|item| item["section"] == "audit")
        .expect("audit");
    assert_eq!(audit["record_count"], 0);
    assert_eq!(outputs[2].status.code(), Some(0));
}

#[cfg(unix)]
#[test]
fn configured_provider_is_shared_by_list_show_and_doctor_without_secret_projection() {
    let outputs = dxb_provider_session(&[
        &["runtime", "start", "--format", "json"],
        &["provider", "list", "--format", "json"],
        &[
            "provider",
            "show",
            "deepseek-production",
            "--format",
            "json",
        ],
        &[
            "runtime",
            "doctor",
            "--section",
            "provider",
            "--format",
            "json",
        ],
        &["runtime", "stop", "--host-stop", "-y", "--format", "json"],
    ]);
    for (index, output) in outputs.iter().enumerate() {
        assert_eq!(
            output.status.code(),
            Some(0),
            "step {index}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(!stdout.contains("secret-canary-not-for-output"));
    }
    for output in &outputs[1..=3] {
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("deepseek-production"), "{stdout}");
        assert!(stdout.contains("\"generation\":7"), "{stdout}");
        assert!(stdout.contains("ready"), "{stdout}");
    }
}

#[cfg(unix)]
#[test]
fn configured_real_provider_executes_task_and_commits_result() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let mock = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let mut request = [0_u8; 8192];
                    let read = stream.read(&mut request).unwrap();
                    let request = String::from_utf8_lossy(&request[..read]);
                    assert!(request.starts_with("POST /v1/chat/completions"));
                    assert!(request.contains("authorization: Bearer secret-canary"));
                    assert!(request.contains("known prompt"));
                    let body = serde_json::json!({
                        "choices": [{"message": {
                            "content": "mock-real-provider-result",
                            "reasoning_content": null
                        }}],
                        "usage": {
                            "prompt_tokens": 2,
                            "completion_tokens": 3,
                            "reasoning_tokens": 0
                        }
                    })
                    .to_string();
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                    .unwrap();
                    stream.flush().unwrap();
                    return;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "provider call timed out"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(error) => panic!("mock provider accept failed: {error}"),
            }
        }
    });

    let state = StateDir::new();
    let runtime_root = state.path().join("dxbot/runtime");
    fs::create_dir_all(&runtime_root).unwrap();
    let config_path = runtime_root.join("provider-config.json");
    fs::write(
        &config_path,
        serde_json::json!({
            "schema_version": 1,
            "adapter": "deepseek-flash",
            "provider_id": "deepseek-production",
            "capability": "llm-chat",
            "generation": 7,
            "endpoint": endpoint,
            "model": "test-model",
            "credential_ref": "env:DXBOT_TEST_PROVIDER_TOKEN",
            "timeout_ms": 3000
        })
        .to_string(),
    )
    .unwrap();
    fs::set_permissions(&config_path, fs::Permissions::from_mode(0o600)).unwrap();
    let invoke = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_dxb"))
            .args(args)
            .env("XDG_STATE_HOME", state.path())
            .env("DXBOT_TEST_PROVIDER_TOKEN", "secret-canary")
            .env_remove("HOME")
            .output()
            .unwrap()
    };

    let start = invoke(&["runtime", "start", "--format", "json"]);
    assert_eq!(
        start.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&start.stderr)
    );
    let create = invoke(&["bot", "create", "alpha", "--format", "json"]);
    assert_eq!(
        create.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&create.stderr)
    );
    let activate = invoke(&["bot", "activate", "alpha", "--format", "json"]);
    assert_eq!(
        activate.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&activate.stderr)
    );
    let submit = invoke(&[
        "task",
        "submit",
        "alpha",
        "--text",
        "known prompt",
        "--format",
        "json",
    ]);
    assert_eq!(
        submit.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&submit.stderr)
    );
    let submitted: serde_json::Value = serde_json::from_slice(&submit.stdout).unwrap();
    let committed = submitted["committed_payload"].as_object().unwrap();
    assert_eq!(committed["state"], "admitted");
    assert!(committed["execution_ref"].as_str().is_some());
    assert!(committed["context_plan_ref"].as_str().is_some());
    let task_ref = committed["task_ref"].as_str().unwrap();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let result = loop {
        let result = invoke(&["task", "result", task_ref, "--format", "json"]);
        assert_eq!(
            result.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        if value["state"] == "succeeded" {
            break value;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "task did not complete: {value}"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    };
    assert_eq!(result["result"]["output"], "mock-real-provider-result");
    assert!(result["result"]["execution_ref"].as_str().is_some());
    assert_eq!(
        result["result"]["evidence"][0]["provider_id"],
        "deepseek-production"
    );
    let audit_path = runtime_root.join("audit-outbox.json");
    let audit_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let audit = loop {
        if let Ok(bytes) = fs::read(&audit_path) {
            let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            let actions = value["records"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|record| record["intent"]["action"].as_str())
                .collect::<std::collections::BTreeSet<_>>();
            if actions.contains("execution-running")
                && actions.contains("provider-dispatched")
                && actions.contains("execution-succeeded")
            {
                break value;
            }
        }
        assert!(
            std::time::Instant::now() < audit_deadline,
            "required audit intents were not projected"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    };
    let audit_text = audit.to_string();
    assert!(!audit_text.contains("secret-canary"));
    assert!(!audit_text.contains("known prompt"));
    let original_records = audit["records"].as_array().unwrap();
    let original_keys = original_records
        .iter()
        .map(|record| record["intent"]["key"].as_str().unwrap().to_owned())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(original_keys.len(), original_records.len());

    mock.join().unwrap();
    let stop = invoke(&["runtime", "stop", "--host-stop", "-y", "--format", "json"]);
    assert_eq!(
        stop.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&stop.stderr)
    );

    // The sequenced outbox is reconstructable from producer-owned canonical
    // intents. Losing only the projection must rebuild the same keys exactly
    // once on startup, without redispatching the completed Provider call.
    fs::remove_file(&audit_path).unwrap();
    let restart = invoke(&["runtime", "start", "--format", "json"]);
    assert_eq!(
        restart.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&restart.stderr)
    );
    let rebuilt: serde_json::Value =
        serde_json::from_slice(&fs::read(&audit_path).expect("rebuilt audit outbox")).unwrap();
    let rebuilt_records = rebuilt["records"].as_array().unwrap();
    let rebuilt_keys = rebuilt_records
        .iter()
        .map(|record| record["intent"]["key"].as_str().unwrap().to_owned())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(rebuilt_keys, original_keys);
    assert_eq!(rebuilt_keys.len(), rebuilt_records.len());

    let final_stop = invoke(&["runtime", "stop", "--host-stop", "-y", "--format", "json"]);
    assert_eq!(
        final_stop.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&final_stop.stderr)
    );
}

#[cfg(unix)]
#[test]
fn slow_provider_call_is_cancelled_by_durable_task_control_without_shutdown_leak() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let mock = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("provider connection");
        let mut request = [0_u8; 8192];
        let read = stream.read(&mut request).expect("provider request");
        let request = String::from_utf8_lossy(&request[..read]);
        assert!(request.starts_with("POST /v1/chat/completions"));
        assert!(request.contains("slow cancellable prompt"));
        started_tx.send(()).expect("call started signal");
        stream
            .set_read_timeout(Some(std::time::Duration::from_millis(200)))
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let mut byte = [0_u8; 1];
            match stream.read(&mut byte) {
                Ok(0) => return,
                Ok(_) => {}
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) => {}
                Err(_) => return,
            }
            assert!(
                std::time::Instant::now() < deadline,
                "cancelled transport did not close the provider socket"
            );
        }
    });

    let state = StateDir::new();
    let runtime_root = state.path().join("dxbot/runtime");
    fs::create_dir_all(&runtime_root).unwrap();
    let config_path = runtime_root.join("provider-config.json");
    fs::write(
        &config_path,
        serde_json::json!({
            "schema_version": 1,
            "adapter": "deepseek-flash",
            "provider_id": "deepseek-production",
            "capability": "llm-chat",
            "generation": 7,
            "endpoint": endpoint,
            "model": "test-model",
            "credential_ref": "env:DXBOT_TEST_PROVIDER_TOKEN",
            "timeout_ms": 10000
        })
        .to_string(),
    )
    .unwrap();
    fs::set_permissions(&config_path, fs::Permissions::from_mode(0o600)).unwrap();
    let invoke = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_dxb"))
            .args(args)
            .env("XDG_STATE_HOME", state.path())
            .env("DXBOT_TEST_PROVIDER_TOKEN", "secret-cancel-canary")
            .env_remove("HOME")
            .output()
            .unwrap()
    };

    assert!(
        invoke(&["runtime", "start", "--format", "json"])
            .status
            .success()
    );
    assert!(
        invoke(&["bot", "create", "alpha", "--format", "json"])
            .status
            .success()
    );
    assert!(
        invoke(&["bot", "activate", "alpha", "--format", "json"])
            .status
            .success()
    );
    let submit = invoke(&[
        "task",
        "submit",
        "alpha",
        "--text",
        "slow cancellable prompt",
        "--format",
        "json",
    ]);
    assert_eq!(
        submit.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&submit.stderr)
    );
    let submitted: serde_json::Value = serde_json::from_slice(&submit.stdout).unwrap();
    let task_ref = submitted["committed_payload"]["task_ref"]
        .as_str()
        .expect("task ref")
        .to_owned();
    started_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("provider call starts");

    let cancel = invoke(&[
        "task",
        "cancel",
        &task_ref,
        "--if-execution-generation",
        "1",
        "--reason",
        "operator abort",
        "-y",
        "--format",
        "json",
    ]);
    assert_eq!(
        cancel.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&cancel.stderr)
    );

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let result = invoke(&["task", "result", &task_ref, "--format", "json"]);
        assert_eq!(
            result.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        if value["state"] == "cancelled" {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Task did not cancel: {value}"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    mock.join().expect("provider socket closed by cancellation");
    let stop = invoke(&["runtime", "stop", "--host-stop", "-y", "--format", "json"]);
    assert_eq!(
        stop.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&stop.stderr)
    );
}

#[test]
fn provider_unavailable_human_error_has_safe_next_action() {
    let outputs = dxb_session(&[
        &["runtime", "start", "--format", "json"],
        &["bot", "create", "alpha"],
        &["bot", "activate", "alpha"],
        &["runtime", "stop", "--host-stop", "-y", "--format", "json"],
    ]);
    assert_eq!(outputs[2].status.code(), Some(16));
    let error = String::from_utf8_lossy(&outputs[2].stderr);
    assert!(error.contains("ProviderUnavailable"));
    assert!(error.contains("next action: runtime-doctor"));
    assert!(error.contains("\"section\":\"provider\""));
    assert_eq!(outputs[3].status.code(), Some(0));
}

#[test]
fn unknown_path_suggestion_is_bounded_and_never_executes() {
    let close = dxb(&["taks", "submit"]);
    assert_eq!(close.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&close.stderr).contains("did you mean 'task submit'?"));

    let far = dxb(&["completely-unrelated"]);
    assert_eq!(far.status.code(), Some(3));
    assert!(!String::from_utf8_lossy(&far.stderr).contains("did you mean"));
}

#[cfg(unix)]
fn run_built_provider_failure(
    status: Option<&str>,
    body: String,
    expired_deadline: bool,
    expected_state: &str,
    expected_kind: &str,
) {
    let (endpoint, mock) = if let Some(status) = status {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let status = status.to_owned();
        let mock = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("provider request");
            let mut request = [0_u8; 8192];
            let read = stream.read(&mut request).unwrap();
            assert!(String::from_utf8_lossy(&request[..read]).contains("failure-matrix-prompt"));
            write!(
                stream,
                "HTTP/1.1 {status}\r\ncontent-type: application/json\r\nretry-after: 3\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
            stream.flush().unwrap();
        });
        (endpoint, Some(mock))
    } else {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);
        (endpoint, None)
    };

    let state = StateDir::new();
    let runtime_root = state.path().join("dxbot/runtime");
    fs::create_dir_all(&runtime_root).unwrap();
    let config_path = runtime_root.join("provider-config.json");
    fs::write(
        &config_path,
        serde_json::json!({
            "schema_version": 1,
            "adapter": "deepseek-flash",
            "provider_id": "deepseek-production",
            "capability": "llm-chat",
            "generation": 7,
            "endpoint": endpoint,
            "model": "test-model",
            "credential_ref": "env:DXBOT_TEST_PROVIDER_TOKEN",
            "timeout_ms": 1000
        })
        .to_string(),
    )
    .unwrap();
    fs::set_permissions(&config_path, fs::Permissions::from_mode(0o600)).unwrap();
    let invoke = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_dxb"))
            .args(args)
            .env("XDG_STATE_HOME", state.path())
            .env("DXBOT_TEST_PROVIDER_TOKEN", "secret-failure-matrix-canary")
            .env_remove("HOME")
            .output()
            .unwrap()
    };

    assert!(
        invoke(&["runtime", "start", "--format", "json"])
            .status
            .success()
    );
    assert!(
        invoke(&["bot", "create", "alpha", "--format", "json"])
            .status
            .success()
    );
    assert!(
        invoke(&["bot", "activate", "alpha", "--format", "json"])
            .status
            .success()
    );
    let submit = if expired_deadline {
        invoke(&[
            "task",
            "submit",
            "alpha",
            "--text",
            "failure-matrix-prompt",
            "--deadline",
            "1",
            "--format",
            "json",
        ])
    } else {
        invoke(&[
            "task",
            "submit",
            "alpha",
            "--text",
            "failure-matrix-prompt",
            "--format",
            "json",
        ])
    };
    assert_eq!(
        submit.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&submit.stderr)
    );
    let submitted: serde_json::Value = serde_json::from_slice(&submit.stdout).unwrap();
    let task_ref = submitted["committed_payload"]["task_ref"].as_str().unwrap();
    let process_ref = submitted["committed_payload"]["process_ref"]
        .as_str()
        .unwrap();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let result = invoke(&["task", "result", task_ref, "--format", "json"]);
        assert_eq!(result.status.code(), Some(0));
        let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        if value["state"] == expected_state {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "provider failure did not terminate as {expected_state}: {value}"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    if let Some(mock) = mock {
        mock.join().unwrap();
    }

    let process = invoke(&["process", "show", process_ref, "--format", "json"]);
    assert_eq!(process.status.code(), Some(0));
    let process: serde_json::Value = serde_json::from_slice(&process.stdout).unwrap();
    let reason = process["terminal_reason"]
        .as_str()
        .expect("typed terminal reason");
    assert!(
        reason.contains(&format!("kind={expected_kind}")),
        "{reason}"
    );
    assert!(!reason.contains("secret-failure-matrix-canary"));
    assert!(!reason.contains("secret-upstream-detail"));

    let doctor = invoke(&["runtime", "doctor", "--format", "json"]);
    assert_eq!(doctor.status.code(), Some(0));
    let doctor: serde_json::Value = serde_json::from_slice(&doctor.stdout).unwrap();
    let resource = doctor["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["section"] == "resource")
        .unwrap();
    assert_eq!(resource["active_permits"], 0, "Core Lease leaked: {doctor}");

    let stored = fs::read_to_string(runtime_root.join("application-state.json")).unwrap();
    assert!(!stored.contains("secret-failure-matrix-canary"));
    assert!(!stored.contains("secret-upstream-detail"));
    let stop = invoke(&["runtime", "stop", "--host-stop", "-y", "--format", "json"]);
    assert_eq!(stop.status.code(), Some(0));
}

#[cfg(unix)]
#[test]
fn built_provider_failure_matrix_is_typed_secret_safe_and_leak_free() {
    let ordinary = "secret-upstream-detail".to_owned();
    for (status, expected_kind) in [
        ("401 Unauthorized", "invalid-request"),
        ("429 Too Many Requests", "rate-limited"),
        ("503 Service Unavailable", "upstream-unavailable"),
    ] {
        run_built_provider_failure(
            Some(status),
            ordinary.clone(),
            false,
            "failed",
            expected_kind,
        );
    }
    run_built_provider_failure(
        Some("200 OK"),
        "not-json-secret-upstream-detail".to_owned(),
        false,
        "failed",
        "protocol-violation",
    );
    let oversized = serde_json::json!({
        "choices": [{"message": {
            "content": "x".repeat(1024 * 1024 + 1),
            "reasoning_content": null
        }}],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "reasoning_tokens": 0}
    })
    .to_string();
    run_built_provider_failure(
        Some("200 OK"),
        oversized,
        false,
        "failed",
        "output-exceeded",
    );
    run_built_provider_failure(
        None,
        String::new(),
        false,
        "recoveryrequired",
        "transport-unavailable",
    );
    run_built_provider_failure(
        None,
        String::new(),
        true,
        "recoveryrequired",
        "deadline-exceeded",
    );
}

#[cfg(unix)]
#[test]
fn killed_runtime_recovers_unknown_effect_and_explicitly_resumes_with_fresh_attempt() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (first_started_tx, first_started_rx) = std::sync::mpsc::channel();
    let (first_closed_tx, first_closed_rx) = std::sync::mpsc::channel();
    let mock = std::thread::spawn(move || {
        let (mut first, _) = listener.accept().expect("first provider connection");
        let mut request = [0_u8; 8192];
        let read = first.read(&mut request).expect("first provider request");
        assert!(String::from_utf8_lossy(&request[..read]).contains("restart recovery prompt"));
        first_started_tx.send(()).expect("first started");
        first
            .set_read_timeout(Some(std::time::Duration::from_millis(200)))
            .unwrap();
        let close_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let mut byte = [0_u8; 1];
            match first.read(&mut byte) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
            assert!(
                std::time::Instant::now() < close_deadline,
                "killed host left provider socket open"
            );
        }
        first_closed_tx.send(()).expect("first closed");

        let (mut second, _) = listener.accept().expect("second provider connection");
        let read = second.read(&mut request).expect("second provider request");
        assert!(String::from_utf8_lossy(&request[..read]).contains("restart recovery prompt"));
        let body = serde_json::json!({
            "choices": [{"message": {
                "content": "recovered-on-fresh-attempt",
                "reasoning_content": null
            }}],
            "usage": {"prompt_tokens": 2, "completion_tokens": 4, "reasoning_tokens": 0}
        })
        .to_string();
        write!(
            second,
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
        second.flush().unwrap();
    });

    let state = StateDir::new();
    let runtime_root = state.path().join("dxbot/runtime");
    fs::create_dir_all(&runtime_root).unwrap();
    let config_path = runtime_root.join("provider-config.json");
    fs::write(
        &config_path,
        serde_json::json!({
            "schema_version": 1,
            "adapter": "deepseek-flash",
            "provider_id": "deepseek-production",
            "capability": "llm-chat",
            "generation": 7,
            "endpoint": endpoint,
            "model": "test-model",
            "credential_ref": "env:DXBOT_TEST_PROVIDER_TOKEN",
            "timeout_ms": 10000
        })
        .to_string(),
    )
    .unwrap();
    fs::set_permissions(&config_path, fs::Permissions::from_mode(0o600)).unwrap();
    let invoke = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_dxb"))
            .args(args)
            .env("XDG_STATE_HOME", state.path())
            .env("DXBOT_TEST_PROVIDER_TOKEN", "secret-restart-canary")
            .env_remove("HOME")
            .output()
            .unwrap()
    };

    assert!(
        invoke(&["runtime", "start", "--format", "json"])
            .status
            .success()
    );
    assert!(
        invoke(&["bot", "create", "alpha", "--format", "json"])
            .status
            .success()
    );
    assert!(
        invoke(&["bot", "activate", "alpha", "--format", "json"])
            .status
            .success()
    );
    let submit = invoke(&[
        "task",
        "submit",
        "alpha",
        "--text",
        "restart recovery prompt",
        "--format",
        "json",
    ]);
    assert_eq!(
        submit.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&submit.stderr)
    );
    let submitted: serde_json::Value = serde_json::from_slice(&submit.stdout).unwrap();
    let task_ref = submitted["committed_payload"]["task_ref"]
        .as_str()
        .expect("task ref")
        .to_owned();
    let process_ref = submitted["committed_payload"]["process_ref"]
        .as_str()
        .expect("process ref")
        .to_owned();
    first_started_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("first provider call");

    let pid = fs::read_to_string(runtime_root.join(".runtime-host.pid"))
        .expect("host pid")
        .trim()
        .to_owned();
    let killed = Command::new("kill")
        .args(["-KILL", pid.as_str()])
        .status()
        .unwrap();
    assert!(killed.success());
    first_closed_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("provider socket closed after kill");

    let restart = invoke(&["runtime", "start", "--format", "json"]);
    assert_eq!(
        restart.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&restart.stderr)
    );
    let restarted: serde_json::Value = serde_json::from_slice(&restart.stdout).unwrap();
    assert_eq!(restarted["host_generation"], 2);
    let uncertain = invoke(&["task", "result", &task_ref, "--format", "json"]);
    assert_eq!(uncertain.status.code(), Some(0));
    let uncertain: serde_json::Value = serde_json::from_slice(&uncertain.stdout).unwrap();
    assert_eq!(uncertain["state"], "recoveryrequired");
    let process = invoke(&["process", "show", &process_ref, "--format", "json"]);
    let process: serde_json::Value = serde_json::from_slice(&process.stdout).unwrap();
    assert_eq!(process["state"], "recoveryrequired");
    let persisted: serde_json::Value = serde_json::from_slice(
        &fs::read(runtime_root.join("application-state.json")).expect("application snapshot"),
    )
    .unwrap();
    assert!(
        persisted["side_effects"]
            .as_array()
            .is_some_and(|effects| effects.iter().any(|effect| effect["status"] == "unknown"))
    );

    let resume = invoke(&["task", "resume", &task_ref, "--format", "json"]);
    assert_eq!(
        resume.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&resume.stdout),
        String::from_utf8_lossy(&resume.stderr)
    );
    let resumed: serde_json::Value = serde_json::from_slice(&resume.stdout).unwrap();
    assert_eq!(resumed["committed_payload"]["state"], "admitted");
    assert_eq!(resumed["committed_payload"]["execution_generation"], 2);

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let result = invoke(&["task", "result", &task_ref, "--format", "json"]);
        let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        if value["state"] == "succeeded" {
            assert_eq!(value["result"]["output"], "recovered-on-fresh-attempt");
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "fresh attempt did not finish: {value}"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    mock.join().expect("two provider attempts observed");
    let final_process = invoke(&["process", "show", &process_ref, "--format", "json"]);
    let final_process: serde_json::Value = serde_json::from_slice(&final_process.stdout).unwrap();
    assert_eq!(final_process["state"], "completed");
    let audit_path = runtime_root.join("audit-outbox.json");
    let audit_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let audit: serde_json::Value =
            serde_json::from_slice(&fs::read(&audit_path).expect("recovery audit outbox")).unwrap();
        let records = audit["records"].as_array().expect("audit records");
        let keys = records
            .iter()
            .map(|record| record["intent"]["key"].as_str().unwrap())
            .collect::<std::collections::BTreeSet<_>>();
        let has_recovery = records
            .iter()
            .any(|record| record["intent"]["action"] == "execution-recovery-required");
        let has_success = records
            .iter()
            .any(|record| record["intent"]["action"] == "execution-succeeded");
        if has_recovery && has_success {
            assert_eq!(
                keys.len(),
                records.len(),
                "recovery audit replay duplicated keys"
            );
            assert!(!audit.to_string().contains("secret-restart-canary"));
            assert!(!audit.to_string().contains("restart recovery prompt"));
            break;
        }
        assert!(
            std::time::Instant::now() < audit_deadline,
            "RecoveryRequired and success audit intents were not projected: {audit}"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let stop = invoke(&["runtime", "stop", "--host-stop", "-y", "--format", "json"]);
    assert_eq!(stop.status.code(), Some(0));
}

// ─────────────────────────────────────────────────────────────────────────
// Operational end-to-end built-binary journeys (DXB-DEL-067 §7).
//
// OJ-004 (Memory continuity), OJ-005 (Multi-Bot durable collaboration), and
// OJ-007 (repeated supervised restart / resource ceiling). The canonical
// behaviours are also covered as library UoW tests in
// `crates/application/tests/execution.rs`; the tests below promote them to the
// full built `dxb` subprocess chain required by §15.1 items 3-4. Where the
// frozen 63-command CLI surface has no read projection for an internal
// artifact (e.g. the immutable Context Plan / memory refs), the assertions read
// the canonical JSON artifacts written by the built Runtime Host subprocess
// (`application-state.json` under the runtime root), never a second command
// table. No public CLI command is added by these tests.
// ─────────────────────────────────────────────────────────────────────────

/// Isolated session with an owner-only provider configuration written to the
/// runtime root before any `dxb` invocation. Admission and Context Plan commit
/// happen at `task submit` time and do not require the provider endpoint to be
/// reachable, so a dead loopback endpoint is sufficient to exercise
/// `bot activate`/`task submit` for the memory- and delegation-continuity
/// journeys while keeping every step hermetic and credential-free.
#[cfg(unix)]
struct BuiltSession {
    state: StateDir,
    runtime_root: PathBuf,
}

#[cfg(unix)]
impl BuiltSession {
    fn with_mock_provider(
        outputs: Vec<&'static str>,
    ) -> (Self, std::thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let mock = std::thread::spawn(move || {
            let mut requests = Vec::with_capacity(outputs.len());
            for output in outputs {
                let (mut stream, _) = listener.accept().expect("built journey provider request");
                let mut request = [0_u8; 64 * 1024];
                let read = stream.read(&mut request).expect("provider request bytes");
                requests.push(String::from_utf8_lossy(&request[..read]).into_owned());
                let body = serde_json::json!({
                    "choices": [{"message": {
                        "content": output,
                        "reasoning_content": null
                    }}],
                    "usage": {
                        "prompt_tokens": 2,
                        "completion_tokens": 3,
                        "reasoning_tokens": 0
                    }
                })
                .to_string();
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
                stream.flush().unwrap();
            }
            requests
        });
        (Self::with_endpoint(endpoint), mock)
    }

    fn with_endpoint(endpoint: String) -> Self {
        let state = StateDir::new();
        let runtime_root = state.path().join("dxbot/runtime");
        fs::create_dir_all(&runtime_root).unwrap();
        let config_path = runtime_root.join("provider-config.json");
        fs::write(
            &config_path,
            serde_json::json!({
                "schema_version": 1,
                "adapter": "deepseek-flash",
                "provider_id": "deepseek-production",
                "capability": "llm-chat",
                "generation": 7,
                "endpoint": endpoint,
                "model": "test-model",
                "credential_ref": "env:DXBOT_TEST_PROVIDER_TOKEN",
                "timeout_ms": 3000
            })
            .to_string(),
        )
        .unwrap();
        fs::set_permissions(&config_path, fs::Permissions::from_mode(0o600)).unwrap();
        Self {
            state,
            runtime_root,
        }
    }

    fn invoke(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_dxb"))
            .args(args)
            .env("XDG_STATE_HOME", self.state.path())
            .env("DXBOT_TEST_PROVIDER_TOKEN", "secret-oj-canary")
            .env_remove("HOME")
            .output()
            .unwrap()
    }

    fn invoke_ok(&self, args: &[&str]) -> serde_json::Value {
        let output = self.invoke(args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "`dxb {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.contains("secret-oj-canary"),
            "credential leaked into stdout of `dxb {}`",
            args.join(" ")
        );
        serde_json::from_slice(&output.stdout).unwrap_or(serde_json::Value::Null)
    }

    fn application_state(&self) -> serde_json::Value {
        let bytes = fs::read(self.runtime_root.join("application-state.json"))
            .expect("built subprocess wrote application-state.json");
        serde_json::from_slice(&bytes).expect("canonical application snapshot is valid JSON")
    }

    fn wait_task_state(&self, task_ref: &str, expected: &str) -> serde_json::Value {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let value = self.invoke_ok(&["task", "result", task_ref, "--format", "json"]);
            if value["state"] == expected {
                return value;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "Task {task_ref} did not reach {expected}: {value}"
            );
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    }

    /// Proposes a typed Memory assertion in `scope` and drives the authorized
    /// promotion: `memory-promote` is a high-risk mutation that fails closed
    /// with `approval-required` (exit 8) and parks a typed Approval, which the
    /// local principal then approves so the assertion becomes Accepted. Returns
    /// the internal (single-prefixed) memory id recorded in canonical state.
    fn propose_and_authorized_promote(&self, scope: &str, statement: &str) -> String {
        let propose = self.invoke_ok(&[
            "memory",
            "propose",
            scope,
            "--text",
            statement,
            "--evidence",
            "observation:a",
            "--evidence",
            "observation:b",
            "--format",
            "json",
        ]);
        assert_eq!(propose["committed_payload"]["state"], "proposed");
        let proposal_ref = propose["committed_payload"]["memory_ref"]
            .as_str()
            .expect("memory proposal ref")
            .to_owned();

        // memory-promote parks pending approval and must not commit an accepted
        // assertion on its own.
        let parked = self.invoke(&[
            "memory",
            "promote",
            &proposal_ref,
            "--target-scope",
            scope,
            "--evidence",
            "review:a",
            "--evidence",
            "review:b",
            "--format",
            "json",
        ]);
        assert_eq!(
            parked.status.code(),
            Some(8),
            "memory-promote must fail closed as approval-required: {}",
            String::from_utf8_lossy(&parked.stdout)
        );
        let parked: serde_json::Value = serde_json::from_slice(&parked.stdout).unwrap();
        assert_eq!(parked["code"], "approval-required");
        let approval_ref = parked["next_actions"]
            .as_array()
            .expect("typed next actions")
            .iter()
            .find(|action| action["command_key"] == "approval-approve")
            .and_then(|action| action["args"]["approval"].as_str())
            .expect("parked approval ref")
            .to_owned();

        // Authorized approval promotes the assertion to Accepted.
        let approved = self.invoke_ok(&["approval", "approve", &approval_ref, "--format", "json"]);
        assert_eq!(approved["committed_payload"]["decision"], "approve");

        let accepted: Vec<String> = self.application_state()["memories"]
            .as_array()
            .expect("memories")
            .iter()
            .filter(|memory| memory["status"] == "accepted")
            .map(|memory| memory["id"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(
            accepted.len(),
            1,
            "exactly one accepted assertion after promotion"
        );
        accepted.into_iter().next().unwrap()
    }

    /// Runs a high-risk mutation that fails closed with `approval-required`
    /// (exit 8), then approves the parked Approval named in its typed
    /// `next_actions`. Used for `project-member-set`, which is high-risk.
    fn mutate_high_risk_with_approval(&self, args: &[&str]) {
        let parked = self.invoke(args);
        assert_eq!(
            parked.status.code(),
            Some(8),
            "expected approval-required parking for `dxb {}`: {}",
            args.join(" "),
            String::from_utf8_lossy(&parked.stdout)
        );
        let parked: serde_json::Value = serde_json::from_slice(&parked.stdout).unwrap();
        assert_eq!(parked["code"], "approval-required");
        let approval_ref = parked["next_actions"]
            .as_array()
            .expect("typed next actions")
            .iter()
            .find(|action| action["command_key"] == "approval-approve")
            .and_then(|action| action["args"]["approval"].as_str())
            .expect("parked approval ref")
            .to_owned();
        let approved = self.invoke_ok(&["approval", "approve", &approval_ref, "--format", "json"]);
        assert_eq!(approved["committed_payload"]["decision"], "approve");
    }
}

/// OJ-004 — Memory continuity through the built `dxb` subprocess chain.
///
/// Task A commits a durable Task/Execution and its immutable Context Plan; a
/// typed Memory Proposal is authored and, after an authorized promotion, the
/// accepted assertion survives a genuine Runtime Host stop/restart and is
/// bounded-retrieved into Task B's immutable Context Plan / provider request —
/// without Task A's result being auto-promoted into Memory.
#[cfg(unix)]
#[test]
fn oj004_memory_continuity_survives_restart_into_next_context_plan() {
    let (session, provider) =
        BuiltSession::with_mock_provider(vec!["task-a-result", "task-b-result"]);

    // Bring the Runtime Host up and admit Task A against Bot `alpha`.
    session.invoke_ok(&["runtime", "start", "--format", "json"]);
    session.invoke_ok(&["bot", "create", "alpha", "--format", "json"]);
    session.invoke_ok(&["bot", "activate", "alpha", "--format", "json"]);
    let task_a = session.invoke_ok(&[
        "task", "submit", "alpha", "--text", "task A", "--format", "json",
    ]);
    let task_a_ref = task_a["committed_payload"]["task_ref"]
        .as_str()
        .expect("Task A durable task_ref");
    assert_eq!(task_a["committed_payload"]["state"], "admitted");
    assert!(
        task_a["committed_payload"]["context_plan_ref"]
            .as_str()
            .is_some(),
        "Task A must commit an immutable Context Plan ref"
    );
    let task_a_result = session.wait_task_state(task_a_ref, "succeeded");
    assert_eq!(task_a_result["result"]["output"], "task-a-result");
    assert_eq!(
        task_a_result["result"]["evidence"][0]["provider_id"],
        "deepseek-production"
    );

    // Author a typed Memory Proposal in Bot `alpha` scope, then promote it
    // through the authorized approval path so it becomes an Accepted assertion.
    let memory_id =
        session.propose_and_authorized_promote("bot:alpha", "remembered across restart");

    // Capture the canonical accepted assertion before restart.
    let before = session.application_state();
    let accepted_before: Vec<&serde_json::Value> = before["memories"]
        .as_array()
        .expect("memories array")
        .iter()
        .filter(|memory| memory["status"] == "accepted")
        .collect();
    assert_eq!(accepted_before.len(), 1, "exactly one accepted assertion");
    assert_eq!(accepted_before[0]["id"].as_str(), Some(memory_id.as_str()));
    let accepted_revision = accepted_before[0]["revision"]
        .as_i64()
        .expect("accepted memory revision");
    assert!(
        !before.to_string().contains("secret-oj-canary"),
        "no credential in canonical state"
    );

    // Genuine Runtime Host stop and restart.
    session.invoke_ok(&["runtime", "stop", "--host-stop", "-y", "--format", "json"]);
    session.invoke_ok(&["runtime", "start", "--format", "json"]);

    // Task B admitted after restart must bounded-retrieve the SAME accepted
    // revision into its immutable Context Plan / bounded provider request.
    let task_b = session.invoke_ok(&[
        "task",
        "submit",
        "alpha",
        "--text",
        "task B uses prior memory",
        "--format",
        "json",
    ]);
    let task_b_ref = task_b["committed_payload"]["task_ref"]
        .as_str()
        .expect("Task B durable task_ref")
        .to_owned();
    assert_ne!(task_a_ref, task_b_ref, "Task B is a distinct durable Task");
    assert_eq!(task_b["committed_payload"]["state"], "admitted");
    let task_b_result = session.wait_task_state(&task_b_ref, "succeeded");
    assert_eq!(task_b_result["result"]["output"], "task-b-result");

    // The CLI ref is `task:<canonical-id>`; canonical state keys by the
    // canonical id. Resolve Task B by stripping the single CLI-facing prefix.
    let task_b_canonical = task_b_ref
        .strip_prefix("task:")
        .expect("CLI task ref is prefixed")
        .to_owned();
    let after = session.application_state();
    // Locate Task B's execution and its immutable Context Plan.
    let task_b_execution_refs: Vec<String> = after["tasks"]
        .as_array()
        .expect("tasks")
        .iter()
        .find(|task| task["id"].as_str() == Some(task_b_canonical.as_str()))
        .expect("Task B present after restart")["execution_refs"]
        .as_array()
        .expect("execution refs")
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(task_b_execution_refs.len(), 1);
    let task_b_plan = after["executions"]
        .as_array()
        .expect("executions")
        .iter()
        .find(|execution| execution["id"].as_str() == Some(task_b_execution_refs[0].as_str()))
        .expect("Task B execution present")["context_plan"]
        .clone();
    let plan_memory_refs = task_b_plan["memory_refs"].as_array().expect("memory refs");
    assert_eq!(
        plan_memory_refs.len(),
        1,
        "the accepted assertion is bounded-retrieved into Task B's plan: {task_b_plan}"
    );
    assert_eq!(
        plan_memory_refs[0]["memory_id"].as_str(),
        Some(memory_id.as_str()),
        "same accepted memory id enters the next Context Plan"
    );
    assert_eq!(
        plan_memory_refs[0]["revision"].as_i64(),
        Some(accepted_revision),
        "same accepted revision enters the next Context Plan"
    );
    assert!(
        task_b_plan["bounded_context"]
            .as_str()
            .expect("bounded context")
            .contains("remembered across restart"),
        "accepted statement appears in the bounded provider request"
    );

    // Task Result is never auto-promoted into Memory: still exactly one
    // assertion after two admitted Tasks and a restart.
    let accepted_after = after["memories"]
        .as_array()
        .expect("memories")
        .iter()
        .filter(|memory| memory["status"] == "accepted")
        .count();
    assert_eq!(
        accepted_after, 1,
        "no auto-promotion of Task results into Memory across restart"
    );
    let requests = provider.join().expect("two successful Provider calls");
    assert_eq!(requests.len(), 2);
    assert!(requests[0].contains("task A"));
    assert!(requests[1].contains("task B uses prior memory"));
    assert!(requests[1].contains("remembered across restart"));

    session.invoke_ok(&["runtime", "stop", "--host-stop", "-y", "--format", "json"]);
}

/// OJ-005 — Multi-Bot durable collaboration through the built `dxb` chain.
///
/// A Project with two additional members is established through `dxb`; the
/// owner delegates durable Tasks to each recipient with an authorized sender.
/// Each recipient owns its own durable Task/Execution/Core-Lease-bearing
/// Context Plan, joined by typed refs. Duplicate (exact-retry) delegation does
/// not create a duplicate recipient Task, and the collaboration graph survives
/// a genuine Runtime Host restart unchanged.
#[cfg(unix)]
#[test]
fn oj005_multi_bot_durable_delegation_survives_restart() {
    let (session, provider) =
        BuiltSession::with_mock_provider(vec!["recipient-result-one", "recipient-result-two"]);
    session.invoke_ok(&["runtime", "start", "--format", "json"]);

    for bot in ["alpha", "beta", "gamma"] {
        session.invoke_ok(&["bot", "create", bot, "--format", "json"]);
    }

    // Owner-scoped Project and channel; alpha owns it, beta/gamma join.
    // Project membership is high-risk and parks pending approval; the local
    // principal authorizes each membership so beta/gamma become active members.
    session.invoke_ok(&[
        "project",
        "create",
        "collaboration",
        "bot:alpha",
        "--format",
        "json",
    ]);
    session.invoke_ok(&[
        "channel",
        "create",
        "project:collaboration",
        "coordination",
        "--format",
        "json",
    ]);
    for bot in ["beta", "gamma"] {
        session.mutate_high_risk_with_approval(&[
            "project",
            "member",
            "set",
            "project:collaboration",
            bot,
            "member",
            "--format",
            "json",
        ]);
    }

    // Authorized sender (alpha) delegates a durable Task to each recipient.
    // Recipients must be active members so the delegated Task can be admitted
    // with a provider binding.
    for bot in ["alpha", "beta", "gamma"] {
        session.invoke_ok(&["bot", "activate", bot, "--format", "json"]);
    }

    let delegate_beta = [
        "task",
        "submit",
        "project:collaboration",
        "--text",
        "work for beta",
        "--delegate-to-bot",
        "beta",
        "--requested-sender-bot",
        "alpha",
        "--format",
        "json",
    ];
    let beta_task = session.invoke_ok(&delegate_beta);
    let beta_task_ref = beta_task["committed_payload"]["task_ref"]
        .as_str()
        .expect("beta recipient task_ref")
        .to_owned();
    assert_eq!(beta_task["committed_payload"]["state"], "admitted");

    // Note: exact-retry delegation idempotency is a wire/operation-layer
    // property keyed on the idempotency key. Two independent built `dxb`
    // invocations always derive fresh operation/idempotency identity (see
    // `cli/src/identity.rs`), so genuine duplicate suppression is exercised via
    // the durable journal replay path and the application UoW test
    // `three_bot_delegations_create_recipient_executions_once_and_survive_restart`
    // rather than by re-invoking the binary. This journey asserts the durable
    // recipient-owned graph and its restart continuity below.

    let gamma_task = session.invoke_ok(&[
        "task",
        "submit",
        "project:collaboration",
        "--text",
        "work for gamma",
        "--delegate-to-bot",
        "gamma",
        "--requested-sender-bot",
        "alpha",
        "--format",
        "json",
    ]);
    let gamma_task_ref = gamma_task["committed_payload"]["task_ref"]
        .as_str()
        .expect("gamma recipient task_ref")
        .to_owned();
    assert_ne!(beta_task_ref, gamma_task_ref);
    let beta_result = session.wait_task_state(&beta_task_ref, "succeeded");
    let gamma_result = session.wait_task_state(&gamma_task_ref, "succeeded");
    let outputs = [
        beta_result["result"]["output"].as_str().unwrap(),
        gamma_result["result"]["output"].as_str().unwrap(),
    ]
    .into_iter()
    .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        outputs,
        ["recipient-result-one", "recipient-result-two"]
            .into_iter()
            .collect()
    );
    for result in [&beta_result, &gamma_result] {
        assert_eq!(
            result["result"]["evidence"][0]["provider_id"],
            "deepseek-production"
        );
        assert!(result["result"]["execution_ref"].as_str().is_some());
    }
    let provider_requests = provider.join().expect("recipient Provider calls");
    assert_eq!(provider_requests.len(), 2);
    assert!(
        provider_requests
            .iter()
            .any(|request| request.contains("work for beta"))
    );
    assert!(
        provider_requests
            .iter()
            .any(|request| request.contains("work for gamma"))
    );

    // Verify the recipient-owned durable graph in canonical state before
    // restart: two Tasks (owned by beta/gamma), two Executions with recipient
    // Context Plans, two accepted delegations, two Processes.
    let assert_graph = |state: &serde_json::Value, label: &str| {
        let delegations = state["delegations"].as_array().expect("delegations");
        assert_eq!(delegations.len(), 2, "{label}: two durable delegations");
        for delegation in delegations {
            assert_eq!(delegation["status"], "accepted", "{label}: accepted");
        }
        let tasks = state["tasks"].as_array().expect("tasks");
        // Owners include the two recipients (join by typed ref, not copy).
        let owners: std::collections::BTreeSet<&str> = tasks
            .iter()
            .map(|task| task["owner"].as_str().unwrap())
            .collect();
        assert!(
            owners.contains("bot:beta") && owners.contains("bot:gamma"),
            "{label}: recipient-owned Tasks present: {owners:?}"
        );
        // Each recipient Task owns exactly one Execution whose Context Plan is
        // scoped to the project and bound to the recipient Bot.
        for recipient in ["beta", "gamma"] {
            let recipient_owner = format!("bot:{recipient}");
            let task = tasks
                .iter()
                .find(|task| task["owner"].as_str() == Some(recipient_owner.as_str()))
                .unwrap_or_else(|| panic!("{label}: recipient {recipient} task"));
            let execution_refs = task["execution_refs"].as_array().expect("execution refs");
            assert_eq!(
                execution_refs.len(),
                1,
                "{label}: recipient {recipient} owns one Execution attempt"
            );
            let execution = state["executions"]
                .as_array()
                .expect("executions")
                .iter()
                .find(|execution| execution["id"] == execution_refs[0])
                .unwrap_or_else(|| panic!("{label}: recipient {recipient} execution"));
            let plan = &execution["context_plan"];
            assert_eq!(
                execution["status"], "succeeded",
                "{label}: recipient Execution committed a terminal result"
            );
            assert_eq!(
                plan["scope_ref"].as_str(),
                Some("project:collaboration"),
                "{label}: recipient Execution Context Plan is project-scoped"
            );
            assert_eq!(
                plan["bot_ref"].as_str(),
                Some(recipient_owner.as_str()),
                "{label}: recipient Execution bound to recipient Bot"
            );
        }
        assert_eq!(
            state["processes"]
                .as_array()
                .expect("processes")
                .iter()
                .filter(|process| process["lifecycle"] == "completed")
                .count(),
            2,
            "{label}: each recipient has an independently completed Process"
        );
        assert_eq!(
            state["side_effects"]
                .as_array()
                .expect("side effects")
                .iter()
                .filter(|effect| effect["status"] == "confirmed")
                .count(),
            2,
            "{label}: each Provider result atomically confirms its side effect"
        );
    };

    let before = session.application_state();
    assert_graph(&before, "before restart");

    // Genuine restart; the durable collaboration graph must be identical.
    session.invoke_ok(&["runtime", "stop", "--host-stop", "-y", "--format", "json"]);
    session.invoke_ok(&["runtime", "start", "--format", "json"]);
    let after = session.application_state();
    assert_graph(&after, "after restart");

    // Recipient Tasks and delegations are observable/joinable through frozen
    // read commands after restart (result reference join surface).
    let beta_show = session.invoke_ok(&["task", "show", &beta_task_ref, "--format", "json"]);
    assert!(beta_show["owner"].as_str().is_some());
    let gamma_show = session.invoke_ok(&["task", "show", &gamma_task_ref, "--format", "json"]);
    assert!(gamma_show["owner"].as_str().is_some());

    session.invoke_ok(&["runtime", "stop", "--host-stop", "-y", "--format", "json"]);
}

/// OJ-007 — Repeated supervised restart / resource ceiling through the built
/// `dxb` chain.
///
/// Performs repeated built Runtime Host start/stop cycles and asserts: the
/// Instance identity is stable, HostGeneration is strictly increasing, each
/// graceful `runtime stop --host-stop` unpublishes the endpoint (subsequent
/// `runtime status` fails closed), no Core Lease leaks (`runtime doctor`
/// reports `active_permits == 0`), the canonical state file stays bounded, and
/// a representative Task/Process/Memory continues across every restart.
#[cfg(unix)]
#[test]
fn oj007_repeated_supervised_restart_preserves_identity_and_resource_ceiling() {
    let (session, provider) = BuiltSession::with_mock_provider(vec!["long-running-result"]);

    // First boot and durable representative state: a Bot, an admitted Task
    // (with a Process), and an accepted Memory assertion.
    let start = session.invoke_ok(&["runtime", "start", "--format", "json"]);
    let instance_id = start["instance_id"]
        .as_str()
        .expect("instance id")
        .to_owned();
    let mut last_generation = start["host_generation"].as_i64().expect("host generation");

    session.invoke_ok(&["bot", "create", "alpha", "--format", "json"]);
    session.invoke_ok(&["bot", "activate", "alpha", "--format", "json"]);
    let task = session.invoke_ok(&[
        "task",
        "submit",
        "alpha",
        "--text",
        "long running task",
        "--format",
        "json",
    ]);
    let task_ref = task["committed_payload"]["task_ref"]
        .as_str()
        .expect("task ref")
        .to_owned();
    let process_ref = task["committed_payload"]["process_ref"]
        .as_str()
        .expect("process ref")
        .to_owned();
    let task_result = session.wait_task_state(&task_ref, "succeeded");
    assert_eq!(task_result["result"]["output"], "long-running-result");
    assert_eq!(
        task_result["result"]["evidence"][0]["provider_id"],
        "deepseek-production"
    );
    let requests = provider.join().expect("representative Provider call");
    assert_eq!(requests.len(), 1);
    assert!(requests[0].contains("long running task"));
    session.propose_and_authorized_promote("bot:alpha", "durable knowledge");

    session.invoke_ok(&["runtime", "stop", "--host-stop", "-y", "--format", "json"]);

    let baseline_security =
        fs::read(session.runtime_root.join("security-state.json")).expect("security state");
    assert!(
        String::from_utf8_lossy(&baseline_security).contains("approved"),
        "representative authorized Approval is durable"
    );
    let baseline_audit: serde_json::Value = serde_json::from_slice(
        &fs::read(session.runtime_root.join("audit-outbox.json")).expect("audit outbox"),
    )
    .expect("audit JSON");
    let baseline_audit_keys = baseline_audit["records"]
        .as_array()
        .expect("audit records")
        .iter()
        .map(|record| record["intent"]["key"].as_str().unwrap().to_owned())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        !baseline_audit_keys.is_empty(),
        "required Audit records exist"
    );
    assert_eq!(
        baseline_audit_keys.len(),
        baseline_audit["records"].as_array().unwrap().len(),
        "audit keys are unique before restart"
    );

    let state_path = session.runtime_root.join("application-state.json");
    let baseline_size = fs::metadata(&state_path).expect("state file").len();
    // Generous ceiling: the representative fixture is tiny; a leak/unbounded
    // growth regression would blow well past this bound.
    let size_ceiling = baseline_size.max(4096) * 4;

    // Repeated supervised restart cycles.
    for cycle in 0..3 {
        // Endpoint was unpublished by the previous graceful host stop: status
        // must fail closed rather than observe a stale endpoint.
        let stale = session.invoke(&["runtime", "status", "--format", "json"]);
        assert_ne!(
            stale.status.code(),
            Some(0),
            "cycle {cycle}: graceful host stop must unpublish the endpoint"
        );

        let restart = session.invoke_ok(&["runtime", "start", "--format", "json"]);
        assert_eq!(
            restart["instance_id"].as_str(),
            Some(instance_id.as_str()),
            "cycle {cycle}: Instance identity is stable across restart"
        );
        let generation = restart["host_generation"]
            .as_i64()
            .expect("host generation");
        assert!(
            generation > last_generation,
            "cycle {cycle}: HostGeneration strictly increases ({last_generation} -> {generation})"
        );
        last_generation = generation;

        // Representative Task/Process/Memory continuity through frozen reads.
        let task_show = session.invoke_ok(&["task", "show", &task_ref, "--format", "json"]);
        assert!(
            task_show["owner"].as_str().is_some(),
            "cycle {cycle}: representative Task continues across restart"
        );
        let process_show =
            session.invoke_ok(&["process", "show", &process_ref, "--format", "json"]);
        assert!(
            process_show["state"].as_str().is_some(),
            "cycle {cycle}: representative Process continues across restart"
        );

        // No Core Lease leak: doctor reports zero active permits after a clean
        // restart with no in-flight provider activity.
        let doctor = session.invoke_ok(&["runtime", "doctor", "--format", "json"]);
        let resource = doctor["items"]
            .as_array()
            .expect("doctor items")
            .iter()
            .find(|item| item["section"] == "resource")
            .expect("resource section");
        assert_eq!(
            resource["active_permits"], 0,
            "cycle {cycle}: no Core Lease leak after restart: {doctor}"
        );

        // Representative Memory continuity: the accepted assertion persists.
        let accepted = session.application_state()["memories"]
            .as_array()
            .expect("memories")
            .iter()
            .filter(|memory| memory["status"] == "accepted")
            .count();
        assert_eq!(
            accepted, 1,
            "cycle {cycle}: representative accepted Memory continues across restart"
        );
        assert_eq!(
            fs::read(session.runtime_root.join("security-state.json"))
                .expect("security continuity"),
            baseline_security,
            "cycle {cycle}: Approval/Security state remains byte-stable"
        );
        let audit: serde_json::Value = serde_json::from_slice(
            &fs::read(session.runtime_root.join("audit-outbox.json")).expect("audit continuity"),
        )
        .expect("audit JSON");
        let audit_keys = audit["records"]
            .as_array()
            .expect("audit records")
            .iter()
            .map(|record| record["intent"]["key"].as_str().unwrap().to_owned())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            audit_keys, baseline_audit_keys,
            "cycle {cycle}: Audit continuity"
        );
        assert_eq!(
            audit_keys.len(),
            audit["records"].as_array().unwrap().len(),
            "cycle {cycle}: no duplicate Audit projection"
        );

        // Bounded on-disk canonical state: repeated restarts must not grow the
        // durable snapshot without bound.
        let size = fs::metadata(&state_path).expect("state file").len();
        assert!(
            size <= size_ceiling,
            "cycle {cycle}: canonical state grew unbounded ({size} > {size_ceiling})"
        );

        // Graceful drain/unpublish for this cycle.
        session.invoke_ok(&["runtime", "stop", "--host-stop", "-y", "--format", "json"]);
    }
}
