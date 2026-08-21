---
title: "Goal·Task·Execution·Live Revision 모델"
document_id: "DXB-DOM-023"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-015", "DXB-ARC-017", "DXB-DOM-020", "DXB-DOM-021", "DXB-DOM-027"]
---

# Goal·Task·Execution·Live Revision 모델

## 1. 목적

Goal, Task, Execution을 분리하고 Thread linkage, Waiting/Suspension continuation, Host-established Capability binding, Side Effect write-ahead, redirect/reprioritize semantics를 crash-safe하게 고정한다.

## 2. 개념

| 개념 | 의미 |
|---|---|
| Goal | 장기 의도/성공 조건 |
| Thread | 영속 작업·문맥 경계; Task와 1:1이 아님 |
| Task | 실행 가능한 지속 작업과 state/revision |
| Task Specification Revision | Task 목표/지시의 immutable revision |
| Execution | 특정 Task revision을 수행하는 한 attempt |
| Core Lease | Execution의 일시 실행 권한 |
| ProviderSelectionRef | Capability binding별 pinned Provider reference |
| Continuation/Checkpoint | Waiting/Suspended 재개에 필요한 최소 영속 상태 |
| Control Directive | running Task에 대한 durable redirect/suspend/reprioritize 등 |
| Side Effect Entry | 비멱등 외부 변경 intent/outcome/reconcile |

Execution 실패/중단이 Task 실패를 자동 의미하지 않는다.

## 3. Task State 의미

기존 `Draft → Ready ↔ Blocked → Scheduled → Running ↔ Waiting → terminal/retry/cancel` 의미를 유지한다.

v0.4는 추가로 control-induced `SuspendRequested/Suspended/Resume` 의미를 도입한다. 정확한 enum은 ADR에서 고정할 수 있으나 다음은 불변이다.
- Suspended는 dependency `Waiting`과 다르다.
- suspended Task는 신규 Execution admission에서 제외된다.
- resume는 idempotent하게 새 Execution을 만든다.
- terminal Task를 redirect/resume가 암묵 부활시키지 않는다.

Task terminal은 Succeeded/Failed/Cancelled다. Timeout은 Execution outcome이며 Task policy가 후속 상태를 결정한다.

## 4. Task 구성

TaskId, owner BotId, primary ThreadId, Goal/Parent refs, specification revision, acceptance criteria, input refs, priority/deadline, resource budget, parallelism hint, required capabilities, dependencies/delegation, retry policy, current status/revision, result refs, correlation/causation을 가진다.

Task는 UI/Provider Session/Core handle을 소유하지 않는다. 한 Thread는 여러 Task를 가질 수 있다.

## 5. Immutable Execution Snapshot

Execution 시작 시 고정:
- Task + Task Specification revision
- primary Thread revision/context reference
- Identity/Brain/Memory/Policy snapshot revisions
- required Capability/resource/deadline budget
- attempt number
- 생성된 Capability binding별 ProviderSelectionRef
- applicable Control/Directive epoch

동일 Execution의 snapshot/prompt/provider binding을 redirect 때문에 mutation하지 않는다.

## 6. Redirect

`DXB-RUN-036`이 runtime semantics의 Canonical Owner다.

Task 관점:
1. Redirect Command가 expected Task/spec revision을 검증
2. 새 Task Specification revision + Directive commit
3. old Execution은 old revision 그대로 cooperative yield/terminal
4. checkpoint/result fragment를 old Execution history로 보존
5. 새 Execution이 새 spec/thread/context snapshot으로 시작

`redirect = old Execution prompt string mutation`은 금지한다.

## 7. Suspend / Resume

Suspend commit은 target revision과 directive를 durable하게 남기며, safe point에서 resume에 필요한 checkpoint/Continuation을 보존한다.

Resume는:
- expected suspended revision 확인
- continuation/checkpoint checksum/compatibility 확인
- resume guard/idempotency consume
- 새 Execution 생성

crash 후 duplicate resume/duplicate Side Effect를 만들지 않는다.

## 8. Waiting Continuation

Waiting Task는 status와 유효 Continuation을 같은 Unit of Work에서 Commit한다. 최소 의미는 Task/spec revision, wait condition, completed/pending dependency/delegation refs, checkpoint/Artifact refs, deadline/not-before, correlation/causation, resume guard, continuation revision/checksum이다.

Waiting과 Suspended가 동일 persistence representation을 일부 재사용할 수 있으나 semantic reason과 lifecycle owner를 구분한다.

## 9. Side Effect Write-Ahead

`Prepare Intent/Key/Action Digest → durable commit → Provider Host guard → external effect → normalized outcome commit`을 유지한다.

Redirect/Suspend/Cancel이 발생해도 이미 Confirmed effect를 되돌린 것으로 간주하지 않는다. external outcome이 불명확하면 `Unknown/Reconciliation Required`를 보존한다.

## 10. Retry / Provider Fallback

- semantic retry는 새 Execution attempt
- Provider fallback도 같은 Capability binding을 old Execution에서 mutate하지 않고 새 Execution에서 새 selection
- Provider-local transport retry는 Contract 허용 범위만
- Unknown Side Effect 자동 retry 금지
- cancel terminal 후 신규 retry/redirect/resume 금지

## 11. Reprioritize / Parallelism Hint

Task priority/resource/parallelism request는 revisioned policy input이다. 사용자가 “Core 2개로”라고 말해도 fixed Core count를 Task/Bot identity로 저장하지 않는다. Scheduler가 hard limit/fairness/parallel benefit에 따라 Lease를 결정한다.

## 12. Thread 관계

- primary Thread는 Task context provenance를 제공한다.
- Thread archive가 Task cancel을 암묵 수행하지 않는다.
- Thread branch/fork는 source Execution mutation이 아니라 새 Thread/Task lineage 생성이다.
- Thread A control이 B/C Task revision을 변경하지 않는다.

## 13. Race

- cancel vs complete: terminal revision 한 쪽만 승리
- redirect vs complete: Task/spec revision과 old Execution fencing으로 결정
- suspend vs complete: 먼저 committed semantic이 승리, late result는 merge/fencing
- resume duplicate: continuation revision + guard
- reprioritize vs admission: Scheduler가 revisioned priority snapshot 적용
- side effect vs suspend/cancel: ledger state 우선, Unknown을 보존

## 14. 검증 기준

- AT-TASK-003 Waiting Continuation Recovery
- AT-SFX-001 Side Effect reconciliation
- AT-CTRL-003 Redirect가 running Execution을 mutation하지 않고 새 revision/Execution으로 전환
- AT-CTRL-004 Suspend/Resume crash recovery가 한 번만 resume
- Task revision 변경이 old Execution/Continuation snapshot을 바꾸지 않음
- ProviderSelectionRef가 동일 Execution/binding에서 immutable
- Thread A redirect가 Thread B/C Task/Execution에 영향 없음
