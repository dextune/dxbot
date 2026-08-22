---
title: "저장소와 데이터 모델"
document_id: "DXB-ARC-015"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-ARC-014", "DXB-ARC-011"]
---
# 저장소와 데이터 모델

## 1. Durable owner

Domain state/journal, outbox/inbox, Operation Receipt, Directive, Side Effect Ledger, Approval/Authority/ActionGrant, Durable Process, Audit Intent, schema/version metadata와 snapshot retention을 durable state로 관리한다. CLI Session/socket/local journal은 Canonical business state가 아니다.

## 2. Atomic mutation boundary

적용 가능한 mutation은 다음을 하나의 crash-safe transaction 또는 증명된 atomic protocol로 기록한다.

```text
aggregate revision/state
canonical event/journal
outbox
Operation Receipt + Principal/IdempotencyKey/RequestDigest binding
ResolvedBindingDigest + authorization/policy generation refs
result/outcome reference
audit intent when required
```

response 손실은 commit을 되돌리지 않는다.

## 3. Idempotency index와 bounded horizon

P0 IdempotencyKey 형식은 `ik1:<issuance-epoch>:<random-128+>`다. Runtime은 configured acceptance horizon과 bounded future skew를 검증한다.

- horizon 안 same principal/key+same RequestDigest는 existing receipt를 반환한다.
- same key+different digest는 conflict다.
- horizon 밖 key는 binding이 compact됐더라도 `idempotency-key-expired`로 거부한다.
- key binding/tombstone은 acceptance horizon+recovery margin 이상 유지한다.
- 정확한 기간 숫자는 M6 policy/ADR에서 freeze한다.

따라서 오래된 key를 새로운 mutation key로 재해석하지 않으면서 metadata를 무한 보존하지 않는다.

## 4. Nonterminal recovery

nonterminal Receipt, Directive, Side Effect, Process는 owner kind, revision, lease/deadline, last progress, reconciliation policy를 가진다. startup에서 owner가 없으면 safe resume 또는 `RecoveryRequired`로 결정한다. 단순 TTL 삭제를 금지한다.

## 5. Snapshot과 local journal

CLI pagination 수명 동안 DB read transaction을 유지하지 않는다. versioned projection 또는 bounded materialized keyset을 사용한다. Local journal은 payload/secret/Authority/outcome을 저장하지 않으며 Runtime receipt가 source of truth다.

## 6. Schema 분리

`Public Wire Schema ≠ Domain Serialization ≠ Persistence Schema`. CLI upgrade가 data migration을 직접 수행하지 않는다.
