---
title: "Repository·CI·Release 운영"
document_id: "DXB-ENG-054"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-011", "DXB-ARC-017", "DXB-ENG-050", "DXB-ENG-052", "DXB-ENG-053"]
---

# Repository·CI·Release 운영

## 1. 목적

Repository layout/naming, Provider dependency firewall, Host enforcement, Conformance/Scaffold drift, Removal cleanliness, 문서 추적성을 CI에서 자동 검증한다. Repository naming/layout의 Canonical Owner다.

## 2. Normative Layout

```text
crates/       # core/internal Rust + provider-host/sdk/contract ownership
providers/    # replaceable Provider implementations
plugins/      # first-party/reference Plugin packages
apps/         # deployable applications
docs/         # normative/design/operations docs
tests/        # cross-crate integration/e2e
fixtures/     # versioned test fixtures
migrations/   # persistent schema migrations
benchmarks/   # benchmark code/manifests
schemas/      # public/persistence/config schemas
generated/    # generated artifacts with declared source
scripts/      # repository automation/scaffold tooling
```

## 3. Naming Rule

사용자 정의 directory/non-Rust file stem과 Rust package는 lowercase kebab-case, Rust `*.rs`는 snake_case를 사용한다. Tool-mandated allowlist 외 예외를 만들지 않는다.

## 4. Provider Package Gate

Provider package는 표준 skeleton 또는 동등한 명시 소유 구조를 따른다.

Required:
- typed config
- provider implementation
- DTO mapping
- error mapping
- conformance test
- fixture where applicable

Forbidden responsibility scan:
- global retry manager
- permission/approval system
- scheduler/resource admission
- domain repository
- global telemetry/audit subsystem

파일명 기반 scan만으로 판정하지 않고 dependency/API/behavior test와 함께 사용한다.

## 5. Dependency Firewall Gate

`cargo metadata`/workspace policy로 Provider가 Capability Contract, Provider SDK, 공개 Kernel, approved external dependency 외 internal crate에 의존하지 않는지 검사한다.

금지 edge:
- Provider→Domain/Application/Runtime/Storage
- Provider→Provider
- Domain/Application→concrete Provider
- Provider SDK→internal mutable API re-export

## 6. Provider Host Enforcement Gate

- production direct Provider invocation 탐지
- 모든 Runtime consumer가 Host facade 사용
- Host-path integration test
- Draining/provider generation activity guard
- permission/resource/deadline/cancel/output/accounting probe

compile-time architecture rule과 runtime conformance를 함께 사용한다.

## 7. Fast CI Gate

- format/compile/lint
- naming/layout
- document ID/depends_on/link
- dependency firewall
- Provider Host enforcement architecture test
- schema/API/Contract diff classification
- scaffold/generated drift
- requirement/acceptance/risk link

## 8. Full CI Gate

- unit/property
- storage/recovery
- Waiting/Side Effect crash
- Routine restart
- Provider Host + Executable Conformance + Reference Provider
- replacement/removal/multi-selection
- lifecycle/start/drain/stop/failure
- migration fixtures
- minimal/provider-free build
- performance/resource leak smoke

## 9. Scaffold Generator Gate — P1

`dxb-dev new-provider`가 생성하는 template을 golden fixture로 검증한다.

- current Capability/SDK version
- naming/layout
- allowed dependencies only
- conformance test included
- no obsolete API/TODO
- regenerate diff deterministic

## 10. Contract Change Gate

Capability/Common public diff를 Additive/Clarification/Deprecation/Breaking으로 분류한다. Breaking이면 ADR, migration, affected Provider inventory, SDK/Conformance/Reference/Acceptance/Risk update가 없으면 merge를 차단한다.

## 11. Removal Cleanliness Gate

Provider/Plugin 제거 PR은 Cargo dependency/feature, registry, config schema, docs, API catalog, derived data, generated client, fixture, SBOM orphan을 검사한다. 관련되지 않은 core acceptance가 통과해야 한다.

## 12. 문서 3회 Review Gate

v0.3 Architecture Package와 동등한 범위의 Normative 변경은 `DXB-ENG-052`의 Structural → Contract/Traceability → Development Flow/Cross-Layer Review 증거를 남긴다.

## 13. 검증 기준

- naming/layout/dependency/Host bypass 위반이 CI에서 차단된다.
- Provider 제거 후 orphan dependency/registry/config가 0이다.
- Scaffold template이 SDK/Contract와 drift하지 않는다.
- Breaking Contract change가 영향 파일 누락 상태로 merge되지 않는다.
- docs links/traceability/manifest가 clean하다.
