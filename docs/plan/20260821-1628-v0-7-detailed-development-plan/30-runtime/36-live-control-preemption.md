---
title: "Live Control과 Cooperative Preemption"
document_id: "DXB-RUN-036"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-023", "DXB-DOM-024", "DXB-DOM-026", "DXB-DOM-027", "DXB-RUN-030", "DXB-RUN-031", "DXB-RUN-033"]
---

# Live Control과 Cooperative Preemption

## 1. 목적

v0.5 out-of-band Control Channel, immutable redirect, durable suspend/resume, cooperative preemption, Scheduler ownership을 유지하면서 ProcessRef/ActionGrant/current Authorization과 Runtime Memory pressure의 안전한 control linkage를 추가한다.

## 2. 비회귀 계약

- Work Queue와 Runtime Control Channel은 분리되고 bounded다.
- inspect/report/cancel/safety/recovery가 일반 backlog에서 무기한 starvation되지 않는다.
- running Execution snapshot/Context/Provider binding을 mutation하지 않는다.
- redirect는 durable Directive + Task revision + safe yield + new Execution이다.
- suspend는 Waiting과 다르며 checkpoint/Continuation을 durable하게 보존한다.
- resume는 idempotent new Execution이다.
- Core Lease는 Scheduler만 발급/회수한다.
- Side Effect Unknown을 control 편의로 재실행하지 않는다.

## 3. v0.6 Control Path

```text
Message / explicit action / Resource pressure policy
→ intent or policy trigger
→ authenticated principal/system principal
→ current Authorization Decision
→ Task/Supervisor/Process relation check
→ ActionGrant check if required
→ expected revision/generation
→ Canonical Command/Directive
→ bounded Runtime Control Channel
→ Execution Supervisor
→ cooperative safe point / Scheduler
```

Process/pressure state가 worker handle을 직접 kill하지 않는다.

## 4. Process Cancel / Timeout

- Process cancel/timeout은 신규 process step admission을 중단한다.
- 이미 running child Task/Execution을 멈춰야 하면 명시적 Task/Live Control Directive를 발행한다.
- Process terminal state가 child terminal state를 덮어쓰지 않는다.
- late completion은 reconcile/audit하고 Process를 implicit resume하지 않는다.

## 5. ActionGrant / Authorization Fencing

- grant-required control은 current ActionGrant action digest/use budget/expiry/policy revision을 검증한다.
- retry/resume/new Execution이 동일 승인으로 무한 control/Side Effect를 만들지 못한다.
- Membership/Authority/Grant revoke 뒤 stale pending Directive를 새로 적용하지 않는다.

## 6. Memory Pressure Preemption

Critical/Emergency pressure에서 policy는 restartable/low-priority running work를 cooperative preempt/cancel 후보로 만들 수 있다.
- raw async task/worker kill로 Canonical state를 우회하지 않는다.
- suspend/cancel/checkpoint/Continuation/Side Effect safe point 의미를 재사용한다.
- control/recovery headroom을 보존한다.
- 일반 work를 줄이기 위해 Canonical Memory/History를 삭제하지 않는다.

## 7. Waiting / Suspension Resource Release

Waiting/Suspended 상태는 active Core Lease와 large transient Context/Provider buffer reservation을 계속 보유하지 않는다. durable checkpoint/reference를 남기고 resume 시 Scheduler/Resource Governance에서 재-admission한다.

## 8. Race

기존 race에 추가:
- Process cancel vs child complete
- ActionGrant revoke/exhaust vs pending redirect/effect
- Critical pressure vs new Execution admission
- suspend checkpoint commit vs memory reservation release
- terminal release vs late Provider result

expected revision/generation/fencing + existing terminal precedence로 결정한다.

## 9. 검증 기준

- 기존 AT-CTRL-002~005 유지.
- Process state 변경만으로 child Task direct mutation 0.
- AT-SEC-004 grant replay budget 우회 0.
- AT-RMEM-001 Waiting/Suspended transient reservation 장기 보유 0.
- Critical/Emergency pressure에서도 control/recovery starvation 0.
