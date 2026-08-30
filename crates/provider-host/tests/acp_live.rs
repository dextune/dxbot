//! Live, opt-in integration test for the official DeepSeek Harness ACP provider
//! (`DXB-DEL-068` H6/H7).
//!
//! Unlike `acp.rs` (which drives a deterministic in-repo fake agent), this test
//! spawns the **real pinned DSH ACP server in place** and drives it end to end
//! through the canonical `Execute` path (`DeepSeekHarnessAcpProvider::execute`).
//! It is `#[ignore]` by default and only runs when the operator opts in with
//! the required environment, so CI without the pinned harness+credential stays
//! green.
//!
//! Required environment (all must be set, or the test panics so a silent no-op
//! cannot masquerade as a pass):
//!
//! * `DXBOT_TEST_DSH_BIN`   — absolute path to the pinned DSH executable, in
//!   place (`.../apps/cli/lib/bin.js`). It is driven where it lives so its
//!   sibling package/module tree resolves; it is never copied.
//! * `DXBOT_TEST_DSH_PATCH` — absolute path to the audited list-form patch
//!   whose `baseURL` is `https://api.minimax.io/anthropic`.
//! * `DXBOT_TEST_DSH_HOME`  — absolute isolated `DSH_HOME` directory.
//! * `MINIMAX_API_KEY`      — the MiniMax credential (never printed/stored).
//!
//! The provider's spawn-time asset+ancestor integrity check requires the
//! command and patch (and every ancestor up to a trusted `0700`-owner barrier
//! or the filesystem root) to be free of group/other write bits. A production
//! deployment installs these under an immutable, root-owned path that satisfies
//! this; a developer checkout must ensure the same (e.g. `chmod g-w` the tree)
//! before running, exactly as the runbook requires.
//!
//! Run it by sourcing the deployment credential file and pointing at the pinned
//! assets, e.g.:
//!
//! ```text
//! set -a; . /home/dextune/.config/dxbot/provider.env; set +a
//! DXBOT_TEST_DSH_BIN=.../apps/cli/lib/bin.js \
//! DXBOT_TEST_DSH_PATCH=.../deploy/dsh/dsh-acp.patch.yaml \
//! DXBOT_TEST_DSH_HOME=/tmp/dxbot-dsh-home \
//! cargo test -p provider-host --features dsh-acp --test acp_live -- --ignored --nocapture
//! ```
//!
//! Secret discipline: the model output text and the API key are NEVER printed
//! or stored. The test asserts only that the output is non-empty, that the
//! provider identity/generation are exact, and that the child process (and its
//! whole process group) is reaped.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(all(unix, feature = "dsh-acp"))]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dxbot_core::types::ProviderId;
use provider_host::{
    AcpConfigInput, AcpLimits, AcpProviderConfig, CancellationToken, DeepSeekHarnessAcpProvider,
    ExecuteOutcome, ExecuteProvider, ExecuteRequest, PermissionPolicy,
};

/// A `0700` owner-only workspace root under `/tmp`, used as the ACP session
/// cwd. Only the workspace is synthesized; the pinned harness is driven where
/// it lives so its module tree resolves.
struct Barrier(PathBuf);

