# DXBOT Runtime service runbook

This runbook is the supported Linux user-service profile for the P0 local Operational Runtime. The packaged unit is `deploy/systemd/dxbot-runtime.service`; packaging installs it under the user unit directory and installs `dxb` as `/usr/bin/dxb`.

## Install and configure

1. Install the unit, then run `systemctl --user daemon-reload`.
2. Create the provider config at `$XDG_STATE_HOME/dxbot/runtime/provider-config.json` with mode `0600`. It stores only `env:<NAME>` as `credential_ref`; never put credential bytes in this JSON.
3. If a credential environment variable is needed, place `NAME=value` in `%E/dxbot/provider.env` (normally `$XDG_CONFIG_HOME/dxbot/provider.env`) with mode `0600`. The optional `EnvironmentFile` is read only by the service manager/Runtime Host and must not be attached to diagnostics or incident artifacts.
4. Enable with `systemctl --user enable --now dxbot-runtime.service`.

For MiniMax M3, the owner-only config is:

```json
{
  "schema_version": 1,
  "adapter": "minimax-m3",
  "provider_id": "minimax-m3-production",
  "capability": "llm-chat",
  "generation": 1,
  "endpoint": "https://api.minimax.io/anthropic/v1",
  "model": "MiniMax-M3",
  "credential_ref": "env:MINIMAX_API_KEY",
  "timeout_ms": 60000
}
```

`generation` is an owner-chosen positive generation and must advance when replacing the configured Provider. The actual `MINIMAX_API_KEY` value belongs only in the owner-only service environment file. The Runtime uses Anthropic-compatible `/messages` with scoped `x-api-key`; do not append `/messages` to `endpoint`.

### Official DeepSeek Harness (DSH) ACP provider

For the pinned official DeepSeek Harness ACP subprocess provider (`DXB-DEL-068` H6/H7), use schema v2 with a `harness` block instead of an HTTP `endpoint`. An example config is shipped at `deploy/dsh/provider-config.acp.example.json`; the audited DSH profile patch is `deploy/dsh/dsh-acp.patch.yaml`.

```json
{
  "schema_version": 2,
  "adapter": "deepseek-harness-acp",
  "provider_id": "deepseek-harness-acp",
  "capability": "llm-chat",
  "generation": 1,
  "model": "MiniMax-M3",
  "credential_ref": "env:MINIMAX_API_KEY",
  "timeout_ms": 60000,
  "harness": {
    "command": "/usr/lib/dxbot/dsh/bin/dsh",
    "args": ["--profile", "acp", "--patch", "/usr/lib/dxbot/dsh/dsh-acp.patch.yaml"],
    "patch_path": "/usr/lib/dxbot/dsh/dsh-acp.patch.yaml",
    "dsh_home": "/var/lib/dxbot/dsh-home",
    "workspace": "/var/lib/dxbot/workspace",
    "path_env": "/usr/bin:/bin",
    "permission": "reject",
    "source_commit": "cd5ef8148158c3a752a658978873241fdf8e2bbc",
    "source_version": "0.1.2-alpha.1",
    "command_sha256": "<sha256sum of your pinned dsh executable>",
    "patch_sha256": "1fe22c1e2a674c5f0912e612a9d59259a6b52fb50d90889a4c326bddbb9b6b7a",
    "limits": {
      "max_line_bytes": 262144, "max_output_bytes": 1048576,
      "max_update_frames": 4096, "max_stderr_bytes": 16384,
      "handshake_deadline_ms": 15000, "prompt_deadline_ms": 120000,
      "eof_grace_ms": 6000, "term_grace_ms": 3000
    }
  }
}
```

Operational rules for the DSH ACP provider:

