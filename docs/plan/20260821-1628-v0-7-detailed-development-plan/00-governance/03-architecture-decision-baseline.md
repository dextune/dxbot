---
title: "초기 아키텍처 결정 기준선"
document_id: "DXB-GOV-003"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-GOV-002"]
---

# 초기 아키텍처 결정 기준선

## 1. 목적

ADR-0001~0073의 v0.6 의미를 유지하고, v0.7의 **Application Contract + CLI-only active Interface scope**, Runtime Host 독립성/bootstrap, protocol compatibility, CLI machine contract에 필요한 기본 결정을 추가한다. exact IPC, enum, numeric exit code, credential backend는 증거 없이 고정하지 않는다.

## 2. v0.6 결정 유지

Persistent Main Conversation, Thread/Session 분리, Scope-Aware Epistemic Memory, immutable Execution, Scheduler Core ownership, Provider Session non-ownership, Project/Channel Brain 금지, Durable Process replay boundary, Common Authorization Decision, ActionGrant, Collaboration termination, Runtime Memory envelope/reservation/pressure/staged recovery 의미를 모두 유지한다.

## 3. v0.7 신규 결정 후보

| ADR | 기본 결정 | 재검토 트리거 |
|---|---|---|
| ADR-0074 | v0.7 active product Interface는 Headless Application Contract + CLI로 제한하고 `DXB-IFC-042/043` 요구를 active baseline에서 supersede | 새 Interface 도입 버전 승인 |
| ADR-0075 | Runtime Host process lifetime과 CLI/shell/session lifetime을 분리하며 CLI disconnect/SIGINT/crash는 explicit command 없이 Runtime lifecycle을 변경하지 않음 | 비협상 |
| ADR-0076 | public control contract는 Command/Query/Subscription을 구분하고 Domain Event/internal queue/Provider contract와 혼합하지 않음 | 비협상 |
| ADR-0077 | local control transport는 semantic보다 하위 adapter다. principal resolution, byte bound, deadline/cancel, version negotiation, reconnect/backpressure, no Domain leakage를 만족해야 함 | platform/security benchmark가 구체 IPC 선택을 요구 |
| ADR-0078 | protocol/schema/runtime/client/data-schema version을 분리하고 incompatible pair는 explicit failure, silent semantic downgrade 금지 | 비협상 |
| ADR-0079 | CLI human output과 machine output을 분리하고 JSON/JSONL, stdout/stderr, stable error/exit class를 contract로 관리 | exact field/number는 compatibility evidence로 조정 가능 |
| ADR-0080 | CLI mutation은 CommandId/IdempotencyKey/expected revision을 사용하고 response-loss retry가 duplicate effect를 만들지 않음 | 비협상 |
| ADR-0081 | CLI credential은 command history/plain diagnostic 노출을 기본 UX로 사용하지 않고 Common Authorization owner에 귀결 | platform credential backend 결정 |
| ADR-0082 | CLI watch/follow/SIGINT/broken pipe는 local observation lifetime만 종료하며 target Task cancel은 explicit command로 분리 | 비협상 |
| ADR-0083 | Runtime이 미기동일 때만 narrow Host Lifecycle Adapter/Client가 OS service manager/launcher로 start/status/stop/readiness를 수행하며 Domain state 접근·mutation은 금지 | host/process architecture 변경 |

## 4. ADR로 반드시 닫을 항목

v0.6 미결정 항목에 더해 다음을 M1~M5 Gate 전에 닫는다.

- Application Contract command/query/subscription exact schema registry
- stable public error taxonomy와 retry disposition
- protocol/schema compatibility range/negotiation
- local control transport 및 platform-specific endpoint lifecycle
- Runtime daemon/service lifecycle과 Host Lifecycle Adapter 구현/권한 경계
- event cursor retention/resync window
- CLI JSON/JSONL schema compatibility policy
- CLI exit code numeric registry
- TTY confirmation/`--yes`/non-interactive behavior
- local principal/profile/credential storage 방식
- Runtime/CLI packaging/upgrade compatibility
- deterministic embedded/in-process test adapter 허용 범위

## 5. Architecture Decision Evidence

각 ADR은 Context/Decision/Alternatives, Canonical Owner, invariants, compatibility/removal, security, resource/memory/cache, concurrency/race, recovery, API/schema, Provider independence, deterministic fixture, benchmark/fault evidence, rollout/rollback을 포함한다.

Interface/Host ADR은 추가로 다음을 증명한다.
- CLI가 Domain/Runtime internal crate를 우회 참조하지 않음
- Host Lifecycle Client가 Domain fallback/mutator가 아님
- CLI process lifetime과 Runtime lifecycle 분리
- large response/watch boundedness
- old CLI/new Runtime 및 new CLI/compatible old Runtime matrix

## 6. 검증 기준

- ADR-0074~0083이 Acceptance/Risk/OQ 중 하나 이상에 연결된다. ADR-0083은 R-080/R-088, OQ-069, AT-CLI-001/002와 연결한다.
- TUI/Web 제거가 Backend semantic 또는 Headless Contract 제거로 해석되지 않는다.
- exact IPC/exit number/credential backend를 benchmark/security evidence 전에 Normative 필수로 고정하지 않는다.
- CLI convenience/bootstrap 구현이 v0.6 Canonical Owner를 복제하지 않는다.
