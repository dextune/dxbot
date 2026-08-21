---
title: "Durable Cross-Aggregate Process Orchestration"
document_id: "DXB-RUN-038"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-015", "DXB-DOM-023", "DXB-DOM-025", "DXB-RUN-030", "DXB-RUN-031", "DXB-RUN-032", "DXB-RUN-033", "DXB-RUN-036"]
---

# Durable Cross-Aggregate Process Orchestration

## 1. 목적

여러 Canonical Owner를 횡단하는 장기 작업이 crash/retry/restart/duplicate delivery 이후에도 정확히 이어지도록 **진행 상태만을 소유하는 Durable Process Runtime**을 정의한다.

Durable Process는 새 Brain/Agent/Task engine이 아니다. 단일 Aggregate transaction 또는 기존 Task/Execution만으로 충분한 작업을 모두 Process로 감싸지 않는다.

## 2. 적용 기준

Durable Process 사용 조건:
1. 둘 이상의 Canonical Owner를 횡단한다.
2. 여러 command/outcome 사이 진행 상태가 있다.
3. crash 후 어느 step까지 committed 되었는지 복원해야 한다.
4. duplicate delivery/partial success/late outcome을 reconcile해야 한다.

예:

```text
Manager Task
→ Delegation Message
→ Researcher Task
→ Parent Waiting
→ Result
→ Reviewer Validation
→ Memory Promotion
→ Project Memory Commit
```

단일 Task/Execution, 단일 Memory commit, 단일 Project mutation은 기본적으로 Process가 필요하지 않다.

## 3. Canonical Owner

본 문서가 단독 소유한다.
- Process identity/lifecycle
- ProcessDefinitionVersion
- committed step/progress
- participating Aggregate refs
- pending/issued semantic command refs
- Activity/outcome refs
- retry/reconciliation state
- total budget/deadline refs
- terminal reason
- replay boundary
- Process revision

소유하지 않는다.
- Task/Execution state
- Channel Membership/Role/Authority
- Memory Record/Assertion state
- Control Directive
- Core Lease
- Provider lifecycle/session
- Side Effect Ledger
- Runtime MemoryReservation

## 4. 최소 논리 모델

```text
ProcessId
ProcessDefinitionVersion
RootIntentRef / RootMessageRef
Correlation / Causation
CurrentStepRef
ParticipantAggregateRefs
CommittedOutcomeRefs
PendingCommandRefs
ActivityRefs
BudgetRef / Deadline
Recovery / Reconciliation State
TerminalReason?
Revision
```

정확한 enum/DB schema/engine은 ADR 대상이다.

## 5. Command Boundary

Process는 다른 Aggregate를 직접 mutate하지 않는다.

```text
Process step decision
→ existing Application/Command boundary
→ typed Command
→ target Aggregate decides/commits
→ durable outcome/event ref
→ Process observes ref
→ next deterministic transition
```

같은 logical step이 replay되어도 `ProcessId + DefinitionVersion + StepRef + semantic command/causation key` 또는 동등한 stable key로 duplicate child/mutation을 방지한다.

## 6. Replay-Safe 영역

허용:
- committed Event/State 읽기
- recorded Activity outcome 재사용
- idempotency/revision/fencing 검사
- pending command reconciliation
- deterministic next-step 계산

Replay 중 직접 실행 금지:
- LLM call
- Tool execution
- external HTTP/API
- file/external DB side effect
- Provider-specific non-deterministic operation
- wall clock/random에 직접 의존하는 decision

시간/랜덤이 필요하면 입력/결과를 durable하게 캡처하는 별도 outcome boundary를 사용한다.

## 7. Activity Boundary

Activity는 새 Agent/Core/일반 Aggregate가 아니다.

우선 재사용:
- Execution result/outcome
- Provider Host call result
- Tool execution result
- Side Effect Intent/Ledger/Reconciliation

별도 Activity entity는 기존 owner로 표현할 수 없는 orchestration metadata가 실제로 필요한 경우에만 ADR로 도입한다.

