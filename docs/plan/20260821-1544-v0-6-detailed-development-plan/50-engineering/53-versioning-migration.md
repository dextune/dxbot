---
title: "버전·호환성·Migration"
document_id: "DXB-ENG-053"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-015", "DXB-ARC-016", "DXB-ARC-017", "DXB-DOM-022", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-038", "DXB-RUN-036", "DXB-IFC-040"]
---

# 버전·호환성·Migration

## 1. 목적

v0.5의 Persistent Bot/Project/Channel/Conversation/Thread/Memory/Task/Provider semantics를 보존하면서 v0.6 Durable Process, epistemic Memory metadata/relation, ActionGrant를 additive하게 도입한다. v0.5 fixture가 v0.6에서 동일 의미로 동작하는 것이 P0 gate다.

## 2. v0.5 → v0.6 신규 버전 대상

- Durable Process identity/progress/definition version/outcome refs
- Process/Task/Message optional linkage
- Memory Epistemic Kind / Assertion State / temporal metadata
- evidence/dependency/retraction/revalidation/quarantine relation
- Information Label/declassification policy refs
- Authorization Decision/ActionGrant schema/event
- Collaboration Cycle status/terminal projection
- Runtime resource policy/config/telemetry schema

Provider Contract/SDK/Plugin version discipline은 유지한다.

## 3. Memory Migration

원칙:
- MemoryId/revision/content/provenance/ScopeRef 유지
- epistemic metadata 부재를 explicit legacy state로 표현
- 과거 content를 임의 `Verified`로 승격 금지
- background reclassification이 Canonical content를 rewrite하지 않음
- existing recall compatibility를 fixture로 검증

`legacy-unclassified`는 권장 의미 예시이며 exact enum은 ADR 대상이다.

## 4. Active Collaboration / Process Migration

과거 v0.5 collaboration/task history를 근거 없이 새 Durable Process로 재구성하지 않는다.
- migration 시 명확한 active cross-aggregate state만 explicit mapping 검토
- 매핑할 수 없는 active state는 recovery-required/explicit operator action 사용 가능
- old Task/Execution/Message IDs 유지
- Process 생성 때문에 child state를 복제하지 않음

## 5. ActionGrant Migration

v0.5 approval/membership/authority를 일괄 ActionGrant로 변환하지 않는다. v0.6에서 grant가 필요한 신규 action부터 current policy/approval로 발급한다. 이미 external Side Effect가 Unknown인 상태에 새 grant를 붙여 자동 retry하지 않는다.

## 6. Runtime Memory Migration

MemoryReservation/active permit/current pressure state는 persistent migration 대상이 아니다. upgrade/restart 후 configured Runtime Memory envelope을 초기화하고 active Task/Continuation을 새 admission에서 reconcile한다.

## 7. Idempotency / Crash Safety

migration marker/source-target schema version을 durable하게 관리한다. partial migration crash 후:
- duplicate Process/Grant/Memory relation 생성 0
- existing Memory content rewrite 0
- legacy state가 Verified로 변질 0
- existing ID/revision/provenance 보존

## 8. API Compatibility

Bot-only v0.5 command에 ProcessId/CycleId/epistemic metadata를 필수화하지 않는다. 기존 read response에 additive field를 제공하며 old client가 모르는 Assertion State를 `Verified`로 추론하지 않도록 version/schema compatibility를 정의한다.

## 9. Rollback / Downgrade

- preflight/backup/checkpoint
- v0.5 binary가 v0.6 Process/Grant/Memory epistemic state를 이해하지 못하면 downgrade-blocked 명시
- rollback이 v0.6 Shared Memory를 Bot Memory로 섞거나 retracted item을 Verified로 되살리지 않음
- schema rollback과 external Side Effect/retraction audit를 분리

## 10. Verification

AT-MIG-001 기존 v0.4→v0.5 compatibility를 유지하고 v0.6 추가 fixture에서:
- v0.5 IDs/revisions/provenance unchanged
- legacy Memory auto-Verified 0
- active legacy collaboration synthetic Process 생성 0
- Bot-only/Project/Channel/Provider/Live Control semantic unchanged
- Runtime reservation은 migration state가 아님

## 11. 검증 기준

- migration/rollback deterministic/idempotent.
- v0.5 Acceptance suite가 v0.6 migrated data에서 통과.
- retraction/quarantine state를 downgrade가 조용히 무시해 authoritative knowledge로 만들지 않음.
- exact physical enum/schema를 ADR 전 migration 문서에서 과고정하지 않음.
