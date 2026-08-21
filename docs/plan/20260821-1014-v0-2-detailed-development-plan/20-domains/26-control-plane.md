---
title: "Bot Control Plane"
document_id: "DXB-DOM-026"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P1/P2"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-ARC-016", "DXB-DOM-020", "DXB-DOM-025"]
---

# Bot Control Plane

## 1. 목적

Bot/Task/Core/Memory/Routine/Provider/Plugin을 중앙에서 관찰·운영하되 Domain과 Persistence를 우회하지 않는 관리 평면을 정의한다.

## 2. 원칙

- 모든 변경은 Application Command Handler를 통한다.
- 조회는 Projection watermark/stale 상태를 표시한다.
- Control Plane은 Brain/Domain owner가 아니다.
- Provider/Plugin 관리도 Registry/Plugin Manager의 lifecycle command를 사용한다.
- DB 직접 편집 UI를 제공하지 않는다.

## 3. 기능 영역

### Bot
create/activate/deactivate/archive/restore/delete, identity/profile/resource policy, health/history.

### Task/Execution/Core
submit/inspect/cancel/retry, DAG/delegation, Waiting Continuation 상태, active Core/lease, result/trace, side-effect reconciliation operation.

### Routine
- list/get/create/update
- enable/disable
- trigger/schedule preview
- next/last occurrence
- missed/overlap policy
- run-now는 일반 Task 생성 Command로 구현
- occurrence/restart reconciliation inspect

### Memory
search/revision/provenance/conflict/retention/compaction/archive/forget. Canonical bytes와 cache/index bytes를 구분한다.

### Provider/Capability
- capability catalog
- registered Provider/version/health
- selector policy and effective selection preview
- deprecate/drain/detach status
- unsupported/stale config diagnostics
- conformance evidence reference

### Plugin
- installed/validated/enabled/disabled/draining/removal-pending
- manifest/version/digest
- permission grants
- provided/required capabilities
- resource health
- upgrade/rollback/uninstall operation
- plugin-owned data retention/migration status

## 4. Command/Query 결과

Command는 committed/accepted/rejected/conflict와 revision/event/operation ID를 제공한다. Query는 projection version, source watermark, stale/degraded flags를 제공한다.

Provider/Plugin이 제거된 경우 API는 관련 resource를 조용히 숨기거나 임의 fallback하지 않고 `deprecated`, `unsupported`, `unavailable`, `removed` 등 stable 상태를 제공한다.

## 5. 장기 Operation

Provider drain, Plugin upgrade/uninstall, Memory archival, reconciliation은 `Operation` resource로 실행한다. client disconnect가 operation을 취소하지 않는다. 각 Operation은 idempotency, actor, impact summary, progress, rollback/compensation 상태를 가진다.

## 6. 안전장치

- destructive/bulk dry-run
- expected revision
- structured approval
- rate/resource limit
- audit reason
- migration/rollback preflight
- Plugin purge와 irreversible migration은 강화 approval

## 7. 검증 기준

- CLI/TUI/Web이 같은 Routine/Provider/Plugin schema를 사용한다.
- removed Provider를 참조하는 config가 diagnostic에 나타난다.
- Plugin uninstall이 background operation으로 drain/data policy를 추적한다.
- side-effect Unknown entry를 operator가 조회/reconcile할 수 있다.
- Control Plane 종료가 실행 중 Runtime 의미를 바꾸지 않는다.
