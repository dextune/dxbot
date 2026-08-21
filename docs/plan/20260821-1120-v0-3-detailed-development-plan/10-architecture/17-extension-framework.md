---
title: "Common Framework와 Extension SPI"
document_id: "DXB-ARC-017"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-ARC-011", "DXB-ARC-012", "DXB-GOV-003"]
---

# Common Framework와 Extension SPI

## 1. 목적과 Canonical Ownership

이 문서는 DXBOT의 **Extension Framework 구현 규범**의 Canonical Owner다. `12-shared-contracts-extension-model.md`가 Capability의 의미와 I/O 계약을 소유한다면, 본 문서는 그 계약을 Provider가 안전하고 정형화된 방식으로 구현하게 만드는 Common Framework, Provider Host, Lifecycle, SDK, Conformance, Scaffold, 품질 등급과 AI 위임 경계를 소유한다.

> 확장 기능 개발자는 DXBOT 아키텍처를 재설계하지 않는다. Common Framework가 제공하는 안정된 계약과 Extension Slot을 구현한다.

```text
Frontier-class Architecture Work
        ↓
Common Framework / Stable Contract / Test Framework
        ↓
Typed Extension SPI Slot
        ↓
Provider / Adapter Implementation
        ↓
Capability-specific External Integration
```

## 2. Common Framework와 Extension Zone

```text
┌──────────────────────────────────────────────────────────┐
│                    DXBOT COMMON                          │
│ Domain / Application / State / Policy / Recovery         │
│ Capability Contracts                                     │
│ Provider Lifecycle / Registry / Selector / Provider Host │
│ Permission / Resource / Deadline / Cancellation          │
│ Error Normalization / Retry Disposition                  │
│ Side-effect Guard / Idempotency / Reconciliation         │
│ Telemetry / Audit / Config / Version Compatibility       │
│ Testkit / Conformance / SDK / CI Architecture Gates      │
└───────────────────────────┬──────────────────────────────┘
                            │ Stable SPI Boundary
             ┌──────────────┼──────────────┐
             ▼              ▼              ▼
         Provider A     Provider B     Provider C
```

Provider 내부 최적화 품질과 무관하게 Input/Output, Error, Lifecycle, Cancellation/Deadline, Side Effect/Idempotency, Security, Resource, Version/Compatibility, Observability/Audit 의미는 Common 때문에 흔들리지 않아야 한다.

## 3. 책임 분리

### 3.1 Common이 반드시 소유하는 책임

| 영역 | Common Canonical Owner |
|---|---|
| Lifecycle | validate/start/quiesce/stop/health state interpretation |
| Registration | 등록/해제/immutable registry generation |
| Selection | compatible set + explicit Selector/Policy |
| Security | permission/grant/approval enforcement |
| Resource | concurrency/memory/cost/rate admission/accounting |
| Deadline | effective deadline 계산과 전파 |
| Cancellation | cancellation wiring, observation, termination contract |
| Retry | retry eligibility/disposition와 상위 attempt policy |
| Side Effect | intent/idempotency/reconciliation guard |
| Telemetry | 공통 span/metric/log envelope |
| Audit | 민감·관리 작업의 공통 감사 |
| Config | schema validation/snapshot/version/digest |
| Errors | stable taxonomy와 provider error normalization contract |
| Recovery | crash/restart/reconcile orchestration |
| Compatibility | Capability/API/SDK contract version |
| Testing | conformance/fault/architecture/removal test framework |

Provider가 위 기능을 자체 시스템으로 중복 구현하거나 우회하는 것을 금지한다.

### 3.2 Extension이 소유하는 책임

```text
Provider-specific Config
+ External SDK/API Adapter
+ Capability-specific Business Logic
+ Provider DTO ↔ DXBOT DTO Mapping
+ Provider Error → Stable Error Mapping
```

