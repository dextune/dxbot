---
title: "Headless Application Contract와 crash-safe submission"
document_id: "DXB-IFC-040"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-DOM-026", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-032", "DXB-RUN-033", "DXB-RUN-038", "DXB-ARC-014", "DXB-ARC-015"]
---
# Headless Application Contract와 crash-safe submission

## 1. Contract Kernel

```text
Command      = durable mutation intent + Operation Receipt
Query        = bounded snapshot/live read
Subscription = cursor 기반 at-least-once observation
Host Action  = Runtime 부재/비가용 시 process bootstrap or explicit escalation
```

generic method/payload dispatch, Domain Event와 public event 공유, Provider capability와 Application operation 병합을 금지한다.

## 2. Authenticated Request Context

```text
ProtocolVersion / SchemaVersion / ClientVersion
ClientRequestId
InstanceId
Deadline?
Trace/Correlation?
RequestedPrincipalContext?  // hint only
AuthenticatedPeerContext    // server-created, wire input 아님
ResolvedPrincipalRef        // server-created
```

P0 Runtime은 peer credential과 Instance metadata로 Principal을 resolve한다. client가 보낸 PrincipalRef를 Authority로 신뢰하지 않는다.

## 3. Selector

Canonical ID가 최종 식별자다. exact name/alias는 explicit scope에서 principal-visible 후보가 하나일 때만 resolve한다. fuzzy/prefix/last-used mutation은 금지한다. destructive/high-risk mutation은 ID와 expected revision/generation을 요구한다.

## 4. Command identity

```text
CommandId          // logical mutation identity
IdempotencyKey     // principal-local unique key
ClientRequestId    // one transmission/attempt correlation
RequestDigest      // canonical action/target/payload/schema binding
```

P0에서 `(PrincipalRef, IdempotencyKey)`는 instance 안에서 하나의 logical mutation에만 사용한다. same key+same digest는 existing receipt/outcome, different binding은 conflict다. `ClientRequestId`는 retry마다 달라질 수 있고 receipt recovery key가 아니다.

## 5. Client submission protocol

```text
1. parse + local validation
2. resolve/generate CommandId and IdempotencyKey
3. canonical RequestDigest 계산
4. per-command Prepared journal exclusive create
5. file fsync + parent directory fsync
6. transmit with fresh ClientRequestId
7. Runtime authenticate/resolve principal
8. lookup key binding or atomic mutation commit
9. append Sent/Observed journal record and fsync
10. terminal 후 retention/compaction
```

journal capacity가 full이면 송신 전에 실패한다. payload/secret/Authority/outcome은 journal에 저장하지 않는다. CLI crash 후 자동 replay하려면 caller가 원래 typed payload를 다시 제공해야 한다. payload가 없으면 operation lookup만 수행하고 blind mutation을 만들지 않는다.

## 6. Operation Receipt

```text
operation_id, command_id, idempotency_key
principal_ref, action, target_ref?, request_digest
protocol/schema version
state: Accepted | Pending | AwaitingSafePoint | Committed
     | Rejected | Superseded | RecoveryRequired
outcome/error/retry/reconciliation refs
receipt_revision, retention
```

조회 key는 exactly one of `OperationId`, `CommandId`, authenticated principal의 `IdempotencyKey`다. receipt는 authorization/revision을 대체하지 않는다.

## 7. Operation metadata SSOT

Rust `application-contract` source는 operation별 다음 metadata를 소유한다.

```text
operation name and kind
CLI command path
selector/target kind
required revision/generation
typed payload fields and input source
option conflicts/defaults
wait capability/default
output forms
security/resource classification
Acceptance ID
```

`DXB-IFC-041/042`, parser/help/schema/golden은 이 metadata에서 생성·검증한다.

## 8. Query/Subscription

Query는 stable sort+ID tie-breaker, snapshot watermark, opaque integrity-protected cursor를 사용한다. cursor는 query/filter/sort/scope/principal/snapshot/schema/expiry에 결박된다.

Subscription은 at-least-once다. JSONL은 header/item|event/progress/terminal을 구분하고 terminal 없이 complete success로 간주하지 않는다. gap/expired/reauth는 silent start-now가 아니라 explicit resync다.

## 9. Error/Retry

typed error는 class, safe message, target/candidates/current revision, RetryDisposition, operation/cursor hint를 가진다. retry/security/exit를 문자열 matching으로 결정하지 않는다.

## 10. Version/Schema

Public DTO ≠ Domain Aggregate ≠ Persistence Row. Rust contract source에서 schema, golden, operation/input/help/exit metadata를 결정적으로 생성한다.
