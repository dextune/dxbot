---
title: "버전·호환성·Migration"
document_id: "DXB-ENG-053"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-015", "DXB-ARC-016", "DXB-ARC-017", "DXB-DOM-022", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-038", "DXB-RUN-036", "DXB-IFC-040"]
---

# 버전·호환성·Migration

## 1. 목적

v0.6 Backend Canonical data와 identity를 보존하면서 v0.7 Application Contract/CLI protocol compatibility를 추가한다. Backend data migration과 client/protocol compatibility를 서로 다른 문제로 관리한다.

## 2. v0.6 → v0.7 Backend Data

이상적으로 Canonical data migration을 발생시키지 않는다.

unchanged 기본:
- BotId
- ConversationId / ThreadId / lineage
- ProjectId / ChannelId
- TaskId / ExecutionId
- MemoryId / revision / provenance / epistemic state
- Durable Process identity/progress
- Directive / Side Effect ledger
- ActionGrant identity/consumption

Interface 개발 때문에 ID를 재발급하거나 Domain row를 rewrite하지 않는다.

## 3. Protocol / Schema Version 분리

```text
protocol_version ≠ schema_version ≠ runtime_version ≠ client_version ≠ data_schema_version
```

각 버전은 compatibility range/feature negotiation 규칙을 가진다. one-number lockstep을 기본값으로 가정하지 않는다.

## 4. Compatibility Matrix

최소 matrix:

```text
old CLI ↔ current compatible Runtime
current CLI ↔ previous compatible Runtime
incompatible CLI ↔ Runtime
```

검증:
- compatible pair semantic 동일
- unknown optional read field safe handling
- incompatible mutation schema explicit failure
- silent downgrade/reinterpretation 0
- unsupported capability를 추측 호출하지 않음

## 5. Command Compatibility

mutation schema 변화는 특히 엄격하다.
- action/target/expected revision/idempotency 의미가 바뀌면 explicit contract versioning
- client가 모르는 required mutation semantic은 fail
- retry key/receipt semantics를 버전 업그레이드가 깨뜨리지 않음
- old client가 stale/new authorization field를 무시해 권한을 우회하지 못함

## 6. Subscription Compatibility

- event identity/cursor/watermark semantics version 관리
- client가 모르는 optional event field는 compatible 범위에서 무시 가능
- cursor retention/resync behavior 변화는 compatibility 문서화
- schema mismatch 상태에서 silent stream continuation 금지

## 7. Planning Baseline Migration

v0.7 문서 baseline은 다음을 수행한다.
- v0.6 Backend normative docs 상속
- `DXB-IFC-040/041` 강화
- `DXB-IFC-042/043` active scope supersession
- 관련 Interface 전용 milestone/gate/Risk/OQ active inheritance 제거
- readme/manifest inventory 갱신

## 8. Rollback / Downgrade

- Runtime data downgrade 안전성은 기존 v0.6 migration 정책 유지
- CLI binary downgrade가 data migration을 자동 수행하지 않음
- incompatible protocol pair는 blocked로 표시
- Runtime service/CLI packaging upgrade와 canonical schema migration을 분리

## 9. Verification

- v0.6 data fixture가 v0.7 Runtime/Application Contract에서 unchanged identity로 load
- old/new compatible pair contract fixture
- incompatible pair explicit fail
- command idempotency across reconnect/version negotiation
- event cursor/resync compatibility
- data schema/protocol version confusion 0

## 10. 검증 기준

- v0.6 Backend data migration 0이 기본.
- CLI/Runtime independent upgrade가 silent semantic corruption을 만들지 않음.
- incompatible pair explicit failure.
- superseded Interface 제거가 Backend data에 영향 없음.
