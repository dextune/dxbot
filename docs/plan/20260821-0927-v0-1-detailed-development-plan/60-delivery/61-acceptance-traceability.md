---
title: "수용 기준과 원칙 추적성"
document_id: "DXB-DEL-061"
version: "0.1.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-DEL-060", "DXB-ENG-052"]
---


# 수용 기준과 원칙 추적성

## 1. 목적

최상위 컨셉 원칙이 설계 문서, 구현 단위, 자동화 테스트로 끊김 없이 연결되도록 한다.

## 2. 책임 범위

- 제품 원칙 PR
- 기능/비기능 요구
- 소유 문서/crate
- acceptance test ID
- release gate
- 증거 저장 위치

실제 issue/commit 링크는 개발 저장소 생성 후 추가한다.

## 3. 최상위 원칙 추적

| ID | 원칙 | 소유 문서 | 구현 소유 | 수용 테스트 |
|---|---|---|---|---|
| PR-01 | Session 기반 Agent가 아님 | BASE, DOM-020 | domain/runtime | AT-BOT-001 |
| PR-02 | Bot 본체는 Identity+Memory+State | DOM-020/022 | domain/storage | AT-BOT-002 |
| PR-03 | Bot 장기 지속 | DOM-020, ARC-015 | runtime/storage | AT-BOT-003 |
| PR-04 | Bot당 하나의 Brain | DOM-021 | domain/runtime | AT-BRAIN-001 |
| PR-05 | 여러 Core로 동시 관여 | DOM-024 | runtime | AT-CORE-001 |
| PR-06 | Core는 복제 Bot이 아님 | GOV-002, DOM-024 | domain | AT-CORE-002 |
| PR-07 | 생성 시 Core 수 미지정 | DOM-024, IFC-040 | API/runtime | AT-CORE-003 |
| PR-08 | Runtime 동적 Core | DOM-024 | scheduler | AT-CORE-004 |
| PR-09 | 최대 Core만 정책 제한 | RUN-031 | scheduler | AT-CORE-005 |
| PR-10 | Brain/Memory 공유, Context 분리 | DOM-021/022 | runtime/memory | AT-CTX-001 |
| PR-11 | Bot 간 호출·협업 | DOM-025 | botnet | AT-NET-001 |
| PR-12 | 중앙 Control Plane | DOM-026 | control | AT-CTRL-001 |
| PR-13 | DeepSeek-inspired Harness | ARC-013 | harness | AT-HAR-001 |
| PR-14 | Rust 중심 Runtime | ARC-011, ENG-050 | workspace | AT-ARC-001 |
| PR-15 | CLI→TUI→Web | IFC-041/042/043 | interfaces | AT-IFC-001 |
| PR-16 | UI와 Runtime 분리 | ARC-010, IFC-040 | control/interfaces | AT-IFC-002 |

## 4. P0 기능 요구

| ID | 요구 | 검증 |
|---|---|---|
| FR-BOT-001 | Bot create/load/activate/deactivate | AT-BOT-003 |
| FR-BOT-002 | Identity revision | AT-BOT-004 |
| FR-MEM-001 | Memory record/revision/provenance | AT-MEM-001 |
| FR-MEM-002 | Working Memory 분리/승격 | AT-MEM-002 |
| FR-TASK-001 | Task/Execution 분리 | AT-TASK-001 |
| FR-TASK-002 | retry/cancel/idempotency | AT-TASK-002 |
| FR-CORE-001 | Core Lease와 max limit | AT-CORE-005 |
| FR-HAR-001 | Fake/External Adapter 교체 | AT-HAR-001 |
| FR-STO-001 | Journal/state/outbox atomic | AT-STO-001 |
| FR-SEC-001 | default deny/approval/sandbox | AT-SEC-001 |
| FR-CLI-001 | UI 없는 핵심 기능 | AT-IFC-001 |
| FR-REC-001 | crash/restart recovery | AT-REC-001 |
| FR-OBS-001 | end-to-end correlation | AT-OBS-001 |

## 5. 비기능 요구

| ID | 요구 | 기준 |
|---|---|---|
| NFR-CON-001 | 동시성 안전 | race/model tests 통과 |
| NFR-MEM-001 | bounded memory | unbounded 0, RSS budget |
| NFR-PERF-001 | Runtime overhead | ENG-051 예산 |
| NFR-REL-001 | committed data 유실 없음 | crash injection |
| NFR-SEC-001 | secret/path/permission | security gate |
| NFR-MNT-001 | 중복 기능 없음 | architecture/duplication review |
| NFR-EXT-001 | Provider 교체 가능 | conformance suite |
| NFR-OBS-001 | 추적 가능 | correlation + audit |
| NFR-CMP-001 | migration/compatibility | fixture tests |
| NFR-UI-001 | interface independence | dependency/API tests |

