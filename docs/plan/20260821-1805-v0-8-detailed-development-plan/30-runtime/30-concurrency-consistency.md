---
title: "동시성·상태 소유권·일관성"
document_id: "DXB-RUN-030"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-DOM-024", "DXB-DOM-027", "DXB-DOM-029"]
---

# 동시성·상태 소유권·일관성

## 1. 목적

Runtime Host, Application Command, selector, snapshot Query, Subscription, file export가 동시에 실행될 때 race winner와 stale write 차단을 명확히 한다. lock 구현보다 revision·generation·fencing·idempotency 의미를 우선한다.

## 2. Ownership Matrix

| 상태 | Owner | Consistency key |
|---|---|---|
| Domain Aggregate | 해당 Domain owner | ID + revision |
| Operation Receipt | Application Command owner | CommandId + IdempotencyKey + RequestDigest |
| Runtime Instance | Runtime composition | InstanceId + HostGeneration |
| Query Snapshot | Query/projection owner | SnapshotId/Watermark + schema |
| Subscription cursor | event/projection boundary | EventId + Cursor/Watermark |
| CLI local wait/journal | CLI process | bounded request reference; non-canonical |
| export temp file | CLI safe writer | path identity + exclusive create |

## 3. Runtime start/stop race

### concurrent start

두 `runtime start`가 같은 InstanceId/data root를 대상으로 하면 user service manager와 instance lock/fencing이 하나의 winner를 정한다.

- winner만 새 HostGeneration을 활성화한다.
- loser는 이미 시작 중/Ready인 generation을 발견하고 동일 instance descriptor를 반환한다.
- 둘이 각기 다른 endpoint/data writer가 되는 것을 금지한다.

### start vs stop/restart

Host lifecycle operation은 instance descriptor generation과 compare-and-act한다. stale stop이 새 generation을 종료하지 못한다. readiness 전 stop은 startup cancellation 또는 supervisor stop으로 귀결되며 Domain Task cancel로 매핑하지 않는다.

## 4. Command race

### same key / same digest
기존 receipt/outcome을 반환한다. commit 수는 하나다.

### same key / different digest or binding
principal/action/target/payload/schema 중 하나라도 다르면 `idempotency-conflict`다. 첫 요청을 덮어쓰거나 두 번째 mutation을 실행하지 않는다.

### selector vs mutation
selector resolution 결과의 ResourceId/revision을 command에 고정하고 commit 직전 owner가 current authorization와 expected revision을 재검증한다. name rename이나 authority revoke가 race하면 stale/forbidden으로 실패한다.

### terminal race
Task completion/cancel, approval consume/revoke, process advance/cancel, Memory promote/retract는 revision/fencing으로 하나의 authoritative decision만 commit한다.

## 5. Snapshot Query race

snapshot-consistent Query는 first page에서 SnapshotId/Watermark와 stable sort를 고정한다. concurrent insert/update/delete는 current snapshot 결과를 변경하지 않는다. cursor가 다른 filter/sort/scope/snapshot/schema에 사용되면 `foreign-cursor`다.

live/eventual Query는 snapshot 보장을 주장하지 않고 response에 source watermark/observed_at를 포함한다.

## 6. Subscription race

- reconnect 시 current principal/version을 재검증한다.
- connection drop과 event commit이 경쟁하면 cursor resume 또는 explicit gap/resync로 관찰한다.
- local SIGINT/broken pipe는 subscriber만 정리하고 target Task/Process를 변경하지 않는다.
- slow consumer는 bounded buffer 이후 disconnect/gap policy를 적용한다.

## 7. Export race

output path는 no-follow/exclusive temp create 후 write/fsync/close/atomic rename을 사용한다. destination가 이미 존재하거나 symlink/special file로 바뀌면 실패한다. partial failure에서 temp file은 안전하게 삭제하거나 explicit recoverable path로 보고한다.

## 8. 검증 기준

- concurrent start 100회 fixture에서 active HostGeneration 1개.
- same key/different digest가 mutation 0회와 typed conflict를 반환.
- rename/revoke/revision race에서 잘못된 대상 write 0.
- snapshot iteration 중 concurrent mutation에도 snapshot 내 duplicate/missing 0.
- reconnect/broken pipe 뒤 subscriber·task·file handle leak 0.
- stale stop/endpoint descriptor가 새 Runtime generation에 영향 0.
