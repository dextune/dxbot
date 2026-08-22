---
title: "Goal·Task·Execution·Side Effect 모델"
document_id: "DXB-DOM-023"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-DOM-020", "DXB-ARC-014"]
---
# Goal·Task·Execution·Side Effect 모델

## 1. P0 구분

```text
Goal      = Task intent에 포함되는 desired-outcome value object; 독립 P0 aggregate/CLI가 아님
Task      = durable work intent와 lifecycle
Execution = immutable Task revision을 수행하는 attempt
CoreLease = Execution에 임시 배정된 Scheduler resource
```

## 2. Task와 Execution

Task는 Owner Bot/Scope, Intent/Input refs, priority/deadline/budget, lifecycle, current Execution ref, result/evidence refs를 소유한다.

```text
Submitted → Admitted → Running
Submitted/Admitted → Deferred | Rejected | Cancelled
Running → Suspended | Completed | Failed | Cancelled | RecoveryRequired
Suspended → Resuming → Running
```

`cancel requested`는 Directive 상태이고 terminal `Cancelled`와 다르다. Execution은 TaskRevision, ContextPlan, Provider generation, Capability, grant, budget/deadline를 pin한다. retry는 새 Execution identity를 만든다.

## 3. Side Effect Ledger

Canonical key는 `SideEffectId + revision`이다.

```text
Prepared → Dispatched → Confirmed | Failed | Unknown
Unknown → Reconciling → Confirmed | Failed | ManualResolutionRequired
```

최소 field:

```text
action digest / target ref / idempotency binding
before-boundary / after-boundary evidence
provider/activity generation
state / outcome ref / reconciliation policy
```

`Unknown`을 transient failure로 축약하거나 blind retry하지 않는다. `side-effect reconcile`은 이 aggregate만 변경하며 Task/Memory를 직접 rewrite하지 않는다.

## 4. Archive/revoke default

Bot/Project/Channel archive 또는 authority revoke는 신규 admission을 차단한다. 이미 committed Side Effect는 reconcile하고, active Task는 owner policy가 drain/cancel directive를 생성한다. archive command가 child terminal state를 직접 쓰지 않는다.

## 5. Result와 Memory

Task result는 Artifact/Evidence reference로 commit하고 Memory write와 분리한다. Memory 반영은 proposal/validation/promotion을 거친다.
