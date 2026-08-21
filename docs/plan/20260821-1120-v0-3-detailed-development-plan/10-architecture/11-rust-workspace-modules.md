---
title: "Rust Workspace와 모듈 경계"
document_id: "DXB-ARC-011"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-GOV-003"]
---

# Rust Workspace와 모듈 경계

## 1. 목적

의미 소유권과 실제 변형 축에 맞는 crate 경계를 유지하면서 Common Framework와 Extension SPI를 code dependency graph에 반영한다. Provider/Plugin을 탈부착할 수 있고 불필요한 crate·trait·할당·컴파일 비용을 만들지 않는다.

## 2. 권장 Repository Layout

```text
dxbot/
├─ Cargo.toml
├─ Cargo.lock
├─ crates/
│  ├─ dxb-kernel/
│  ├─ dxb-domain/
│  ├─ dxb-application/
│  ├─ dxb-storage/
│  ├─ dxb-harness/               # Harness capability contract
│  ├─ dxb-provider-sdk/          # extension-safe stable surface
│  ├─ dxb-provider-host/         # common invocation/lifecycle enforcement
│  ├─ dxb-runtime/
│  ├─ dxb-control/
│  ├─ dxb-cli/
│  └─ dxb-testkit/
├─ providers/
│  ├─ dxb-harness-deepseek/
│  ├─ dxb-harness-native/
│  ├─ dxb-model-<provider>/
│  ├─ dxb-storage-<provider>/
│  └─ dxb-executor-<provider>/
├─ plugins/
├─ apps/
├─ tests/
├─ fixtures/
├─ migrations/
├─ benchmarks/
├─ examples/
├─ scripts/
├─ schemas/
├─ generated/
└─ docs/
```

초기 구현에서는 실제 필요가 없는 빈 디렉터리/추상 crate를 미리 만들지 않는다. `dxb-provider-host`를 별도 crate로 분리하기 전에도 동등한 `dxb_runtime::provider_host` module 책임 경계를 유지해야 하며 split 여부는 independent testing/reuse/dependency pressure를 보고 ADR로 확정할 수 있다.

## 3. Naming Rule

- 사용자 정의 directory와 비-Rust source file은 lowercase kebab-case.
- Rust `*.rs` module/source file은 `snake_case`.
- Cargo package name은 kebab-case를 사용하고 Rust import name의 underscore mapping을 따른다.
- `Cargo.toml`, `Cargo.lock`, `.github` 등 tool 고정 파일은 explicit allowlist.
- CI가 새 path를 검사한다.

## 4. 주요 crate 책임

### `dxb-kernel`
ID, Revision, Deadline, bounded size/limit primitive, digest 등 작고 안정된 값 타입. business helper dumping ground 금지.

### `dxb-domain`
Bot/Memory/Goal/Task/Execution/Core/Routine의 순수 상태 의미. I/O, async runtime, Provider/Plugin/Host 타입 금지.

### `dxb-application`
Use Case, Unit of Work, idempotency, auth context, side-effect orchestration. Provider concrete dependency 금지.

### Capability Contract crate/module
Capability별 Request/Response/Event/Error/Config/Resource/Cancel/Deadline/Idempotency/Side Effect/Version semantics를 소유한다. 하나의 mega-contract crate로 모든 Capability를 결합하지 않는다. `dxb-harness`는 Harness 계약의 예다.

### `dxb-provider-sdk`
Provider가 의존할 수 있는 안정 surface를 소유한다. contract type, lifecycle/config/error/telemetry/test helper를 제공할 수 있으나 Domain/Runtime/Storage mutable internals를 재노출하지 않는다.

### `dxb-provider-host`
Provider invocation pipeline, lifecycle manager/registry integration, permission/resource/deadline/cancellation/telemetry/error normalization의 공통 조정을 소유한다. Domain state machine을 구현하지 않고 Application/Policy/Security/Resource Port를 사용한다.

### `dxb-runtime`
Bot coordinator, Task/Routine runtime, Scheduler, recovery composition, Provider Host/Registry composition, structured shutdown.

### `dxb-control`
Control server/client schema 및 transport abstraction. CLI/TUI/Web가 runtime internal function을 직접 호출하지 않게 한다.

### `dxb-testkit`
virtual clock, deterministic executor, fake/reference Provider, failpoint, conformance fixture, architecture test helper를 소유한다.

## 5. Dependency 방향

```text
                    dxb-kernel
                   ↗    ↑     ↖
          dxb-domain   contracts   dxb-provider-sdk
              ↑          ↑              ↑
      dxb-application    │          provider crates
              ↑          │
      dxb-provider-host ─┘
              ↑
          dxb-runtime
              ↑
          dxb-control
              ↑
          interfaces
```

Provider 기본 허용 dependency:

```text
Capability Contract
+ dxb-provider-sdk
+ explicit public Kernel value types
+ approved external SDK
```

금지:
- `dxb-domain → provider/plugin/ui/host implementation`
- `dxb-application → concrete provider`
- `provider → dxb-domain internals`
- `provider → dxb-application internals`
- `provider → dxb-runtime internals`
- `provider → dxb-storage internals`
- Provider crate 간 직접 dependency
- Plugin package → internal non-public crate API
- `dxb-provider-sdk`를 통한 금지 dependency의 transitive re-export

`dxb-application`이 Capability 실행을 필요로 하면 concrete Host crate에 의존하지 않고 Application Port를 정의한다. Runtime composition이 `dxb-provider-host` 구현을 그 Port에 연결해 dependency cycle을 만들지 않는다.

## 6. Standard Provider Package

```text
providers/dxb-<capability>-<provider>/
├─ Cargo.toml
├─ src/
│  ├─ lib.rs
│  ├─ config.rs
│  ├─ provider.rs
│  ├─ error.rs
│  └─ mapping.rs
├─ tests/
│  └─ conformance.rs
└─ fixtures/
```

Provider가 lifecycle/registry/security/resource/retry/telemetry framework를 별도 module로 다시 만들지 않는다.

## 7. Trait/Feature 원칙

- trait는 실제 Capability/Port와 비결정 요소 교체 지점에만 둔다.
- Domain 내부 닫힌 상태는 enum을 우선한다.
- Provider 선택을 Cargo feature만으로 표현하지 않는다.
- compile-time feature는 독립 제거 가능한 build surface에만 사용한다.
- `ProviderContext`를 범용 service locator trait/object map으로 만들지 않는다.

## 8. Provider Scaffold

P1에서 `dxb-dev new-provider <capability> <provider>` 형태의 generator를 도입해 package/config/provider/error/mapping/conformance/fixture/benchmark/doc skeleton을 생성한다. Scaffold template은 Provider SDK/Conformance version과 함께 검증되고 drift test를 가진다.

## 9. Provider 제거 검증

- Provider crate dependency 제거
- registry registration 제거
- provider-specific config migration/error
- Domain/Runtime build 성공
- core acceptance를 fake/reference Provider로 실행
- stale config 명시 오류
- forbidden/orphan dependency 0
- Host path가 unrelated Provider에서 유지됨

## 10. 검증 기준

- `cargo metadata` 기반 dependency firewall에서 금지 edge가 0건이다.
- Domain tests가 DB/network/Provider 없이 실행된다.
- Application Port와 Host implementation 사이에 dependency cycle이 없다.
- Provider package가 표준 skeleton 또는 동등한 소유 경계를 따른다.
- Provider A 제거 profile에서 core acceptance가 통과한다.
- Provider SDK public surface가 Runtime internals를 누출하지 않는다.
