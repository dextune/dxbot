//! Binary-level regression tests proving that unwired CLI paths never report
//! success or print projected command payloads as committed results.

#![allow(clippy::unwrap_used)]

use std::fs;
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

#[test]
fn help_and_version_remain_local_successes() {
    let help = dxb(&["--help"]);
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("DXBOT command line interface"));

    let version = dxb(&["--version", "--format", "json"]);
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).contains("client_version"));
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
        &["runtime", "status", "--format", "json"],
        &["bot", "create", "alpha"],
        &["runtime", "stop", "--host-stop", "-y", "--format", "json"],
    ]);

    let start = &outputs[0];
    assert_eq!(start.status.code(), Some(0), "runtime start should succeed");
    let start_stdout = String::from_utf8_lossy(&start.stdout);
    assert!(start_stdout.contains("\"status\":\"started\""));

    let status = &outputs[1];
    assert_eq!(
        status.status.code(),
        Some(0),
        "runtime status should succeed"
    );
    assert!(String::from_utf8_lossy(&status.stdout).contains("\"status\":\"running\""));

    let create = &outputs[2];
    assert_eq!(create.status.code(), Some(0), "bot create should commit");
    assert!(String::from_utf8_lossy(&create.stdout).contains("committed"));

    let stop = &outputs[3];
    assert_eq!(stop.status.code(), Some(0), "host stop should succeed");
    assert!(String::from_utf8_lossy(&stop.stdout).contains("\"status\":\"stopped\""));
}