- The harness is pinned to the exact upstream identity in `DXB-DEL-064`: `source_commit` must be `cd5ef8148158c3a752a658978873241fdf8e2bbc` and `source_version` must be `0.1.2-alpha.1` (tag `dsh-v0.1.2-alpha.1`). Any other value fails closed. There is no floating branch and no `latest` follow.
- `args` must be exactly `["--profile", "acp", "--patch", "<patch_path>"]` with the trailing element equal to `patch_path`. No omission, extra, duplicate, reordered, or mismatched argument is accepted.
- `command_sha256` and `patch_sha256` are lowercase 64-hex SHA-256 digests. Compute `command_sha256` with `sha256sum <command>` for your pinned `dsh` executable. Compute `patch_sha256` with `sha256sum deploy/dsh/dsh-acp.patch.yaml`; the current audited patch digest is `1fe22c1e2a674c5f0912e612a9d59259a6b52fb50d90889a4c326bddbb9b6b7a`. Before it resolves any secret, the Runtime independently recomputes both digests from disk, compares them to the config values, and additionally requires the on-disk patch to match the compile-time audited digest of the repository `deploy/dsh/dsh-acp.patch.yaml`. The command digest is deployment-specific but is compared to the actual on-disk executable.
- Filesystem preflight (evaluated before secret resolution, via `symlink_metadata`): `command` and `patch_path` must be direct regular files (never symlinks), neither group- nor other-writable, and within bounded sizes; `command` must additionally be owner-executable. `dsh_home` and `workspace` must be direct existing directories (never symlinks); `dsh_home` must be owner-only (mode `0700`, no group/other bits). Any violation fails closed with a category-only diagnostic that never contains a path, digest, or file content.
- All `command`, `patch_path`, `dsh_home`, and `workspace` paths must be absolute; the Runtime validates them and rejects relative or `..`-traversal paths at load.
- Spawn-time asset recheck (evaluated again immediately before every `Command::new`, on the live execution path): the provider re-opens `command` and `patch_path` through `symlink_metadata` (never following a symlink), re-confirms each is a direct regular file with the required mode (owner-executable command; neither group- nor other-writable command or patch) and bounded size, streams a fresh SHA-256, and constant-time compares it to the configured digests. The patch is *additionally* compared to the same canonical compile-time audited digest used at composition (a single value threaded from the Runtime config owner into the provider — there is no second, independently computed audited digest). The provider then walks every ancestor directory of both assets and rejects a symlinked or group/other-writable ancestor until it reaches a **trusted barrier**, defined exactly as: either (1) the filesystem root reached through a chain of ancestors none of which were group/other-writable, or (2) an owner-only directory (mode has no group/other bits, i.e. `0700`-equivalent) owned by the service's effective uid. This makes a `mkdtemp` `0700` root created under the sticky world-writable `/tmp` a valid barrier (the world-writable `/tmp` above it is never inspected), while still rejecting any group/other-writable ancestor *below* the barrier. Every violation fails closed with the same category-only diagnostic (no path, digest, mode, size, or uid).
  - **Limitation (honest):** this recheck is *path-based*, not a kernel `fd`-bound execution. It substantially narrows the composition→spawn time-of-check/time-of-use window and defeats leaf swaps and writable-ancestor swaps that are observable via `symlink_metadata` at spawn, but it does **not** make the binding provably race-free: a sufficiently precise attacker who can win the remaining micro-race between the final `symlink_metadata`/hash and the kernel's `execve` resolution could still substitute content. The deployment requirement therefore stands and is not optional: `command`, `patch_path`, and their ancestor directories up to a trusted barrier must live on an **immutable, root-owned deployment path** (e.g. `/usr/lib/dxbot/...` and `/var/lib/dxbot/...` owned by root, non-writable by the service uid and by group/other). The path-based recheck is defense in depth on top of that requirement, never a substitute for it.
