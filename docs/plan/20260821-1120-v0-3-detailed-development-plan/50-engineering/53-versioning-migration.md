---
title: "버전·호환성·Migration"
document_id: "DXB-ENG-053"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-015", "DXB-ARC-016", "DXB-ARC-017", "DXB-IFC-040"]
---

# 버전·호환성·Migration

## 1. 목적

Persistent Bot/Memory를 보호하면서 DB/Event/API/Config/Capability Contract/Provider SDK/Provider/Plugin을 독립 버전으로 진화시키고 Common Contract 변경 비용을 엄격히 통제한다.

## 2. 독립 버전 대상

Runtime binary, Domain Event, DB/Snapshot/Memory, Control API, Config/Profile, Artifact manifest, Capability Contract, Provider SDK, Provider package/protocol/config, Conformance suite, Plugin package/public API/config/data schema, Projection/Index generation.

## 3. Capability Contract 변경 분류

### Additive Compatible
기존 Provider/Consumer 의미를 바꾸지 않는 optional field/feature negotiation 추가. 기존 Conformance 유지 + 신규 optional test.

### Behavior Clarification
기존 의도를 명확화하지만 observable behavior가 바뀌지 않아야 한다. ambiguous Provider inventory를 확인한다.

### Deprecation
대체 contract/field/feature와 removal window를 제공한다. 신규 사용 경고/차단 계획을 둔다.

### Breaking Semantic Change
input/output/error/lifecycle/cancel/deadline/resource/idempotency/side-effect/version 의미가 기존 구현과 호환되지 않는 변경.

필수 동반:
- ADR
- migration/compatibility plan
- affected Provider inventory
- Provider SDK update
- Conformance update
- Reference Provider update
- Acceptance/Risk update
- rollout/rollback/removal plan

Extension 구현 편의를 이유로 Breaking을 patch처럼 처리하지 않는다.

## 4. Provider Compatibility Matrix

Provider registration 전에 다음을 검증한다.

```text
Runtime ↔ Capability Contract
Capability Contract ↔ Provider implementation
Provider SDK ↔ Provider source/package
Provider config schema ↔ persisted config
Plugin public API ↔ Plugin package (해당 시)
```

Incompatible Provider는 Ready Registry에 publish하지 않는다.

## 5. Feature / Provider Removal

Active → Deprecated → New-use-blocked → Draining → Detached → Compatibility window → config/data cleanup → dependency/code removal → Removed를 따른다.

Provider 제거가 Bot Identity/Memory schema migration을 요구하면 boundary 침투 결함을 우선 검토한다.

## 6. Provider SDK Versioning

SDK는 internal Runtime layout을 그대로 재export하지 않는다. stable surface의 breaking change는 영향을 받는 Provider inventory와 rebuild/certification 범위를 명시한다. generated scaffold template도 SDK/contract version과 함께 갱신한다.

## 7. Conformance Versioning

Conformance suite는 Contract version에 매핑한다. suite 자체가 semantic contract를 암묵 변경하지 않으며 test 추가가 실제 behavior tightening이면 Behavior Clarification/Breaking 여부를 먼저 분류한다.

## 8. Plugin Version / Migration

Plugin package/public API/capability contract/config/data schema는 독립 버전을 가진다. Plugin Provider도 일반 Provider compatibility gate를 통과한다.

## 9. Waiting / Side Effect Schema

Continuation과 Side Effect Ledger는 P0 persistent schema이며 version decoder/migration fixture를 가진다. Provider Contract migration이 ledger action digest/idempotency 의미를 소실시키지 않는다.

## 10. Rollback

binary/config/schema/data/external side effect/Provider/SDK/Contract/Plugin을 구분한다. Side Effect는 DB rollback만으로 외부 상태를 되돌렸다고 간주하지 않는다.

## 11. 검증 기준

- old fixture DB/Event/Continuation/Ledger를 current Runtime이 복원한다.
- incompatible Contract/SDK Provider가 Ready가 되지 않는다.
- Breaking Capability change가 필수 영향 문서 없이 CI를 통과하지 않는다.
- Provider removal 후 core acceptance가 통과한다.
- downgrade 불가능 변경이 배포 전에 표시된다.
