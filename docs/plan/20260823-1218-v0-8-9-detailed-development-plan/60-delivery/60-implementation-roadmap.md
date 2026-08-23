---
title: "구현 로드맵과 누적 Milestone Gate v0.8.9"
document_id: "DXB-DEL-060"
version: "0.8.9"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-BASE-000", "DXB-ARC-010", "DXB-ARC-018", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042", "DXB-ENG-052"]
---
# 구현 로드맵과 누적 Milestone Gate v0.8.9

Gate는 누적된다. `Mx` subset을 freeze하려면 M0A부터 Mx까지의 모든 Acceptance가 실제 evidence로 PASS해야 한다. 아래 registry는 각 Acceptance가 **처음 요구되는 milestone**만 한 번 기록한다.

<!-- milestone-exit-registry:start -->
- `M0A` | `AT-BASE-001,AT-PLAN-002,AT-DOC-CONTRACT-001`
- `M1A` | `AT-STORAGE-001,AT-IDEMP-001`
- `M1B` | `AT-CONTRACT-001,AT-SUBMIT-001,AT-JOURNAL-001,AT-APP-005`
- `M2` | `AT-BOOT-001,AT-SECSTATE-001,AT-AUDIT-001,AT-HOST-001,AT-SEC-005`
- `M3` | `AT-VERSION-001,AT-CLI-CORE-001,AT-APP-006,AT-CLI-009,AT-CLI-CONFIRM-001,AT-EXPORT-001`
- `M4` | `AT-MEMBER-001,AT-DELEGATE-001`
- `M5` | `AT-CLI-010,AT-HARNESS-001,AT-APP-007`
- `M6` | `AT-SCHEMA-001`
<!-- milestone-exit-registry:end -->

## M0A — 문서·검증 기준선

active package self-contained baseline, ADR registry, CLI dual projection, command/input/Acceptance/milestone relation과 negative self-test를 닫는다. 제품 runtime 구현 PASS를 주장하지 않는다.

Exit: M0A registry 전부 PASS.

## M1A — Storage·Idempotency spike

DB 제품을 freeze하지 않고 atomic state/Receipt/two binding indexes, Instance-global CommandId, principal key, tombstone compaction, snapshot, disk pressure를 failpoint로 증명한다.

Exit: M0A 누적 PASS + M1A registry PASS. public persistence trait/DTO/parser freeze 금지.

## M1B — Contract source·Submission prototype

하나의 metadata source에서 `CliInput`과 `CommandPayload`, parser/help/schema/output을 생성한다. durable `Prepared/Dispatching`, single-writer journal, binding lookup recovery, mutation Receipt semantics를 executable fixture로 닫는다.

Exit: M0A~M1B 누적 PASS. 내부 prototype만 허용하며 public CLI subset freeze 금지.

## M2 — Security·Host 최소 subset

Runtime host start/status/graceful shutdown/explicit host stop, approval list/show/approve/deny, atomic bootstrap, Principal/Authority/Approval/Audit, endpoint·terminal fail-closed를 구현한다.

Exit: M0A~M2 누적 PASS 후 `DXB-IFC-041` M2 command subset만 freeze.

## M3 — Bot-only CLI

version compatibility, Bot/Conversation/Thread/Task/Operation의 Bot-only path, selector ambiguity, bounded page/`--all`, safe output, destructive confirmation과 non-TTY behavior를 구현한다.

Exit: M0A~M3 누적 PASS 후 M3 subset만 freeze. Project/Channel/Multi-Bot과 Task Directive controls는 포함하지 않는다.

## M4 — Collaboration

Project/Channel lifecycle, membership/Authority atomicity, typed delegation을 기존 Application/Domain 경계 위에 추가한다.

Exit: M0A~M4 누적 PASS 후 M4 subset만 freeze.

## M5 — Streaming·Memory·Process·Provider

Task/Process watch, Task Directive controls와 `applied` observation, Memory, side-effect reconcile, provider query, real Harness canary를 추가한다. M3에서 이미 증명한 pagination/safe output/security/confirmation semantics를 재정의하지 않고 재사용한다.

Exit: M0A~M5 누적 PASS 후 M5 subset만 freeze.

## M6 — Full contract freeze

전체 63 operation generated parser/help/schema/error/exit snapshot, compatibility/migration/performance/security regression을 닫는다.

Exit: 모든 Acceptance PASS + generated snapshot drift 0. 그 전에는 full CLI surface freeze를 금지한다.
