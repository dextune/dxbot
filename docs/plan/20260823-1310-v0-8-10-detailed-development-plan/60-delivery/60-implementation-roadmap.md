---
title: "구현 로드맵과 누적 Milestone Gate v0.8.10"
document_id: "DXB-DEL-060"
version: "0.8.10"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-BASE-000", "DXB-ARC-010", "DXB-ARC-018", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042", "DXB-IFC-043", "DXB-ENG-052"]
---
# 구현 로드맵과 누적 Milestone Gate v0.8.10

Gate는 누적된다. `Mx` subset을 freeze하려면 M0A부터 Mx까지의 모든 Acceptance가 실제 evidence로 PASS해야 한다. 아래 registry는 각 Acceptance가 처음 요구되는 milestone만 한 번 기록한다.

<!-- milestone-exit-registry:start -->
- `M0A` | `AT-BASE-001,AT-PLAN-002,AT-DOC-CONTRACT-001`
- `M1A` | `AT-STORAGE-001,AT-IDEMP-001`
- `M1B` | `AT-CONTRACT-001,AT-SUBMIT-001,AT-JOURNAL-001,AT-APP-005`
- `M2` | `AT-BOOT-001,AT-SECSTATE-001,AT-AUDIT-001,AT-HOST-001,AT-SEC-005,AT-CLI-DISCOVERY-001`
- `M3` | `AT-VERSION-001,AT-CLI-CORE-001,AT-APP-006,AT-CLI-009,AT-CLI-CONFIRM-001,AT-EXPORT-001,AT-CLI-INTERACTIVE-001,AT-CLI-AUTOMATION-001,AT-CLI-RECOVERY-001`
- `M4` | `AT-MEMBER-001,AT-DELEGATE-001`
- `M5` | `AT-CLI-010,AT-HARNESS-001,AT-APP-007`
- `M6` | `AT-SCHEMA-001`
<!-- milestone-exit-registry:end -->

## M0A — 문서·검증 기준선

active package self-contained baseline, CLI dual projection, user journey owner, command/input/Acceptance/milestone relation과 negative self-test를 닫는다. 제품 Runtime 구현 PASS를 주장하지 않는다.

## M1A — Storage·Idempotency spike

DB 제품을 freeze하지 않고 atomic state/Receipt/two binding indexes, Instance-global CommandId, principal key, tombstone compaction, snapshot, disk pressure를 failpoint로 증명한다.

## M1B — Contract source·Submission prototype

하나의 metadata source에서 parser/help, user selector preflight, CliInput/CommandPayload, output/error schema를 생성한다. durable journal과 binding recovery를 executable fixture로 닫는다. internal prototype만 허용한다.

## M2 — Security·Host·Discovery 최소 subset

Runtime host lifecycle, atomic bootstrap, Principal/Authority/Approval/Audit와 endpoint security에 더해 local help/version, first-run Instance selection, no silent fallback/auto-start와 provider doctor 경로를 구현한다.

Exit: M0A~M2 누적 PASS 후 M2 subset만 freeze.

## M3 — Bot-only 사용자 CLI

Bot/Conversation/Thread/Task/Operation Bot-only path, user selector→CAS preflight, first useful path, ambiguity, human/json output, non-TTY, actionable error, journal 재진입, bounded page/`--all`, safe output, destructive confirmation을 구현한다.

Exit: M0A~M3 누적 PASS 후 M3 subset만 freeze.

## M4 — Collaboration

Project/Channel lifecycle, user selector 기반 membership/Channel send, Authority atomicity와 typed delegation을 기존 경계 위에 추가한다.

## M5 — Streaming·Memory·Process·Provider

Task/Process watch, Task Directive controls, Memory, side-effect reconcile, provider query와 real Harness canary를 추가한다. M3 user experience 계약을 재정의하지 않고 재사용한다.

## M6 — Full contract freeze

전체 63 operation generated parser/help/preflight/schema/error/exit/journey snapshot과 compatibility/migration/performance/security regression을 닫는다.

각 단계 Exit는 해당 registry와 모든 선행 Acceptance PASS다. 그 전에는 subset/full freeze를 금지한다.
