---
title: "Rust Workspace와 모듈 경계"
document_id: "DXB-ARC-011"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-GOV-003"]
---

# Rust Workspace와 모듈 경계

## 1. 목적

의미 소유권과 실제 변형 축에 맞는 crate 경계를 유지하면서 Provider/Plugin을 탈부착할 수 있고, 불필요한 crate·trait·할당·컴파일 비용을 만들지 않는 workspace를 정의한다.

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
│  ├─ dxb-harness/               # Harness capability contract only
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
├─ plugins/                       # first-party/reference plugin packages only
├─ apps/
│  ├─ dxb-daemon/
│  ├─ dxb-tui/
│  └─ dxb-web/
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

초기 구현에서는 실제 필요가 없는 빈 디렉터리/추상 crate를 미리 만들지 않는다. 위 구조는 **소유 위치 규칙**이며 feature가 존재할 때 적용한다.

## 3. Naming Rule

- 사용자 정의 directory와 비-Rust source file은 lowercase kebab-case.
- Rust `*.rs` module/source file은 `snake_case`.
- Cargo package/crate의 package name은 kebab-case를 사용하고 Rust import name의 underscore mapping을 따른다.
- `Cargo.toml`, `Cargo.lock`, `.github`, platform/tool 고정 파일은 explicit allowlist.
- kebab-case Rust module을 위해 `#[path]`를 남용하지 않는다.
- CI가 새 path를 검사하고 예외는 owner/reason/expiry 또는 tool mandate를 가진다.

## 4. 주요 crate 책임

### `dxb-kernel`
ID, Revision, Deadline, bounded size/limit primitive, digest 등 작고 안정된 값 타입. business entity/helper dumping ground 금지.

### `dxb-domain`
Bot/Memory/Goal/Task/Execution/Core/Routine의 순수 상태 의미. I/O, async runtime, Provider/Plugin 타입 금지.

### `dxb-application`
Use Case, Unit of Work, idempotency, auth context, side-effect ledger orchestration. Domain rule을 복제하지 않는다.

### `dxb-storage`
Domain-specific Store Port와 embedded implementation. 범용 `Repository<T>` 금지.

### `dxb-harness`
DXBOT Harness Capability Contract와 shared conformance fixtures만 소유한다. DeepSeek/Native implementation은 Provider crate로 분리 가능하고 장기적으로 분리하는 것을 기본 방향으로 한다.

### `dxb-runtime`
Bot coordinator, Task/Routine runtime, Scheduler, execution host, provider registry composition, structured shutdown.

### `dxb-control`
Control server/client schema 및 transport abstraction. CLI/TUI/Web가 runtime internal function을 직접 호출하지 않게 한다.

### `dxb-testkit`
Fake Provider, virtual clock, deterministic executor, failpoint, fixtures. production crate에 test-only state를 넣지 않는다.

## 5. Dependency 방향

```text
dxb-kernel
   ↑
dxb-domain
   ↑
dxb-application
   ↑
dxb-runtime ──> stable capability crates
   ↑               ↑
dxb-control     provider crates
   ↑
interfaces
```

금지:
- `dxb-domain → storage/runtime/provider/plugin/ui`
- `dxb-application → concrete provider`
- Provider crate 간 직접 dependency
- Plugin package → internal non-public crate API
- `dxb-runtime → CLI/TUI/Web`
- 제거된 feature의 orphan dependency

## 6. Trait/Feature 원칙

- trait는 실제 Capability/Port와 비결정 요소 교체 지점에만 둔다.
- Domain 내부 닫힌 상태는 enum을 우선한다.
- Provider 선택을 Cargo feature만으로 표현하지 않는다. runtime registry/selector가 필요한 경우 별도 계약을 둔다.
- compile-time feature는 독립적으로 제거 가능한 build surface에만 사용한다.
- feature 조합 폭발을 막고 최소/대표/전체 matrix를 CI에서 compile한다.

## 7. Provider 제거 검증

각 Provider는 다음 build profile에서 빠질 수 있어야 한다.

- Provider crate dependency 제거
- registry registration 제거
- provider-specific config 제거 또는 migration
- domain/runtime build 성공
- testkit fake로 core acceptance 실행
- stale config를 명시 오류로 검출

## 8. 검증 기준

- `cargo metadata` 기반 dependency gate에서 금지 edge가 0건이다.
- Domain tests가 DB/network/Provider 없이 실행된다.
- Provider A 제거 profile에서 core acceptance가 통과한다.
- naming validator가 Rust source 예외와 tool allowlist 외 위반을 차단한다.
