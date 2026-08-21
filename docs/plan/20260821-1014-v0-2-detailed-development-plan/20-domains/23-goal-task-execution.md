---
title: "Goal·Task·Execution 모델"
document_id: "DXB-DOM-023"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-015", "DXB-DOM-020", "DXB-DOM-021"]
---

# Goal·Task·Execution 모델

## 1. 목적

Goal, Task, Execution을 분리하고 Waiting Continuation, Provider snapshot, Side Effect write-ahead를 P0 crash-safety 계약으로 고정한다.

## 2. 개념

| 개념 | 의미 |
|---|---|
| Goal | 장기 의도/성공 조건 |
| Task | 실행 가능한 지속 작업과 상태기계 |
| Execution | Task 수행의 한 attempt |
| Core Lease | Execution의 일시 실행 권한 |
| Continuation | Waiting Task를 재개하는 영속 최소 상태 |
| Side Effect Entry | 비멱등 외부 변경의 intent/outcome/reconcile 상태 |

Execution 실패가 Task 실패를 자동 의미하지 않는다.

## 3. Task State

`Draft → Ready ↔ Blocked → Scheduled → Running ↔ Waiting → Succeeded | RetryableFailed → Scheduled | Failed | CancelRequested → Cancelled`

Task terminal은 Succeeded/Failed/Cancelled다. Timeout은 Execution outcome이며 Task policy가 후속 상태를 결정한다.

## 4. Task 구성

TaskId, owner BotId, Goal/Parent refs, specification revision, acceptance criteria, input refs, priority/deadline, resource budget, required capabilities, dependencies/delegation, retry policy, current status/revision, result refs, correlation/causation을 가진다.

Provider preference는 capability selection hint일 수 있으나 Task Domain 의미는 concrete Provider에 종속되지 않는다.

## 5. Execution Snapshot

Execution 시작 시 다음을 pin한다.
- Task revision
- Identity/Brain/Memory/Policy snapshot revisions
- selected Provider ID/version/config digest
- capability grants
- resource/deadline
- attempt number

Execution 중 hot reload로 Provider를 변경하지 않는다.

## 6. Waiting Continuation Contract

Task가 `Waiting`을 Commit한다면 같은 Unit of Work에 유효 Continuation을 저장해야 한다.

Continuation 최소 의미:
- Task revision
- wait condition/version
- completed child/dependency set
- pending child/dependency/delegation set
- pending external message/result refs
- checkpoint/Artifact refs
- deadline/not-before
- correlation/causation
- resume guard/idempotency token
- continuation revision/checksum

`Waiting without Continuation` 또는 `non-Waiting Task에 active Continuation`은 invariant violation이다.

## 7. Waiting Recovery

복구 알고리즘의 의미:
1. Waiting Task와 Continuation을 load
2. completed refs의 outcome/idempotency 확인
3. pending refs만 다시 subscribe/wait
4. 새 결과를 continuation revision으로 merge
5. 모든 조건 충족 시 `ResumeTask`를 idempotent하게 한 번 Commit
6. consumed Continuation을 terminal/cleared marker로 전환

완료된 Child A/B를 crash 후 재실행하지 않는다. 늦은 duplicate result는 correlation + result ID + revision으로 dedup한다.

## 8. Side Effect Write-Ahead

비멱등 외부 변경:

```text
Prepare Intent/Key/Action Digest
        ↓ durable commit
Execute External Effect
        ↓
Commit Outcome
```

semantic 상태:
- Prepared: 실행 허가/intent durable
- Executing: 선택적 관측 상태
- Confirmed: 외부 결과 확인
- Failed: 실행 결과가 실패로 확정
- Unknown: 외부 결과를 신뢰성 있게 판정할 수 없음
- Reconciled: status lookup/compensation/manual decision으로 정리

정확한 enum 구현은 ADR에서 세분화할 수 있다.

## 9. Retry

- retry는 새 Execution attempt
- Side Effect Entry가 Confirmed이면 동일 effect를 다시 수행하지 않음
- Unknown이면 자동 retry 금지, reconciliation 먼저
- 외부 system idempotency key/status lookup 사용 가능
- retry budget은 횟수, elapsed, cost, deadline을 포함
- cancel 후 신규 retry 금지

## 10. Routine과 Task

Routine occurrence는 일반 `SubmitTask`와 동일한 Task 생성 경로를 사용한다. Routine은 Task state machine을 우회하지 않는다. occurrence ID를 idempotency key scope에 포함한다.

## 11. Acceptance Scenario

```text
Parent Task
→ Child A/B/C
→ A/B 완료
→ Waiting + Continuation Commit
→ Runtime kill
→ restart
→ C만 다시 기다림
→ C 결과 한 번 통합
→ Parent 한 번 Resume
```

Side Effect scenario:
`Prepared commit → external mutate → crash before outcome → restart → Unknown → reconcile → no duplicate mutate`.

## 12. 검증 기준

- AT-TASK-003 Waiting Continuation Recovery를 통과한다.
- AT-SFX-001 Side Effect crash reconciliation을 통과한다.
- Task revision 변경이 진행 중 Execution/Continuation snapshot을 바꾸지 않는다.
- Provider fallback이 동일 Execution을 변이하지 않고 새 attempt를 만든다.
