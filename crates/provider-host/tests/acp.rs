//! Deterministic actual-subprocess ACP conformance for the official DeepSeek
//! Harness ACP provider (`DXB-DEL-068` H6/H7).
//!
//! A tiny fake ACP agent is compiled with `rustc` into a temp directory and
//! spawned as a real child process. It speaks the exact ACP v1 newline-
//! delimited JSON-RPC 2.0 surface the provider drives (`initialize`,
//! `session/new`, `session/prompt`, `session/update`, `session/request_permission`,
//! `session/cancel`). Its behaviour is selected by argv `--mode <name>`, so the
//! provider exercises spawn/handshake/update/result/cancel/hang/crash/malformed/
//! contaminated/oversized/permission paths against a genuine process — never an
//! in-process stub.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(all(unix, feature = "dsh-acp"))]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dxbot_core::types::ProviderId;
use provider_host::{
    AcpConfigInput, AcpLimits, AcpProviderConfig, AllowOnceGrant, CancellationToken,
    DeepSeekHarnessAcpProvider, ExecuteError, ExecuteOutcome, ExecuteProvider, ExecuteRequest,
    PermissionPolicy,
};

const FAKE_SERVER_SOURCE: &str = include_str!("fixtures/fake_acp_server.rs");

struct Workspace(PathBuf);

