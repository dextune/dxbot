---
title: "Rust 구현 규칙"
document_id: "DXB-ENG-050"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-011", "DXB-ARC-012", "DXB-ARC-016", "DXB-ARC-017", "DXB-RUN-030"]
---

# Rust 구현 규칙

## 1. 목적

공통성·재사용성·메모리/캐시 효율·동시성 안전성과 모듈 제거성을 코드 리뷰/CI에서 강제하며, Extension 구현이 Common Framework를 우회하지 못하게 한다.

## 2. 불변 구현 원칙

1. 동일 사실의 Canonical Owner는 하나다.
2. Domain rule/state machine/policy는 한 소유 위치에만 존재한다.
3. Consumer가 concrete Provider를 직접 호출하지 않는다.
4. production Provider call은 Provider Host를 거친다.
5. Provider crate 간 직접 dependency를 금지한다.
6. Provider는 Thin Implementation Principle을 따른다.
7. Provider에 `RuntimeContext`/service locator/Domain Store/Scheduler/Registry mutator를 전달하지 않는다.
8. Provider가 global retry/permission/resource admission/persistence/global telemetry를 재구현하지 않는다.
9. Provider/Plugin 제거 후 orphan dependency/config/feature가 남지 않는다.
10. collection/channel/cache는 bounded by item+bytes다.
11. spawned async task는 owner/cancel/join path가 있다.
12. `unsafe`는 safety contract/test/benchmark 없이는 금지한다.
13. 성능 최적화는 profile/benchmark 전후 증거가 있다.

## 3. 타입 / 소유권

ID Newtype, unit/revision 타입화, external DTO/domain/persistence/wire type 분리, borrow 우선, large immutable data 공유는 실제 수명/측정 근거를 사용한다. lock/DB borrow를 await boundary 넘어 보관하지 않는다.

## 4. Capability / Provider 규칙

허용:

```text
Provider → Capability Contract
Provider → dxb-provider-sdk
Provider → explicit public Kernel value types
Provider → approved external SDK
```

금지:

```text
Provider → dxb-domain internals
Provider → dxb-application internals
Provider → dxb-runtime internals
Provider → dxb-storage internals
Provider A → Provider B
Consumer → Provider concrete API
```

Provider-specific DTO/error/session/config는 Adapter 밖으로 누출하지 않는다. Provider 오류는 typed mapping으로 normalize한다.

## 5. Thin Provider 구현 범위

표준 구현 파일의 책임:
- `config.rs`: provider-specific config
- `provider.rs`: capability-specific external integration
- `error.rs`: provider error mapping
- `mapping.rs`: DTO mapping
- `tests/conformance.rs`: Common suite factory/fixture

Provider 내부에 `retry_manager.rs`, `permission.rs`, `scheduler.rs`, `telemetry_system.rs`, `domain_repository.rs` 같은 Common 책임 복제 모듈을 만들지 않는다.

## 6. Provider Host 규칙

- Host pre/post pipeline을 capability별 call site에서 중복 구현하지 않는다.
- Host는 Domain rule을 재구현하지 않고 owner Port를 사용한다.
- Provider generation/activity guard를 호출 전체 수명에 유지한다.
- deadline/cancellation/output/resource accounting이 cleanup path에서도 실행된다.
- test-only direct Provider call과 production Host integration test를 구분한다.

## 7. Plugin 규칙

Plugin Provider도 Provider SDK/Host/Conformance/Dependency Firewall을 따른다. Plugin Host와 Provider Host를 하나의 privileged context로 합치지 않는다.

## 8. Feature Removal 규칙

Provider dependency 없는 minimal profile, Domain/Core Runtime compile, stale config detection, explicit unsupported, unrelated acceptance, orphan dependency/feature/registration 0을 검증한다.

## 9. Async / Concurrency

fire-and-forget/unbounded channel 금지, bounded blocking, cancellation cleanup, child join, atomic ordering 근거를 요구한다. Provider internal worker도 lifecycle owner와 stop/join을 가진다.

## 10. Allocation / Cache

예상 분포 기반 capacity, serialization intermediate 최소화, Artifact spill, cache source/key/version/max bytes/invalidation을 명시한다. Provider SDK helper가 불필요한 clone/allocation을 강제하지 않도록 benchmark한다.

## 11. Error / Side Effect

- stable typed error + retry disposition
- partial outcome 보존
- Unknown side effect를 transient로 매핑 금지
- Provider-local transport retry는 contract가 허용한 semantic-invisible 범위만
- semantic retry는 Common Runtime이 새 Attempt로 수행

## 12. Quality Tier

Tier A(Common/Contract/Host/Security/Recovery/Conformance/SDK)는 API stability/property/concurrency/fault/benchmark 영향 검토를 수행한다. Tier B(기존 Provider/Adapter)는 compile+conformance+firewall+Host-path+removal gate를 우선 통과한다.

Tier B 구현 중 Common Contract 수정 필요가 드러나면 임시 우회하지 않고 Tier A 작업으로 재분류한다.

## 13. Review Checklist

- classification/owner/quality tier가 명확한가
- existing Capability/SDK를 재사용하는가
- concrete Provider/Host bypass가 없는가
- forbidden dependency가 없는가
- Common retry/security/resource/telemetry를 중복 구현하지 않는가
- clone/allocation/channel byte cap이 정당한가
- cancellation/quiescence가 완결되는가
- removal/migration/conformance가 가능한가
- test/traceability/risk가 갱신됐는가

## 14. 검증 기준

- architecture test에서 금지 dependency/Host bypass가 0건이다.
- Provider-free minimal build가 존재한다.
- 모든 Provider가 Capability Conformance를 통과한다.
- spawned task/process/activity/permit leak가 없다.
- hot benchmark가 allocation count/bytes를 기록한다.
