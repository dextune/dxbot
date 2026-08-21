---
title: "초기 아키텍처 결정 기준선"
document_id: "DXB-GOV-003"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-GOV-002"]
---

# 초기 아키텍처 결정 기준선

## 1. 목적

구현팀이 핵심 구조를 암묵적으로 재결정하지 않도록 기본 결정을 명시한다. 정식 검증 후 개별 ADR로 승격한다.

## 2. 기존 결정 유지

ADR-0001~ADR-0033의 v0.3 결정은 그대로 유지한다. 특히 Bot 중심 Session 비소유성, Lease Core, immutable Snapshot, bounded queue/cache, Side Effect write-ahead, Routine/Continuation, Provider Host/Lifecycle/Complete Contract/Thin Provider/Dependency Firewall/Reference+Conformance/Tier A-B 결정은 v0.4에서도 유효하다.

## 3. v0.4 신규 결정

| ADR | 기본 결정 | 재검토 트리거 |
|---|---|---|
| ADR-0034 | Bot당 하나의 Persistent Main Conversation | multi-conversation이 Bot Identity와 동등한 제품 요구로 증명됨 |
| ADR-0035 | 작업 문맥의 Canonical 독립 단위는 Thread이며 Interface/Provider Session과 분리 | Thread보다 다른 영속 owner가 필요하다는 증거 |
| ADR-0036 | Main Conversation은 Thread 0가 아니라 Communication + Supervisor Control Surface | 동일 의미를 단일 Thread로 단순화해도 owner/UX/recovery가 보존된다는 증거 |
| ADR-0037 | Conversation History와 Memory는 별도 Canonical 의미 | history와 knowledge retention이 완전히 동일하다는 제품 결정 |
| ADR-0038 | Thread-local Memory는 explicit promotion 없이 Bot-global로 승격 금지 | 제품 정책이 automatic global promotion을 명시적으로 채택 |
| ADR-0039 | Thread transcript 크기와 model Context 크기 분리, bounded Thread-aware Context Plan | 비협상 |
| ADR-0040 | 일반 Work Queue와 bounded Runtime Control Channel 분리 | 단일 queue가 control starvation 없음과 resource isolation을 증명 |
| ADR-0041 | Execution immutable + redirect는 Directive/Task revision/new Execution | 비협상 |
| ADR-0042 | Suspend/Resume는 durable checkpoint/Continuation + idempotent new Execution | 비협상 |
| ADR-0043 | Live interruption은 cooperative preemption/safe point, Hard Real-Time 미보장 | 실행 기반이 강한 transactional hard preemption을 제공 |
| ADR-0044 | Control request는 Scheduler Core Lease ownership을 침범하지 않음 | 비협상 |
| ADR-0045 | Provider Session은 Thread/Bot Canonical owner가 될 수 없음 | 비협상 |
| ADR-0046 | Thread branch/fork는 source mutation 없이 lineage + new Task/Execution | merge semantics가 별도 ADR로 확정될 때 세부 보강 |

## 4. ADR 필수 항목

Context/Decision/Alternatives, Classification/Owner, Invariants, Compatibility/Migration/Removal, Security/Resource/Failure, Concurrency/Race, Provider Host/SDK/Conformance 영향, Conversation/Thread/Control 영향, rollout/rollback, verification evidence를 포함한다.

## 5. Session / Thread 결정 상세

- Interface Session은 연결 수명이다.
- Provider Session은 optimization이다.
- Thread는 persistent Domain이다.
- Session connect/disconnect가 Thread revision을 생성하지 않는다.
- Provider Session loss가 Thread identity/Memory/Task를 손상시키지 않는다.

## 6. Immutable Redirect 결정 상세

running Execution의 Context Plan, Task spec revision, Provider binding을 mid-flight mutation하지 않는다. Redirect Command는 durable Directive와 새 Task Specification revision을 commit하고 old Execution을 safe yield한 뒤 새 Execution을 시작한다.

Provider native steering은 responsiveness optimization으로 사용할 수 있으나 Canonical revision을 대체하지 않는다.

## 7. Control Channel 결정 상세

Work Queue backlog와 무관한 bounded Control Channel을 둔다. inspect/report/cancel/safety control의 starvation 방지와 control flood의 일반 work starvation 방지를 모두 검증한다. 정확한 quota/precedence 값은 Policy/Open Question이 소유한다.

## 8. Core Ownership 결정 상세

“Core 추가/감소” user intent는 priority/parallelism/resource/rebalance hint로 변환한다. 실제 Lease 발급/회수는 Scheduler가 resource/fairness 정책으로 결정한다.

## 9. Migration 영향

v0.4 persistent schema는 Main Conversation/Thread/lineage/message scope, Thread Memory scope, Control Directive/Suspension metadata를 추가할 수 있다. v0.3의 기존 message/session/provider handle을 추측해 ThreadId로 승격하지 않는다. migration mapping이 불명확하면 explicit migration-required/legacy-unassigned scope를 사용한다.

## 10. 검증 기준

- 각 v0.4 ADR 후보가 Acceptance/Risk/Test에 연결된다.
- Session/Thread/Provider Session 용어가 문서/API에서 재혼합되지 않는다.
- redirect가 old Execution mutation 없이 복구 가능하다.
- control queue starvation/resource race가 deterministic test로 검증 가능하다.
- Core Lease ownership이 Control Plane로 이동하지 않는다.
