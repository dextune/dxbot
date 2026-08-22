---
title: "자원·비용·Runtime Memory Admission 관리"
document_id: "DXB-RUN-031"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-RUN-030", "DXB-DOM-024"]
---
# 자원·비용·Runtime Memory Admission 관리

장기 실행 Runtime의 process-wide memory/cost/concurrency를 하나의 Resource Governor가 bounded하게 관리한다. Core count는 memory ceiling이 아니다.

## Runtime envelope

control/recovery headroom, canonical working set, execution/context/provider activity, query snapshot, subscription buffer, receipt/idempotency metadata, projection/cache, export/recovery temporary bytes를 구분해 accounting한다. item cap과 byte cap을 모두 둔다.

## Pressure state

`Normal → Constrained → Critical → Emergency`.

Critical/Emergency에서는 신규 semantic work를 commit 전에 거부하고 control/recovery headroom을 보존한다. OOM을 retryable response로 약속하지 않는다.

## Local Journal capacity

- `PreparedUnsent`: Runtime binding이 없으며 explicit user abandon 또는 age/cap policy로 `Abandoned` 전환 가능하다.
- `SentUnknown`: operation lookup으로 binding 유무를 확인하기 전 자동 삭제·재전송하지 않는다.
- `Observed`: Runtime receipt ref를 보존한다.
- `Terminal`: retention 후 compact 가능하다.
- `Abandoned`: local-only state이며 Runtime effect 없음이 확인되거나 송신 전 상태여야 한다.

capacity가 full이면 신규 mutation을 송신 전에 실패한다. eviction으로 uncertain mutation을 숨기지 않는다.

## Durable nonterminal capacity

Receipt/Directive/SideEffect/Process `RecoveryRequired`가 budget을 점유하면 recovery admission을 우선하고 신규 low-priority mutation을 거부한다. nonterminal을 TTL로 삭제하지 않는다. operator reconcile이나 safe terminal decision만 capacity를 해제한다.

## Paging/stream/export

`--all`은 bounded page fetch→incremental encode/write→release→next cursor 순서다. subscriber와 writer buffer는 item+byte cap, slow-consumer, cancellation, cleanup을 가진다.
