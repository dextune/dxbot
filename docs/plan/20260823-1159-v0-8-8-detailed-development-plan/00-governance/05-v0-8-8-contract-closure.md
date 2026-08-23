---
title: "v0.8.8 CLI 구현 계약 폐쇄"
document_id: "DXB-GOV-005"
version: "0.8.8"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-BASE-000", "DXB-GOV-003", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042", "DXB-RUN-030", "DXB-RUN-032", "DXB-RUN-033", "DXB-DEL-060", "DXB-DEL-061"]
---

# v0.8.8 CLI 구현 계약 폐쇄

이 문서는 v0.8.7 적대적 검토에서 확인된 구현 차단 모호성을 기능 확장 없이 폐쇄한다. 같은 package 안의 기존 문장과 충돌할 경우 본 문서의 v0.8.8 교정 규칙이 우선한다.

## 1. Active baseline 폐쇄

- `INV-001~015`와 `ADR-0095~0124`는 active package 내부에서 해석 가능해야 한다. 과거 package의 문장을 의미 규범으로 요구하지 않는다.
- provenance는 근거일 뿐 active semantics의 owner가 아니다.
- validator PASS는 문법·관계성 검증이며 Rust state machine, storage durability, authorization correctness, real Harness 실행 증거를 대신하지 않는다.

## 2. Workspace dependency와 owner

`A -> B`는 A가 B에 의존함을 뜻한다.

```text
cli -> application-contract + control-client + host-action-client
control-server -> application + application-contract
application -> domain + application-contract
runtime-composition -> application + storage + provider-host + control-server
provider-host -> capability-contract + provider-sdk
provider -> capability-contract + provider-sdk + approved external SDK
```

CLI의 Domain/Storage/Runtime internal/Provider 직접 의존, Provider의 Application/Domain/Storage/UI 직접 의존을 금지한다. `application-operation`은 logical owner이며 별도 crate를 강제하지 않는다. P0 Common Extension Framework가 P1 Plugin에 의존해서는 안 되며 Plugin은 P0 framework의 소비자다.

## 3. Common Command Envelope

모든 Application Command는 다음 공통 의미를 가진다.

```text
ProtocolVersion / SchemaVersion / ClientVersion
ClientRequestId / InstanceId / effective Deadline
operation key + typed input schema version
CommandId + IdempotencyKey
raw typed selector + materialized semantic payload
```

- CommandId는 logical mutation identity다.
- IdempotencyKey는 principal-local epoch-bearing key다.
- ClientRequestId는 transport attempt correlation이며 retry마다 새로 만든다.
- 최초 `Prepared` 전에 CommandId/IdempotencyKey를 확정한다.
- `--profile`, discovery hint, rendering, wait timeout, `--all`, `--output`, confirmation은 local control이며 wire semantic/RequestDigest/Authority에 넣지 않는다.
- `--all`은 bounded page Query 반복이지 server-side unbounded flag가 아니다.
- `--output`은 local safe writer target이다.

## 4. RequestDigest와 submission 순서

RequestDigest는 canonicalization version, protocol/schema, operation key, raw typed selector, materialized payload bytes 또는 ArtifactRef digest, semantic option을 결박한다. local path/stdin fd/profile/output/rendering/wait/ClientRequestId는 제외한다. server-only resolved identity/revision/generation/authorization/policy/provider generation은 `ResolvedBindingDigest`가 소유한다.

송신 순서는 하나다.

```text
parse/validate
-> bounded content materialization
-> CommandId/IdempotencyKey 확정
-> RequestDigest 계산
-> Prepared exclusive create + file fsync + parent fsync
-> Dispatching append + fsync
-> 첫 request byte 송신
```

`Dispatching` durability 이전에는 어떤 request byte도 송신하지 않는다.

## 5. CLI Local Journal

Local Journal은 canonical Domain/Receipt store가 아니다. versioned record는 sequence, kind, time, InstanceId, operation key, CommandId, IdempotencyKey, RequestDigest, previous-record digest, record digest를 가진다. raw secret/Authority/canonical outcome은 저장하지 않는다.

