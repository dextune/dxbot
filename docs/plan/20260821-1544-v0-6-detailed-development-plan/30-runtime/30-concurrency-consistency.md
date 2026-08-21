---
title: "동시성·상태 소유권·일관성"
document_id: "DXB-RUN-030"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-ARC-014", "DXB-ARC-017", "DXB-DOM-024", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029"]
---

# 동시성·상태 소유권·일관성

## 1. 목적

v0.5 race/fencing 규칙을 유지하면서 Durable Process, Memory retraction/revalidation, ActionGrant, Collaboration Run, Runtime Memory reservation/pressure의 winner와 cleanup 경계를 추가한다.

## 2. Ownership Matrix 확장

| 상태 | Canonical/Runtime Owner | 일관성 방식 |
|---|---|---|
| Durable Process | `DXB-RUN-038` | revision + stable step/command causation + outcome refs |
| Scoped/Epistemic Memory | `DXB-DOM-022` | immutable revision + assertion state + relation + expected revision |
| Authorization Decision/ActionGrant | `DXB-RUN-032` | policy revision + grant consumption revision/digest |
| Collaboration Run routing/terminal | `DXB-RUN-037` | CycleId/ProcessRef + bounded counters + terminal fencing |
| Runtime Memory reservation/pressure | `DXB-RUN-031` | hierarchical permit/counter + idempotent release |
| Task/Execution/Supervisor | `DXB-DOM-023` | expected revision + immutable execution fencing |
| Core Lease | Scheduler | lease/fencing |

Process/Grant/Reservation이 child owner의 state를 복제하지 않는다.

## 3. 기존 Race 비회귀

v0.5까지의 membership revoke, Role/Authority change, Channel archive, promotion/source update, simultaneous promotion, routing/membership change, Manager redirect/research completion, Context build/Memory update와 v0.4 redirect/cancel/suspend/resume/Side Effect/Lease/Provider races를 모두 유지한다.

## 4. v0.6 신규 Race

### Process Advance vs Duplicate Activity Outcome
- same logical outcome receipt는 Process revision/idempotency key로 한 번만 step progress에 반영한다.
- 이미 committed outcome이 있으면 replay가 Provider/Tool을 다시 호출하지 않는다.

### Process Recovery vs Late Result
- startup recovery 중 늦게 도착한 result는 current Process revision/terminal state와 reconcile한다.
- terminal Process를 late result가 암묵 재개하지 않는다.

### Process Cancel vs Child Completion
- Process cancel은 신규 step admission을 막는다.
- child Task completion이 먼저 commit되었다면 result는 audit/reconcile하되 Process terminal precedence를 명시한다.
- child Task를 Process state만으로 직접 cancel하지 않는다.

### Memory Retract vs Promotion
- PromotionProposal은 source revision/assertion state/digest를 pin한다.
- source가 retract/supersede되면 stale proposal은 commit 전 재평가한다.

### Memory Retract vs Context Build
- Context Plan이 선택한 immutable revision/assertion state를 pin한다.
- 새 Context는 current retract/revalidation 상태를 반영한다.

### ActionGrant Consume vs Duplicate Execution
- 동일 action digest/semantic receipt가 remaining use를 중복 소비하거나 반대로 여러 Side Effect를 허용하지 않게 한다.
- grant exhaustion/revoke와 effect preflight 사이 winner는 grant revision + current authorization으로 결정한다.

### Collaboration Terminate vs Late Participant Result
- terminal Cycle의 late result는 새 activation/cycle을 자동 생성하지 않는다.
- explicit new root intent/command만 새 Cycle을 시작한다.

### Declassification Approval vs Publication
- approval/policy revision을 target commit precondition으로 pin한다.
- revoke/expiry 이후 stale approval로 publication commit 불가.

### MemoryReservation Acquire vs Cancel/Timeout/Lease Expiry
- acquire success 이후 cancel이면 permit을 반드시 release한다.
- cancel 이후 늦게 acquire가 완료되어 orphan permit을 만들지 않는다.
- release는 idempotent하며 double release가 상위 budget을 왜곡하지 않는다.

### Reservation Release vs Late Provider Chunk
- terminal/release 이후 late stream chunk가 local buffer를 무제한 확장하지 않는다.
- Provider generation/cancellation + cumulative byte accounting으로 discard/reconcile한다.

### Pressure Transition vs Admission
- admission decision은 관측한 pressure/budget generation을 사용한다.
- Critical/Emergency로 전환된 뒤 stale Normal snapshot만으로 memory-heavy work를 무제한 시작하지 않는다.

### Cache Eviction vs Pinned Shared Prefix
- immutable shared prefix가 active Context에 pin된 동안 유효 reference를 유지하되 pin lifetime이 eviction을 영구 차단하지 않는다.

### Shutdown vs Task/Subscriber/Permit Cleanup
- shutdown 후 spawned task/subscriber/stream/MemoryReservation/Lease가 owner보다 오래 남지 않는다.

## 5. Queue / Buffer

기존 모든 production queue/channel의 item+byte cap을 유지한다. v0.6에서는 local cap 외 process-wide accounted/reserved byte ceiling을 함께 적용한다. local queue가 자신의 cap 안에 있어도 global budget 부족이면 신규 admission을 제한할 수 있다.

## 6. Lock / Async

- async mutex guard를 외부 I/O await 동안 보유하지 않는다.
- Resource admission hot path는 O(1)에 가까운 permit/counter를 우선하고 scope 전체 mutable map scan을 피한다.
- nested resource acquire는 composite admission 또는 고정 order로 deadlock을 방지한다.
- unrelated resource 대기 중 large memory permit을 장기 점유하지 않는다.
- spawned task는 owner/cancellation/join/retained-payload upper bound를 가진다.

## 7. 검증 기준

- 각 신규 race에 deterministic barrier/fake clock/fake budget fixture가 있다.
- duplicate Process step/Activity/Grant consume 0.
- retract 후 stale proposal/Context가 current policy를 우회하지 않음.
- pressure transition 중 global memory admission bypass 0.
- shutdown/quiescence 후 permit/task/subscriber retained leak 0.
- v0.5 race fixture regression 0.
