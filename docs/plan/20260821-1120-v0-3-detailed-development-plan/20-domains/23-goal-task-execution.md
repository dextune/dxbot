---
title: "Goal·Task·Execution 모델"
document_id: "DXB-DOM-023"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-015", "DXB-ARC-017", "DXB-DOM-020", "DXB-DOM-021"]
---

# Goal·Task·Execution 모델

## 1. 목적

Goal, Task, Execution을 분리하고 Waiting Continuation, Host-established Capability별 Provider binding, Side Effect write-ahead를 P0 crash-safety 계약으로 고정한다.

## 2. 개념

| 개념 | 의미 |
|---|---|
| Goal | 장기 의도/성공 조건 |
| Task | 실행 가능한 지속 작업과 상태기계 |
| Execution | Task 수행의 한 attempt |
| Core Lease | Execution의 일시 실행 권한 |
| ProviderSelectionRef | Provider Host가 Capability binding별 확정한 Provider/version/config/Registry generation의 opaque reference |
| Continuation | Waiting Task를 재개하는 영속 최소 상태 |
| Side Effect Entry | 비멱등 외부 변경의 intent/outcome/reconcile 상태 |

Execution 실패가 Task 실패를 자동 의미하지 않는다.

## 3. Task State

`Draft → Ready ↔ Blocked → Scheduled → Running ↔ Waiting → Succeeded | RetryableFailed → Scheduled | Failed | CancelRequested → Cancelled`

Task terminal은 Succeeded/Failed/Cancelled다. Timeout은 Execution outcome이며 Task policy가 후속 상태를 결정한다.

## 4. Task 구성

TaskId, owner BotId, Goal/Parent refs, specification revision, acceptance criteria, input refs, priority/deadline, resource budget, required capabilities, dependencies/delegation, retry policy, current status/revision, result refs, correlation/causation을 가진다.

Provider preference는 selection hint/policy input일 수 있으나 Task Domain 의미는 concrete Provider에 종속되지 않는다. Task가 concrete Provider handle/process/session을 소유하지 않는다.

## 5. Execution Snapshot과 Capability Binding

Execution 시작 시 다음을 고정한다.
- Task revision
- Identity/Brain/Memory/Policy snapshot revisions
- required Capability/resource/deadline budget
- attempt number
- 생성된 Capability binding별 `ProviderSelectionRef`

Execution이 사용할 Capability가 모두 사전에 확정될 필요는 없다. Provider Host는 binding을 Execution 준비 단계에서 eager하게 또는 first-use 시 lazy하게 만들 수 있다. 한 번 만들어진 동일 binding은 Execution 동안 Provider ID/version/config/Registry generation이 immutable하다.

한 Execution에서 `ModelGateway`, `ToolExecutor`, `MemoryIndex` 등 서로 다른 Capability binding이 각각 다른 Provider를 가질 수 있다. 이를 “Execution당 concrete Provider 하나”로 축약하지 않는다.

Provider-specific permit/activity는 실제 invocation의 Host 범위에서 획득/반환한다. Scheduler/Task가 이를 직접 소유하거나 mint하지 않는다.

## 6. Waiting Continuation Contract

Task가 `Waiting`을 Commit한다면 같은 Unit of Work에 유효 Continuation을 저장해야 한다.

Continuation 최소 의미:
- Task/Specification revision
- wait condition/version
- completed child/dependency set
- pending child/dependency/delegation set
- pending external message/result refs
- checkpoint/Artifact refs
- deadline/not-before
- correlation/causation
- resume guard/idempotency token
- continuation revision/checksum

`Waiting without Continuation` 또는 non-Waiting Task에 active Continuation은 invariant violation이다.

## 7. Waiting Recovery

1. Waiting Task + Continuation load
2. completed refs outcome/idempotency 확인
3. pending refs만 subscribe/wait
4. 새 결과를 continuation revision으로 merge
5. 조건 충족 시 `ResumeTask`를 idempotent하게 한 번 Commit
6. consumed Continuation을 terminal/cleared marker로 전환

완료 Child를 crash 후 재실행하지 않는다. duplicate result는 correlation/result ID/revision으로 dedup한다.

## 8. Side Effect Write-Ahead와 Provider Host

```text
Prepare Intent/Key/Action Digest
        ↓ durable commit
Provider Host verifies side-effect guard
        ↓
Provider executes external effect
        ↓
Host normalizes outcome/accounting
        ↓
Commit Outcome
```

semantic 상태: `Prepared | Executing(optional) | Confirmed | Failed | Unknown | Reconciled`.

Provider Host가 required Intent/Idempotency guard 없이 non-idempotent semantic call을 실행하지 않는다.

## 9. Retry Ownership

- Task semantic retry는 새 Execution attempt다.
- retry eligibility/disposition은 stable error, idempotency, Side Effect state, deadline, resource/cost budget을 보는 Common Runtime/Policy 책임이다.
- Provider가 Task retry scheduler를 소유하지 않는다.
- Provider-local transport retry는 Capability Contract가 허용한 semantic-invisible 범위만 가능하다.
- Confirmed Side Effect는 동일 effect 재수행 금지.
- Unknown은 자동 semantic retry 금지, reconciliation 우선.
- cancel 후 신규 retry 금지.
- **동일 Capability binding의 Provider fallback은 새 Execution attempt에서 새 Host selection으로 수행한다.**

## 10. Provider Failure / Version Mismatch

- incompatible Contract/SDK Provider는 신규 binding selection 전에 `Ready` Registry에 들어오지 않는다.
- pin된 Provider가 call 전에 Draining/Failed가 되면 Host/Lifecycle policy가 기존 pin의 허용 범위를 판단하고 stable unavailable outcome을 반환한다.
- call 중 crash/timeout/cancel은 Host가 stable outcome + activity/accounting cleanup을 수행한다.
- Side Effect 결과가 불명확하면 `Unknown/Reconciliation Required` 의미를 보존한다.
- silent fallback으로 기존 binding을 다른 Provider로 바꾸지 않는다.

## 11. Routine과 Task

Routine occurrence는 일반 `SubmitTask`와 동일한 Task 생성 경로를 사용한다. Routine은 Task state machine/Provider Host를 우회하지 않는다. occurrence ID를 idempotency scope에 포함한다.

## 12. Acceptance Scenario

Waiting:
`Parent → Child A/B/C → A/B 완료 → Waiting+Continuation Commit → kill → restart → C only wait → C result once → Parent resume once`.

Side Effect:
`Prepared commit → Host guard → external mutate → crash before outcome → restart → Unknown → reconcile → no duplicate mutate`.

Provider fallback:
`Execution N, binding X pinned Provider A → A unavailable/fails → N terminal outcome → policy creates Execution N+1 → Host selects Provider B for binding X`.

## 13. 검증 기준

- AT-TASK-003 Waiting Continuation Recovery 통과
- AT-SFX-001 Side Effect reconciliation 통과
- Task revision 변경이 진행 중 Execution/Continuation snapshot을 바꾸지 않음
- ProviderSelectionRef는 Host가 binding별 생성하고 동일 Execution의 동일 binding에서 immutable
- 여러 Capability binding이 서로 다른 Provider를 사용 가능
- 동일 binding의 fallback이 기존 Execution을 변이하지 않고 새 attempt 생성
- permission/resource/deadline/retry/Side Effect가 Provider 내부에 중복 소유되지 않음
