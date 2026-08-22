---
title: "통합 구현 로드맵과 단계별 Gate v0.8.6"
document_id: "DXB-DEL-060"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-BASE-000", "DXB-ARC-010", "DXB-ARC-018", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042", "DXB-ENG-052"]
---
# 통합 구현 로드맵과 단계별 Gate v0.8.6

## M0A — Semantic Closure / Validator

- v0.8.6 active package
- digest/receipt/wait/error/exit/output contract
- security and side-effect canonical state
- typed 63-operation metadata/input snapshot
- 5 adversarial reviews + 2 rechecks
- actual read-only validator

Exit: `AT-BASE-001`, `AT-PLAN-002`, document-level `AT-CONTRACT-001` executable.

## M1A — Storage Proof

crate 대량 생성 전에 atomic mutation/receipt/outbox/audit-intent, key horizon, crash, snapshot retention, disk-full, compaction을 증명한다.

Exit: `AT-STORAGE-001`, `AT-IDEMP-001`. 실패 시 public persistence trait와 parser를 freeze하지 않는다.

## M1B — Kernel / Domain / Contract Source

typed IDs, Bot/Main Conversation/Task/SideEffect minimum, Receipt/Directive, selector, error/exit/output schema, operation metadata generator, Reference Provider를 구현한다.

Exit: `AT-CONTRACT-001`, `AT-SUBMIT-001`, `AT-JOURNAL-001`.

## M2 — Instance / Bootstrap / Security

single instance/fencing, XDG/UDS, UID→Principal, default policies, AuthorityBinding/Approval/ActionGrant, durable audit, ControlReady를 구현한다.

Exit: `AT-HOST-001`, `AT-BOOT-001`, `AT-SECSTATE-001`, `AT-AUDIT-001`.

## M3 — Bot-only CLI

create/send/submit/watch/result/operation lookup/restart vertical slice를 구현한다. 전체 parser freeze는 M1A/M1B/M2 required fixture가 PASS한 뒤다.

## M4 — Multi-Bot / Project / Channel

typed membership CAS와 recipient execution delegation을 구현한다. Channel/Project가 Brain/Scheduler/Memory owner가 되지 않아야 한다.

## M5 — Recovery / Streaming / Real Harness

snapshot/cursor/subscription gap, safe export, SideEffect reconcile, real Harness success/failure/cancel canary를 구현한다.

## M6 — Compatibility / Security / Performance Freeze

compatibility/retention 숫자 ADR, schema freeze, soak, packaging/install smoke, 5회 review와 2회 recheck를 실행한다.

TUI/Web/Plugin lifecycle 전체 CLI는 비목표다.
