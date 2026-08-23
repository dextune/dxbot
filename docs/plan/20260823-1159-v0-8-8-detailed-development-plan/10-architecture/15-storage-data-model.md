---
title: "저장소와 데이터 모델"
document_id: "DXB-ARC-015"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-ARC-014", "DXB-ARC-011"]
---
# 저장소와 데이터 모델

## Operation binding index

P0는 Instance 안에서 다음 uniqueness를 동시에 유지한다.

```text
(PrincipalRef, IdempotencyKey) -> CommandId, RequestDigest, OperationId
(PrincipalRef, CommandId)      -> IdempotencyKey, RequestDigest, OperationId
```

same key 또는 same CommandId가 다른 counterpart/digest와 결박되면 conflict다.

existing binding lookup은 current selector resolution보다 먼저 수행한다. absent binding에 대해서만 ResolvedBindingDigest를 계산하고 atomic commit한다.

## Atomic mutation

적용 가능한 mutation은 aggregate state/event/outbox, Receipt binding, ResolvedBindingDigest, result ref, required AuditIntent를 한 crash-safe transaction 또는 증명된 atomic protocol로 기록한다.

Membership mutation은 Membership row/generation과 Security owner의 AuthorityBinding create/revoke, AuditIntent를 하나의 Unit of Work로 commit한다. 중간 성공 상태를 외부에 노출하지 않는다.

IdempotencyKey는 epoch-bearing format/horizon을 유지한다. horizon 밖 key는 binding이 compact됐더라도 신규 mutation으로 재해석하지 않는다.

Snapshot/Projection은 bounded하고 rebuild 가능하며 CLI pagination 동안 long-lived DB transaction을 유지하지 않는다.