- `Prepared`만 provably-unsent다.
- `Dispatching` 이상은 server lookup 전 replay/abandon/evict 금지다.
- truncated final record는 무시할 수 있으나 invalid middle record, chain mismatch, unknown journal version은 자동 send/replay/delete를 금지한다.
- same CommandId writer race는 first creator가 owner다. 동일 key/digest loser는 observation mode, 불일치는 local conflict다.
- nonterminal journal은 capacity cleanup 대상이 아니다. capacity가 부족하면 신규 mutation을 send 전에 거부한다.

## 6. Binding-first retry

server 순서는 다음으로 고정한다.

```text
authenticate
-> existing IdempotencyKey/CommandId binding lookup
-> existing binding이면 counterpart + RequestDigest 비교 후 stored Receipt 반환
-> absent일 때만 selector resolve -> authorize -> policy/provider resolve
-> atomic operation/domain/receipt/audit commit
```

retry에서 alias 변경, revoke/archive, policy/provider generation 변경을 먼저 재평가하지 않는다. horizon 밖 idempotency key를 신규 mutation으로 재해석하지 않는다.

## 7. Receipt·Approval·Domain outcome

PreAcceptError, Receipt `Rejected`, committed Domain `Rejected/Deferred`를 분리한다. Approval 대기 operation은 `Accepted + ApprovalRef`이며 Approval decision은 새 operation을 만들지 않고 original operation을 continuation한다. wakeup 유실은 startup reconcile로 복구한다.

Application Command output은 `out-operation-v1`이다. P0 `target-terminal` wait는 없다. `applied`는 Directive/Host Action owner가 있는 operation에만 허용한다.

exit precedence는 local transport/wait failure, PreAcceptError class, Receipt RecoveryRequired, ApprovalRequired, requested wait predicate, committed Domain outcome 순으로 해석한다. ApprovalRef가 있는 Accepted Receipt는 effect가 commit되지 않았으므로 approval-required exit가 success보다 우선한다.

## 8. Exposed lifecycle 폐쇄

Bot:

```text
Provisioning -> Inactive -> Activating -> Active|Degraded
Active|Degraded -> Quiescing -> Inactive
Inactive|Active|Degraded -> Archived -> Restoring -> Inactive
```

Project:

```text
Active -> Archiving -> Archived -> Restoring -> Active
```

Task:

```text
Submitted -> Admitted -> Running
Submitted|Admitted -> Deferred|Rejected|Cancelled
Running -> Suspended|Completed|Failed|Cancelled|RecoveryRequired
Suspended -> Resuming -> Running
```

pre-running Task cancel은 expected Task revision으로 가능하다. Running cancel/suspend는 authoritative ExecutionGeneration을 fence한다. archive/revoke는 신규 admission을 차단하되 child Task/Process canonical state를 직접 terminal rewrite하지 않는다.

## 9. Create와 membership concurrency

Bot/Project/Channel create는 policy-owned normalized namespace uniqueness + idempotency + unique-index winner를 사용하며 global Instance revision CAS를 요구하지 않는다. Bot identity creation은 Provider readiness와 독립적이고 Provider availability는 Activate/Task admission에서 평가한다.

Project/Channel membership 변경은 scope revision과 membership generation/absent CAS를 검증하며 Membership row + AuthorityBinding create/revoke + required AuditIntent를 하나의 Unit of Work에서 commit한다.

## 10. Milestone/Acceptance 수렴

- M2: Runtime Host, bootstrap, Approval/Authority/Audit와 최소 submission recovery.
- M3: Bot-only create/show/send/task submit-show-result/operation show-reconcile, 기본 bounded page/export.
- M4: Project/Channel/Membership/Delegation.
- M5: watch/subscription, Memory, Process, Provider observation, side-effect reconcile, real Harness, hardening.
- M6: full schema compatibility/performance/security/release freeze.

어떤 command subset도 그 command가 참조하는 Acceptance가 해당 milestone 또는 이전 milestone에서 executable evidence를 갖기 전에 freeze하지 않는다. schema freeze와 구현 milestone을 동일시하지 않는다.

## 11. 과도설계 방지

v0.8.8에서 새 CLI command, 범용 RPC/IDL/Workflow engine, state-per-crate, DB/Harness 제품 선결정, Plugin/TUI/Web/distributed scope, 근거 없는 numeric threshold를 추가하지 않는다. 목표는 기존 계약의 폐쇄·중복 제거·단계 정렬이다.
