---
title: "구현 로드맵과 단계별 Gate"
document_id: "DXB-DEL-060"
version: "0.1.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-ARC-010", "DXB-ENG-052"]
---


# 구현 로드맵과 단계별 Gate

## 1. 목적

기능을 화면 중심이 아니라 제품 의미와 위험 순서대로 구현하고, 각 단계가 다음 단계의 전제 조건을 자동화된 검증으로 충족하게 한다.

## 2. 책임 범위

- milestone/epic
- 선후 의존성
- 단계별 산출물과 exit gate
- 원 컨셉 Phase와 상세 계획 매핑
- scope control
- 기술 부채와 실험 관리

기간 추정보다 **완료 조건과 의존성**을 기준으로 한다.

## 3. 원칙

- Runtime 의미를 먼저 검증한다.
- CLI는 초기부터 bootstrap interface로 존재하고 기능 완성도를 단계적으로 높인다.
- Dynamic Core 전 단일 Core 경로를 완성한다.
- Multi-Bot 전 단일 Bot의 복구·메모리·Task를 완성한다.
- Web은 Control API와 Projection이 안정된 후 시작한다.
- 분산 실행은 단일 노드의 Lease/Event/Idempotency가 검증된 후 진행한다.
- 각 milestone은 vertical slice와 failure path를 포함한다.
- 실험 Provider/최적화가 canonical path를 갈라놓지 않는다.

## 4. Milestone 개요

| Milestone | 핵심 결과 | Concept Phase |
|---|---|---|
| M0 Foundation & Harness Spike | 용어/계약/workspace/testkit/sidecar feasibility | Phase 1 |
| M1 Persistent Single Bot | 재시작 가능한 Bot + local CLI | Phase 2 + CLI bootstrap |
| M2 Memory/Task/Harness Loop | 기억·Task·단일 Core 실사용 | Phase 2/3 |
| M3 Dynamic Core | bounded 병렬 실행·merge·자원 관리 | Phase 3 |
| M4 Multi-Bot Network | 내구성 위임·메시지·협업 | Phase 4 |
| M5 Control API & TUI | 관리 평면과 Interactive terminal | Phase 5/6 |
| M6 Web Control Center | Bot ecosystem 운영 UI | Phase 7 |
| M7 Distributed/Production Hardening | 원격 Core, HA, multi-tenant 후보 | 확장 단계 |

## 5. M0 — Foundation & Harness Spike

### 목적
제품 의미와 기술 위험을 코드 골격·테스트로 고정한다.

### 구현
- Rust workspace와 crate dependency gate
- typed IDs/errors/clock/id generator
- Command/Event/State skeleton
- embedded storage spike
- deterministic testkit
- Harness Port + Fake
- DeepSeek sidecar protocol/lifecycle spike
- bounded channel/structured shutdown skeleton
- config/version/observability base
- bootstrap `runtime status`, `doctor`

### 제외
- 실제 Memory ranking
- 자동 병렬 Core
- Web/TUI
- Bot 간 위임

### Exit Gate
- Domain crate 무I/O 테스트
- Event atomic append/replay
- Fake Harness streaming/cancel
- sidecar start/stop/protocol proof 또는 명시적 대안 ADR
- crash 후 clean recovery spike
- architecture/lint/docs CI
- `ADR-0001~0010` 승인

## 6. M1 — Persistent Single Bot

### 구현
- Bot create/activate/deactivate
- Identity revisions
- Bot coordinator single writer
- minimal Bot state/Memory namespace
- local Control API
- CLI bot/runtime commands
- graceful shutdown
- backup/restore smoke
- session-independent access

### Vertical Slice
`dxb bot create → task/message 제출 → daemon 종료 → 재시작 → 동일 Bot 조회`

### Exit Gate
- BotId/Identity/Memory namespace 복원
- 마지막 CLI/Session 종료가 Bot 삭제를 유발하지 않음
- duplicate activation 차단
- quiescence 후 resource leak 0
- kill/restart fault test

## 7. M2 — Memory/Task/Harness Loop

### 구현
- Memory record/revision/provenance
- Working Memory와 Proposal/Commit
- exact/metadata recall
- Goal/Task/Execution 상태기계
- single Core Lease
- Context Plan/Brain Snapshot
- Fake + DeepSeek Adapter candidate
- Tool/Sandbox/Approval basic pipeline
- result/artifact/trajectory
- retry/cancel/idempotency
- CLI task/memory/trace

### Vertical Slice
장기 Memory를 가진 Bot이 Task를 수행하고 Tool 결과를 Artifact로 보존하며 검토된 Memory Proposal만 Commit한다.