## 6. 수용 테스트 정의

### AT-BOT-001 — Session 독립
1. Bot 생성
2. 두 CLI/TUI session 연결
3. Memory/Task 생성
4. 모두 종료
5. daemon 재시작
6. 같은 BotId/Memory/Task 확인

### AT-BOT-003 — Persistent Restore
commit 직후 여러 failpoint에서 process kill 후 Identity revision, event sequence, Memory digest를 검증한다.

### AT-BRAIN-001 — Single Brain Semantics
한 Bot에서 두 Core가 같은 Identity/Policy revision을 사용하고 별도 Working Context를 유지함을 확인한다.

### AT-CORE-002 — Core 비정체성
Core 생성/종료가 Bot count/Identity/Memory namespace를 바꾸지 않고 Core에 Persona/long-term store가 없음을 검사한다.

### AT-CORE-005 — Dynamic Limit
Bot 생성에 core count가 없고 workload에 따라 1→N→0으로 변하며 global/per-Bot 상한을 넘지 않는다.

### AT-CTX-001 — Shared/Isolated
공통 Memory revision은 두 Core가 읽지만 Core A scratch data는 explicit finding 전 Core B에 나타나지 않는다.

### AT-MEM-002 — Promotion
Core 종료 전 Working Memory 후보를 만들고 승인되지 않은 후보가 long-term store에 존재하지 않음을 검증한다.

### AT-TASK-002 — Retry/Cancel
세 번 retry가 세 Execution을 만들고 cancellation/completion race에서 한 terminal Task만 Commit된다.

### AT-NET-001 — Durable Delegation
Message duplicate/loss/restart를 주입해 target Task 하나, result 하나, 양방향 correlation을 검증한다.

### AT-CTRL-001 — Control Plane
CLI/TUI/Web client가 동일 Command를 보내 동일 Event를 만들고 DB direct dependency가 없음을 검사한다.

### AT-HAR-001 — Harness Replaceability
Fake와 DeepSeek Adapter에 동일 scripted scenario/conformance를 실행하고 Domain outcome schema를 비교한다.

### AT-IFC-001 — CLI Completeness
Bot/Task/Memory/Core/Message/Runtime P0 기능을 headless CLI로 수행한다.

### AT-IFC-002 — Interface Separation
UI process를 kill해도 Bot/Task/Core가 정책대로 계속 실행되고 storage/runtime crate 의존성이 없다.

### AT-STO-001 — Atomic Commit
event/state/outbox 중간 failpoint에서 전부 commit 또는 전부 rollback됨을 확인한다.

### AT-SEC-001 — Least Privilege
untrusted content가 secret/path/network Tool을 지시해도 approval/grant 없이 실행되지 않는다.

### AT-REC-001 — Recovery
Core/Harness/DB/sidecar kill 후 lease/outbox/checkpoint를 reconcile하고 permit/orphan이 남지 않는다.

### AT-OBS-001 — Traceability
CommandId에서 Bot→Task→Execution→Core→Harness→Tool→Memory Proposal까지 조회한다.

## 7. 증거 형식

각 테스트 결과는:
- test ID
- source commit/build
- config/schema/provider versions
- test seed/workload
- pass/fail
- metrics
- logs/trace/artifact digest
- platform
- exception/waiver
를 보존한다.

## 8. 예외상황

- 모델 E2E가 비결정적이어도 Runtime invariant 테스트를 대체할 수 없다.
- 수동 UI 검증만으로 P0 원칙을 닫지 않는다.
- 문서 ID가 바뀌면 추적성 표를 같은 변경에서 갱신한다.
- 테스트 skip은 성공이 아니며 capability/environment reason을 표시한다.
- waiver는 owner/expiry/위험을 가진다.

## 9. 확장성

P1/P2 요구는 같은 ID 체계를 사용한다. plugin/provider certification과 multi-tenant 요구는 별도 trace matrix로 확장한다. 자동 도구가 문서 ID, test annotation, CI result를 연결한다.

## 10. 구현 우선순위

- **P0:** 위 17개 핵심 AT 자동화
- **P1:** Multi-Bot/TUI/security/performance 확장
- **P2:** Web/distributed/multi-tenant
- **P3:** 운영 SLO와 certification

## 11. 검증 기준

- 모든 PR-01~16이 최소 하나의 자동 테스트를 가진다.
- P0 FR/NFR에 소유 문서와 crate가 비어 있지 않다.
- CI report에서 요구 ID별 상태를 조회할 수 있다.
- Accepted 문서 변경 시 영향 테스트 목록이 자동 계산되거나 명시된다.
