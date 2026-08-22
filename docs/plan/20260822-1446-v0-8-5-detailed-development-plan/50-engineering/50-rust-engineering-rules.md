---
title: "Rust 구현 규칙 v0.8.5"
document_id: "DXB-ENG-050"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-ARC-011", "DXB-ARC-018", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042", "DXB-RUN-030"]
---
# Rust 구현 규칙 v0.8.5

## Boundaries

```text
CLI → application-contract + control client + narrow host-lifecycle
Control adapter → application ports
Runtime composition → application/storage/provider-host
```

CLI→Domain/Runtime/Storage/concrete Provider, Control→Store direct mutation, Provider→Application internals를 금지한다.

## Contract source

`application-contract` logical boundary가 operation/input/output/error/exit metadata를 소유한다. parser/help/schema/golden을 생성하고 문자열 operation registry를 client/server에 복사하지 않는다.

## Client submission

- `CommandId`, `IdempotencyKey`, `ClientRequestId`를 distinct newtype으로 둔다.
- RequestDigest canonicalization은 locale, hash-map order, incidental JSON ordering에 의존하지 않는다.
- per-command journal을 exclusive create하고 write/fsync/parent-fsync 후 send한다.
- multi-process global journal map atomic replace는 금지한다.
- journal capacity failure는 send 전에 반환한다.
- payload 없는 crash recovery에서 blind replay하지 않는다.

## Bootstrap/security

Principal은 server-side peer mapping 결과 타입이며 wire request principal과 혼용하지 않는다. default policy refs는 immutable generation으로 저장하고 resolved result를 반환한다.

## Storage

M1A profile을 통과한 adapter만 P0 persistence port를 구현한다. CLI pagination lifetime과 DB transaction guard를 공유하지 않는다. compaction과 disk admission은 active snapshot/receipt/process reference를 고려한다.

## Async/resource

모든 task/channel/cache는 owner/cancel/join 및 item+byte cap을 가진다. async lock guard를 잡은 채 slow I/O, serialization, fsync를 await하지 않는다. blocking file sync는 bounded blocking boundary로 격리한다.

## Safe writer

Linux writer는 opened parent directory 기준 no-follow, exclusive temp, bounded write, file fsync, atomic no-replace publish, parent fsync를 하나의 owned component로 제공한다. path check 후 일반 rename으로 overwrite하는 구현은 금지한다.

## Lifecycle

P0 Bot enum에 Delete state를 넣지 않는다. Runtime graceful stop과 host escalation token/type을 분리한다. local timeout/cancel token과 Runtime Task token을 공유하지 않는다.

## Harness

Reference Provider와 real Harness Adapter는 같은 Provider Host pipeline을 사용한다. production unavailable을 Reference Provider로 silent fallback하지 않는다.
