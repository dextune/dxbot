---
title: "동시성·상태 소유권·일관성"
document_id: "DXB-RUN-030"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-ARC-014", "DXB-ARC-017", "DXB-DOM-024", "DXB-DOM-027"]
---

# 동시성·상태 소유권·일관성

## 1. 목적

다수 Bot/Thread/Core/Provider/Plugin과 동시에 들어오는 Supervisor command가 data race, context leak, duplicate resume/side effect, control starvation, Provider drain race를 만들지 않도록 owner와 race winner semantics를 고정한다.

## 2. Single-Writer / Immutable Read

- Bot/Conversation/Thread/Task/Memory/Routine Canonical write는 Aggregate revision + Command path를 사용한다.
- Execution/Core는 immutable/versioned snapshot을 읽고 Result/Proposal을 반환한다.
- Thread-local Memory와 Execution Working Context를 scope/revision으로 구분한다.
- Registry/Config는 immutable generation swap.
- running Execution의 Task spec/Provider binding/Context Plan mutation 금지.
- Runtime Control signal은 durable Directive를 대체하지 않는다.

## 3. Ownership Matrix

| 상태 | Owner | 동시성 방식 |
|---|---|---|
| Bot state | Bot/Application coordinator | revisioned single write |
| Main Conversation/Thread/lineage | Conversation/Thread owner | revision + idempotency |
| Conversation Message append | Conversation owner | stable sequence/revision |
| Thread/Bot Memory | Memory service | scope + immutable revision/conflict |
| Task/spec/Continuation/Suspension | Task owner | expected revision + UoW |
| Control Directive | Control/Task command owner | DirectiveId + target revision/epoch |
| Execution snapshot | Execution owner | immutable + fencing |
| Core Lease | Scheduler | lease/fencing |
| Side Effect Ledger | Application/Execution ledger | action key + transactional state |
| Provider lifecycle/activity | Common Provider Host/Lifecycle | generation + activity guard |
| Plugin lifecycle | Plugin Manager | serialized operation + generation |
| UI/Interface Session | each Interface | Derived/Ephemeral |

## 4. Queue / Channel

모든 queue/channel은 item+byte cap, overflow/backpressure, close/send failure semantics를 가진다.

별도 queue class:
- Normal Work Queue
- Runtime Control Channel
- Provider Host admission/transport buffers
- Event subscriber buffers

Control Channel을 Work Queue alias로 구현하지 않는다. control flood도 bounded/rate-limited이며 다른 Bot의 control path와 일반 work를 영구 starvation시키지 않는다.

## 5. 기존 주요 Race

cancel vs complete, Waiting resume vs duplicate child, Routine trigger vs restart, Provider drain vs new call, Provider stop vs late callback, Provider health vs lifecycle, Plugin disable vs callback, Side Effect Unknown vs retry, Lease expiry vs result의 v0.3 semantics를 유지한다.

## 6. v0.4 신규 Race

### Redirect vs Complete
- completion terminal commit이 먼저면 redirect는 conflict/follow-up policy.
- redirect Task/spec revision commit이 먼저면 old Execution late completion은 new revision 결과를 덮지 못한다.

### Redirect vs Cancel
- cancel terminal/safety semantic 확정 후 redirect가 Task를 부활시키지 않는다.
- 동일 expected revision의 상충 command는 one-winner revision/precedence rule을 사용한다.

### Suspend vs Complete
- completion이 먼저 terminal이면 suspend no-op/conflict.
- suspend가 먼저면 old result는 fencing/merge rule.

### Suspend vs Side Effect
- external outcome uncertain이면 Unknown/Reconciliation Required 유지.
- suspend/resume가 동일 effect 재수행 권한을 만들지 않는다.

### Resume vs Duplicate Resume
- suspension/continuation revision + resume guard/idempotency로 새 Execution 하나.

### Reprioritize vs Scheduler Admission
- Scheduler는 revisioned priority snapshot을 읽는다.
- already-issued Lease를 field mutation하지 않고 rebalance/yield policy로 처리한다.

### Thread Archive vs Running Task
- archive가 running Task를 암묵 cancel하지 않는다.
- explicit preflight/policy/operation으로 판정한다.

### Thread Branch vs Concurrent Update
- source Thread revision을 pin한다.
- branch 후 source와 child는 독립 revision.

### Simultaneous Supervisor Commands
- arrival wall-clock만으로 winner를 정하지 않는다.
- expected revision, idempotency, security/safety precedence, committed terminal state를 사용한다.

## 7. Cross-Thread Isolation

- Thread A local Memory/Context/Directive가 B/C scope를 mutate하지 않는다.
- Bot-global Memory promotion은 Memory subsystem의 explicit promotion command다.
- same Bot global policy/security restriction은 별도 shared owner에서 모든 Thread에 적용될 수 있으나 Thread command로 masquerade하지 않는다.

## 8. Lock / Async

- async mutex guard + external I/O await 금지
- blocking work bounded pool
- child spawn은 supervisor owner
- cancellation-safe cleanup
- Control signal processing은 Canonical transaction lock을 장기간 보유하지 않음
- safe-point notification을 lock-free/short critical section으로 설계할 수 있으나 atomics는 model test 근거 필요

## 9. Consistency

Canonical Command는 strong revision. Projection/Index/UI는 eventual + watermark/stale. report의 live state는 observational이며 Canonical Task revision을 대체하지 않는다.

## 10. 검증 기준

- 신규 race 각각 deterministic barrier/fake clock test가 있다.
- AT-THREAD-001 cross-thread isolation 통과.
- AT-CTRL-002 control starvation 방지 통과.
- AT-CTRL-003/004 redirect/suspend recovery race 통과.
- Draining Provider가 신규 Host activity를 얻지 못함.
- shutdown 후 task/process/permit/activity/control queue leak 0.
