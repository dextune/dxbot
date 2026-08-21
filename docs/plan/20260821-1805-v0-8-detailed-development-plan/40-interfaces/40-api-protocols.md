---
title: "Headless Application Contract와 Control Protocol"
document_id: "DXB-IFC-040"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-026", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-032", "DXB-RUN-033", "DXB-RUN-038", "DXB-ARC-014", "DXB-ARC-015"]
---

# Headless Application Contract와 Control Protocol

## 1. 목적

실행 중 DXBOT Runtime의 모든 P0 use-case를 Domain internal API 없이 노출하는 public contract를 정의한다. 본 문서는 Command/Query/Subscription, Resource Selector, Operation Receipt, pagination snapshot/cursor, stream partial/terminal, structured error, version/schema의 Canonical Owner다.

> **Runtime 의미는 Domain/Runtime owner가 소유하고, Application Contract는 그 의미를 typed DTO로 노출하며, consumer는 재해석하지 않는다.**

## 2. Contract Kernel

```text
Command      = durable mutation intent와 Operation Receipt
Query        = bounded snapshot/live read
Subscription = cursor 기반 at-least-once observation
Host Lifecycle Operation = Runtime 부재 시 process/service bootstrap
```

다음을 하나의 generic envelope나 method string으로 합치지 않는다.
- Command / Query / Subscription
- Domain Event / public Subscription Event
- Message / Task / Control Directive
- Provider Capability Contract
- internal Work Queue / Control Channel
- Host Lifecycle Operation / Domain Command

generic `control <json>` mutation, arbitrary method/payload dispatch, untyped process-control fallback을 명시적으로 금지한다.

## 3. 공통 Request Context

적용 가능한 요청은 다음을 가진다.

```text
ProtocolVersion
SchemaVersion
ClientVersion
ClientRequestId
PrincipalContext
InstanceId
Deadline?
Trace/CorrelationContext?
```

Runtime은 endpoint descriptor와 request의 InstanceId가 현재 instance와 일치하는지 검증한다. connection/session ID는 Domain identity가 아니다.

## 4. Resource Addressing

### 4.1 타입

```text
ResourceSelector =
  ById { kind, id }
| ByExactName { kind, name, scope? }
| ByAlias { kind, alias, scope? }

ResolvedResourceRef {
  kind,
  canonical_id,
  canonical_scope?,
  revision?,
  generation?,
  display_name?,
  resolved_from
}
```

### 4.2 결정 규칙

1. Canonical ID가 최종 식별자다.
2. ID selector는 kind/instance를 검증하고 name lookup보다 우선한다.
3. name/alias는 Unicode normalization과 project-defined exact comparison 후 **명시한 scope 안에서 principal에게 보이는 후보가 하나일 때만** resolve한다.
4. P0 mutation에서 fuzzy, prefix, substring, recency, “last used” selection을 사용하지 않는다.
5. 후보가 0개면 `not-found`, 2개 이상이면 `ambiguous-target`과 bounded candidate summaries를 반환한다.
6. destructive/high-risk mutation은 resolved canonical ID와 expected revision/generation을 요구한다. name-only convenience는 resolve 결과를 표시한 뒤 command에 ID/revision을 pin한다.
7. profile/current context는 selector 입력을 줄일 수 있으나 authority/canonical state가 아니다.
8. resolve 후 Application/Domain owner가 current authorization, target existence, expected revision/generation을 commit 직전에 다시 검증한다.
9. machine output은 `ResolvedResourceRef`를 반환해 script가 최종 target을 확인할 수 있게 한다.

### 4.3 Scope precedence

명시적 scope가 최우선이다. scope 생략이 허용되는 Query는 다음 순서로 후보를 수집하되 하나가 아니면 실패한다.

```text
explicit ResourceId
→ explicit Project/Channel/Bot parent
→ instance-global principal-visible exact candidates
→ ambiguous/not-found
```

