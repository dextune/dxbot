---
title: "동시성·상태 소유권·일관성"
document_id: "DXB-RUN-030"
version: "0.1.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-ARC-014", "DXB-DOM-024"]
---


# 동시성·상태 소유권·일관성

## 1. 목적

다수 Bot과 Core가 동시에 실행되어도 데이터 경쟁, 교착, 상태 혼합, 메모리 폭증, 취소 누락이 발생하지 않도록 Rust Runtime의 동시성 모델을 정의한다.

## 2. 책임 범위

- async runtime 사용 원칙
- Bot별 쓰기 직렬화
- immutable snapshot과 메시지 전달
- channel/lock/structured concurrency
- consistency level과 race 처리
- blocking I/O 분리

## 3. 기본 동시성 모델

### 3.1 Bot별 단일 쓰기 조정자
각 활성 Bot은 하나의 Coordinator가 Bot-level Command 순서를 소유한다.
- bounded mailbox
- 한 번에 하나의 aggregate mutation decision
- long-running model/tool I/O는 mailbox 밖의 Core/Execution task에서 수행
- 완료 결과는 revision을 포함한 Command로 다시 전달
- Coordinator는 Core 전체 수명 동안 lock을 잡지 않음

### 3.2 Core별 Structured Task
Core는 supervisor 아래 child task로 실행한다.
- parent cancellation
- deadline
- owned resources
- join/abort/checkpoint contract
- terminal result exactly once
- child process/tool ownership

### 3.3 공유 읽기
Identity, Brain Policy, capability registry, Memory selection은 immutable snapshot/`Arc`로 공유한다. mutable global object를 직접 참조하지 않는다.

## 4. 상태 소유권 표

| 상태 | Writer | Reader | 일관성 |
|---|---|---|---|
| Bot Aggregate | Bot Coordinator/Application UoW | Projection/Core snapshot | revision serializable |
| Task/Execution | Task Handler | Scheduler/Core/UI | expected revision |
| Core Lease | Scheduler/Executor reconciliation | Runtime/UI | fencing token |
| Memory Record | Memory Command Handler | Brain/Core/Search | revisioned snapshot |
| Provider Registry | Runtime lifecycle manager | new Executions | immutable generation |
| Resource Counters | Admission controller | Scheduler/metrics | atomic/eventual view |
| Projection | Projector | UI/Query | eventual + watermark |
| Working Context | 해당 Core | 해당 Core/explicit result channel | isolated |
| Bot Message | Bot Network store | source/target | durable at-least-once |

## 5. Lock 규칙

- lock보다 ownership/message passing을 우선한다.
- async mutex guard를 잡은 채 network/DB/model/tool await 금지.
- lock acquisition order를 문서화하며 다중 lock을 가능한 한 금지한다.
- read-heavy registry는 immutable generation swap을 우선한다.
- global `Arc<Mutex<HashMap<...>>>`를 핵심 상태 저장소로 사용하지 않는다.
- lock scope는 작은 primitive update로 제한한다.
- poisoning에 의존하지 않고 오류/복구 계약을 명시한다.
- sync lock과 async lock 선택은 critical section에서 await 여부로 결정한다.
- lock contention, hold time, wait time을 계측한다.

## 6. Channel과 Backpressure

모든 channel은 다음을 선언한다.
- producer/consumer owner
- capacity와 산정 근거
- item 최대 크기
- overflow policy
- shutdown semantics
- lag metric
- cancellation behavior

Overflow 선택:
- 중요한 Domain command/event: reject 또는 durable queue로 전환
- telemetry: sample/drop + counter
- model delta: bounded coalesce/spill
- UI event: cursor resync
- duplicate wake-up: coalesce
- approval request: durable pending state

unbounded channel은 금지한다.

## 7. Async/Blocking 분리