impl Barrier {
    fn new(tag: &str) -> Self {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("dxbot-acp-live-{tag}-{suffix}"));
        std::fs::create_dir_all(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
}

impl Drop for Barrier {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

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

/// Generous but finite live bounds. The live handshake includes a real model
/// catalog probe and the prompt is a real MiniMax turn, so these are larger
/// than the deterministic fake's bounds but still bounded.
fn limits() -> AcpLimits {
    AcpLimits {
        max_line_bytes: 256 * 1024,
        max_output_bytes: 1024 * 1024,
        max_update_frames: 4096,
        max_stderr_bytes: 16 * 1024,
        handshake_deadline: Duration::from_secs(30),
        prompt_deadline: Duration::from_secs(120),
        eof_grace: Duration::from_secs(6),
        term_grace: Duration::from_secs(3),
    }
}

fn require_env(name: &str) -> String {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => value,
        _ => panic!(
            "live ACP test requires env {name}; set DXBOT_TEST_DSH_BIN, \
             DXBOT_TEST_DSH_PATCH, DXBOT_TEST_DSH_HOME and MINIMAX_API_KEY \
             (source the deployment provider.env) then run with --ignored"
        ),
    }
}

#[test]
#[ignore = "live: requires the pinned DSH harness + MINIMAX_API_KEY; run with --ignored"]
fn acp_live_executes_through_canonical_execute() {
    let bin = require_env("DXBOT_TEST_DSH_BIN");
    let patch = require_env("DXBOT_TEST_DSH_PATCH");
    let dsh_home = require_env("DXBOT_TEST_DSH_HOME");
    let api_key = require_env("MINIMAX_API_KEY");

    let bin_path = PathBuf::from(&bin);
    let patch_path = PathBuf::from(&patch);
    assert!(
        bin_path.is_absolute(),
        "DXBOT_TEST_DSH_BIN must be absolute"
    );
    assert!(
        patch_path.is_absolute(),
        "DXBOT_TEST_DSH_PATCH must be absolute"
    );

    // Assert the audited patch preserves the list-form + `/anthropic` baseURL
    // that the direct-official probe proved works. This never prints the file.
    let patch_text = std::fs::read_to_string(&patch_path).expect("read audited patch");
    assert!(
        patch_text.contains("baseURL: https://api.minimax.io/anthropic"),
        "audited patch must pin the MiniMax /anthropic baseURL"
    );
    assert!(
        patch_text.trim_start().starts_with("#") || patch_text.contains("- id:"),
        "audited patch must be the list-form patch"
    );

    // Isolated workspace (0700 barrier) as the session cwd.
    let workspace = Barrier::new("ws");

    // Digests of the real in-place assets, exactly as a deployment would pin.
    let command_sha256 = file_sha256_hex(&bin_path);
    let patch_sha256 = file_sha256_hex(&patch_path);
    let args = vec![
        "--profile".to_owned(),
        "acp".to_owned(),
        "--patch".to_owned(),
        patch.clone(),
    ];

    let config = AcpProviderConfig::new(AcpConfigInput {
        id: ProviderId("deepseek-harness-acp".to_owned()),
        capability: "llm-chat",
        generation: 1,
        command: &bin,
        args: &args,
        patch_path: &patch,
        dsh_home: &dsh_home,
        workspace: workspace.0.to_str().unwrap(),
        path_env: std::env::var("PATH").as_deref().unwrap_or("/usr/bin:/bin"),
        permission: PermissionPolicy::Reject,
        limits: limits(),
        source_commit: provider_host::ACP_SOURCE_COMMIT,
        source_version: provider_host::ACP_SOURCE_VERSION,
        command_sha256: &command_sha256,
        patch_sha256: &patch_sha256,
        audited_patch_sha256: &patch_sha256,
        api_key: &api_key,
    })
    .expect("valid live acp config");

    let provider = DeepSeekHarnessAcpProvider::new(config);

    // A tool-free greeting: the smallest prompt that still exercises a full
    // real MiniMax turn to a clean `end_turn`.
    let request = ExecuteRequest::bounded(
        ProviderId("deepseek-harness-acp".to_owned()),
        1,
        "Reply with a single short greeting word.".to_owned(),
        "greeting".to_owned(),
        Some(64),
        None,
    );

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let cancel = CancellationToken::new();

    let result = runtime.block_on(async {
        let mut execute = provider.execute(&request, &cancel);
        tokio::time::timeout(Duration::from_secs(150), &mut execute)
            .await
            .expect("live execution must not exceed the test bound")
    });

    let result = match result {
        Ok(result) => result,
        // Do not print any provider text; the category is safe to surface.
        Err(error) => panic!("live ACP execute failed: {error:?}"),
    };

    // Exact provider identity + generation.
    assert_eq!(
        result.provider_id,
        ProviderId("deepseek-harness-acp".to_owned())
    );
    assert_eq!(result.provider_generation, 1);

    match &result.outcome {
        ExecuteOutcome::Succeeded { output, .. } => {
            // Non-empty output WITHOUT ever printing it or the key.
            assert!(
                !output.trim().is_empty(),
                "live ACP output must be non-empty (len only): len={}",
                output.len()
            );
            assert!(
                !output.contains(&api_key),
                "output must never contain the credential"
            );
            eprintln!("live ACP succeeded: output_bytes={}", output.len());
        }
        other => panic!("expected Succeeded, got {other:?}"),
    }

    // The child (and its whole process group) must be reaped by teardown: no
    // `dsh`/node grandchild may outlive the execution. Confirm by checking that
    // no live process still names our isolated DSH_HOME after a brief grace.
    std::thread::sleep(Duration::from_millis(500));
    let leaked = live_pids_with_dsh_home(&dsh_home);
    assert!(
        leaked.is_empty(),
        "DSH child/grandchild leaked past teardown: {} pid(s)",
        leaked.len()
    );
}

/// Return live pids whose `/proc/<pid>/environ` names our isolated DSH_HOME.
/// Reads only variable presence, never printing any value. Best-effort: a
/// platform without `/proc` yields an empty list (the process-group SIGKILL
/// sweep in teardown is the authoritative reap regardless).
fn live_pids_with_dsh_home(dsh_home: &str) -> Vec<i32> {
    let needle = format!("DSH_HOME={dsh_home}");
    let mut leaked = Vec::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return leaked;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(|value| value.parse::<i32>().ok()) else {
            continue;
        };
        let environ = entry.path().join("environ");
        let Ok(bytes) = std::fs::read(&environ) else {
            continue;
        };
        // NUL-separated KEY=VALUE records.
        if bytes
            .split(|byte| *byte == 0)
            .any(|record| record == needle.as_bytes())
        {
            leaked.push(pid);
        }
    }
    leaked
}
