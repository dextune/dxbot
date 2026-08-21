---
title: "저장소와 데이터 모델"
document_id: "DXB-ARC-015"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-011"]
---

# 저장소와 데이터 모델

## 1. 목적

Persistent Bot의 Canonical State를 crash-safe하게 보존하고 Routine, Waiting Continuation, Side Effect Ledger, Provider/Plugin lifecycle을 중복 원본 없이 저장한다.

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

### Waiting Continuation
논리 필드:
- TaskId / Task revision
- wait condition/type/version
- completed/pending dependency refs
- delegated child/message refs
- checkpoint/artifact refs
- deadline/not-before
- correlation/causation
- resume guard/idempotency token
- continuation revision/checksum

`Task status=Waiting`과 유효 Continuation은 같은 Unit of Work에서 Commit한다. 하나만 존재하는 상태는 corruption으로 취급한다.

### Routine
- RoutineId / BotId / revision
- enabled state
- trigger/schedule definition + version
- Task template/reference
- timezone/clock policy
- overlap policy
- missed occurrence policy
- next occurrence
- last occurrence/outcome
- occurrence dedup records

Routine occurrence ID는 동일 trigger를 재처리해도 Task가 하나만 생성되도록 한다.

### Side Effect Ledger
- ledger entry ID
- Task/Execution/Tool/action references
- idempotency key/action digest
- intent payload reference/sensitivity
- semantic state
- provider/external reference
- attempt timestamps
- normalized outcome
- reconciliation state/evidence

외부 호출을 수행하기 전 Intent를 durable commit한다.

### Memory
기존 `memory-items`, immutable revisions, links, tombstones, index queue 구조를 유지한다. Long-term canonical data는 runtime cache pressure로 삭제하지 않는다.

### Plugin/Provider Operation
Core Domain과 분리된 operational catalog에:
- provider/plugin ID/version
- config generation/digest
- lifecycle state
- install package digest/signature metadata
- plugin-owned data schema/version/retention policy
- migration/uninstall marker
를 둘 수 있다. Provider 세션/runtime handle은 Canonical Domain State가 아니다.

## 4. Transaction Boundaries

### Command Commit
Event + Current State + idempotency + Outbox를 원자 처리한다.

### Waiting Commit
Task Waiting transition + Continuation + 필요한 correlation/outbox를 원자 처리한다.

### Routine Occurrence
occurrence claim/dedup + generated Task + Routine next/last state를 일관되게 Commit한다.

### Side Effect
1. Intent/key commit
2. external effect execute
3. outcome commit
4. uncertain crash window는 Unknown/reconciliation queue

외부 시스템과 DB 사이의 분산 원자성을 가장하지 않는다.

## 5. Data Lifecycle / Removal

Provider/Plugin 제거 시:
- Core Domain rows를 삭제하지 않는다.
- provider-specific operational rows의 migration/retain/purge를 명시한다.
- Plugin-owned data는 manifest uninstall policy에 따라 `retain | export-and-remove | migrate | purge | block` 중 지원된 의미를 적용한다.
- `block`이면 active dependency 또는 안전한 data disposition이 해결되기 전 uninstall 완료를 Commit하지 않는다.
- derived index/cache/projection은 stale owner reference를 정리한다.
- removed ID를 참조하는 config/state는 silent fallback하지 않는다.

## 6. Long-term Memory Growth

계측 대상:
- canonical bytes
- revision count
- item count
- unused age
- retention class
- compaction eligibility
- cold/archive bytes
- deletion/tombstone backlog

threshold 값은 Config/Policy SSOT에서 소유하고 benchmark/운영 정책으로 결정한다.

## 7. 검증 기준

- kill injection에서 Waiting 상태와 Continuation이 분리 저장되지 않는다.
- side-effect external success 후 outcome commit 전 crash가 Unknown으로 복구된다.
- Routine restart가 occurrence 중복 Task를 만들지 않는다.
- Provider 제거 후 Bot/Memory/Task DB가 그대로 load된다.
- Plugin uninstall `block` 정책이 dependency가 남은 상태의 제거 완료를 거부한다.
- cache/index 삭제가 Canonical Memory를 손상시키지 않는다.
