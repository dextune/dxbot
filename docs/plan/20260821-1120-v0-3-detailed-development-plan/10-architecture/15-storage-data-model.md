---
title: "저장소와 데이터 모델"
document_id: "DXB-ARC-015"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-011", "DXB-ARC-017"]
---

# 저장소와 데이터 모델

## 1. 목적

Persistent Bot의 Canonical State를 crash-safe하게 보존하고 Routine, Waiting Continuation, Side Effect Ledger, Provider/Plugin operational lifecycle을 중복 원본 없이 저장한다. Provider의 live process/session/activity를 Domain Canonical State로 승격하지 않는다.

## 2. 저장 계층

| 계층 | Canonical 여부 | 의미 |
|---|---|---|
| Domain Journal | Canonical | 상태 변경 사실/순서 |
| Current State | Canonical Materialization | Aggregate 현재 상태/revision |
| Memory Records/Revisions | Canonical | 기억 내용/출처/수명 |
| Task Continuations | Canonical | Waiting 재개 최소 상태 |
| Routine State/Occurrences | Canonical | schedule/trigger와 Task 생성 dedup |
| Side Effect Ledger | Canonical operation state | 외부 변경 intent/outcome/reconcile |
| Inbox/Outbox | Canonical delivery state | durable async delivery |
| Artifact Store | Canonical content | 큰 결과/trace/checkpoint |
| Provider/Plugin Operational Catalog | Canonical operational intent/version metadata | config/version/admin lifecycle/migration markers |
| Plugin Data Catalog | Canonical ownership metadata | plugin-owned data 위치/retention/version |
| Projection/Index/Cache | Derived/Ephemeral | 조회/검색/성능 |

Derived 저장소는 rebuild 경로를 가진다.

## 3. 논리 데이터 모델

### Core
- `bots`
- `bot-events`
- `goals`
- `tasks`
- `task-edges`
- `executions`
- `core-leases`
- `messages`
- `inbox-receipts`
- `outbox`

실제 DB table naming은 DB convention ADR에 따를 수 있으며 repository file naming 규칙과 동일 개념이 아니다.

### Execution Capability Bindings

Execution은 0개 이상의 Capability binding을 가질 수 있고 각 binding은 필요에 따라 다음 opaque metadata를 저장한다.
- binding identifier / Capability ID / Contract version
- Provider ID/version
- config digest/generation
- Registry generation / `ProviderSelectionRef` durable representation
- selection policy revision

raw Provider process handle, SDK object, connection/session object는 저장하지 않는다. resume token이 필요하면 Capability Contract가 version/compatibility 의미를 정의하고 checkpoint/Artifact reference로 저장한다.

### Waiting Continuation

TaskId/Task revision, wait condition/version, completed/pending dependency refs, delegated child/message refs, checkpoint/Artifact refs, deadline/not-before, correlation/causation, resume guard/idempotency token, continuation revision/checksum을 가진다.

`Task status=Waiting`과 유효 Continuation은 같은 Unit of Work에서 Commit한다.

### Routine

RoutineId/BotId/revision, enabled, trigger/schedule definition+version, Task template/ref, timezone/clock policy, overlap/missed policy, next/last occurrence/outcome, occurrence dedup record를 가진다.

### Side Effect Ledger

- ledger entry ID
- Task/Execution/Capability binding/action refs
- idempotency key/action digest
- intent payload ref/sensitivity
- semantic state
- selected Provider/external reference where needed
- attempt timestamps
- normalized outcome
- reconciliation state/evidence

외부 call 전에 Intent를 durable commit한다. Provider Host가 Side Effect guard를 검증한다.

### Memory

`memory-items`, immutable revisions, links, tombstones, index queue 구조를 유지한다. Long-term canonical data는 Runtime/cache/Provider pressure로 삭제하지 않는다.

### Provider Operational Catalog

Core Domain과 분리된 Common operational catalog에 다음을 둘 수 있다.
- Provider ID/version
- provided Capability/Contract version range
- Provider SDK compatibility
- config schema version/generation/digest
- administrative desired state: enabled/deprecated/drain-requested/disabled 등
- last validated compatibility/health summary
- last known lifecycle state + lifecycle generation for diagnostics
- quarantine/incompatible reason
- migration/removal marker
- last Conformance evidence reference

