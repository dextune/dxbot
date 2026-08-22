---
title: "Durable Process Orchestration"
document_id: "DXB-RUN-038"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-RUN-030", "DXB-RUN-033", "DXB-DOM-023", "DXB-DOM-029"]
---

# Durable Process Orchestration

## 1. 목적

여러 Aggregate·Activity·Side Effect에 걸친 장기 진행을 crash/restart 후 재개할 수 있게 하되 Task, Memory, Directive, Approval state를 Process 안에 중복 저장하지 않는다.

## 2. Process가 소유하는 것

```text
ProcessId
DefinitionId / DefinitionVersion
ProcessRevision
Scope/Initiator refs
Current step/activity refs
Waiting condition / Continuation refs
Child Task/Directive/Approval/SideEffect refs
Progress/outcome references
Lifecycle / terminal reason
```

Process가 소유하지 않는 것:
- child Task lifecycle 복사본
- Memory assertion 내용/epistemic state 복사본
- Membership/Authority copy
- Provider Session/Core Lease
- CLI watch/subscriber state

## 3. Lifecycle

```text
Created → Running → Waiting
Running/Waiting → Suspending → Suspended → Resuming → Running
Running/Waiting/Suspended → Completing → Completed
                         → Cancelling → Cancelled
                         → Failed | RecoveryRequired
```

step advance는 expected process revision과 child outcome reference를 검증한다. late/duplicate activity result는 idempotently 무시하거나 conflict로 기록한다.

## 4. Command와 Receipt

Process create/control mutation은 Application Operation Receipt와 연결한다. receipt는 submission/commit/recovery 상태를 설명하고 Process Aggregate는 durable progress를 소유한다. 둘을 같은 state로 복제하지 않는다.

## 5. Waiting과 Recovery

Waiting은 wake condition, continuation/checkpoint, deadline, subscription/callback reference를 durable하게 기록한다. Runtime restart 시 transient task/channel/timer를 복원하지 않고 current canonical state에서 다시 등록한다.

## 6. Observation

`process show/watch`는 child owner refs와 process progress를 bounded DTO로 제공한다. public Subscription은 cursor/gap/terminal contract를 사용한다. subscriber disconnect가 process를 cancel하지 않는다.

## 7. Control

범용 `process control`이 child typed operation을 우회하지 않는다. Process-level cancel/suspend가 필요하면 definition과 owner가 영향을 정의하고 child Task/Side Effect를 안전하게 coordinate한다.

## 8. 검증 기준

- crash/restart 후 동일 ProcessId/DefinitionVersion으로 재개.
- duplicate/late activity result가 progress를 두 번 advance하지 않음.
- Task/Memory/Membership state 중복 0.
- response loss 뒤 receipt에서 ProcessId/outcome 조회 가능.
- watch disconnect가 Process lifecycle에 영향 0.
