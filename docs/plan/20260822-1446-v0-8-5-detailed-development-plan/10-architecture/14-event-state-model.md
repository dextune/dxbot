---
title: "Command·Event·State·Projection 모델"
document_id: "DXB-ARC-014"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-GOV-002"]
---

# Command·Event·State·Projection 모델

## 1. Commit 모델

```text
Command validation/authorization/revision
→ aggregate decide
→ atomic journal/state/outbox/operation-receipt commit
→ projection/event publication
→ response or later reconciliation
```

Domain State, Domain Event, Operation Receipt, Projection, public Subscription Event는 별도 타입과 owner를 가진다.

## 2. Operation Receipt

receipt는 Application command journal에 원자적으로 기록한다. commit 후 response가 손실되어도 동일 key+digest 요청은 새 effect를 만들지 않고 기존 receipt/outcome을 반환한다. key가 같고 digest가 다르면 conflict다.

상태:

```text
Accepted → Pending/AwaitingSafePoint → Committed
         ↘ Rejected | Superseded | RecoveryRequired
```

receipt는 authorization·expected revision을 대체하지 않는다.

## 3. Event와 Projection

Canonical Event는 storage/replay owner가 소유한다. public event는 current authorization, schema mapping, redaction, resource cap을 거쳐 생성한다. Projection은 rebuild 가능한 Derived State이며 receipt, authorization, Memory truth를 대체하지 않는다.

## 4. Query Snapshot과 cursor

snapshot은 stable ordering과 source watermark를 고정한다. cursor는 snapshot 내부의 다음 position이며 query digest를 바꿔 재사용할 수 없다. snapshot retention 종료는 explicit `cursor-expired`다.

## 5. Subscription terminal

public stream은 at-least-once다. event identity로 dedupe할 수 있고 gap은 terminal/resync record로 노출한다. CLI disconnect와 broken pipe는 Domain Event가 아니며 Task/Process terminal transition을 만들지 않는다.

## 6. 검증 기준

- duplicate delivery duplicate effect 0
- response-loss outcome 복구 가능
- cursor foreign/filter reuse 0
- public event가 internal queue item을 노출하지 않음
- partial stream을 complete로 표시 0
