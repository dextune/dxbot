---
title: "저장소와 데이터 모델"
document_id: "DXB-ARC-015"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-011"]
---

# 저장소와 데이터 모델

## 1. Persistent owner

Domain aggregate/journal, outbox/inbox, Side Effect ledger, Durable Process, Operation Receipt, schema/version metadata를 durable state로 관리한다. CLI Session, socket, terminal state, subscriber buffer는 Canonical business state가 아니다.

## 2. Operation Receipt storage

최소 저장 의미:

`CommandId, IdempotencyKey, PrincipalRef, Action, TargetRef, RequestDigest, protocol/schema version, submitted_at, state, outcome/error ref, retry disposition, receipt revision, retention class`.

- nonterminal receipt는 terminal/reconcile 전 TTL 삭제 금지
- terminal receipt와 key binding은 P0 기본 최소 30일 유지
- GC 후 요청은 `operation-expired`로 실패하며 blind replay하지 않음
- payload/secret 원문은 receipt에 중복 저장하지 않고 canonical command data/ref를 사용

## 3. Runtime Instance metadata

persistent data root는 `InstanceId`, data schema version, last clean shutdown, next HostGeneration을 가진다. endpoint path, PID, socket, readiness는 runtime directory의 ephemeral metadata이며 Domain identity가 아니다.

## 4. Cursor와 subscriber state

Query snapshot metadata는 retention 동안 durable 또는 재현 가능한 owner를 가진다. subscriber socket/buffer를 durable truth로 저장하지 않는다. Event Cursor는 event/projection retention을 참조하고 expired면 resync한다.

## 5. CLI local journal

bounded non-canonical reference만 저장하며 Runtime receipt와 conflict할 때 Runtime이 우선한다. local journal 손실이 duplicate effect를 허용하지 않도록 사용자 제공 `--request-id`와 operation query를 지원한다.

## 6. Schema 분리

```text
Public Wire Schema ≠ Domain Serialization ≠ Persistence Schema
```

protocol/schema compatibility와 data migration을 별도 version으로 관리한다.

## 7. 검증 기준

- CLI-specific Canonical table 0
- receipt/payload/secret 불필요 중복 0
- Runtime restart 후 same InstanceId/Domain ID 유지
- expired receipt/cursor가 explicit error
