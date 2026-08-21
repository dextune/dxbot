---
title: "Bot Control Plane"
document_id: "DXB-DOM-026"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P1/P2"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-ARC-016", "DXB-ARC-017", "DXB-DOM-020", "DXB-DOM-025"]
---

# Bot Control Plane

## 1. 목적

Bot/Task/Core/Memory/Routine/Capability/Provider/Plugin을 중앙에서 관찰·운영하되 Domain/Persistence/Common Framework를 우회하지 않는 관리 평면을 정의한다.

## 2. 원칙

- 모든 변경은 Application Command Handler와 해당 Canonical Owner를 통한다.
- 조회는 Projection watermark/stale 상태를 표시한다.
- Control Plane은 Brain/Domain/Provider Lifecycle owner가 아니다.
- Provider 관리도 Common Registry/Lifecycle Manager의 command를 사용한다.
- Plugin 관리는 Plugin Manager lifecycle을 사용한다.
- Control Plane이 Provider Host를 우회해 Provider를 직접 실행하지 않는다.
- DB 직접 편집 UI를 제공하지 않는다.

## 3. 기능 영역

### Bot
create/activate/deactivate/archive/restore/delete, identity/profile/resource policy, health/history.

### Task/Execution/Core
submit/inspect/cancel/retry, DAG/delegation, Waiting Continuation 상태, active Core/lease, result/trace, side-effect reconciliation operation.

### Routine
list/get/create/update, enable/disable, trigger/schedule preview, next/last occurrence, missed/overlap policy, 일반 Task 기반 run-now, occurrence/restart reconciliation inspect.

### Memory
search/revision/provenance/conflict/retention/compaction/archive/forget. Canonical bytes와 cache/index bytes를 구분한다.

### Capability / Provider
- Capability Contract ID/version/optional feature metadata
- registered Provider ID/version/config digest/SDK compatibility
- Standard Lifecycle state: Declared/Validated/Starting/Ready/Draining/Stopped/Degraded/Failed/Quarantined/Incompatible
- health observation과 lifecycle state의 구분
- selector policy/effective selection preview와 registry generation
- provider activity/in-flight reference, drain/quiescence progress
- deprecated/removed/stale config diagnostics
- Conformance suite version/evidence reference
- Reference Provider 여부/compatibility evidence
- Provider removal/dependency cleanliness operation 결과

Control Plane의 lifecycle 표시는 Common Lifecycle Manager의 상태를 투영하며 Provider가 임의 보고한 `idle/ready`를 authority로 사용하지 않는다.

### Plugin
- installed/validated/enabled/disabled/draining/removal-pending
- manifest/version/digest
- permission grants
- provided/required capabilities
- 각 제공 Provider의 별도 Common Lifecycle state
- resource health
- upgrade/rollback/uninstall operation
- plugin-owned data retention/migration status

## 4. Command / Query 결과

Command는 committed/accepted/rejected/conflict와 revision/event/operation ID를 제공한다. Query는 projection version, source watermark, stale/degraded flags를 제공한다.

Provider/Plugin이 제거되거나 incompatible인 경우 관련 resource를 숨기거나 임의 fallback하지 않고 `deprecated`, `incompatible`, `unsupported`, `unavailable`, `removed` 등의 stable 상태/오류를 제공한다.

## 5. Provider 관리 Operation

Provider lifecycle을 Control Plane이 직접 mutation하지 않는다.

- validate/start/restart request
- deprecate/new-use block
- drain/quiesce
- detach/stop
- config/contract compatibility revalidation
- quarantine/re-enable approval
- conformance evidence refresh

장기 작업은 `Operation`으로 노출하고 target provider ID/version/generation, actor, reason, idempotency, progress, terminal outcome, cleanup/leak status를 기록한다.

## 6. 일반 장기 Operation

Plugin upgrade/uninstall, Memory archival, Side Effect reconciliation 등도 `Operation` resource로 실행한다. client disconnect가 operation을 취소하지 않는다. 각 Operation은 impact summary와 rollback/compensation 상태를 가진다.

## 7. 안전장치

- destructive/bulk dry-run
- expected revision/generation
- structured approval
- rate/resource limit
- audit reason
- migration/compatibility/rollback preflight
- Plugin purge와 irreversible migration은 강화 approval
- quarantined/incompatible Provider의 silent Ready 전환 금지

## 8. 관측성과 개발 증거

Control Plane은 운영 상태와 함께 Capability/Provider 개발 품질 증거를 조회 가능하게 할 수 있다.

- Contract/SDK/Conformance version
- last conformance result/evidence digest
- lifecycle generation
- Host admission/call/error metrics 요약
- dependency/removal cleanliness status

이 정보는 운영 판단을 위한 Projection이며 CI Acceptance의 원본 증거를 대체하지 않는다.

## 9. 검증 기준

- CLI/TUI/Web이 같은 Routine/Provider/Plugin schema를 사용한다.
- Provider 상태가 Common Lifecycle vocabulary와 일치한다.
- Provider health와 lifecycle state를 혼동하지 않는다.
- removed/incompatible Provider config가 diagnostic에 나타난다.
- Plugin uninstall이 제공 Provider drain/data policy를 추적한다.
- Side Effect Unknown을 operator가 조회/reconcile할 수 있다.
- Control Plane 종료가 실행 중 Runtime 의미를 바꾸지 않는다.
