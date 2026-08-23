---
title: "Command·Receipt·Directive·Event·State 모델"
document_id: "DXB-ARC-014"
version: "0.8.9"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-ARC-010", "DXB-GOV-002"]
---
# Command·Receipt·Directive·Event·State 모델

## Request acceptance boundary

```text
parse/decode
→ authenticate Principal
→ validate IdempotencyKey principal/epoch
→ Instance-global CommandId lookup + Principal-scoped IdempotencyKey lookup
→ [existing] stored principal/key/digest binding 비교 후 current visibility로 Receipt projection
→ [absent] selector resolve → authorize → policy resolve
→ operation accept/decide
→ atomic state/event/receipt/binding/audit-intent commit
```

CommandId가 다른 Principal이나 key에 이미 결박됐거나 key가 다른 CommandId/digest에 결박됐으면 존재 정보를 누출하지 않는 conflict다. existing binding은 retry 당시 alias/authorization 변화 때문에 사라지지 않는다. observation response의 payload/redaction은 current visibility policy를 적용할 수 있다.

## PreAcceptError와 Receipt

parser, malformed input, unauthenticated peer, principal-mismatched/expired key처럼 operation binding을 만들 수 없는 실패는 Receipt 없는 `PreAcceptError`다.

Operation Receipt Owner는 `application-operation`이다.

```text
Accepted
├─ Committed
├─ Rejected
├─ Superseded
└─ RecoveryRequired → Committed | Rejected | Superseded
```

`Committed`는 Application operation의 canonical effect가 commit됐다는 뜻이다. Task가 생성되고 이후 admission에서 `Rejected/Deferred`가 된 경우에도 operation은 `Committed`다.

Approval 필요 시 operation은 `Accepted` + `ApprovalRef`를 유지한다. approve/deny가 원 operation intent를 재평가해 terminal Receipt로 전진시킨다.

Directive는 별도 상태기계이며 `Applied`는 Directive가 실제 적용된 경우에만 사용한다. Target terminal은 별도 Query/Subscription으로 관찰한다.

## Retention identity

full Receipt를 compact하더라도 CommandId, principal-scoped key digest, RequestDigest, terminal disposition, expiry를 가진 최소 tombstone은 acceptance/recovery horizon 동안 유지한다. tombstone 또는 expired key는 신규 operation으로 승격되지 않는다.