Provider-specific protocol의 handshake/connection retry가 필요하더라도 DXBOT의 semantic retry/attempt/permission/resource 정책을 대체해서는 안 된다. 외부 재호출이 새 Side Effect 또는 새 semantic attempt라면 Common으로 위임한다. 이를 **Thin Provider Principle**로 정의한다.

## 4. Standard Provider Lifecycle

상태와 전이는 Common이 소유하며 Provider는 hook/health observation만 제공할 수 있다.

```text
Declared → Validated → Starting → Ready → Draining → Stopped
                              ↕
                           Degraded
Validation/compatibility/security failure → Failed / Quarantined / Incompatible
```

표준 hook 개념:

```rust
trait ProviderLifecycle {
    fn validate(...);
    async fn start(...);
    async fn quiesce(...);
    async fn stop(...);
    fn health(...);
}
```

정확한 Rust signature는 구현 ADR/API freeze에서 확정한다.

불변조건:
- `Ready` 전 신규 selection 금지
- `Draining` 진입 후 신규 selection/activity 차단
- in-flight activity/reference와 quiescence 판정은 Common 소유
- Provider의 임의 `idle=true`로 unload 불가
- stop은 owned child/process/channel/resource 정리 후 완료
- health sample과 lifecycle state 구분
- incompatible/quarantined Provider는 재검증/운영자 조치 없이 자동 복귀 금지

## 5. Capability Contract Package

각 Optional Capability는 `DXB-ARC-012`를 기준으로 다음을 가진다.

```text
Request
Response
Streaming Event
Stable Error / Outcome
Config Contract
Capability Metadata
Cancellation Contract
Deadline Contract
Resource Contract
Idempotency Contract
Side Effect Classification
Version / Compatibility Contract
Conformance Suite
```

Provider는 계약을 구현하며 의미를 변경하지 않는다. optional behavior는 metadata/negotiation으로 표현한다.

## 6. Provider Host — Mandatory Invocation Boundary

```text
Runtime Consumer
       ↓
Provider Host
       ↓
Provider SPI
       ↓
Provider Implementation
```

공통 pipeline:

```text
Request/schema validation
→ Registry generation + compatible Provider resolution
→ Explicit selection + generation pin
→ Permission/approval enforcement
→ Resource admission/reservation
→ Effective deadline calculation
→ Cancellation wiring
→ Side-effect/idempotency guard preparation
→ Common telemetry/audit start
→ Provider call
→ Error/outcome normalization
→ Output/size/schema validation
→ Usage/resource accounting
→ Side-effect outcome/reconciliation handoff
→ Telemetry/audit finalize
→ permit/activity release
```

불변조건:
- production Provider call의 Host 우회 금지
- direct SPI unit test가 있어도 integration은 실제 Host path 검증
- Host는 Domain 의미를 재구현하지 않고 owner Port를 사용
- Host failure와 Provider failure를 구분
- 같은 Capability Provider는 같은 Host pre/post condition을 적용받음

## 7. Minimum Provider Surface

Provider에 `RuntimeContext`, service locator, raw DB, Scheduler, Domain Store, Registry mutator를 전달하지 않는다. Capability별 `ProviderCallContext`는 request metadata, effective deadline/cancellation, approved transport/credential handle, resource grant, bounded output sink, telemetry facade 등 필요한 최소 표면만 가진다.

금지 예:

```text
Provider::new(RuntimeContext)
ProviderContext.get::<DomainStore>()
ProviderContext.scheduler().spawn_unbounded(...)
```

새 Common service가 필요하면 locator를 추가하지 않고 Contract/SDK 변경 필요성을 Tier A에서 검토한다. 이를 **Minimum Provider Surface Principle**로 정의한다.

## 8. Retry, Error, Side Effect

- 외부 오류는 stable error/outcome으로 typed mapping한다.
- retry eligibility는 Common Contract/Policy가 결정한다.
- Task/Execution semantic retry는 새 attempt를 만드는 상위 Runtime 책임이다.
- Provider-local transport retry는 Contract가 명시적으로 허용하고 호출 의미·deadline·idempotency가 변하지 않을 때만 가능하다.
- non-idempotent/결과 불명 Side Effect를 Provider가 독자 retry하지 않는다.
- `Unknown/Reconciliation Required`를 transient failure로 축약하지 않는다.

