//! Binary-level regression tests proving that unwired CLI paths never report
//! success or print projected command payloads as committed results.

#![allow(clippy::unwrap_used)]

use std::process::Command;

fn dxb(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_dxb"))
        .args(args)
        .output()
        .unwrap()
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

#[test]
fn runtime_start_does_not_fabricate_bootstrap_success() {
    let output = dxb(&["runtime", "start"]);
    assert_eq!(output.status.code(), Some(10));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not connected"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("started"));
}

#[test]
fn projected_bot_create_is_not_reported_as_committed() {
    let output = dxb(&["bot", "create", "alpha"]);
    assert_eq!(output.status.code(), Some(10));
    assert!(String::from_utf8_lossy(&output.stderr).contains("no operation was created"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("bot_ref"));
}

#[test]
fn projected_task_directive_is_not_reported_as_applied() {
    let output = dxb(&["task", "cancel", "task-1"]);
    assert_eq!(output.status.code(), Some(10));
    assert!(String::from_utf8_lossy(&output.stderr).contains("no operation was created"));
}

#[test]
fn global_option_values_are_not_misparsed_as_command_groups() {
    let output = dxb(&["--profile", "work", "runtime", "status"]);
    assert_eq!(output.status.code(), Some(10));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("unknown command group: work"));
}

#[test]
fn invalid_global_format_is_usage_error() {
    let output = dxb(&["--format", "yaml", "version"]);
    assert_eq!(output.status.code(), Some(2));
}
