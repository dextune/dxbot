---
title: "Channel Runtime Orchestration"
document_id: "DXB-RUN-037"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-021", "DXB-DOM-024", "DXB-DOM-025", "DXB-DOM-029", "DXB-RUN-030", "DXB-RUN-031", "DXB-RUN-032", "DXB-RUN-036", "DXB-RUN-038"]
---

# Channel Runtime Orchestration

## 1. 목적

v0.5 speaker/recipient selection, bounded fan-out, turn admission, manager-first routing, backpressure, membership fencing을 유지하면서 **Single-Bot-first admission + causal Collaboration Run total budget + progress/stall + explicit terminal reason + Runtime Memory admission**을 추가한다.

> Channel Coordinator는 **Brain이 아니다**. reasoning은 선택된 Persistent Bot Brain이 수행한다.

## 2. Canonical Owner가 아닌 것

Orchestration은 Bot Identity/Brain, Channel Membership/Role/Authority, Memory, Thread, Task/Supervisor, Core Lease, Provider lifecycle, Authorization Policy, Durable Process state를 소유하지 않는다.

Channel-specific routing/termination policy와 Cycle runtime 의미만 소유한다. 둘 이상의 Canonical Owner를 횡단하는 durable 진행은 `DXB-RUN-038`을 사용한다.

## 3. Collaboration Run / Interaction Cycle

필요 시 하나의 causal run은 다음 logical boundary를 가진다.

```text
CycleId / ProcessRef?
Root Intent / Root Message
Causation Chain
Selected Participants
Pending Task / Delegation refs
Round / Hop / Activation counters
Token / Cost / Deadline Budget
Concurrent Activity Budget
Progress Marker
Verification State
Terminal Reason
```

이 값이 Channel Membership/Task/Process Canonical state를 복제하지 않는다.

## 4. Single-Bot-First Admission

기본은 하나의 Persistent Bot이다. Multi-Bot 승격 후보:
- 독립 병렬 decomposition이 가능
- 별도 Reviewer/검증 가치가 큼
- 전문 역할의 독립 Context가 실제 필요
- 사용자가 명시적으로 요청
- cost/latency/resource budget 안에서 기대 이득이 있음

P0에서는 deterministic policy를 사용하고 학습 기반 router를 요구하지 않는다.

금지:
- Channel이므로 항상 Manager+Researcher+Reviewer 활성화
- participant 수가 많을수록 품질이 높다고 가정
- 모든 요청을 debate로 변환

## 5. Routing Pipeline

```text
Channel Event / Root Intent
→ validate active/readable state
→ Single-Bot-first or bounded multi admission
→ bounded candidate participant set
→ explicit recipient / mention / role policy
→ current membership/authorization recheck
→ total Cycle budget check
→ process-wide Runtime Memory admission
→ target Bot dispatch
→ Bot Brain
→ Task/Execution
→ Scheduler/Core Lease
```

Manager delegation은 durable Bot Network/Task path를 사용한다.

## 6. Total Budget

per-message activation cap 외:
- total Bot activation
- delegation depth/fan-out
- total round/hop
- token/cost
- deadline/wall budget
- concurrent Activity

정확한 숫자는 `DXB-RUN-031` Policy SSOT가 소유한다. late message가 terminal Cycle의 counters를 되살리지 않는다.

## 7. Progress / Stall

LLM의 “진행 중” 문자열로 progress를 판정하지 않는다.
Runtime 관측 후보:
- new evidence/result produced
- pending Task count 변화
- acceptance coverage 변화
- unresolved blocker 변화
- duplicate claim/result 비율
- same-state repeated cycle

정확한 stall detector는 benchmark/OQ 대상이다.

## 8. Terminal Semantics

최소 의미:
- Succeeded
- Partial
- NoProgress
- LoopDetected
- BudgetExhausted
- TimedOut
- Cancelled
- AuthorizationRevoked
- Failed / RecoveryRequired

Cycle은 terminal reason 없이 영구 지속되지 않는다. terminal 이후 late result는 audit/reconciliation 대상으로 보존할 수 있으나 새 Cycle을 implicit 생성하지 않는다.

## 9. Reviewer / Verification

Reviewer Role만으로 결과가 사실이 되지 않는다. 가능한 범위에서:
- Claim
- Evidence refs
- source provenance
- conflicts
- acceptance criteria
- verification result

를 연결한다. 동일 모델 Bot들의 다수결은 `Verified`의 충분조건이 아니다.

## 10. Runtime Memory / Backpressure

- selected participant/response concurrency도 global Runtime Memory admission을 통과한다.
- Channel local queue cap이 남아 있어도 process-wide budget 부족이면 admission을 줄인다.
- response aggregation/large history를 global budget 밖에서 별도 materialize하지 않는다.
- Critical/Emergency pressure에서 신규 Multi-Bot collaboration을 억제하고 Single/partial/defer/reject policy를 적용할 수 있다.
- slow Provider/consumer가 unbounded response accumulation을 만들지 않는다.

## 11. Recovery

- duplicate event는 Message/Event idempotency로 dedup
- ProcessRef가 있는 causal run은 committed process outcome/progress에서 복구
- terminal Cycle의 late result는 implicit resume 금지
- Provider Session loss는 Context Plan에서 새 call 가능하나 replay가 이미 완료된 Activity를 재호출하는 것과 구분
- Presence는 Derived로 rebuild

## 12. 검증 기준

- 기존 AT-CHANNEL-002/AT-COLLAB-001 유지.
- AT-COLLAB-002 A→B→C→A loop가 total budget/terminal reason으로 종료.
- AT-COLLAB-003 Single vs Multi utility/cost evidence가 default policy에 연결.
- AT-RMEM-001 Channel fan-out이 global byte budget을 우회하지 않음.
- Coordinator가 LLM reasoning/Task/Memory/Core owner가 되지 않음.