implicit CLI context가 mutation 후보를 자동 한 개로 축소하지 않는다.

## 5. Command Contract

### 5.1 Command Envelope

```text
CommandId
IdempotencyKey
PrincipalRef
Action
Target ResolvedResourceRef? / CreateScopeRef?
ExpectedRevision / ExpectedGeneration?
RequestDigest
Correlation / Causation
Deadline?
Approval / ActionGrantRef?
Payload
ProtocolVersion / SchemaVersion
SubmittedAt
```

Client는 network transmission 전에 CommandId와 IdempotencyKey를 생성한다. RequestDigest는 canonicalized action/target/payload/schema/binding에서 결정적으로 계산한다. secret의 원문을 digest metadata나 diagnostic에 노출하지 않는다.

### 5.2 Operation Receipt

```text
OperationReceipt {
  operation_id,
  command_id,
  idempotency_key,
  principal_ref,
  action,
  target_ref?,
  request_digest,
  protocol_version,
  schema_version,
  submitted_at,
  accepted_at?,
  updated_at,
  state,
  canonical_outcome_ref?,
  result_summary?,
  error?,
  retry_disposition,
  reconciliation_hint?,
  receipt_revision,
  retention_expires_at?
}
```

상태:

```text
Accepted
Pending
AwaitingSafePoint
Committed
Rejected
Superseded
RecoveryRequired
```

`Accepted/Pending/AwaitingSafePoint`는 semantic effect가 완료됐다는 뜻이 아니다. `Committed`만 canonical mutation outcome이 결정되었음을 뜻한다. Rejected는 durable refusal이며 Superseded/RecoveryRequired는 후속 조회·reconcile이 필요하다.

### 5.3 Idempotency binding

- same key + same principal/action/target/request digest/schema → existing receipt/outcome 반환
- same key + 다른 principal/action/target/payload/schema digest → `idempotency-conflict`
- receipt는 authorization, approval, expected revision을 대체하지 않는다.
- retry에서 current authorization/revision policy를 재검증하되 이미 committed outcome은 새 effect 없이 반환한다.
- external Side Effect Unknown은 Side Effect Ledger/Reconciliation을 사용한다.

### 5.4 Durability와 retention

receipt/outcome ref는 canonical mutation commit과 같은 durability boundary 또는 crash-safe atomic protocol에 기록한다.

- nonterminal receipt는 TTL만으로 GC하지 않는다.
- terminal receipt와 key binding은 **최소 30일** 보존한다.
- policy가 더 길게 보존할 수 있으며 exact capacity/compaction은 Runtime config/benchmark가 소유한다.
- expiry 뒤 조회는 `operation-expired`와 available audit/outcome reference를 반환한다.
- expiry를 이유로 CLI가 same key mutation을 blind replay하지 않는다.

### 5.5 CLI local journal

허용하는 bounded metadata:

```text
profile / InstanceId
CommandId / IdempotencyKey
RequestDigest
operation_id?
submitted_at / last_checked_at
```

secret, full payload/result, Authority, approval token, canonical outcome을 저장하지 않는다. Runtime receipt가 source of truth다.

## 6. Query Contract

### 6.1 Page Request/Response

```text
PageRequest {
  selector/scope,
  filter,
  sort,
  page_size,
  cursor?
}

PageResult<T> {
  items,
  next_cursor?,
  snapshot_ref?,
  source_watermark,
  observed_at,
  consistency,
  stale/degraded?,
  total_count?  // optional
}
```

page_size와 encoded/decoded byte ceiling을 모두 적용한다. total count는 optional이며 full materialization을 요구하지 않는다.

### 6.2 Ordering

각 Query operation은 stable primary sort와 canonical ID/revision tie-breaker를 schema metadata에 선언한다. 예:
- created_at + canonical_id
- revision + canonical_id
- score desc + canonical_id