- Filesystem preflight redux: the same non-writable / non-symlink / bounded-size / correct-mode rules the Runtime enforces at composition are what the spawn-time recheck re-verifies; a drift between composition and spawn (mutation, replacement, symlink, permission relaxation, or writable-ancestor swap) fails closed at spawn.
- `model` must be exactly `MiniMax-M3`. There is no alternate provider and no silent model/provider fallback. Before it publishes any provider activity, the Runtime verifies the `session/new` config-option state advertises the exact model `currentValue` `["minimax","MiniMax-M3"]`; a drifted or absent route fails closed and no prompt is sent.
- `DSH_HOME` must be a dedicated, isolated directory owned by the service — never the ambient user DSH home. The child runs with `env_clear()` and receives only `PATH`, `DSH_HOME`, and `MINIMAX_API_KEY`.
- The child is spawned directly (no shell) in its own process group. Teardown is a bounded ladder — an optional graceful `session/close` (only when the agent advertised `sessionCapabilities.close`) → EOF (stdin close) → `SIGTERM` → `SIGKILL` — applied to the whole process group, and always finishes with an unconditional `SIGKILL` sweep of the group so a grandchild cannot survive a graceful/EOF child exit. `active_permits` must return to zero and no child or grandchild process may leak.
- `permission` is the unattended upper bound for `session/request_permission`: `reject` (the safe default) or `allow-once`. `allow-once` is only an *upper bound*: the provider selects an `allow_once` option **only** when the execution also carries a request-scoped allow-once authority grant that the scheduler minted from an `Approved` runtime-security Approval bound to action `provider-tool-allow-once`, target the exact execution reference, and a positive policy generation. Absent that approved authority — or under `reject` — every permission request is cancelled. A malformed or unattributed tool event fails protocol and never grants. DSH sandbox/approval and any ambient credential are defense in depth, never the DXBOT authority. Each tool side effect is recorded as a bounded, provider-neutral disposition in the Application Side Effect evidence; an `Unknown` disposition forces `RecoveryRequired` with no blind retry.
- Optional `harness.resource_policy` declares OS resource governance for the ACP child (`DXB-DEL-068` H10). It is additive and optional; absent, only the process-model bounds apply (single ACP process/call, queue=64, active=4, and the frame/output/stderr/deadline limits above). Every declared limit (`cpu`, `memory`, `pids`, `open_files`, `file_size`) has a positive-finite `max` and an explicit `enforcement`: `deployment-policy` (enforced by a named deployment mechanism such as a systemd unit's `CPUQuota`/`MemoryMax`/`TasksMax`/`LimitNOFILE`/`LimitFSIZE`, recorded via `deployment_ref`) or `required-in-process`. A `required-in-process` limit **fails the composition closed**: DXBOT does not fabricate unprivileged in-process OS enforcement, so it refuses to start rather than run unrestricted while pretending the control exists. `max_concurrent_processes` must be positive (P0 uses 1 process per call). `network_ref` names the deployment mechanism that enforces the network stance (empty means the default no-network deployment posture). The DSH sandbox may be one such enforcement mechanism but is never the canonical authority. Example:

  ```json
  "resource_policy": {
    "max_concurrent_processes": 1,
    "cpu": { "max": 200, "enforcement": "deployment-policy", "deployment_ref": "systemd:CPUQuota" },
    "memory": { "max": 1073741824, "enforcement": "deployment-policy", "deployment_ref": "systemd:MemoryMax" },
    "pids": { "max": 64, "enforcement": "deployment-policy", "deployment_ref": "systemd:TasksMax" },
    "open_files": { "max": 256, "enforcement": "deployment-policy", "deployment_ref": "systemd:LimitNOFILE" },
    "network_ref": "systemd:IPAddressDeny"
  }
  ```
- Every ACP execution uses a fresh process and a fresh DSH session; there is no pooling, no multiplex, and no automatic `session/resume` in P0.
- Never place the `MINIMAX_API_KEY` value in the config, the patch, argv, or any diagnostic; only the `env:MINIMAX_API_KEY` reference appears in config.

The unit sets `XDG_STATE_HOME=%S`, `UMask=0077`, a bounded 30-second startup/stop window, and `Restart=on-failure`. `ExecStart` runs the packaged foreground Runtime Host service entrypoint. The hidden token is a packaging interface, not a user CLI command and must not be invoked by automation outside this unit contract.

## Readiness and health

Use these bounded, authenticated checks:

```text
dxb runtime status --format json
dxb runtime doctor --format json
```

A healthy service reports control/provider/storage/audit/resource/recovery sections. Provider unavailable is not converted to ReferenceProvider fallback. `active_permits` must return to zero after work drains.

## Graceful stop and restart

Always use the supervisor first:

```text
systemctl --user stop dxbot-runtime.service
systemctl --user restart dxbot-runtime.service
```

`ExecStop` sends the authenticated, HostGeneration-fenced `runtime stop --host-stop` request. Startup and shutdown are owned by a single `RuntimeComposition` (`DXB-DEL-068` H8): the Runtime Host builds exactly one composition graph whose extensions are thin lifecycle adapters over the already-constructed canonical owners (Application/Security/Audit/Provider runtime/Execution coordinator/Control endpoint), with the control endpoint, PID artifact, and discovery entry as the terminal publication extensions. Any failure at or after the first publication step rolls back in strict reverse order, so a failed start leaves **zero** published endpoint, PID, or discovery entry and zero live Execution permits. Runtime graceful-stop ordering is:

```text
admission stop
→ active Provider cancellation/drain and Core Lease release (execution coordinator join)
→ ProviderHost bounded drain (reclaim superseded Draining generations; no slot leak)
→ Runtime-owned Tokio runtime stop
→ final required AuditIntent projection/checkpoint
→ control endpoint unpublish
→ discovery unpublish
→ PID artifact removal
→ process exit / single-writer lock handoff
```

The graceful-stop order is a distinct documented sequence (admission-first), not the strict reverse of startup; the reverse-order composition rollback is used only for the startup-failure path. The observable shutdown evidence steps are `admission-stopped, activity-drained, provider-drained, provider-runtime-stopped, audit-checkpointed, endpoint-unpublished, discovery-unpublished, pid-removed`.

The next supervised start waits at most 10 seconds for a departing owner to hand off the same exclusive lock, then fails closed instead of allowing two writers. `TimeoutStopSec=30s` is larger than that bounded handoff/drain window. SIGKILL is fallback-only; after forced termination the next start marks uncertain in-flight Provider activity `RecoveryRequired` and never blindly redispatches it.

## Backup, restore, and migration

Backup/restore is offline-only and uses the Runtime Host's same exclusive lock. Stop the service before invoking an operator tool linked to `runtime_host::OfflineStorage`; run `RestoreMode::DryRun` before explicitly authorized `RestoreMode::Apply`. A backup set contains Application, Security, and present coordination/audit artifacts, an owner-only SHA-256 manifest, and bounded files. Digest mismatch, symlink, path traversal, oversize, or a live Runtime fails closed.

Application v1→v2 migration runs before canonical stores open, while the host owns the lock, and captures a complete pre-migration backup. Never delete `backups/pre-migration-v1-*` until post-upgrade health and recovery checks pass.

## Incident checks

- Repeated restart: InstanceId stays stable; HostGeneration strictly increases.
- Crash during Provider call: inspect `task result`, `process show`, recovery doctor counts, Side Effect `unknown`, and durable audit records before explicit resume.
- Storage pressure: stop admission, preserve current state files and backup set, free space, then restart. Atomic temp writes leave the previously renamed durable file unchanged on `StorageFull`.
- Audit corruption: preserve `audit-outbox.json` for incident analysis. Startup intentionally fails closed; do not truncate or regenerate it silently.
- Secret safety: never paste provider environment files, raw prompts, or credential canaries into tickets, logs, backup manifests, or doctor output.
