---
title: "테스트·검증·3회 Review 전략 v0.8.5"
document_id: "DXB-ENG-052"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-ENG-050", "DXB-ENG-051", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042", "DXB-ARC-018", "DXB-RUN-033"]
---
# 테스트·검증·3회 Review 전략 v0.8.5

## Deterministic fixtures

- package discovery/version/review revision
- source provenance is nonnormative
- command registry ↔ input registry set equality
- Prepared journal fsync-before-send crash points
- multi-process per-command journal
- CommandId/IdempotencyKey lookup; ClientRequestId recovery rejection
- bootstrap UID→Principal and default policy atomic initialization
- storage commit/outbox/receipt crash matrix
- snapshot version retention/expiry/disk pressure
- Runtime graceful stop vs explicit host escalation
- descriptor-relative atomic no-replace export race
- Project/Channel membership CAS
- delegated Task sender/recipient/correlation
- Reference Provider and real Harness Host-path canary

## New Acceptance

- `AT-SUBMIT-001`: every crash point yields lookup/reissue with same effect or explicit recovery-required; duplicate effect 0.
- `AT-BOOT-001`: concurrent first init creates one Instance/principal/default policy generation set.
- `AT-STORAGE-001`: M1A atomic/snapshot/disk profile PASS.
- `AT-MEMBER-001`: Project/Channel membership set/remove/list is typed, CAS-safe and authorization-bound.
- `AT-DELEGATE-001`: delegated Task executes under recipient Bot identity without sender Scheduler/Core access.
- `AT-HARNESS-001`: real Adapter completes and fails one bounded Task through Provider Host.
- `AT-EXPORT-001`: destination creation race never overwrites and partial sensitive file remains inaccessible.
- `AT-PLAN-002`: active validator and built-in negative fixtures execute in PR/main.

## Review 1 — Structural

49 active Markdown, IDs, dependencies, links, package version, source status, command/input registry, Acceptance/Risk/Milestone 연결을 전체 검사한다.

## Review 2 — Cross-layer

```text
CLI parse → journal durable prepare → UDS peer auth → principal resolve
→ Application operation → storage atomic commit → Runtime/Provider Host
→ receipt/query/event → safe writer
```

각 crash window, ambiguous selector, stale revision, disk pressure, endpoint loss, host escalation, export race를 삽입한다.

## Review 3 — Adversarial

generic framework, duplicate registry, Reference Provider production fallback, client authority, global journal map, long-lived snapshot transaction, implicit host stop, P0 delete state가 없는지 검사한다.

발견 수정 후 각 Review는 전체 active package에서 재실행한다.
