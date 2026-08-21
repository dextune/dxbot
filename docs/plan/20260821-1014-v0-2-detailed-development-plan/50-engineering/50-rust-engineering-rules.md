---
title: "Rust 구현 규칙"
document_id: "DXB-ENG-050"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-011", "DXB-ARC-012", "DXB-ARC-016", "DXB-RUN-030"]
---

# Rust 구현 규칙

## 1. 목적

공통성·재사용성·메모리/캐시 효율·동시성 안전성과 모듈 제거성을 코드 리뷰/CI에서 강제한다.

## 2. 불변 구현 원칙

1. 동일 사실의 Canonical Owner는 하나다.
2. Domain rule/state machine/policy는 한 소유 위치에만 존재한다.
3. Optional Capability Consumer가 concrete Provider type을 직접 호출하지 않는다.
4. Provider crate 간 직접 dependency를 금지한다.
5. Plugin public API를 내부 Rust trait/struct ABI와 동일시하지 않는다.
6. Provider/Plugin 제거 후 orphan dependency/config/feature가 남지 않는다.
7. `clone()`과 allocation은 의미·수명·비용을 설명할 수 있어야 한다.
8. collection/channel/cache는 bounded by item+bytes다.
9. spawned async task는 owner/cancel/join path가 있다.
10. panic은 programmer invariant에 제한한다.
11. `unsafe`는 별도 safety contract와 test/benchmark 없이는 금지한다.
12. 성능 최적화는 profile/benchmark 전후 증거가 있다.

## 3. 타입 / 소유권

- ID는 Newtype
- unit/bytes/tokens/cost/revision을 타입화
- raw `String`으로 semantic state를 표현하지 않음
- impossible state를 표현하기 어렵게 설계
- external DTO/domain/persistence/wire type 분리
- 호출자는 ownership을 유지하고 callee는 가능한 borrow
- large immutable data는 Arc/Bytes/reference를 실제 profile 근거에 따라 사용
- lock/DB borrow를 await boundary 넘어 보관하지 않음
- hot collection에 large/pointer-heavy object를 직접 중복 보관하지 않음

## 4. Capability / Provider 규칙

허용 dependency:
`Consumer → Capability Contract ← Provider`

금지:
`Consumer → Provider A concrete API`
`Provider A → Provider B`
`Domain → Capability implementation`

Provider-specific DTO, error, session ID, config struct는 Adapter 밖으로 누출하지 않는다. Provider 오류는 stable DXBOT error/outcome으로 normalize한다.

복수 Provider selector는 하나의 policy/registry service가 소유한다. call site마다 다른 fallback chain을 구현하지 않는다.

## 5. Plugin 규칙

- Plugin code는 public Plugin SDK/Wire만 사용
- internal crate visibility를 우회하는 FFI/pointer handoff 금지
- Plugin Host가 lifecycle/resource/cancellation을 소유
- Plugin이 Domain Store를 직접 mutate하지 않음
- Plugin data namespace와 migration/uninstall owner 명시
- Plugin callback/hook가 global mutable singleton을 요구하지 않음
- native code/unsafe Plugin은 별도 security review

## 6. Feature Removal 규칙

선택 기능은 삭제 전 다음을 compile/test한다.

- 해당 feature/provider/plugin dependency 없는 minimal profile
- Domain/Core Runtime compile
- stale config detection
- related API unsupported/deprecated behavior
- unrelated acceptance suite
- dependency graph orphan 0
- dead feature flag/registration 0

temporary shim은 owner/expiry/removal issue 없이 추가하지 않는다.

## 7. Async / Concurrency

- fire-and-forget 금지
- unbounded channel 금지
- async mutex guard + external I/O await 금지
- blocking work는 bounded pool
- cancellation branch cleanup
- timeout과 실제 child termination을 구분
- atomics ordering 근거 필요
- lock-free는 model/benchmark 근거 필요
- callback panic을 provider/plugin dispatch boundary에서 격리

## 8. Allocation / Cache

- 예상 분포 기반 capacity
- serialization intermediate tree 최소화
- large output Artifact spill
- pool/arena는 task-local lifecycle이 명확할 때만
- cache는 canonical source/key/version/max bytes/invalidation/rebuild를 명시
- 동일 Canonical data의 Bot/Core/UI별 중복 cache 금지
- Canonical Memory 삭제를 cache eviction 코드에 넣지 않음

구체 성능 목표와 benchmark 방법은 `DXB-ENG-051`을 참조하며 본 문서는 그 값을 소유하지 않는다.

## 9. Error / Side Effect

- library/domain typed error
- stable code + retry disposition
- partial outcome을 하나의 error로 소실하지 않음
- `Unknown side effect`를 transient retryable로 매핑 금지
- cleanup error가 원본 error를 덮지 않음
- errors 문자열 parsing 분기 금지

## 10. Naming / File

Repository naming은 `54-repository-ci-release.md`가 Canonical Owner다. Rust source module은 snake_case, 사용자 정의 non-Rust path는 kebab-case 원칙을 따른다. naming 편의를 위해 불필요한 `#[path]`를 쓰지 않는다.

## 11. Review Checklist

- classification/owner가 명확한가
- existing Capability를 재사용하는가
- concrete Provider 침투가 없는가
- clone/allocation/channel byte cap이 정당한가
- state/policy/retry/cache 중복이 없는가
- cancellation/quiescence가 완결되는가
- Waiting/Side Effect crash path가 있는가
- disable/remove/migration이 가능한가
- Plugin data/permission owner가 있는가
- test/traceability/risk가 갱신됐는가

## 12. 검증 기준

- architecture test에서 금지 dependency가 0건이다.
- production unbounded channel/unsafe/unwrap은 allowlist 외 0건이다.
- Provider-free minimal build가 존재한다.
- 모든 spawned task가 supervisor/join/cancel owner를 가진다.
- hot benchmark가 allocation count/bytes를 기록한다.
