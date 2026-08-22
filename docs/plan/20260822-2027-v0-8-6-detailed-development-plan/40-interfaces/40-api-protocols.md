---
title: "Headless Application Contract와 crash-safe submission"
document_id: "DXB-IFC-040"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-DOM-026", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-032", "DXB-RUN-033", "DXB-RUN-036", "DXB-RUN-038", "DXB-ARC-014", "DXB-ARC-015"]
---
# Headless Application Contract와 crash-safe submission

## 1. Contract Kernel

```text
Command      = durable Application mutation + Operation Receipt
Query        = bounded snapshot/live read
Subscription = cursor 기반 at-least-once observation
Host Action  = Runtime process bootstrap/status/explicit stop
```

generic method/payload dispatch, Domain Event와 public event 공유, Provider Capability와 Application operation 병합을 금지한다.

## 2. Authenticated context

wire request에는 ProtocolVersion, SchemaVersion, ClientVersion, ClientRequestId, InstanceId, Deadline?, Trace?가 있다. AuthenticatedPeerContext와 ResolvedPrincipalRef는 server-created이며 client input이 아니다.

## 3. Digest 분리

```text
RequestDigest = H(
  canonicalization_version,
  protocol/schema,
  operation_name,
  raw_typed_selector,
  typed_payload,
  semantic_options
)
```

semantic options에는 requested budget/deadline/mode가 포함될 수 있다. profile, rendering, output path, wait predicate, ClientRequestId, server principal/resolved target은 제외한다.

```text
ResolvedBindingDigest = H(
  RequestDigest,
  ResolvedPrincipalRef,
  ResolvedResourceRef + revision/generation,
  AuthorizationDecisionRef,
  effective policy/provider generations
)
```

server가 계산해 Receipt/Audit에 기록한다.

## 4. Identity와 IdempotencyKey

```text
CommandId       = logical mutation identity
IdempotencyKey  = ik1:<issuance-epoch>:<random-128+>
ClientRequestId = one transport attempt correlation
```

Runtime은 acceptance horizon/future skew를 검증한다. expired key는 `idempotency-key-expired`로 거부하며 binding 삭제 후 새 mutation으로 취급하지 않는다.

## 5. Client submission

```text
parse + typed validation
→ generate CommandId/IdempotencyKey
→ calculate RequestDigest
→ exclusive PreparedUnsent file
→ file fsync + parent fsync
→ transmit with fresh ClientRequestId
→ server principal/selector resolution
→ ResolvedBindingDigest + key lookup/atomic commit
→ Observed/Terminal local journal update
```

payload가 없는 crash recovery는 operation lookup만 수행한다. original typed payload+same digest가 없으면 blind replay하지 않는다.

## 6. Operation Receipt

```text
operation_id / command_id / idempotency_key
principal_ref / request_digest / resolved_binding_digest
action / resolved_target_ref
state: Accepted | Committed | Rejected | Superseded | RecoveryRequired
outcome/error/reconciliation refs
receipt_revision / retention_policy_generation
```

`RecoveryRequired`는 expected receipt revision을 가진 explicit reconcile로만 전진한다.

## 7. Wait Predicate

- `accepted`: durable Receipt binding이 존재할 때
- `committed`: operation 자체의 canonical mutation이 commit될 때
- `applied`: metadata가 지정한 Directive/Runtime effect가 Applied일 때
- `target-terminal`: metadata가 지정한 target lifecycle terminal일 때
- `immediate`: Query/Host read
- `follow`: Subscription

각 operation은 허용 predicate와 default를 metadata에 선언한다. unsupported wait는 usage error다. local timeout은 rollback이 아니다.

## 8. Machine result schema

Single JSON `dxbot.result.v1`:

```text
schema, status(complete|accepted|partial|error)
request_id, operation, instance_id
resolved_target?, receipt?, data?, error?, reconciliation?
```

JSONL `dxbot.stream.v1` record:

```text
header | item | event | progress | terminal
terminal.status = complete | partial | gap | error
terminal.last_safe_cursor? / receipt? / reconciliation?
```

writer가 terminal record를 생성하려 노력하되 broken pipe에서는 전달을 보장하지 않는다.

## 9. Stable error/exit registry

<!-- exit-registry:start -->
- `0` | `success` | complete or requested accepted state
- `2` | `usage` | parser option/command misuse
- `3` | `invalid-input` | typed validation or malformed data
- `4` | `not-found` | target absent or invisible
- `5` | `ambiguous-target` | selector resolves to multiple candidates
- `6` | `conflict` | stale revision/generation, digest mismatch, superseded
- `7` | `permission-denied` | authorization or information-flow deny
- `8` | `approval-required` | ApprovalId returned; no effect committed
- `9` | `resource-exhausted` | bounded admission/capacity deny
- `10` | `runtime-unavailable` | host/control/runtime not ready
- `11` | `incompatible` | protocol/schema/provider incompatibility
- `12` | `timeout` | local request/wait deadline reached
- `13` | `interrupted` | local SIGINT/cancel/broken pipe observation ended
- `14` | `partial-or-resync` | partial machine result, stream gap, cursor restart required
- `15` | `recovery-required` | uncertain outcome requires lookup/reconcile
- `16` | `provider-unavailable` | required Provider absent/degraded/failed
- `17` | `storage-or-corruption` | storage, migration, integrity failure
- `18` | `internal-invariant` | unclassified invariant/internal defect
<!-- exit-registry:end -->

Error envelope `dxbot.error.v1`은 `class`, safe message, target/candidates/current revision, RetryDisposition, operation/approval/cursor hint를 가진다. retry/security/exit를 문자열 matching으로 결정하지 않는다.

## 10. Operation metadata SSOT

Rust `application-contract` source는 command key, CLI path/mode, operation/kind, input/target schema, wait set/default, output schema, security class, Acceptance ID를 소유한다. `DXB-IFC-041/042`는 parser freeze 전 committed typed snapshot이며 M1B 이후 generated exact diff 대상이다.
