---
title: "동시성·상태 소유권·일관성"
document_id: "DXB-RUN-030"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-ARC-014", "DXB-ARC-017", "DXB-DOM-024"]
---

# 동시성·상태 소유권·일관성

## 1. 목적

다수 Bot/Core/Provider/Plugin이 동시에 실행되어도 data race, deadlock, 상태 혼합, duplicate resume/side effect, Provider drain race, 메모리 폭증이 발생하지 않도록 상태와 실행 ownership을 고정한다.

## 2. Single-Writer / Immutable Read

- Bot/Task/Memory/Routine Canonical write는 Aggregate revision + Command path로 직렬화한다.
- Core는 immutable/versioned snapshot을 읽고 Result/Proposal을 반환한다.
- mutable Working Context는 Core-local이다.
- Registry/Config는 immutable generation swap을 사용한다.
- shared mutable map이나 `RuntimeContext`를 service locator로 사용하지 않는다.

## 3. Ownership Matrix

| 상태 | Owner | 동시성 방식 |
|---|---|---|
| Bot current state | Bot/Application coordinator | revisioned single write |
| Task/Continuation | Task owner | same Unit of Work + expected revision |
| Routine/Occurrence | Routine service | occurrence ID + revision |
| Memory Record | Memory service | immutable revision/conflict |
| Core Lease | Scheduler | lease/fencing |
| Side Effect Ledger | Application/Execution ledger | action key + transactional state |
| Capability Registry | Common Runtime | immutable generation |
| Provider lifecycle/activity | Common Provider Lifecycle/Host | generation + activity/reference guard |
| Provider internal connection/process | Provider lifecycle implementation | bounded, Host-owned lifecycle scope |
| Plugin lifecycle | Plugin Manager | serialized operation + generation |
| UI state | each Interface | Derived only |

## 4. Provider Concurrency Ownership

Provider가 DXBOT 전역 concurrency, retry worker, scheduler, admission queue를 소유하지 않는다.

- Provider Host가 Provider별 in-flight activity를 증가/감소한다.
- `Draining` 이후 신규 activity 획득은 실패한다.
- quiescence는 Common activity/reference가 0인지와 child/process cleanup을 함께 확인한다.
- Provider 내부 protocol worker가 필요하면 Provider lifecycle scope에 묶이고 bounded이며 stop/join 경로를 가진다.
- Provider가 자체 queue를 둘 경우 transport implementation buffer 수준이어야 하며 Common admission을 우회해 새 semantic work를 축적하지 않는다.

## 5. Queue / Channel

모든 channel/queue는 item+byte cap, send failure/closed semantics, backpressure/overflow policy를 가진다. slow consumer 또는 slow Provider 때문에 producer가 unbounded memory를 보유하지 않는다.

## 6. 주요 Race

### Cancel vs Complete
Task terminal revision에서 한 쪽만 승리하고 late outcome은 history로 남긴다.

### Waiting Resume vs Duplicate Child
Continuation revision/resume guard로 한 번만 resume한다.

### Routine Trigger vs Restart
occurrence ID claim/dedup으로 Task 생성이 한 번이다.

### Provider Drain vs New Execution
Registry generation에서 Draining Provider를 신규 selection에서 제외하고 Host activity acquisition이 새 호출을 차단한다. 이미 pin된 Execution은 drain policy를 따른다.

### Provider Stop vs Late Callback
generation/fencing/activity guard가 stop 이후 late callback의 Canonical commit을 허용하지 않는다.

### Provider Health vs Lifecycle Transition
health observation과 administrative lifecycle transition을 별도 revision으로 처리해 stale health sample이 Ready/Draining 상태를 되돌리지 못하게 한다.

### Plugin Disable vs Callback
Plugin generation과 Provider generation을 각각 fencing하고 제공 Provider를 먼저 Draining한다.

### Side Effect Retry vs Unknown Outcome
ledger가 Unknown이면 Provider/Retry worker가 external call을 재수행할 권한을 얻지 못한다.

### Lease Expiry vs Result
fencing token/revision으로 stale result commit을 거부한다.

## 7. Lock / Async 규칙

- lock order 문서화
- async mutex guard를 잡고 external I/O await 금지
- blocking work는 bounded pool
- cancellation-safe cleanup
- spawn은 supervisor/lifecycle owner 경유
- child task/process join 없이 lifecycle 완료 처리 금지
- atomics는 ordering 근거/모델 test 필요

## 8. Consistency Level

Canonical Command는 strong revision check를 사용한다. Projection/Index/UI는 eventual일 수 있으나 watermark/stale를 표시한다. Provider health/metrics는 근사치일 수 있으나 Lease/Permission/Side Effect authority를 대체하지 않는다.

## 9. 검증 기준

- cancel/complete, Waiting resume, routine restart, provider drain/stop/health, plugin disable, side-effect unknown race에 deterministic test가 있다.
- Draining Provider가 신규 Host activity를 획득하지 못한다.
- stale Provider generation이 Canonical write를 수행하지 못한다.
- production unbounded channel이 없다.
- shutdown 후 owned task/process/permit/activity/registry generation leak가 0이다.