**저장하지 않는 live authority:** process/thread handle, live connection, in-flight activity/permit object, current health runtime object, Registry pointer/object identity.

재시작 후 Common Lifecycle Manager가 persisted config/admin intent와 실제 runtime validation을 사용해 `Declared → Validated → Starting → Ready`를 다시 구성한다. persisted `Ready` 값을 그대로 authority로 복원하지 않는다.

### Plugin Operational Catalog

plugin ID/version, package digest/signature metadata, config/data schema, requested/granted permissions, administrative lifecycle intent, plugin-owned data policy, migration/uninstall marker를 둔다. Plugin runtime process handle은 저장하지 않는다.

## 4. Transaction Boundaries

### Command Commit
Event + Current State + idempotency + Outbox를 원자 처리한다.

### Waiting Commit
Task Waiting transition + Continuation + 필요한 correlation/outbox를 원자 처리한다.

### Routine Occurrence
occurrence claim/dedup + generated Task + Routine next/last state를 일관되게 Commit한다.

### Side Effect
1. Intent/key commit
2. Provider Host guard/admission
3. external effect execute
4. normalized outcome commit
5. uncertain crash window는 Unknown/reconciliation queue

### Provider Lifecycle Operation
관리 intent/config generation/migration marker 같은 durable operation state와 live lifecycle/activity를 분리한다. `drain requested`를 durable하게 기록할 수 있지만 quiesced 판정은 Common live activity/reference + recovery reconciliation 결과로 결정한다.

외부 시스템과 DB 사이의 분산 원자성을 가장하지 않는다.

## 5. Recovery of Capability Bindings

재시작 시 persisted binding을 그대로 `Ready/usable`로 간주하지 않는다.

1. Provider package/config/Contract/SDK compatibility 재검증
2. Provider Common Lifecycle 재구축
3. binding이 가리키는 Provider/version/config generation의 resume compatibility 확인
4. compatible하면 해당 Execution resume path에서 binding 의미 유지
5. 제거/비호환이면 `recovery-required/provider-incompatible/removed` 등 명시 상태로 전환
6. 동일 Execution의 binding을 다른 Provider로 silent rewrite하지 않음
7. 정책상 fallback이 가능하면 새 Execution/Attempt 생성

## 6. Data Lifecycle / Removal

Provider/Plugin 제거 시:
- Core Domain rows 삭제 금지
- provider-specific operational rows retain/migrate/purge 명시
- Execution historical ProviderSelectionRef는 감사/복구 요구에 따라 tombstone/compatibility metadata로 보존 가능
- Plugin-owned data는 manifest uninstall policy `retain | export-and-remove | migrate | purge | block`
- derived index/cache/projection stale owner reference cleanup
- removed ID config/state silent fallback 금지
- dependency/Registry/code 제거 후 historical data decode 필요성을 Migration에서 명시

## 7. Long-term Memory Growth

canonical bytes/revision/item/unused age/retention/compaction/cold/archive/tombstone backlog를 계측한다. threshold는 Policy/Config SSOT가 소유한다.

## 8. 검증 기준

- kill injection에서 Waiting/Continuation이 분리 저장되지 않는다.
- external Side Effect 성공 후 outcome commit 전 crash가 Unknown으로 복구된다.
- Routine restart가 duplicate Task를 만들지 않는다.
- persisted Provider `Ready` marker만으로 재시작 후 Ready가 되지 않는다.
- persisted Capability binding이 compatible Provider에서 의미를 유지하며, 제거/비호환 시 동일 Execution silent rewrite가 없다.
- Provider 제거 후 Bot/Memory/Task DB가 그대로 load된다.
- in-flight Provider activity/permit/process handle을 durable Canonical State로 복원하지 않는다.
- Plugin uninstall `block`이 dependency 남은 제거 완료를 거부한다.
- cache/index 삭제가 Canonical Memory를 손상시키지 않는다.
