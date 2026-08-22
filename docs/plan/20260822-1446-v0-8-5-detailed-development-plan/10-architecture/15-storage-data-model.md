---
title: "저장소와 데이터 모델"
document_id: "DXB-ARC-015"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-ARC-014", "DXB-ARC-011"]
---
# 저장소와 데이터 모델

## 1. Durable owner

Domain aggregate/journal, outbox/inbox, Side Effect ledger, Durable Process, Operation Receipt, schema/version metadata와 snapshot version retention을 durable state로 관리한다. CLI Session, socket, subscriber buffer와 local journal은 Canonical business state가 아니다.

## 2. Atomic mutation boundary

하나의 mutation commit은 적용 가능한 범위에서 다음을 하나의 crash-safe transaction 또는 검증된 atomic protocol로 기록한다.

```text
aggregate revision/state
canonical event/journal
outbox
Operation Receipt + principal/idempotency binding
result/outcome reference
```

response 손실은 commit을 되돌리지 않는다.

## 3. Receipt lookup

P0는 instance 안에서 `(PrincipalRef, IdempotencyKey)`를 유일하게 인덱스한다. 조회는 `OperationId`, `CommandId`, 또는 authenticated principal의 `IdempotencyKey`로 수행한다. `ClientRequestId`는 recovery index가 아니다.

nonterminal receipt는 startup reconcile에서 owner를 재획득하거나 `RecoveryRequired`로 전이하며 단순 TTL로 삭제하지 않는다. terminal binding은 최소 30일 보존한다.

## 4. Snapshot

장시간 DB read transaction을 CLI pagination 수명과 결박하지 않는다. `DXB-ARC-018`에 따라 projection watermark와 versioned row/tombstone 또는 bounded materialized keyset을 사용한다. snapshot pin 때문에 WAL/history가 무제한 성장하지 않도록 disk admission과 expiry를 적용한다.

## 5. CLI local journal

per-command local file은 `CommandId`, `IdempotencyKey`, RequestDigest, operation reference와 state만 저장한다. payload, secret, Authority, outcome을 저장하지 않는다. Runtime receipt가 source of truth다. journal 손실 자체가 자동 replay 권한을 만들지 않는다.

## 6. Schema 분리

`Public Wire Schema ≠ Domain Serialization ≠ Persistence Schema`. CLI upgrade가 data migration을 직접 수행하지 않는다.