- DB driver가 blocking이면 전용 thread pool
- filesystem scan/hash/compression은 bounded blocking pool
- CPU-heavy embedding/serialization은 worker pool
- subprocess wait/read는 nonblocking API 또는 dedicated task
- blocking pool queue도 bounded
- async executor thread에서 긴 CPU loop 금지
- cancellation 시 blocking 작업이 실제 중단 가능한지 구분하고 결과를 폐기할 fencing 적용

## 8. 일관성 모델

- Aggregate write: expected revision 기반 강한 순서
- Cross-Aggregate: transaction 가능 범위는 atomic, 외부는 saga/outbox
- Projection: eventual consistency, watermark 제공
- Core read: execution 시작 snapshot consistency
- Resource metrics: eventual/approximate 가능
- Permission: 호출 시점 policy snapshot + 강화 정책
- Bot Message: at-least-once, consumer idempotency
- Search Index: eventual, canonical recheck

API와 UI는 모든 상태를 "실시간"으로 뭉뚱그리지 않고 일관성 수준을 노출한다.

## 9. Race 처리

### 취소 vs 완료
terminal Event append에 expected revision을 사용해 하나만 승리한다. 패한 결과는 trace/audit에 보존하되 상태를 덮지 않는다.

### Memory update vs delete
tombstone revision이 우선하며 stale writer는 conflict를 받는다.

### Provider unload vs 실행 시작
Execution은 registry generation을 pin한다. unload는 신규 admission 중지 후 old generation reference가 0이 될 때 dispose한다.

### Lease expiry vs late result
fencing token/lease revision을 검증하고 late result의 Commit을 거부한다.

### Bot deactivate vs new Task
lifecycle revision과 admission gate를 같은 decision path에서 검사한다.

## 10. 종료 순서

1. 외부 Command admission 중지
2. Scheduler 신규 Core admission 중지
3. subscriptions/listeners를 closing 상태로 전환
4. Task/Core에 quiesce/cancel 전달
5. child tool/process 종료 대기
6. result/checkpoint Commit
7. outbox/projector/indexer flush 또는 durable checkpoint
8. provider 역순 dispose
9. DB/artifact handles close
10. leak/unfinished summary 기록

완료 조건은 "취소 요청 전송"이 아니라 시스템이 소유한 작업이 quiescent 상태임을 확인한 것이다.

## 11. 예외상황

- Coordinator mailbox full: API backpressure, task coalescing 또는 durable queue
- deadlock 의심: watchdog stack/task dump와 health degraded
- panic: task boundary에서 격리, shared state invariant 검사, crash-loop budget
- slow consumer: cursor 기반 재동기화, producer 무제한 buffer 금지
- clock jump: monotonic deadline 사용
- cancellation ignored by provider: lease revoke와 result fencing
- blocking pool saturation: 신규 heavy operation admission 제한
- orphan task: supervisor registry와 periodic reconciliation

## 12. 확장성

단일 Bot Coordinator는 분산 시 Bot ownership shard로 확장한다. Core task는 remote executor로 이동해도 lease/fencing/cancellation contract를 유지한다. in-memory channel을 durable transport로 바꿀 때 전달 의미를 바꾸지 않는다.

## 13. 구현 우선순위

- **P0:** bounded mailbox, structured task, cancellation, revision races, blocking pool
- **P1:** lock/channel metrics, watchdog, lease fencing, graceful drain
- **P2:** distributed ownership, remote task supervision
- **P3:** adaptive concurrency control

## 14. 검증 기준

- 동시 Task/Memory/property test에서 invariant 위반이 없다.
- `loom` 또는 동등한 모델 검사로 핵심 race를 검증한다.
- 모든 channel 생성 지점에 capacity가 명시된다.
- async lock guard를 넘겨 await하는 경로가 정적/리뷰 검사에서 0건이다.
- shutdown 후 child task/process/permit/DB transaction이 남지 않는다.
- cancellation/complete/delete/unload race 테스트가 deterministic terminal state를 만든다.