## 9. Provider SDK

Provider SDK는 내부 Runtime API가 아니라 **허용된 구현 표면**만 공개한다.

```text
stable contract types/re-exports
lifecycle/config/error helpers
bounded transport/output helpers
telemetry facade
test fixture adapters
conformance helpers/macros
reference implementation examples
```

Domain/Storage/Scheduler/Registry mutable internals, raw secret store, privileged Control bypass를 노출하지 않는다.

## 10. Standard Provider Skeleton과 Scaffold

```text
providers/
  dxb-<capability>-<provider>/
    Cargo.toml
    src/
      lib.rs
      config.rs
      provider.rs
      error.rs
      mapping.rs
    tests/
      conformance.rs
    fixtures/
```

- `config.rs`: Provider 전용 typed config/schema mapping
- `provider.rs`: Capability 실제 구현
- `error.rs`: 외부 오류 → stable DXBOT 오류
- `mapping.rs`: Provider DTO ↔ DXBOT DTO
- `conformance.rs`: Common suite에 fixture/factory 연결

P1 목표:

```text
dxb-dev new-provider <capability> <provider>
```

package/layout, config/SPI/error/DTO skeleton, manifest/metadata, conformance, benchmark, fixture, 문서 template을 생성한다. Agent가 매번 구조와 테스트 시나리오를 재설계하지 않는다.

## 11. Executable Conformance Framework

Common이 scenario를 제공하고 Provider는 fixture/factory와 expected metadata만 제공한다.

개념:

```rust
model_gateway_conformance!(MyModelProvider);
memory_index_conformance!(MyMemoryIndex);
sandbox_conformance!(MySandbox);
```

검증에는 normal, malformed, unsupported/version, timeout, cancel, provider failure, partial/streaming, large output, backpressure, resource exhaustion, permission denied, side-effect unknown, shutdown/drain, restart/re-registration, multi-selection, removal/unavailable을 포함한다. 최소 한 suite는 `Consumer → Provider Host → Provider` 전체 경로로 실행한다.

## 12. Golden Reference Provider

주요 Capability는 Frontier-quality Common 작업에서 deterministic Reference Provider를 유지한다. 올바른 ownership/error/cancel/SDK 패턴을 코드로 보여주고 Conformance self-test에 사용한다. production 기본 Provider일 필요는 없다. negative fixture와 Host integration을 별도로 둬 Reference와 suite의 동시 오류를 탐지한다.

## 13. Extension Dependency Firewall

허용 기본 영역:

```text
Capability Contract
Provider SDK
공개 허용 Kernel value types
승인된 external SDK/library
```

기본 금지:

```text
dxb-domain internals
dxb-application internals
dxb-runtime internals
dxb-storage internals
다른 Provider
Interface/UI
private application modules
Plugin Manager internals
```

필요 공통 기능은 금지 dependency로 우회하지 않고 SDK의 좁은 stable surface로 승격할지 Tier A에서 검토한다. CI는 `cargo metadata`/dependency policy/public API를 검사한다.

## 14. Common Contract Stability

변경은 `Additive Compatible | Behavior Clarification | Deprecation | Breaking Semantic Change`로 분류한다.

Breaking에는 최소 ADR, migration/compatibility, affected Provider inventory, Conformance, Reference Provider, Provider SDK, Acceptance/Risk, rollout/rollback/removal 영향 갱신이 필요하다.

Extension 편의를 위해 Common Contract를 임의 변경하지 않는다. **Common Contract 변경 비용이 Provider 구현 비용보다 높은 것이 정상**이다.

## 15. Quality Tier와 AI Delegation Boundary

### Tier A — Common / Frontier Quality

