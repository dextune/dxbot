---
title: "Headless Application Contract와 사용자 안전 submission"
document_id: "DXB-IFC-040"
version: "0.8.10"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-DOM-026", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-032", "DXB-RUN-033", "DXB-RUN-036", "DXB-RUN-038", "DXB-ARC-014", "DXB-ARC-015"]
---
# Headless Application Contract와 사용자 안전 submission

## Contract Kernel

`Command = durable Application mutation + Operation Receipt`, `Query = bounded read`, `Subscription = cursor observation`, `Host Action = process bootstrap/status/explicit stop`이다. Interface는 이 계약을 호출할 뿐 Domain/Storage state를 소유하지 않는다.

## CliInput → preflight → CommandPayload

`DXB-IFC-042`의 하나의 `in-*` metadata source가 `CliInput`과 `CommandPayload`를 생성한다. user-facing selector와 optional `if-*` CAS는 다음 순서로 materialize된다.

```text
parse CliInput
→ local Instance/endpoint resolution
→ bounded selector/CAS preflight when required
→ canonical target ID + expected revision/generation materialization
→ ContentSource materialization
→ CommandPayload + RequestDigest
→ Prepared
```

preflight는 같은 authenticated Principal과 visibility policy를 사용한다. 0개/복수 target, permission deny, unavailable은 `Prepared` 이전에 끝난다. CLI가 materialize한 canonical ID/revision/generation은 server authority가 아니며 Runtime이 selector resolution, authorization, policy와 CAS를 다시 검증한다. conflict를 최신 값으로 자동 재시도하지 않는다.

`@local` field와 file path, file descriptor, source variant는 wire DTO에 존재하지 않는다. ContentSource는 `Prepared` 전에 bounded bytes 또는 typed ArtifactRef로 materialize한다.

`RequestDigest`는 canonicalization/protocol/schema/operation/raw typed selector/materialized canonical target and CAS/materialized payload bytes 또는 ArtifactRef digest/semantic option을 결박한다. profile, Instance discovery hint, readiness return point, local timeout, `--all`, output path, confirmation, rendering, wait predicate, ClientRequestId는 제외한다.

`ResolvedBindingDigest`는 RequestDigest + ResolvedPrincipal + server-resolved target revision/generation + AuthorizationDecision + effective policy/provider generations를 server가 계산한다.

## Binding-first server protocol

```text
authenticate peer → ResolvedPrincipalRef
→ validate IdempotencyKey principal scope + issuance epoch
→ lookup (InstanceId, CommandId)
→ lookup (PrincipalRef, IdempotencyKey)
→ existing pair: Principal/counterpart/RequestDigest 검증 후 stored Receipt projection
→ both absent: selector and claimed canonical target verify → authorize → policy/provider resolve
→ atomic operation/domain/receipt/two-bindings/audit commit
```

한 index만 존재하거나 counterpart, Principal, RequestDigest가 다르면 conflict다. receipt 존재를 cross-principal caller에게 노출하지 않으며 신규 operation을 생성하지 않는다. 기존 binding을 current alias/authorization 변화 때문에 새 operation처럼 재평가하지 않는다.

full Receipt compaction 뒤에도 최소 tombstone이 global CommandId와 principal key uniqueness를 유지한다. horizon 밖 key는 typed expired/recovery-required로 끝나며 absent submission으로 처리하지 않는다.

## CLI submission protocol

```text
materialized CommandPayload
→ CommandId + principal-bound IdempotencyKey + RequestDigest
→ exclusive journal Prepared + fsync + parent durability
→ single-writer Dispatching append + fsync
→ first socket byte
→ Observed/Terminal update
```

`Prepared`만 provably-unsent다. Dispatching 이상은 Runtime lookup 전 replay/abandon/evict 금지다.

## Error와 Receipt

binding 생성 전 parser/validation/authentication/key-scope/expired-key/preflight 실패는 Receipt 없는 `PreAcceptError`다.

Receipt state는 `Accepted | Committed | Rejected | Superseded | RecoveryRequired`다. Approval 필요 시 `Accepted + ApprovalRef`를 유지한다. Approval decision이 original operation을 재평가해 terminal Receipt로 전진시킨다.

Task/Delegation aggregate가 생성된 뒤 Domain outcome이 `Rejected/Deferred`면 Receipt는 `Committed`다.

## Wait

- Command: `accepted`, `committed`
- Directive-producing Task control: 추가로 `applied`
- Host Action: operation별 `applied` 또는 `immediate`
- Query: `immediate`
- Subscription: `follow`

P0 `target-terminal`은 없다. local timeout/interrupt는 observation만 끝내며 Runtime cancel은 별도 typed operation이다.

## Human과 machine projection

모든 Application Command는 `dxbot.result.v1` + `out-operation-v1`을 사용한다. 최소 field는 request/command/operation/instance, receipt, status, optional committed payload, error/reconciliation이다. CreateBot committed payload는 BotRef와 MainConversationRef를 포함한다.

single result `--format json`은 성공·실패 모두 stdout에 정확히 하나의 complete JSON document를 쓴다. stream `--format jsonl`은 event당 한 줄과 terminal record를 쓴다. stderr에는 diagnostic/progress만 쓰며 machine schema는 TTY 여부에 따라 변하지 않는다.

stable error는 최소 다음을 가진다.

```text
code / category / message
retryable
visible operation/target refs
field violations
optional current revision/generation
optional resume cursor
typed next_actions(command_key + typed args)
```

human renderer만 shell quoting이 적용된 suggestion을 만든다. machine schema에 불안정한 raw shell command string을 계약으로 두지 않는다.

## Stable exit registry

<!-- exit-registry:start -->
- `0` | `success` | requested accepted/committed/applied/immediate/follow condition reached without a higher-priority condition
- `2` | `usage` | parser option/command misuse
- `3` | `invalid-input` | typed validation or malformed data
- `4` | `not-found` | target absent or invisible
- `5` | `ambiguous-target` | selector resolves to multiple candidates
- `6` | `conflict` | stale revision/generation or binding/digest/principal conflict
- `7` | `permission-denied` | authorization or information-flow deny
- `8` | `approval-required` | operation is Accepted and ApprovalRef returned; target effect not committed
- `9` | `resource-exhausted` | bounded admission/capacity deny
- `10` | `runtime-unavailable` | host/control/runtime not ready
- `11` | `incompatible` | protocol/schema/provider incompatibility
- `12` | `timeout` | local request/wait deadline reached
- `13` | `interrupted` | local SIGINT/cancel/broken pipe observation ended
- `14` | `partial-or-resync` | partial result, stream gap, cursor restart required
- `15` | `recovery-required` | uncertain outcome requires lookup/reconcile
- `16` | `provider-unavailable` | required Provider absent/degraded/failed
- `17` | `storage-or-corruption` | storage, migration, integrity failure
- `18` | `internal-invariant` | unclassified invariant/internal defect
<!-- exit-registry:end -->

exit precedence는 integrity/internal > recovery uncertainty > approval-required > permission/conflict/input > availability/resource > local timeout/interruption > success다. requested `accepted`가 도달했더라도 `ApprovalRef`가 있으면 exit 8이다.

## Freeze

Application Contract metadata는 command key/path/mode/kind/input/target/wait/output/security/Acceptance/milestone을 소유한다. Acceptance는 하나의 최초 milestone만 가지며 Gate는 누적된다. M2/M3/M4/M5 subset은 누적 PASS 후에만 freeze하고 full 63-operation schema freeze는 M6에서 한다.