client 임의 정렬을 cursor continuation의 source로 사용하지 않는다.

### 6.3 Snapshot / Live 구분

- `SnapshotConsistent`: first page에서 SnapshotId/Watermark를 고정하고 continuation 전체가 같은 snapshot을 읽는다.
- `LiveEventual`: current projection을 읽으며 페이지 간 변화가 가능함을 response에 표시한다.

P0 list/history/search/export의 기본은 SnapshotConsistent다. operational status/pressure/health는 LiveEventual일 수 있다.

### 6.4 Cursor binding

cursor는 opaque, integrity-protected token이며 최소 다음에 결박된다.

```text
query kind
normalized filter/sort
scope / resolved target
principal/security domain visibility context
snapshot or source watermark
last sort key/tie-breaker
schema version
expiry
```

다른 filter/scope/principal/instance/schema에서 재사용하면 `foreign-cursor`; retention 초과는 `expired-cursor`; authorization 변경으로 동일 visibility를 보장할 수 없으면 `cursor-reauthorization-required`로 전체 Query를 새 snapshot에서 재시작한다.

### 6.5 Partial page와 `--all`

page fetch/encode/write 중 실패하면 이미 출력된 record의 snapshot/source 의미와 `last_safe_cursor`를 terminal metadata로 제공한다. `--all`은 page-by-page incremental writer이며 전체 JSON array를 기본 형식으로 사용하지 않는다.

## 7. Subscription Contract

### 7.1 Event/Envelope

```text
SubscriptionEvent<T> {
  event_id,
  cursor,
  source_ref,
  source_revision?,
  watermark,
  observed_at,
  event_kind,
  payload
}
```

delivery는 at-least-once다. consumer는 EventId로 duplicate를 식별한다. public event는 Domain Event/internal queue와 별도 DTO다.

### 7.2 Cursor observation

P0에서는 subscriber별 durable acknowledgment를 서버 canonical state로 저장하지 않는다. CLI는 JSON decode와 local writer acceptance가 끝난 이벤트의 cursor를 `last_observed_cursor`로 취급하고 bounded local journal 또는 terminal record에 남길 수 있다.

### 7.3 Resume / Gap

- reconnect마다 principal authorization와 protocol/schema를 재협상한다.
- valid retained cursor면 resume한다.
- expired/missing/foreign/visibility-changed cursor면 explicit `gap`과 resync instruction을 반환한다.
- silent start-from-now fallback을 금지한다.
- resync는 bounded snapshot Query 후 새 cursor/watermark로 subscription을 재개한다.

### 7.4 Slow consumer

server/client buffers는 item+byte cap을 가진다. cap에 도달하면 backpressure 가능한 범위에서 적용하고, 지속되는 slow consumer는 explicit terminal reason과 last safe cursor로 disconnect한다. Runtime internal queue를 subscriber가 무기한 점유하지 않는다.

### 7.5 Terminal / Partial

stream은 terminal metadata를 반드시 표현한다.

```text
StreamTerminal {
  status: Complete | Partial | CancelledLocally | Gap | Failed,
  operation/state summary?,
  last_safe_cursor?,
  source_watermark?,
  error?,
  reconciliation_hint?
}
```

SIGINT/broken pipe는 `CancelledLocally` 또는 output이 불가능한 경우 stderr/exit class로 표현하며 target Task/Process를 변경하지 않는다. 중간 오류나 gap을 complete success로 표시하지 않는다.

## 8. Structured Error

```text
ApplicationError {
  code,
  class,
  message_safe,
  target_ref?,
  current_revision/generation?,
  candidates?,
  retry_disposition,
  operation_id?,
  last_safe_cursor?,
  details_schema_version
}
```

필수 class:
- usage/validation
- not-found / ambiguous-target
- conflict/stale-revision/generation
- forbidden / approval-required
- resource/admission/payload-too-large
- runtime/transport-unavailable
- incompatible-version/schema
- local-wait-timeout / local-observation-cancelled
- partial-output
- operation-expired
- recovery-required
- internal/invariant/corruption

