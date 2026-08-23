---
title: "저장소와 데이터 모델"
document_id: "DXB-ARC-015"
version: "0.8.9"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-ARC-014", "DXB-ARC-011"]
---
# 저장소와 데이터 모델

## Operation binding indexes

P0 Runtime Instance는 다음 uniqueness를 동시에 유지한다.

```text
(InstanceId, CommandId)
  -> PrincipalRef, IdempotencyKeyDigest, RequestDigest, OperationId | TerminalTombstone

(PrincipalRef, IdempotencyKey)
  -> CommandId, RequestDigest, OperationId | TerminalTombstone
```

인증 직후 IdempotencyKey의 Principal/issuance epoch를 검증한다. 그 다음 두 index를 모두 조회한다. 한쪽만 존재하거나 principal, counterpart, digest가 다르면 conflict이며 신규 operation을 생성하지 않는다. 둘 다 absent일 때만 selector resolution, authorization, policy resolution, `ResolvedBindingDigest` 계산으로 진행한다.

## Atomic mutation

적용 가능한 mutation은 aggregate state/event/outbox, 두 binding index, Receipt, `ResolvedBindingDigest`, result ref, required AuditIntent를 하나의 crash-safe transaction 또는 증명된 atomic protocol로 기록한다.

Membership mutation은 Membership row/generation과 Security owner의 AuthorityBinding create/revoke, AuditIntent를 하나의 Unit of Work로 commit한다. 중간 성공 상태를 외부에 노출하지 않는다.

## Retention과 compaction

- nonterminal Receipt, Approval continuation, reconciliation owner가 참조하는 binding은 compact하지 않는다.
- key가 acceptance horizon 안에 있는 동안 global CommandId와 principal key uniqueness를 잃지 않는다.
- full Receipt payload를 제거할 때도 최소 tombstone은 CommandId, principal/key digest, RequestDigest, terminal disposition, expiry를 보존한다.
- horizon 밖 key 또는 expired tombstone lookup은 typed `idempotency-expired`/`recovery-required`로 종료하며 absent 신규 mutation으로 재해석하지 않는다.
- 구체 보존 시간과 byte budget은 `DXB-ARC-018` 실행 증거 후 policy/ADR로 고정한다.

Snapshot/Projection은 bounded하고 rebuild 가능하며 CLI pagination 동안 long-lived DB transaction을 유지하지 않는다.