Activity outcome은 replay에서 재사용할 수 있는 durable ref/digest/status를 가져야 하며 large payload 자체는 Artifact/reference로 분리할 수 있다.

## 8. Delivery / Exactly-Once

- transport/message delivery는 at-least-once일 수 있다.
- duplicate delivery는 idempotency + expected revision + durable receipt로 semantic duplicate를 제거한다.
- DXBOT 전체에 global exactly-once execution을 약속하지 않는다.
- non-idempotent external effect 정확성은 기존 Side Effect Ledger/Intent/Reconciliation을 재사용한다.
- Process가 Side Effect safety를 우회하지 않는다.

## 9. Waiting / Retry / Reconciliation

Process가 child result를 기다리는 상태는 active Core/Provider Context를 보유하는 실행 상태가 아니다.
- durable pending refs/continuation만 보존
- polling/notification queue는 bounded
- retry는 semantic command가 적용되었는지 먼저 reconcile
- unknown external effect는 재실행보다 Side Effect reconciliation 우선
- Runtime Memory reservation은 기다리는 동안 보유하지 않고 필요 시 재-admission

## 10. Cancellation / Terminal Isolation

- Process cancel/timeout은 신규 step admission을 중단한다.
- already-running child Task/Execution/Side Effect는 explicit Task/Live Control을 통해 중지한다.
- Process terminal state가 child terminal state를 덮어쓰지 않는다.
- late outcome은 audit/reconcile할 수 있으나 Process를 implicit resume하지 않는다.

최소 terminal 의미:
- Succeeded
- Partial
- Failed
- Cancelled
- TimedOut
- AuthorizationRevoked
- BudgetExhausted
- RecoveryRequired

Channel-specific NoProgress/LoopDetected는 `DXB-RUN-037` Cycle terminal semantic과 연계할 수 있다.

## 11. Authorization / Grant

각 typed Command는 current Authorization Decision을 통과한다. Process 생성 시 허용됐다는 사실이 미래 step의 ambient authority가 아니다.

ActionGrant를 사용하는 step은:
- target Canonical Action digest
- current grant revision/remaining budget
- policy/approval revision
을 검증한다. retry/replan/recovery가 grant 사용량을 우회하지 않는다.

## 12. Resource Budget

Process 전체는 필요 시:
- total Activity/activation
- delegation depth/fan-out
- token/cost
- deadline
- Runtime Memory admission for active work

budget을 가진다. Process가 MemoryReservation을 영속 저장하거나 global budget owner가 되지 않고 `DXB-RUN-031`을 소비한다.

## 13. Crash Window 필수 검증

1. child Task 생성 전/후
2. parent Waiting commit 전/후
3. result 수신 전/후
4. reviewer 승인 전/후
5. Memory promotion proposal 전/후
6. target Memory commit 전/후
7. Process step advance 전/후
8. ack/notification 전/후

어느 window에서도 duplicate child, lost progress, duplicate promotion, replay-time Provider call, grant over-consumption이 발생해서는 안 된다.

## 14. Recovery

startup:
- Process definition version 확인
- current process revision/progress 복원
- pending command가 target에 이미 적용되었는지 reconcile
- committed outcome reuse
- missing/corrupt ref는 RecoveryRequired
- terminal process는 late event로 implicit reopen 금지

Process implementation 버전이 과거 DefinitionVersion을 replay할 수 없으면 migration/compatibility policy 없이 새 definition으로 임의 진행하지 않는다.

## 15. Observability

`ProcessId → StepRef → CommandRef → Task/Activity/Execution → OutcomeRef → Memory/Grant` correlation을 제공한다. process age/stuck/recovery-required/duplicate suppression/terminal reason을 metric/audit으로 노출한다.

## 16. 검증 기준

AT-PROC-001:
- 8개 crash window에서 Process identity/progress 복원
- replay-time LLM/Tool call 0
- duplicate child/delegation/promotion/Side Effect 0
- pending step만 idempotent resume/reconcile
- child state copy/Process direct mutation 0
- terminal late result implicit reopen 0
