---
title: "Command·Receipt·Directive·Event·State 모델"
document_id: "DXB-ARC-014"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-ARC-010", "DXB-GOV-002"]
---
# Command·Receipt·Directive·Event·State 모델

## 1. Atomic command model

```text
request authenticate / digest binding / authorization / revision
→ aggregate decide
→ atomic state + canonical event + outbox + receipt + audit-intent commit
→ projection/public event publication
→ response or reconciliation
```

## 2. 상태 계층

| 계층 | Owner | 상태 의미 |
|---|---|---|
| Local Journal | CLI | 송신 전후 로컬 관찰·복구 |
| Operation Receipt | Application Contract | request binding과 operation commit/recovery |
| Directive | Control owner | safe-point 적용 진행 |
| Target Lifecycle | Domain owner | Bot/Task/Process 등 대상 상태 |

한 계층의 상태를 다른 계층의 성공으로 간주하지 않는다.

## 3. Operation Receipt

```text
Accepted
├─→ Committed
├─→ Rejected
├─→ Superseded
└─→ RecoveryRequired → Committed | Rejected | Superseded
```

- `Accepted`: key/digest/principal binding과 receipt가 durable하다.
- `Committed`: 해당 Application operation의 canonical mutation이 commit됐다.
- `Rejected`: canonical effect 없이 durable rejection이 결정됐다.
- `Superseded`: revision/race에서 다른 결정이 승리했다.
- `RecoveryRequired`: 자동 판정이 안전하지 않아 explicit reconcile이 필요하다.

`AwaitingSafePoint`는 Receipt 상태가 아니라 Directive 상태다.

## 4. Directive와 Target

cancel/suspend/redirect operation이 Directive를 만들면 Receipt `Committed`는 Directive 생성 commit을 뜻한다. Directive `Applied` 또는 Task `Cancelled`는 별도 wait predicate로 관찰한다.

## 5. Event와 Projection

Canonical Event는 storage/replay owner가 소유한다. Public Event는 current authorization, redaction, schema mapping, resource accounting을 거쳐 생성한다. Projection은 rebuild 가능한 Derived State이며 Receipt, Authority, Memory truth를 대체하지 않는다.

## 6. Query/Subscription

snapshot은 stable ordering+ID tie-breaker와 source watermark를 고정한다. public stream은 at-least-once이며 gap은 terminal/resync record로 노출한다. broken pipe 때문에 terminal record가 전달되지 않을 수 있으므로 client는 exit/receipt/cursor로 reconcile한다.