RetryDisposition은 `DoNotRetry | RetrySameOperation | ReconcileFirst | RetryAfter | Reauthorize | RestartQuery`처럼 typed하게 제공한다. 문자열 parsing으로 retry/security 결정을 하지 않는다.

## 9. Runtime Host / Endpoint Discovery

Runtime 부재 시 Host Lifecycle Client가 process/service start/status/stop/readiness만 다룬다. ControlReady 이후의 Domain status/doctor/mutation/query/subscription은 본 Contract를 사용한다.

safe discovery summary:

```text
InstanceId / HostGeneration
host service/process state
Runtime/Control readiness
protocol/schema compatibility
endpoint transport/path relative summary
started_at
```

process handle/PID/socket generation은 Task/Execution identity나 control handle이 아니다.

## 10. Version / Schema SSOT

```text
protocol_version ≠ schema_version ≠ runtime_version ≠ client_version ≠ data_schema_version
```

public DTO와 error/exit metadata의 source는 Rust `application-contract` logical module/crate다. Domain/Persistence type을 derive-export하지 않는다.

생성물:
- deterministic JSON Schema 또는 동등 machine schema
- committed golden request/response/event/error fixtures
- operation registry
- CLI help/exit/schema input metadata

generated artifact는 source hash/generator version을 가진다. PR에서 regenerated output과 committed snapshot drift를 검사한다. remote second language/transport가 실제 필요해질 때 별도 IDL 승격을 검토한다. 범용 RPC/IDL framework를 P0에 새로 만들지 않는다.

## 11. Compatibility

- additive optional read field는 동일 major에서 허용한다.
- required mutation semantic, selector binding, receipt state, cursor meaning의 변경은 incompatible schema major다.
- unknown required Command field/operation은 explicit failure다.
- cursor는 schema major를 넘어 portable하지 않다.
- receipt는 지원 compatibility window에서 operation_id로 조회 가능해야 한다.
- silent downgrade/reinterpretation을 금지한다.

## 12. Security / Resource

- request마다 principal/current authorization를 검증한다.
- selector/cursor/receipt가 Authority를 생성하지 않는다.
- request/response/page/stream cumulative bytes를 제한한다.
- decoded/decompressed expansion을 preflight한다.
- secret/raw sensitive content는 error/receipt/discovery/schema fixture에 넣지 않는다.
- terminal/file rendering safety는 `DXB-IFC-041`, endpoint policy는 `DXB-RUN-032`가 소유한다.

## 13. 검증 기준

### AT-APP-001 — Headless Contract Completeness
Representative Bot/Conversation/Thread/Project/Channel/Task/Memory/Process use-case를 Domain internal API 없이 수행한다.

### AT-APP-002 — Version Compatibility
compatible pair는 semantic을 유지하고 incompatible pair는 explicit failure다.

### AT-APP-003 — Legacy Lost-Response Safety
commit 후 response loss에서 duplicate Canonical effect 0.

### AT-APP-004 — Legacy Resume/Gap Safety
disconnect/gap 후 resume 또는 explicit resync.

### AT-APP-005 — Operation Receipt / Idempotency Binding
same key/same digest는 기존 outcome, different binding은 conflict, CLI restart 뒤 receipt 조회 가능.

### AT-APP-006 — Pagination Snapshot / Cursor Binding
concurrent mutation 중 snapshot duplicate/missing 0, foreign/expired/reauth cursor explicit error.

### AT-APP-007 — Subscription Partial / Terminal / Resume
partial/gap/local cancel을 complete로 오판하지 않고 last safe cursor를 제공한다.

### AT-SCHEMA-001 — Public Schema SSOT / Drift
Rust source에서 deterministic schema/fixture를 생성하고 manual divergent DTO 0.