Domain invariant/state, Capability Contract, Host/Lifecycle/Registry/Selector, Security/Resource/Persistence/Recovery/Concurrency, Error taxonomy, Conformance/Testkit/SDK, Migration/Public compatibility.

요구: 깊은 설계/ADR 영향, property/model/concurrency/fault, 필요한 benchmark/profile, API stability, 코드·문서 교차 검수, removal/rollback.

### Tier B — Extension / Implementation Quality

기존 Capability Provider, 외부 API adapter, DTO/error mapping, provider-specific config/fixture/optional integration.

최소 Gate:

```text
Compile
+ Contract compliance
+ Conformance pass
+ No forbidden dependency
+ No Host/security/resource bypass
+ Removal build/registration cleanliness
+ Resource/leak check where applicable
```

Common Contract 변경이 필요해지는 즉시 Tier A로 재분류한다.

## 16. AI Extension Task Template

```text
Classification: Provider
Quality Tier: B
Capability: <ModelGateway>
Implement: providers/dxb-<capability>-<provider>
Allowed dependencies: capability contract, dxb-provider-sdk, approved external SDK
Forbidden: domain/application/runtime/storage internals, other providers
Required: Config, Provider, Error Mapping, DTO Mapping
Do not implement: global retry, permission, resource admission, scheduler,
                  domain persistence, global telemetry/audit, lifecycle orchestration
Required verification: compile/lint, conformance, Host-path integration,
                       provider removal/dependency gate, leak/resource test
```

작업 규격이 구현 Agent의 임의 아키텍처 판단보다 우선한다.

## 17. Plugin과의 관계

- Plugin Manager: package/install/enable/disable/upgrade/uninstall/data lifecycle
- Plugin Host: process/WASM/native isolation과 package-level execution boundary
- Provider Host: Capability invocation의 Common enforcement

Plugin이 Provider를 제공하면 Plugin lifecycle 후 Registry candidate로 등록하고 실제 call은 동일 Provider Host를 거친다.

## 18. Provider Removal Contract

1. Deprecated
2. 신규 selection 차단
3. Draining
4. Common activity/ref 기준 quiescence
5. Registry detach/stop
6. stale config/reference migration/error
7. provider-owned derived data cleanup
8. dependency/feature/code 제거
9. Provider-free build/restore
10. unrelated Core acceptance + related explicit unsupported

Domain schema 변경이 필요하면 Extension Boundary 침투를 우선 조사한다.

## 19. 적용 단계

### Phase 1 — Common Contract Freeze Foundation / P0
Common/Extension 책임, Lifecycle, Complete Contract, Host, Thin/Minimum Surface, Firewall, Quality Tier/AI Delegation.

### Phase 2 — Provider Development Framework / P0-P1
Provider SDK, Executable Conformance, Reference Provider, Standard skeleton/fixture, CI gates.

### Phase 3 — Automation / P1
Scaffold Generator, contract/schema/test/docs generation, static dependency checks.

### Phase 4 — Ecosystem / P2
external/public SDK, remote Provider, compatibility certification, third-party Plugin/Provider ecosystem.

Phase 1 Gate가 닫히기 전 production Provider를 대규모로 늘리지 않는다.

## 20. 검증 기준

`61-acceptance-traceability.md`가 상세 소유한다.

- AT-SPI-001 Capability I/O Contract
- AT-SPI-002 Standard Provider Lifecycle
- AT-SPI-003 Provider Host Enforcement
- AT-SPI-004 Thin Provider Boundary
- AT-SPI-005 Forbidden Dependency
- AT-SPI-006 Provider Scaffold
- AT-SPI-007 Executable Conformance Suite
- AT-SPI-008 Contract Compatibility
- AT-SPI-009 Reference Provider Equivalence
- AT-SPI-010 Provider Removal

최종적으로 Tier A가 새 Capability의 Contract/Host/Conformance/Reference를 정의하고, Tier B Agent에는 정해진 SPI/파일 TODO 구현과 Conformance 통과만 요구할 수 있어야 한다.
