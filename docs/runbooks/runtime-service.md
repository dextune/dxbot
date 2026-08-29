# DXBOT Runtime service runbook

This runbook is the supported Linux user-service profile for the P0 local Operational Runtime. The packaged unit is `deploy/systemd/dxbot-runtime.service`; packaging installs it under the user unit directory and installs `dxb` as `/usr/bin/dxb`.

## Install and configure

1. Install the unit, then run `systemctl --user daemon-reload`.
2. Create the provider config at `$XDG_STATE_HOME/dxbot/runtime/provider-config.json` with mode `0600`. It stores only `env:<NAME>` as `credential_ref`; never put credential bytes in this JSON.
3. If a credential environment variable is needed, place `NAME=value` in `%E/dxbot/provider.env` (normally `$XDG_CONFIG_HOME/dxbot/provider.env`) with mode `0600`. The optional `EnvironmentFile` is read only by the service manager/Runtime Host and must not be attached to diagnostics or incident artifacts.
4. Enable with `systemctl --user enable --now dxbot-runtime.service`.

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

`ExecStop` sends the authenticated, HostGeneration-fenced `runtime stop --host-stop` request. Runtime ordering is:

```text
admission stop
→ active Provider cancellation/drain and Core Lease release
→ final required AuditIntent projection/checkpoint
→ control endpoint unpublish
→ discovery unpublish
→ PID artifact removal
→ process exit / single-writer lock handoff
```

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
