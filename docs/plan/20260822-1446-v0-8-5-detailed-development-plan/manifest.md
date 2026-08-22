---
title: "DXBOT v0.8.5 문서 매니페스트와 검수 기록"
document_id: "DXB-MANIFEST"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-BASE-000", "DXB-DEL-061", "DXB-DEL-062", "DXB-DEL-063", "DXB-ENG-052", "DXB-ENG-054"]
review_revision: 5
---
# DXBOT v0.8.5 문서 매니페스트와 검수 기록

<!-- manifest-machine:start -->
active_package_path: docs/plan/20260822-1446-v0-8-5-detailed-development-plan
plan_version: 0.8.5
review_revision: 5
markdown_count: 49
source_baseline_commit: 5b55b67fbaaf0f3192c126865e8b665ff26fc5aa
<!-- manifest-machine:end -->

## Resolution

| Finding | Resolution |
|---|---|
| v0.8.5 identity absent | package/readme/manifest version + review revision |
| source precedence conflict | source `normative:false`; adopted INV materialized in BASE |
| stale CI | one-shot write workflow removed; read-only validator added |
| recovery-key gap | CommandId/IdempotencyKey + per-command fsync-before-send |
| incomplete CLI grammar | IFC-042 62-command input registry |
| bootstrap ambiguity | server-derived Principal + atomic default policies |
| no real Harness path | AT-HARNESS-001 canary |
| no Multi-Bot CLI E2E | typed membership + delegated SubmitTask |
| storage profile gap | ARC-018 M1A executable proof |
| export rename race | descriptor-relative atomic no-replace |
| implicit host stop | explicit `--host-stop --host-generation` |
| premature delete states | removed from P0 Bot lifecycle |

## Review evidence

### Review 1 — Structural/Traceability

- active 49 Markdown inventory
- unique IDs and local dependency DAG
- command/input registry equality
- Acceptance/Risk/Milestone linkage
- source nonnormative
- result: local validator PASS

### Review 2 — Cross-Layer Executability

- journal prepare→send→commit→lookup crash model
- bootstrap principal/default policy
- storage proof placement before parser freeze
- graceful/host stop separation
- export no-replace publication
- result: document contract PASS; Rust/storage executable fixtures remain Specified

### Review 3 — Adversarial Scope

- generic RPC/workflow/UI expansion 0
- Reference Provider silent fallback 0
- P0 hard delete state 0
- duplicate reconcile alias removed
- result: PASS

## Evidence boundary

Executed in this change: validator syntax/self-test/local synthetic active-package validation and document relationship reviews.

Not yet executable: Rust build, M1A storage spike, Runtime/CLI E2E, real Harness canary, concurrency/security/performance soak. These remain `Specified`, not `Passed`.
