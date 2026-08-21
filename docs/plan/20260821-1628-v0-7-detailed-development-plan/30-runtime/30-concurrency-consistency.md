---
title: "동시성·상태 소유권·일관성"
document_id: "DXB-RUN-030"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-ARC-014", "DXB-ARC-017", "DXB-DOM-024", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029"]
---

# 동시성·상태 소유권·일관성

## 1. 목적

v0.6의 Process/Memory/Grant/Collaboration/Runtime Memory race 및 기존 Task/Execution/Core/Side Effect race를 전부 유지하고, v0.7의 **duplicate CLI command, lost response, reconnect/watch, signal handling** race를 추가한다.

## 2. Ownership 비회귀

v0.6 Ownership Matrix는 그대로다. 추가 Interface 상태는 다음처럼 분류한다.

| 상태 | Owner | 일관성 방식 |
|---|---|---|
| Application Command receipt | Application/target canonical path | CommandId + IdempotencyKey + expected revision |
| public subscription cursor | Contract/event projection boundary | stable event id + cursor/watermark |
| Control Connection | control adapter | ephemeral connection lifecycle |
| CLI local wait/watch | CLI process | local cancellation/join |

CLI/Control state가 Task/Process/Memory/Core/Authorization state를 복제하지 않는다.

## 3. v0.6 Race 유지

v0.6의 Process advance/late result/cancel, Memory retract/promotion/context, ActionGrant consume, collaboration terminate, declassification, MemoryReservation acquire/release, pressure transition, cache pin, shutdown cleanup 및 v0.5/v0.4 race를 삭제·완화하지 않는다.

## 4. v0.7 신규 Race

### Duplicate CLI Mutation vs Commit
- 같은 logical mutation retry는 동일 idempotency identity를 유지한다.
- first commit winner 이후 duplicate delivery는 동일 outcome/receipt를 반환하거나 conflict로 reconcile한다.
- CLI 재시작 때문에 새 key를 만들어 blind duplicate mutation하지 않는다.

### Command Commit vs Response Loss
- commit 이후 connection이 끊겨도 state rollback을 추측하지 않는다.
- retry/query/reconcile가 committed outcome을 확인한다.
- 외부 Side Effect가 Unknown이면 기존 Side Effect reconciliation을 우선한다.

### CLI Watch Disconnect vs Task/Process Completion
- disconnect가 Task/Process terminal transition을 만들지 않는다.
- completion이 먼저 commit되면 reconnect query/stream에서 Canonical result를 관찰한다.
- cursor retention gap이면 explicit resync한다.

### CLI SIGINT vs Runtime Execution
- SIGINT는 기본적으로 local wait/subscription을 cancel한다.
- target Task/Process cancel은 별도 explicit Command다.
- local cancellation token을 Runtime execution owner token과 공유하지 않는다.

### Runtime Restart / Endpoint Recreation vs CLI Reconnect
- reconnect는 protocol/schema negotiation을 다시 수행한다.
- stale socket/session generation을 Domain identity로 사용하지 않는다.
- in-flight command 상태를 로컬 추측으로 확정하지 않고 receipt/reconciliation을 사용한다.

### Slow stdout / Broken Pipe vs Runtime Stream
- bounded client buffer를 사용한다.
- broken pipe가 Runtime failure/Task failure를 생성하지 않는다.
- slow consumer가 internal Runtime queue를 무한 점유하지 않는다.

## 5. Queue / Buffer

기존 item+byte cap과 process-wide memory ceiling을 유지한다. Control/event/CLI 경로도 예외가 아니다.

- server subscription buffer bounded
- client receive buffer bounded
- stdout serialization chunk bounded
- large query pagination/streaming
- slow client gap/disconnect/resync policy 명시

## 6. 검증 기준

- duplicate CLI mutation semantic effect 0.
- response-loss retry duplicate effect 0.
- SIGINT/disconnect가 target Task/Process cancel을 만들지 않음.
- cursor gap을 silent current state로 오판하지 않음.
- slow stdout/broken pipe로 Runtime queue/resource leak 0.
- v0.6 race fixture regression 0.
