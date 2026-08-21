---
title: "Live Control과 Cooperative Preemption"
document_id: "DXB-RUN-036"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-023", "DXB-DOM-024", "DXB-DOM-026", "DXB-DOM-027", "DXB-RUN-030", "DXB-RUN-031", "DXB-RUN-033"]
---

# Live Control과 Cooperative Preemption

## 1. 목적

v0.4의 out-of-band Control Channel, immutable redirect, durable suspend/resume, cooperative preemption, Scheduler ownership을 그대로 유지하면서 Channel Manager/Supervisor control에 Membership/Authority fencing을 추가한다.

## 2. v0.4 비회귀 계약

1. Work Queue와 Runtime Control Channel은 분리하고 둘 다 bounded다.
2. inspect/report/cancel/safety control이 일반 backlog에서 무기한 starvation되지 않는다.
3. running Execution snapshot/Context Plan/Provider binding을 mutation하지 않는다.
4. redirect는 durable Directive + Task Specification revision + safe yield + new Execution이다.
5. suspend는 Waiting과 다르며 checkpoint/Continuation을 durable하게 보존한다.
6. resume는 idempotent new Execution semantic이다.
7. Provider에 Hard Real-Time preemption을 약속하지 않는다.
8. Side Effect Unknown을 control 편의로 재실행하지 않는다.
9. Core Lease 발급/회수는 Scheduler만 수행한다.
10. Prompt/Provider/Tool output은 Control authority가 아니다.

## 3. Channel Control Path

```text
Channel Message / explicit action
→ intent resolution
→ authenticated principal
→ Project/Channel membership check
→ Role context + effective Authority check
→ target Task + SupervisorRef check
→ expected Task/membership/authority revision
→ Canonical Command/Directive commit
→ bounded Runtime Control Channel
→ Execution Supervisor
→ safe point / Scheduler
```

Channel Message 문자열 자체가 Directive가 아니다.

## 4. Manager / Supervisor

- Manager Role과 effective Authority를 분리한다.
- Manager는 SupervisorRef 또는 별도 approved grant 범위에서만 Task control을 요청한다.
- 다른 Bot의 Brain/Core/Memory를 직접 mutate하지 않는다.
- delegation은 Bot Network/Task Domain을 사용한다.

## 5. Revocation / Authority Change

- revoke/Authority change가 commit된 뒤 신규 Directive는 current generation을 확인한다.
- stale routing/control snapshot으로 redirect/suspend/cancel을 commit하지 않는다.
- 이미 시작된 immutable Execution의 Context를 rewrite하지 않는다.
- external effect 직전 current authorization 재검증이 필요한 class는 Security/Capability policy가 소유한다.

## 6. Race

기존 v0.4 race에 추가:
- Membership revoke vs pending redirect
- Authority downgrade vs report/control
- Supervisor change vs Manager command
- Research completion vs Manager redirect
- Channel archive vs control request

expected revision/generation + terminal precedence + Execution fencing으로 결정한다.

## 7. Report / Inspect

Channel report는 Canonical Task revision, live Runtime state, active Execution/Core/Provider binding, last progress, Channel/Supervisor context, observed_at/stale를 구분한다. Presence projection만으로 task completion을 추론하지 않는다.

## 8. Recovery

startup에서 pending Directive를 복원할 때 current Task/Supervisor/Membership/Authority revision을 확인한다. 과거 권한으로 committed된 뒤 이미 terminal된 Directive는 history로 보존하되 stale runtime signal을 재적용하지 않는다.

## 9. 검증 기준

- AT-CTRL-002~005 기존 Acceptance 전부 유지.
- AT-CHANNEL-003 Role label만으로 Directive commit 불가.
- AT-SEC-002 revoke 후 신규 control denied.
- AT-COLLAB-001 Manager redirect가 Task revision/Directive로만 발생.
- Channel control flood도 item+byte/resource cap을 유지.
