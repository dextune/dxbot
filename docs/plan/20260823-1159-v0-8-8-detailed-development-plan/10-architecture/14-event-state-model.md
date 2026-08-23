---
title: "Command·Receipt·Directive·Event·State 모델"
document_id: "DXB-ARC-014"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-ARC-010", "DXB-GOV-002"]
---
# Command·Receipt·Directive·Event·State 모델

## Request acceptance boundary

```text
parse/decode → authenticate Principal
→ existing CommandId/IdempotencyKey binding lookup
→ [existing] stored digest/binding 비교 후 stored receipt 반환
→ [absent] selector resolve → authorize → policy resolve
→ operation accept/decide → atomic state/event/receipt/audit-intent commit
```

existing binding은 retry 당시의 alias/authorization 변화 때문에 사라지지 않는다. 단, observation response는 current redaction policy를 적용할 수 있다.

## PreAcceptError와 Receipt

parser, malformed input, unauthenticated peer, expired key format처럼 operation binding을 만들 수 없는 실패는 Receipt 없는 `PreAcceptError`다.

Operation Receipt Owner는 `application-operation`이다.

```text
Accepted
├─ Committed
├─ Rejected
├─ Superseded
└─ RecoveryRequired → Committed | Rejected | Superseded
```

`Committed`는 application operation의 canonical effect가 commit됐다는 뜻이다. Task가 생성되고 이후 admission에서 `Rejected/Deferred`가 된 경우에도 operation은 Committed다.

Approval 필요 시 operation은 `Accepted` + `ApprovalRef`를 유지한다. approve/deny가 원 operation intent를 재평가해 terminal Receipt로 전진시킨다.

Directive는 별도 상태기계이며 `Applied`는 Directive가 실제 적용된 경우에만 사용한다. Target terminal은 별도 Query/Subscription으로 관찰한다.
