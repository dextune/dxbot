---
title: "통합 구현 로드맵과 단계별 Gate v0.8.5"
document_id: "DXB-DEL-060"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-BASE-000", "DXB-ARC-010", "DXB-ARC-018", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042", "DXB-ENG-052"]
---
# 통합 구현 로드맵과 단계별 Gate v0.8.5

## M0 — Baseline / Actual Validator

- v0.8.5 active package
- source nonnormative
- stale write workflow 제거
- read-only validator + negative self-test
- command/input registry 100%

Exit: `AT-BASE-001`, `AT-PLAN-002`.

## M1A — Storage Proof

구현 crate 대량 생성 전에 `DXB-ARC-018` spike를 수행한다. atomic mutation/receipt/outbox, crash, snapshot retention, disk-full, compaction을 증명한다.

Exit: `AT-STORAGE-001`. 실패 시 public persistence trait와 parser를 freeze하지 않는다.

## M1B — Kernel / Domain / Contract

typed IDs, Bot/Main Conversation/Task minimum, Operation Receipt, selector, schema metadata, Reference Provider를 구현한다. response-drop vertical slice를 통과한다.

Exit: `AT-APP-001/003/005`, `AT-SUBMIT-001` minimum.

## M2 — Instance / Bootstrap / Security

single instance/fencing, XDG/UDS, UID→Principal, default policy atomic initialization, ControlReady, terminal/safe writer foundation을 구현한다.

Exit: `AT-HOST-001`, `AT-BOOT-001`, endpoint security.

## M3 — Bot-only CLI

per-command journal fsync-before-send, create/send/submit/watch/result/operation lookup/restart를 end-to-end로 구현한다. parser freeze는 IFC-042 Gate 이후다.

Exit: `AT-CLI-001~010`, `AT-SUBMIT-001`.

## M4 — Multi-Bot / Project / Channel

Bot 2개, Project/Channel, typed membership set/remove/list, `task submit --delegate-to-bot`, recipient execution, result/evidence를 구현한다.

Exit: `AT-MEMBER-001`, `AT-DELEGATE-001`. Channel/Project가 Brain/Scheduler/Memory owner가 되어서는 안 된다.

## M5 — Recovery / Streaming / Real Harness

snapshot page, cursor, subscription gap/resync, safe export no-replace, side-effect reconcile와 one real Harness Adapter canary를 구현한다.

Exit: `AT-APP-006/007`, `AT-EXPORT-001`, `AT-HARNESS-001`.

## M6 — Freeze / Release

actual schema/architecture CI, compatibility, security/performance/soak, packaging/install smoke, Review 1/2/3 전체 재실행을 수행한다.

TUI/Web/Plugin lifecycle 전체 CLI는 비목표다.
