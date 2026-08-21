---
title: "동시성·상태 소유권·일관성"
document_id: "DXB-RUN-030"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-ARC-014", "DXB-DOM-024"]
---

# 동시성·상태 소유권·일관성

## 1. 목적

다수 Bot/Core/Provider/Plugin이 동시에 실행되어도 data race, deadlock, 상태 혼합, duplicate resume/side effect, 메모리 폭증이 발생하지 않게 소유권과 직렬화 지점을 고정한다.

## 2. Single-Writer / Immutable Read

- Bot/Task/Memory/Routine의 Canonical write는 Aggregate revision + Command path로 직렬화한다.
- Core는 immutable/versioned snapshot을 읽고 Result/Proposal을 반환한다.
- mutable Working Context는 Core-local이다.
- Registry/Config는 immutable generation swap을 사용한다.
- shared mutable map을 편의상 전역 서비스 locator로 사용하지 않는다.

## 3. Ownership Matrix

| 상태 | Owner | 동시성 방식 |
|---|---|---|
| Bot current state | Bot/Application coordinator | revisioned single write |
| Task/Continuation | Task owner | same Unit of Work + expected revision |
| Routine/Occurrence | Routine service | occurrence ID + revision |
| Memory Record | Memory service | immutable revision/conflict |
| Core Lease | Scheduler | lease/fencing |
| Side Effect Ledger | Application/Execution ledger | action key + transactional state |
| Capability Registry | Runtime registry | immutable generation |
| Plugin lifecycle | Plugin Manager | serialized operation + generation |
| Provider runtime handle | Provider owner | scoped lock/supervisor |
| UI state | each Interface | Derived only |

## 4. Queue / Channel

모든 channel/queue는 item+byte cap, send failure/closed semantics, backpressure/overflow policy를 가진다. slow consumer 때문에 producer가 unbounded memory를 보유하지 않는다.

## 5. 주요 Race

### Cancel vs Complete
Task terminal revision에서 한 쪽만 승리하고 late outcome은 history로 남긴다.

### Waiting Resume vs Duplicate Child
Continuation revision/resume guard로 한 번만 resume한다.

### Routine Trigger vs Restart
occurrence ID claim/dedup으로 Task 생성이 한 번이다.

### Provider Drain vs New Execution
registry generation에서 draining Provider를 신규 selection에서 제외하며 이미 pin된 Execution은 quiescence 정책을 따른다.

### Plugin Disable vs Callback
신규 callback/selection을 막고 generation/lease를 fencing한 뒤 in-flight drain한다.

### Side Effect Retry vs Unknown Outcome
ledger state가 Unknown이면 retry worker가 external call을 재수행할 권한을 얻지 못한다.

### Lease Expiry vs Result
fencing token/revision으로 stale result commit을 거부한다.

## 6. Lock / Async 규칙

- lock order 문서화
- async mutex guard를 잡고 external I/O await 금지
- blocking work는 bounded pool
- cancellation-safe cleanup
- spawn은 supervisor/registry owner 경유
- child task/process join 없이 lifecycle 완료 처리 금지
- atomics는 ordering 근거/모델 test 필요

## 7. Consistency Level

Canonical Command는 strong revision check를 사용한다. Projection/Index/UI는 eventual일 수 있으나 watermark/stale를 표시한다. Provider health/metrics는 근사치일 수 있으나 Lease/Permission/Side Effect authority를 대체하지 않는다.

## 8. 검증 기준

- cancel/complete, duplicate child, routine restart, provider drain, plugin disable, side-effect unknown race에 deterministic test가 있다.
- stale Core/Plugin generation이 Canonical write를 수행하지 못한다.
- unbounded channel이 production에 없다.
- shutdown 후 owned task/process/permit/registry generation leak가 0이다.
