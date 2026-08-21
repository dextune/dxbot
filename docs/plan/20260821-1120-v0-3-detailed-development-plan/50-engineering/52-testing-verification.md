---
title: "테스트·검증 전략"
document_id: "DXB-ENG-052"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-013", "DXB-ARC-016", "DXB-ARC-017", "DXB-ENG-050"]
---

# 테스트·검증 전략

## 1. 목적

상태기계·동시성·복구와 함께 Provider Host/SPI/Conformance/Dependency Firewall을 deterministic한 증거로 검증한다.

## 2. 계층

Unit, Property/Model, Concurrency Model, Component, Contract/Conformance, Host Integration, E2E, Fault/Recovery, Performance, Soak, Security, Architecture/Docs를 사용한다. 핵심 CI는 deterministic Reference/Fake Provider로 외부 모델 비결정성을 분리한다.

## 3. Deterministic Testkit

virtual Clock, deterministic ID/RNG, scripted/reference Harness/Model/Tool/Sandbox, transactional temp store, event recorder, scheduler executor, fault transport, Artifact store, policy fixture, failpoint, invariant checker를 제공한다.

Provider framework fixture:
- fake/reference Provider A/B/C
- lifecycle probe/failure injector
- Host permission/resource/deadline/cancel probe
- malformed/partial/large-output stream
- side-effect external status simulator
- forbidden dependency/Host bypass architecture fixture

## 4. Executable Provider Conformance

Common이 Capability별 suite를 소유하고 Provider는 factory/fixture만 제공한다.

개념:

```rust
model_gateway_conformance!(MyProvider);
memory_index_conformance!(MyProvider);
sandbox_conformance!(MyProvider);
```

공통 시나리오:
- normal/streaming
- malformed input
- unsupported capability/version
- provider failure
- timeout/cancellation
- partial result
- large output/output cap
- backpressure/resource exhaustion
- permission denied
- side-effect unknown/reconciliation
- start/health/degrade/drain/stop
- restart/re-registration
- multiple selection
- removed/unavailable provider
- config/SDK/contract version mismatch

## 5. Host-Path Equivalence

Conformance는 direct SPI test로만 끝내지 않는다. 최소 한 suite는 실제 `Consumer → Provider Host → Provider` 경로를 사용해 permission/resource/deadline/cancellation/telemetry/error normalization을 포함한다.

검증:
- Provider가 Host를 우회할 수 없음
- 모든 Provider에 동일 pre/post condition
- Host error와 Provider error가 구분됨
- permit/activity cleanup
- Draining 이후 신규 call 차단

## 6. Reference Provider

Reference Provider는 SDK/Conformance의 self-test 대상이다. 정상 구현 하나만으로 suite를 검증하지 않고 intentionally broken provider fixture를 사용해 다음을 실패시킨다.

- lifecycle 위반
- output schema 위반
- cancellation 미관측
- side-effect duplicate
- forbidden dependency/ambient authority
- resource/accounting 누수

## 7. Cross-Layer Scenario Suite

기존 Bot Persistence, Multi-Bot, Waiting Recovery, Side Effect, Routine, Provider Replacement/Removal/Multi-Provider, Plugin Lifecycle 시나리오를 유지한다.

Provider Framework 추가 흐름:

`Task/Core Consumer → Provider Host → Registry/Selector → Permission → Resource → SPI → Provider → Normalize → Commit/Recovery → Projection/API`.

## 8. Architecture / Removal Tests

- crate dependency allowlist/firewall
- Domain→Provider/Plugin 금지
- Provider→Domain/Application/Runtime/Storage/Provider 금지
- production direct Provider call 금지
- Provider SDK public surface leak 검사
- minimal/provider-free compile
- orphan dependency/feature/registry/config
- standard scaffold drift
- public API/contract compatibility

## 9. AT-SPI Suite

- AT-SPI-001 Contract completeness
- AT-SPI-002 Lifecycle state/transition
- AT-SPI-003 Host enforcement
- AT-SPI-004 Thin Provider boundary
- AT-SPI-005 Forbidden dependency
- AT-SPI-006 Scaffold generation
- AT-SPI-007 Conformance auto suite
- AT-SPI-008 Contract compatibility
- AT-SPI-009 Reference Provider equivalence/negative fixture
- AT-SPI-010 Provider removal

## 10. 문서 관계성 3회 Review

v0.3 같은 Architecture Package 변경은 다음 세 검토 증거를 분리해 남긴다.

### Review 1 — Structural / Canonical Ownership
file/path/ID/depends_on/Canonical Owner/용어/중복 규칙/naming/manifest를 검사한다.

### Review 2 — Contract / Traceability
Capability↔Provider Host↔Runtime policy↔Testing↔Acceptance↔Risk 연결, 같은 규칙의 SSOT, breaking change 영향과 missing reference를 검사한다.

### Review 3 — Development Flow / Cross-Layer Executability
`Capability Definition → Common Host/SDK/Testkit → Scaffold/Provider → Conformance/CI → Runtime Invocation → Failure/Recovery → Removal`과 실제 `Command → Domain → Persistence → Scheduler → Provider Host → Provider → Recovery → API` 흐름을 정상/실패/취소/crash/version mismatch/removal로 추적한다.

각 Review는 발견 사항을 수정한 뒤 해당 범위를 재확인한다.

## 11. Release Gate

P0:
- format/lint/build
- naming/dependency/docs structural
- unit/property
- storage/Waiting/Side Effect recovery
- Provider Host/Conformance/Reference suite
- provider replacement/removal/multi-select
- forbidden dependency/Host bypass
- security/resource smoke
- migration
- Cross-Layer suite

P1: Scaffold generator drift, Plugin full lifecycle, actual Harness canary, soak를 추가한다.

## 12. 검증 기준

- AT-SPI-001~010이 자동 suite/architecture gate에 연결된다.
- actual model outage가 Core Runtime gate를 흔들지 않는다.
- lifecycle/resource/cancel test가 sleep 기반 race에 의존하지 않는다.
- 세 문서 Review의 별도 evidence가 release artifact에 존재한다.