impl Workspace {
    fn new(tag: &str) -> Self {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("dxb-acp-{tag}-{suffix}"));
        std::fs::create_dir_all(&path).unwrap();
        // The spawn-time ancestor check trusts an owner-only (0700) directory
        // owned by the effective uid as a barrier, which is exactly the
        // mkdtemp-under-sticky-/tmp pattern. Make this root such a barrier so
        // the world-writable /tmp above it is never inspected.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Compile the fake ACP server once per test process into a stable temp path.
fn fake_server_binary() -> PathBuf {
    static COMPILE: Once = Once::new();
    let out_dir = std::env::temp_dir().join("dxb-acp-fake-server");
    let source = out_dir.join("fake_acp_server.rs");
    let binary = out_dir.join("fake_acp_server");
    COMPILE.call_once(|| {
        std::fs::create_dir_all(&out_dir).unwrap();
        // Owner-only barrier so the world-writable /tmp above the compiled fake
        // binary is not treated as a writable ancestor at spawn.
        std::fs::set_permissions(&out_dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::write(&source, FAKE_SERVER_SOURCE).unwrap();
        let status = Command::new("rustc")
            .arg("--edition=2021")
            .arg("-O")
            .arg(&source)
            .arg("-o")
            .arg(&binary)
            .status()
            .expect("rustc must be available to build the fake ACP server");
        assert!(status.success(), "fake ACP server must compile");
        // The spawn-asset check rejects a group/other-writable executable
        // (umask 002 leaves rustc output g+w). Pin it owner-only + executable.
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
    });
    binary
}

fn limits() -> AcpLimits {
    AcpLimits {
        max_line_bytes: 64 * 1024,
        max_output_bytes: 256 * 1024,
        max_update_frames: 1024,
        max_stderr_bytes: 8 * 1024,
        handshake_deadline: Duration::from_secs(10),
        prompt_deadline: Duration::from_secs(10),
        eof_grace: Duration::from_millis(300),
        term_grace: Duration::from_millis(300),
    }
}

fn build_provider(
    command: &Path,
    workspace: &Path,
    mode: &str,
    permission: PermissionPolicy,
    limits: AcpLimits,
) -> DeepSeekHarnessAcpProvider {
    // The provider enforces the exact pinned argv `--profile acp --patch
    // <patch_path>`. The fake agent recovers its scripted behaviour from the
    // patch filename, so encode the mode as `dsh-acp.patch.<mode>.yaml` in the
    // per-test workspace and point the pinned argv at it.
    let patch_path = workspace.join(format!("dsh-acp.patch.{mode}.yaml"));
    std::fs::write(&patch_path, b"# fake acp patch for conformance\n").unwrap();
    // Owner-only so the spawn-asset check (which rejects group/other-writable
    // files under umask 002) accepts it.
    std::fs::set_permissions(&patch_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let patch_str = patch_path.to_str().unwrap().to_owned();
    let args = vec![
        "--profile".to_owned(),
        "acp".to_owned(),
        "--patch".to_owned(),
        patch_str.clone(),
    ];
    // The spawn path re-hashes the on-disk command and patch and constant-time
    // compares them to the configured digests (and the patch to the canonical
    // audited digest). Compute the real digests of the compiled fake binary and
    // the just-written patch so the genuine subprocess still spawns. The patch
    // digest doubles as the "audited" digest for the fake harness fixture.
    let command_sha256 = file_sha256_hex(command);
    let patch_sha256 = file_sha256_hex(&patch_path);
    let config = AcpProviderConfig::new(AcpConfigInput {
        id: ProviderId("deepseek-harness-acp".to_owned()),
        capability: "llm-chat",
        generation: 1,
        command: command.to_str().unwrap(),
        args: &args,
        patch_path: &patch_str,
        dsh_home: workspace.to_str().unwrap(),
        workspace: workspace.to_str().unwrap(),
        path_env: "/usr/bin:/bin",
        permission,
        limits,
        source_commit: provider_host::ACP_SOURCE_COMMIT,
        source_version: provider_host::ACP_SOURCE_VERSION,
        command_sha256: &command_sha256,
        patch_sha256: &patch_sha256,
        audited_patch_sha256: &patch_sha256,
        api_key: "acp-secret-canary-must-not-appear",
    })
    .expect("valid acp config");
    DeepSeekHarnessAcpProvider::new(config)
}

/// Lowercase 64-hex SHA-256 of a file's contents, for spawn-asset digest
/// pinning in the conformance harness.
fn file_sha256_hex(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).expect("read asset for digest");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

fn request() -> ExecuteRequest {
    ExecuteRequest::bounded(
        ProviderId("deepseek-harness-acp".to_owned()),
        1,
        "DXBOT_ACP_PROMPT".to_owned(),
        "bounded-context".to_owned(),
        Some(16),
        None,
    )
}

/// A request carrying a valid request-scoped allow-once authority grant, as the
/// scheduler would mint from an Approved runtime-security Approval.
fn request_with_allow_once_grant() -> ExecuteRequest {
    let mut request = request();
    request.allow_once_grant = Some(AllowOnceGrant {
        approval_ref: "approval-allow-once-1".to_owned(),
        execution_ref: "execution:acp-test".to_owned(),
        policy_generation: 1,
    });
    request
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
}

fn run(mode: &str, permission: PermissionPolicy) -> Result<ExecuteOutcome, ExecuteError> {
    run_with_cancel(mode, permission, CancellationToken::new(), limits())
}

/// Run and return the full `ExecuteResult` so tests can assert the bounded,
/// provider-neutral tool-effect records (`DXB-DEL-068` H10).
fn run_full(
    mode: &str,
    permission: PermissionPolicy,
    request: ExecuteRequest,
) -> Result<provider_host::ExecuteResult, ExecuteError> {
    let binary = fake_server_binary();
    let workspace = Workspace::new(mode);
    let provider = build_provider(&binary, &workspace.0, mode, permission, limits());
    let cancel = CancellationToken::new();
    let runtime = runtime();
    runtime.block_on(async {
        let mut execute = provider.execute(&request, &cancel);
        tokio::time::timeout(Duration::from_secs(20), &mut execute)
            .await
            .expect("execution must not exceed the test bound")
    })
}

fn run_with_request(
    mode: &str,
    permission: PermissionPolicy,
    request: ExecuteRequest,
) -> Result<ExecuteOutcome, ExecuteError> {
    let binary = fake_server_binary();
    let workspace = Workspace::new(mode);
    let provider = build_provider(&binary, &workspace.0, mode, permission, limits());
    let cancel = CancellationToken::new();
    let runtime = runtime();
    let result = runtime.block_on(async {
        let mut execute = provider.execute(&request, &cancel);
        tokio::time::timeout(Duration::from_secs(20), &mut execute)
            .await
            .expect("execution must not exceed the test bound")
    });
    result.map(|value| value.outcome)
}

fn run_with_cancel(
    mode: &str,
    permission: PermissionPolicy,
    cancel: CancellationToken,
    limits: AcpLimits,
) -> Result<ExecuteOutcome, ExecuteError> {
    let binary = fake_server_binary();
    let workspace = Workspace::new(mode);
    let provider = build_provider(&binary, &workspace.0, mode, permission, limits);
    let request = request();
    let runtime = runtime();
    let result = runtime.block_on(async {
        let mut execute = provider.execute(&request, &cancel);
        tokio::time::timeout(Duration::from_secs(20), &mut execute)
            .await
            .expect("execution must not exceed the test bound")
    });
    result.map(|value| value.outcome)
}

#[test]
fn acp_success_returns_bounded_output() {
    let outcome = run("success", PermissionPolicy::Reject).expect("success outcome");
    match outcome {
        ExecuteOutcome::Succeeded { output, .. } => {
            assert!(output.contains("DXBOT_ACP_OK"), "output was {output:?}");
        }
        other => panic!("expected success, got {other:?}"),
    }
}

#[test]
fn acp_version_drift_fails_closed() {
    assert_eq!(
        run("version-drift", PermissionPolicy::Reject),
        Err(ExecuteError::ProtocolViolation)
    );
}

#[test]
fn acp_model_route_drift_fails_closed_before_prompt() {
    // The fake agent advertises a non-MiniMax model `currentValue` in the
    // session/new configOptions. The provider must verify the route BEFORE
    // sending any prompt and fail closed. The happy `success` path (which
    // advertises the exact `["minimax","MiniMax-M3"]` route) proves the same
    // verification is executed on the live path and passes.
    assert_eq!(
        run("route-drift", PermissionPolicy::Reject),
        Err(ExecuteError::ProtocolViolation)
    );
}

#[test]
fn acp_nonempty_auth_methods_fail_closed() {
    assert_eq!(
        run("auth-methods", PermissionPolicy::Reject),
        Err(ExecuteError::ProtocolViolation)
    );
}

#[test]
fn acp_contaminated_stdout_fails_closed() {
    assert_eq!(
        run("contaminated", PermissionPolicy::Reject),
        Err(ExecuteError::ProtocolViolation)
    );
}

#[test]
fn acp_malformed_frame_fails_closed() {
    assert_eq!(
        run("malformed", PermissionPolicy::Reject),
        Err(ExecuteError::ProtocolViolation)
    );
}

#[test]
fn acp_oversized_line_fails_closed() {
    assert_eq!(
        run("oversized", PermissionPolicy::Reject),
        Err(ExecuteError::OutputExceeded)
    );
}

#[test]
fn acp_unknown_response_id_fails_closed() {
    assert_eq!(
        run("unknown-id", PermissionPolicy::Reject),
        Err(ExecuteError::ProtocolViolation)
    );
}

#[test]
fn acp_permission_reject_is_default() {
    assert_eq!(
        run("permission", PermissionPolicy::Reject),
        Ok(ExecuteOutcome::Cancelled)
    );
}

#[test]
fn acp_permission_allow_once_without_authority_is_rejected() {
    // `PermissionPolicy::AllowOnce` is only an upper bound: without a valid
    // request-scoped allow-once authority grant the provider must still reject,
    // so the turn is cancelled and no side effect is authorized
    // (`DXB-DEL-068` H10 Task 7). A DSH/ambient approval never suffices.
    assert_eq!(
        run("permission", PermissionPolicy::AllowOnce),
        Ok(ExecuteOutcome::Cancelled)
    );
}

#[test]
fn acp_permission_allow_once_with_authority_completes() {
    // Only with BOTH the AllowOnce policy AND a valid DXBOT authority grant is
    // the single allow_once option selected and the turn allowed to complete.
    let outcome = run_with_request(
        "permission",
        PermissionPolicy::AllowOnce,
        request_with_allow_once_grant(),
    )
    .expect("authorized allow-once outcome");
    assert!(matches!(outcome, ExecuteOutcome::Succeeded { .. }));
}

#[test]
fn acp_tool_lifecycle_maps_to_bounded_neutral_effects() {
    // A well-formed tool_call + tool_call_update(completed) is mapped to bounded
    // provider-neutral effects (Prepared then Confirmed) attributed to the same
    // correlation, with no raw ACP payload leaking into the records.
    let result = run_full("tool-lifecycle", PermissionPolicy::Reject, request())
        .expect("tool lifecycle completes");
    assert!(matches!(result.outcome, ExecuteOutcome::Succeeded { .. }));
    assert!(
        result.tool_effects.len() >= 2,
        "expected prepared+confirmed effects, got {:?}",
        result.tool_effects
    );
    let dispositions: Vec<_> = result
        .tool_effects
        .iter()
        .map(|effect| effect.disposition)
        .collect();
    assert!(dispositions.contains(&provider_host::ToolDisposition::Prepared));
    assert!(dispositions.contains(&provider_host::ToolDisposition::Confirmed));
    for effect in &result.tool_effects {
        assert_eq!(effect.correlation, "t1", "correlation must be the tool id");
        // No raw ACP method/payload text leaks into the bounded record.
        assert!(!effect.correlation.contains("sessionUpdate"));
    }
}

#[test]
fn acp_unattributed_tool_update_fails_protocol_and_never_grants() {
    // A tool_call_update for a correlation that was never opened by a prior
    // tool_call is unattributed: it must fail protocol, never complete as a
    // false success, and never record a granted effect.
    assert_eq!(
        run("unattributed-tool", PermissionPolicy::Reject),
        Err(ExecuteError::ProtocolViolation)
    );
}

#[test]
fn acp_crash_before_result_fails_closed() {
    let error = run("crash", PermissionPolicy::Reject).expect_err("crash must fail closed");
    assert!(
        matches!(
            error,
            ExecuteError::IncompleteStream | ExecuteError::TransportUnavailable
        ),
        "unexpected {error:?}"
    );
}

#[test]
fn acp_startup_crash_before_handshake_is_transport_unavailable() {
    // Regression for the live defect: the pinned DSH launcher exited during
    // startup BEFORE answering `initialize` (its runtime/module tree was not
    // resolvable from the invoked path), so the client read a clean EOF during
    // the handshake with no protocol frame at all. That earlier collapsed into
    // `IncompleteStream` -> `execution-failed` with no output and a lingering
    // child, hiding the real "never-started server" cause. A never-started ACP
    // server is now classified as a transport/startup failure, matching the
    // official-frame close behaviour (a mid-turn EOF stays `IncompleteStream`,
    // see `acp_crash_before_result_fails_closed`).
    assert_eq!(
        run("startup-crash", PermissionPolicy::Reject),
        Err(ExecuteError::TransportUnavailable)
    );
}

#[test]
fn acp_hang_hits_prompt_deadline() {
    let mut short = limits();
    short.prompt_deadline = Duration::from_millis(500);
    assert_eq!(
        run_with_cancel(
            "hang",
            PermissionPolicy::Reject,
            CancellationToken::new(),
            short
        ),
        Err(ExecuteError::DeadlineExceeded)
    );
}

#[test]
fn acp_local_cancel_before_prompt_returns_cancelled() {
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert_eq!(
        run_with_cancel("hang", PermissionPolicy::Reject, cancel, limits()),
        Ok(ExecuteOutcome::Cancelled)
    );
}

#[test]
fn acp_cancel_during_initialize_handshake_returns_cancelled() {
    let binary = fake_server_binary();
    let workspace = Workspace::new("handshake-hang");
    let provider = build_provider(
        &binary,
        &workspace.0,
        "handshake-hang",
        PermissionPolicy::Reject,
        limits(),
    );
    let marker = workspace.0.join("initialize.received");
    let request = request();
    let cancel = CancellationToken::new();
    let trigger_cancel = cancel.clone();
    let runtime = runtime();

    let result = runtime.block_on(async {
        let trigger = tokio::spawn(async move {
            tokio::time::timeout(Duration::from_secs(5), async {
                while !marker.exists() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("fake agent must receive initialize before cancellation");
            trigger_cancel.cancel();
        });
        let execute = provider.execute(&request, &cancel);
        let result = tokio::time::timeout(Duration::from_secs(10), execute)
            .await
            .expect("handshake cancellation must preempt the handshake deadline");
        trigger.await.expect("cancellation trigger must complete");
        result
    });

    assert!(matches!(
        result,
        Ok(provider_host::ExecuteResult {
            outcome: ExecuteOutcome::Cancelled,
            ..
        })
    ));
}
#[test]
fn acp_secret_never_appears_in_output_or_errors() {
    let outcome = run("echo-secret-stderr", PermissionPolicy::Reject).expect("success");
    let rendered = format!("{outcome:?}");
    assert!(!rendered.contains("acp-secret-canary-must-not-appear"));
    if let ExecuteOutcome::Succeeded { output, .. } = &outcome {
        assert!(!output.contains("acp-secret-canary-must-not-appear"));
    }
}

#[test]
fn acp_child_receives_only_allowed_env() {
    let outcome = run("echo-env-names", PermissionPolicy::Reject).expect("success");
    match outcome {
        ExecuteOutcome::Succeeded { output, .. } => {
            assert!(output.contains("PATH"));
            assert!(output.contains("DSH_HOME"));
            assert!(output.contains("MINIMAX_API_KEY"));
            assert!(!output.contains("|HOME|"), "env leak: {output}");
            assert!(!output.contains("|USER|"), "env leak: {output}");
        }
        other => panic!("expected success, got {other:?}"),
    }
}

#[test]
fn acp_teardown_reaps_whole_process_group() {
    // Prove the EOF → TERM → KILL ladder reaps the whole process group, not
    // just the direct child. The fake agent forks a long-lived grandchild
    // (`sleep 600`) and records its PID under DSH_HOME, then hangs so the
    // prompt deadline forces teardown. After dispose the grandchild must be
    // gone (kill(pid, 0) == ESRCH).
    reap_case_asserts_no_grandchild("grandchild-leak", |limits| {
        limits.prompt_deadline = Duration::from_millis(500);
    });
}

#[test]
fn acp_teardown_reaps_process_group_on_crash() {
    // Same process-group reap guarantee on the crash branch: the fake agent
    // forks the grandchild, then exits hard before any prompt result. The
    // client observes IncompleteStream and still killpg-reaps the group.
    reap_case_asserts_no_grandchild("grandchild-leak-crash", |_| {});
}

/// Run a grandchild-spawning mode with a retained workspace, then assert the
/// recorded grandchild PID no longer exists after the provider disposes.
fn reap_case_asserts_no_grandchild(mode: &str, tune: impl FnOnce(&mut AcpLimits)) {
    let binary = fake_server_binary();
    let workspace = Workspace::new(mode);
    let mut limits = limits();
    tune(&mut limits);
    let provider = build_provider(
        &binary,
        &workspace.0,
        mode,
        PermissionPolicy::Reject,
        limits,
    );
    let request = request();
    let runtime = runtime();
    let cancel = CancellationToken::new();
    let _ = runtime.block_on(async {
        let mut execute = provider.execute(&request, &cancel);
        tokio::time::timeout(Duration::from_secs(20), &mut execute).await
    });

    // The grandchild PID was recorded by the fake agent before it hung.
    let pid_path = workspace.0.join("grandchild.pid");
    let pid_text = std::fs::read_to_string(&pid_path)
        .unwrap_or_else(|error| panic!("grandchild pid file must exist: {error}"));
    let pid: i32 = pid_text.trim().parse().unwrap();

    // Poll briefly for the reap to complete, then require the grandchild gone.
    let mut alive = true;
    for _ in 0..50 {
        if !process_is_alive(pid) {
            alive = false;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    if alive {
        // Best-effort cleanup so a failed assertion does not leak the process.
        let _ = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(pid),
            Some(nix::sys::signal::Signal::SIGKILL),
        );
        panic!("grandchild pid {pid} survived teardown (process-group leak)");
    }
}

/// True if the process still exists. A `kill(pid, 0)` existence probe that
/// returns `ESRCH` means it is gone (fully reaped by process-group teardown).
fn process_is_alive(pid: i32) -> bool {
    // `None` sends signal 0: an existence/permission check without delivery.
    !matches!(
        nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None),
        Err(nix::errno::Errno::ESRCH)
    )
}

#[test]
fn acp_executes_through_provider_host_exact_selection() {
    use provider_host::{
        ProviderHost, ProviderRegistration, ProviderStatus, RegistrationLimits, TaskDescription,
        TaskStatus,
    };
    use std::sync::Arc;

    let binary = fake_server_binary();
    let workspace = Workspace::new("host-select");
    let provider = build_provider(
        &binary,
        &workspace.0,
        "success",
        PermissionPolicy::Reject,
        limits(),
    );
    let runtime = runtime();
    let registration = ProviderRegistration::detached(
        Arc::new(provider),
        provider_host::ProtocolKind::AcpV1Stdio,
        RegistrationLimits {
            max_output_bytes: 256 * 1024,
            max_output_items: 4096,
        },
    );
    let host = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    host.register(registration).expect("register acp provider");
    let info = host
        .get_provider(&ProviderId("deepseek-harness-acp".to_owned()))
        .expect("registered");
    assert_eq!(info.status, ProviderStatus::Ready);

    let task = TaskDescription {
        intent: "DXBOT_ACP_PROMPT".to_owned(),
        context: "bounded-context".to_owned(),
        budget: Some(16),
        deadline: None,
    };
    let result = host
        .execute_task_with_cancel(
            &ProviderId("deepseek-harness-acp".to_owned()),
            1,
            &task,
            CancellationToken::new(),
        )
        .expect("execute through host");
    assert_eq!(result.status, TaskStatus::Completed);
    assert!(result.output.contains("DXBOT_ACP_OK"));
    assert_eq!(
        result.evidence[0].provider,
        ProviderId("deepseek-harness-acp".to_owned())
    );
}

/// Build a provider against a per-test workspace and return both the provider
/// and the on-disk asset paths, so a test can mutate an asset AFTER composition
/// but BEFORE execution and observe the spawn-time recheck fail closed.
fn build_provider_with_paths(
    command: &Path,
    workspace: &Path,
    mode: &str,
) -> (DeepSeekHarnessAcpProvider, PathBuf) {
    let provider = build_provider(command, workspace, mode, PermissionPolicy::Reject, limits());
    let patch_path = workspace.join(format!("dsh-acp.patch.{mode}.yaml"));
    (provider, patch_path)
}

/// Run a single execute attempt against an already-built provider.
fn run_provider(provider: &DeepSeekHarnessAcpProvider) -> Result<ExecuteOutcome, ExecuteError> {
    let request = request();
    let runtime = runtime();
    let cancel = CancellationToken::new();
    runtime
        .block_on(async {
            let mut execute = provider.execute(&request, &cancel);
            tokio::time::timeout(Duration::from_secs(20), &mut execute)
                .await
                .expect("execution must not exceed the test bound")
        })
        .map(|value| value.outcome)
}

#[test]
fn acp_command_mutation_after_composition_fails_at_spawn() {
    // A fully-valid config is composed (digests pinned to the current on-disk
    // command). The command is then REPLACED with different bytes before any
    // execution. The spawn-time recheck rehashes the mutated command, the
    // constant-time compare to the pinned digest fails, and no child spawns.
    let binary = fake_server_binary();
    let workspace = Workspace::new("mutate-command");
    // Copy the real fake binary into the per-test workspace so we can mutate a
    // private copy without disturbing the shared compiled fixture.
    let command = workspace.0.join("dsh");
    std::fs::copy(&binary, &command).unwrap();
    std::fs::set_permissions(&command, std::fs::Permissions::from_mode(0o700)).unwrap();
    let (provider, _patch) = build_provider_with_paths(&command, &workspace.0, "success");

    // Sanity: the unmutated provider spawns and succeeds.
    match run_provider(&provider) {
        Ok(ExecuteOutcome::Succeeded { .. }) => {}
        other => panic!("baseline must succeed, got {other:?}"),
    }

    // Mutate the command on disk (append a byte → different digest, still
    // owner-executable). The next spawn must fail closed at the digest compare.
    let mut bytes = std::fs::read(&command).unwrap();
    bytes.push(0);
    std::fs::write(&command, &bytes).unwrap();
    std::fs::set_permissions(&command, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(run_provider(&provider), Err(ExecuteError::ExecutionFailed));
}

#[test]
fn acp_patch_mutation_after_composition_fails_at_spawn() {
    // Same TOCTOU shape for the audited patch: compose, then overwrite the
    // on-disk patch with different bytes. The spawn recheck rehashes it, the
    // compare to the pinned (and canonical audited) digest fails, no child runs.
    let binary = fake_server_binary();
    let workspace = Workspace::new("mutate-patch");
    let (provider, patch) = build_provider_with_paths(&binary, &workspace.0, "success");

    match run_provider(&provider) {
        Ok(ExecuteOutcome::Succeeded { .. }) => {}
        other => panic!("baseline must succeed, got {other:?}"),
    }

    // Tamper the patch after composition; digest no longer matches.
    std::fs::write(&patch, b"# tampered patch bytes\n").unwrap();
    std::fs::set_permissions(&patch, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(run_provider(&provider), Err(ExecuteError::ExecutionFailed));
}

#[test]
fn acp_group_writable_ancestor_below_barrier_fails_at_spawn() {
    // The command lives under a group/other-writable intermediate directory
    // that sits BELOW any trusted barrier. Even though the command file itself
    // is owner-only and its digest matches, the writable ancestor is a swap
    // vector, so the spawn-time ancestor walk fails closed.
    let workspace = Workspace::new("writable-ancestor"); // 0700 barrier root
    let binary = fake_server_binary();

    // Create a writable intermediate under the 0700 barrier: barrier(0700) /
    // writable(0777) / command. The walk from the command reaches `writable`
    // (group/other-writable) BEFORE the 0700 barrier and must reject it.
    let writable = workspace.0.join("writable");
    std::fs::create_dir_all(&writable).unwrap();
    std::fs::set_permissions(&writable, std::fs::Permissions::from_mode(0o777)).unwrap();
    let command = writable.join("dsh");
    std::fs::copy(&binary, &command).unwrap();
    std::fs::set_permissions(&command, std::fs::Permissions::from_mode(0o700)).unwrap();

    // Keep the patch in the barrier root (a safe ancestor) so the ONLY defect
    // is the writable command ancestor.
    let (provider, _patch) = build_provider_with_paths(&command, &workspace.0, "success");
    assert_eq!(run_provider(&provider), Err(ExecuteError::ExecutionFailed));
}
