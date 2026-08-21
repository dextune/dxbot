---
title: "버전·호환성·Migration"
document_id: "DXB-ENG-053"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-015", "DXB-ARC-016", "DXB-IFC-040"]
---

# 버전·호환성·Migration

## 1. 목적

Persistent Bot/Memory를 보호하면서 DB/Event/API/Config/Provider/Plugin을 독립 버전으로 진화시키고 선택 기능의 제거 lifecycle을 규정한다.

## 2. 독립 버전 대상

Runtime binary, Domain Event, DB, Snapshot, Memory, Control API, Config/Profile, Artifact manifest, Harness/Provider protocol, Capability contract, Plugin package/public API/config/data schema, Bot export, Projection/Index generation.

하나의 global version으로 모든 호환성을 표현하지 않는다.

## 3. 기본 호환성

- supported older reader range
- current schema writer
- unknown major/critical variant reject/quarantine
- additive optional field 우선
- Event immutable
- migration ledger/source digest
- Projection/Index rebuild generation
- API capability negotiation

## 4. Feature Removal Lifecycle

선택 기능 제거는 버전 작업이다.

```text
Active
 → Deprecated
 → New-use-blocked
 → Draining
 → Detached
 → Compatibility window
 → Data/config migration or purge
 → Derived cleanup
 → Dependency/code removal
 → Removed
```

각 단계는 min/max supported runtime/provider/plugin version과 rollback 가능성을 기록한다.

## 5. Provider Removal

- Provider ID/version을 catalog에서 Deprecated로 표시
- selector 신규 사용 차단
- in-flight Execution drain
- persisted execution/trace refs는 역사 데이터로 보존 가능
- stale config는 replacement/migration/error
- Provider-specific resume token은 호환 불가 시 새 attempt/manual 처리
- provider-owned derived data cleanup
- crate/dependency 제거
- Provider-free build + restore acceptance

Provider 제거가 Bot Identity/Memory schema migration을 요구하면 경계 침투 결함을 먼저 검토한다.

## 6. Plugin Version / Migration

독립 버전:
- package
- required DXBOT public API range
- capability contract versions
- config schema
- plugin-owned data schema

upgrade는 preflight → backup/export → migration → candidate enable → health/conformance → switch → old drain → cleanup 순으로 한다. irreversible data migration은 approval/rollback impossibility를 명시한다.

## 7. Plugin Uninstall Data Semantics

Manifest는 `retain | export-and-remove | purge | migrate | block` 중 지원 정책을 선언한다. uninstall operation은 실제 적용 정책과 data digest/evidence를 기록한다.

- Core Domain data는 Plugin uninstall의 owned data가 아님
- Plugin이 생성한 Domain Event/Task/Memory는 일반 Domain retention을 따름
- plugin-owned private tables/files/index만 manifest policy로 정리
- backup retention과 secret deletion도 별도 ledger로 추적

## 8. Config Migration

unknown key silent ignore 금지. deprecated key는 replacement와 removal version을 가진다. removed Provider/Plugin config는 명시 diagnostic이며 safe selector rule 없이 다른 Provider로 자동 전환하지 않는다.

## 9. Waiting / Side Effect Schema

Continuation과 Side Effect Ledger는 P0 persistent schema이며 version decoder/migration fixture를 가진다. migration 중 Waiting 상태만 남거나 ledger action digest가 소실되는 변환을 금지한다.

## 10. Rollback

binary/config/schema/data/external side effect/Provider/Plugin을 구분한다. Side Effect는 DB rollback만으로 외부 상태를 되돌렸다고 간주하지 않는다. irreversible migration은 backup, abort point, recovery procedure가 필수다.

## 11. 검증 기준

- old fixture DB/Event/Continuation/Ledger를 current Runtime이 복원한다.
- Provider removal migration 후 core acceptance가 통과한다.
- Plugin upgrade/uninstall kill test가 data duplication/loss policy를 위반하지 않는다.
- stale config가 silent ignore되지 않는다.
- downgrade 불가능 변경이 배포 전에 표시된다.
