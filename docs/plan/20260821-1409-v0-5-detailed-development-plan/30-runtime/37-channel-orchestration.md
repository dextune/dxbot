---
title: "Channel Runtime Orchestration"
document_id: "DXB-RUN-037"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-021", "DXB-DOM-024", "DXB-DOM-025", "DXB-DOM-029", "DXB-RUN-030", "DXB-RUN-031", "DXB-RUN-032", "DXB-RUN-036"]
---

# Channel Runtime Orchestration

## 1. 목적

Channel participant 수와 무관하게 speaker/recipient selection, bounded fan-out, turn admission, manager-first routing, backpressure, membership fencing을 수행하는 Runtime orchestration을 정의한다.

> Channel Coordinator는 **Brain이 아니다**. 의미 판단/LLM reasoning은 선택된 Persistent Bot의 Brain이 수행한다.

## 2. Canonical Owner가 아닌 것

Channel Orchestration은 다음을 소유하지 않는다.
- Bot Identity/Brain
- Channel Membership/Role/Authority Canonical State
- Memory Record/Scope
- Thread identity/history
- Task state/SupervisorRef
- Core Lease
- Provider lifecycle/selection
- Control Directive

이들은 각각의 Domain/Common owner를 호출한다.

## 3. 입력

- Channel event/message reference
- current Channel revision
- participant eligibility snapshot
- Role/Authority metadata needed for routing
- ParticipationPolicyRef
- mentions/explicit recipients
- current backpressure/resource state
- optional active Task/Supervisor references

raw prompt 문자열만으로 eligibility/authority를 구성하지 않는다.

## 4. Routing Pipeline

```text
Channel Event
→ validate Channel active/readable state
→ bounded candidate participant set
→ explicit recipient / mention resolution
→ role-aware policy routing
→ turn/speaker admission
→ fan-out/concurrency budget
→ current membership recheck
→ target Bot dispatch
→ Bot Brain
→ Task/Execution
→ Scheduler/Core Lease
```

## 5. 기본 Routing 의미

정확한 policy는 ADR/Policy SSOT가 소유하지만 P0에서는 다음 패턴을 지원할 수 있다.
- `@bot-a` → authorized Bot A
- `@researcher` → bounded authorized Researcher subset
- 일반 사용자 요청 → Manager-first policy
- Manager delegation → selected Researcher/Reviewer via durable Bot Network
- 토론/brainstorm 요청 → policy-limited N participants
- Research result → authoritative Supervisor/Manager route

모든 member를 항상 wake하는 default는 금지한다.

## 6. Turn Admission / Backpressure

- per-message activation cap
- response concurrency cap
- queue item+byte cap
- per-Bot/per-Channel fairness
- slow/blocked Provider가 global Channel processing을 무한 점유하지 않음
- overload는 explicit reject/defer/coalesce/partial selection semantic

정확한 숫자는 Resource Policy가 소유한다.

## 7. Membership Generation Fencing

routing snapshot과 commit-time authorization을 분리한다.

1. Coordinator가 eligible snapshot을 읽는다.
2. queue wait 동안 membership이 revoke될 수 있다.
3. dispatch 전 current membership generation을 재검증한다.
4. Shared Memory write/Control commit은 각 Canonical owner에서 current authorization을 다시 검증한다.

stale snapshot은 authority가 아니다.

## 8. Presence Projection

Derived:
```text
BotId
speaking?
processing?
active Execution count
active CoreLeaseId?
background Task count
last activity
observed_at
```

Presence는 Membership/Task truth가 아니며 restart 시 재생성 가능하다.

## 9. Dynamic Core Integration

특정 Channel/Role에 Core를 고정하지 않는다. 같은 Manager Bot도 이벤트마다 다른 Core Lease를 얻을 수 있다. Coordinator는 Scheduler를 우회하지 않는다.

## 10. Failure / Recovery

- transient routing queue loss: durable Channel Message/Event에서 재평가 가능
- duplicate event: Message/Event idempotency로 dedup
- coordinator crash: Canonical membership/message/task를 재조회
- Provider Session loss: target Bot Context Plan으로 새 call
- stale Presence: discard/rebuild

## 11. 금지 패턴

- Coordinator에서 LLM reasoning 수행
- participant 수만큼 항상 Execution 생성
- Channel별 Core 고정
- Role 문자열로 authorization 결정
- Manager가 target Bot worker/Core handle 직접 조작
- runtime presence를 Membership source로 사용
- unbounded response aggregation

## 12. 검증 기준

- AT-CHANNEL-002 member 1/10/100에서 activation이 policy cap을 유지.
- routing 중 revoke된 Bot이 dispatch/control/Shared Memory write를 얻지 못함.
- Manager-first delegation이 durable Bot Network/Task path를 사용.
- runtime restart 후 Presence는 재구축되지만 Membership revision은 변경되지 않음.
- Coordinator 제거/교체가 Channel Domain schema를 변경하지 않음.
