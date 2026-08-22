---
title: "Goal·Task·Execution 모델"
document_id: "DXB-DOM-023"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-020", "DXB-ARC-014"]
---

# Goal·Task·Execution 모델

## 1. 목적

사용자 의도, 장기 목표, 수행 단위, 실제 시도를 분리해 retry·control·crash recovery가 duplicate semantic effect나 identity drift를 만들지 않게 한다.

## 2. 구분

```text
Goal      = 장기적 desired outcome와 policy
Task      = durable work intent와 lifecycle
Execution = Task를 수행하는 하나의 immutable attempt snapshot
Core Lease = Execution에 일시 배정된 Scheduler resource
```

Thread/Message/Process/Provider Session은 Task나 Execution이 아니다.

## 3. Task Aggregate

최소 상태:

```text
TaskId / TaskRevision
Owner Bot / ScopeRef
Intent / Input refs
Priority / Deadline / Budget refs
LifecycleState
CurrentExecutionRef?
SupervisorRef?
Result/Artifact/Evidence refs
ControlDirective refs
CreatedAt / UpdatedAt
```

상태 예:

```text
Submitted → Admitted → Running
Submitted/Admitted → Deferred | Rejected | Cancelled
Running → AwaitingSafePoint | Suspended | Completed | Failed | Cancelled | RecoveryRequired
Suspended → Resuming → Running
```

`cancel requested`와 terminal `Cancelled`를 구분한다.

## 4. Execution

Execution은 TaskRevision, ContextPlan, Provider generation, Capability set, permission/grant, budget/deadline를 pin한다. retry는 동일 Execution을 rewind하지 않고 정책상 새 Execution/Attempt identity를 생성한다. stale generation late result는 fencing한다.

## 5. Side Effect

외부 변경은 intent/action digest/idempotency key, before/after boundary, confirmed/failed/unknown, reconciliation 상태를 durable ledger에 기록한다. `Unknown`을 transient failure로 축약하거나 blind retry하지 않는다.

## 6. Control

report, redirect, suspend, resume, cancel은 `DXB-RUN-036` typed directive를 사용한다. 안전 지점이 필요한 operation은 receipt를 `Pending/AwaitingSafePoint`로 유지한다. CLI timeout/SIGINT는 local observation만 끝내며 Task state를 변경하지 않는다.

## 7. Result와 Memory

Task result는 Artifact/Evidence reference로 commit하고 Memory write와 분리한다. Memory 반영은 proposal/validation/promotion을 거친다. partial provider output을 completed result로 오판하지 않는다.

## 8. Concurrency와 Recovery

- completion vs cancel은 revision/fencing으로 winner를 결정하고 terminal decision은 하나다.
- Runtime crash 후 journal/process/side-effect state로 reconcile한다.
- response loss는 Operation Receipt로 기존 TaskId/outcome을 회수한다.
- duplicate submit key가 새 Task를 생성하지 않는다.

## 9. 검증 기준

- `ThreadId`, `ExecutionId`, `CoreLeaseId`, `ProviderSessionId`를 TaskId로 재사용하지 않는다.
- same idempotency key/digest retry에서 Task 생성 1회.
- completion/cancel race에서 terminal state 1개.
- stale Execution/provider generation의 late write 0.
- Task result commit과 Memory promotion이 독립적으로 추적된다.