### Exit Gate
- Working Memory 자동 장기 저장 금지
- Task retry가 별도 Execution attempts
- Harness Provider 교체 conformance
- secret/path/approval smoke
- event/trace/domain 분리
- 100k Memory metadata benchmark baseline

## 8. M3 — Dynamic Core

### 구현
- work item decomposition contract
- global/per-Bot/provider permits
- bounded queues/fairness
- 복수 Core Lease
- Task Finding/Result Fragment
- deterministic merge/conflict
- lease expiry/fencing
- adaptive fan-out 최소 정책
- memory/cost budgets
- Core inspect/watch/cancel CLI

### Vertical Slice
한 Bot이 독립된 4개 work item을 병렬 실행하고 충돌 없는 결과는 병합하며 stale Core write를 거부한다.

### Exit Gate
- Bot 생성에 core count 없음
- max limit/fairness/permit leak tests
- cancellation/expiry/late result race
- Core context isolation
- 병렬 속도 개선이 overhead를 초과하는 benchmark
- memory hard limit under load

## 9. M4 — Multi-Bot Network

### 구현
- durable Message Envelope
- inbox/outbox/dedup
- Ask/Request/Result/Delegate
- Target Task 생성
- correlation/delegation graph
- capability discovery 최소형
- delegation token/permission
- cycle/fan-out/deadline
- message CLI

### Vertical Slice
Main Bot이 Developer Bot에 Task를 위임하고 Developer Bot이 결과 Artifact를 반환하며 양쪽 timeline이 연결된다.

### Exit Gate
- at-least-once duplicate safety
- A→B→A cycle 차단
- permission non-inheritance
- reply loss/reconciliation
- Bot Memory 자동 공유 금지
- target unavailable/timeout/cancel matrix

## 10. M5 — Control API & TUI

### 구현
- complete local/remote Control API
- Query Projection/watermark
- Event Stream/cursor
- operation/approval/audit
- TUI Bot/Task/Core/Memory/Network/Health
- remote daemon mode
- config reload subset
- doctor/repair

### Exit Gate
- CLI/TUI shared client/schema
- reconnect/gap/duplicate correctness
- UI termination independence
- projection stale 표시
- destructive action audit
- large list/event flood memory bound

## 11. M6 — Web Control Center

### 구현
- Overview/Bot/Task/Core
- Memory and Network views
- approval/audit/admin
- graph neighborhood
- real-time event
- auth/session/security
- frontend/backend compatibility

### Exit Gate
- DB/Runtime direct access 없음
- XSS/permission revoke/cache tests
- large graph/list performance
- version mismatch guard
- Control UX에서 Bot/Core/Session 구분
- accessibility baseline

## 12. M7 — Distributed/Production Hardening

### 구현 후보
- remote Core Executor
- node ownership lease/fencing
- external DB/Artifact/Index
- multi-node event/command routing
- rolling upgrade
- disaster recovery
- multi-tenant isolation
- signed plugin/grant

### Entry Gate
- 단일 노드 P0/P1 invariant와 soak 통과
- 실제 병목/격리 요구의 계측
- 분산 ADR와 failure model
- 운영 인력/관측 준비

### Exit Gate
- node loss/network partition/duplicate delivery
- no split-brain Bot writer
- rolling migration
- tenant isolation
- remote late result fencing
- backup/restore rehearsal

## 13. Cross-Cutting Workstream

각 milestone에서 지속:
- docs/ADR/traceability
- security threat model
- performance/RSS/allocation
- dependency/license/SBOM
- migration fixtures
- fault injection
- observability
- cleanup of temporary shims
- UX terminology consistency

## 14. 예외상황과 Scope Control

- DeepSeek sidecar가 불안정하면 Fake+native minimal Harness로 M1/M2를 진행하되 Adapter contract는 유지한다.
- Web 요구가 앞당겨져도 read-only prototype만 허용하고 Domain write 로직을 넣지 않는다.
- 분산 요구가 조기 발생하면 remote Core만 제한적으로 실험하고 Bot ownership/DB microservice 분해를 동시에 하지 않는다.
- 성능 목표 미달은 profile 없이 새로운 cache/unsafe를 도입할 근거가 아니다.
- milestone gate를 feature flag로 우회할 경우 release 기본 profile에서 비활성화한다.

## 15. 검증 기준

- 각 milestone이 독립 실행 가능한 vertical slice와 failure test를 가진다.
- 다음 milestone 착수 전에 선행 exit gate가 CI에서 green이다.
- 원 컨셉 7개 Phase가 누락 없이 mapping된다.
- CLI bootstrap이 초기부터 존재하되 Web/TUI가 Runtime 구현을 대신하지 않는다.
- M7 진입은 계측된 필요와 분산 ADR 없이 허용되지 않는다.
