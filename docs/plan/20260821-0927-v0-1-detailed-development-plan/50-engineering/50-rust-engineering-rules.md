---
title: "Rust 구현 규칙"
document_id: "DXB-ENG-050"
version: "0.1.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-011", "DXB-RUN-030", "DXB-ENG-051"]
---


# Rust 구현 규칙

## 1. 목적

공통성·재사용성·메모리 효율·캐시 효율·동시성 안전성을 실제 코드 리뷰와 CI에서 강제할 수 있는 Rust 개발 규칙으로 변환한다.

## 2. 책임 범위

- 타입·소유권·API
- 오류·async·concurrency
- 할당·복사·캐시
- trait/enum/generic
- unsafe/dependency
- 문서·테스트·리뷰
- 중복 금지

스타일 취향보다 correctness와 비용 모델을 우선한다.

## 3. 불변 구현 원칙

1. 동일 사실의 Canonical Owner는 하나다.
2. Domain rule은 한 함수/상태기계에만 존재한다.
3. `clone()`은 의미와 비용을 설명할 수 있어야 한다.
4. 컬렉션·채널·캐시는 bounded다.
5. async task는 owner, cancellation, join path를 가진다.
6. 외부 Provider 타입은 Adapter 밖으로 나오지 않는다.
7. 오류는 stable classification과 source를 보존한다.
8. panic은 프로그래머 invariant 위반에만 제한한다.
9. `unsafe`는 안전 계약과 검증 증거 없이 금지한다.
10. 추상화는 실제 변형 축과 소유권을 반영한다.
11. 성능 최적화는 benchmark/profile 전후 비교를 가진다.
12. 임시 shim과 중복 구현은 제거 조건 없이 추가하지 않는다.

## 4. 타입 설계

- ID는 `BotId`, `TaskId` 등 Newtype으로 분리
- 상태 전이는 method/command handler로 제한
- 불가능한 상태를 표현하기 어렵게 설계
- 단위와 범위를 타입화: bytes/tokens/cost/deadline/revision
- raw `String`을 모든 의미에 사용하지 않음
- `Option<bool>` 같은 모호한 표현 대신 의미 있는 enum
- 외부 입력 DTO와 Domain 타입 분리
- serialized enum에 unknown/version 처리
- aggregate 내부 field를 public으로 열지 않음
- builder는 필수 값 누락을 숨기지 않음
- typestate는 복잡도를 줄일 때만 사용

## 5. 소유권과 Borrowing

- 호출자가 데이터를 소유하고 Callee는 가능한 한 borrow
- hot path에서 `String`/`Vec` clone 대신 slice/view/reference
- 장기 공유 불변 데이터는 `Arc<T>`를 고려
- 작은 복사 가능 값은 `Copy`가 더 단순할 수 있음
- `Cow`는 실제 borrowed/owned 두 경로가 존재할 때만
- `Bytes`/shared blob은 streaming/Artifact 경계에 사용
- `Arc<str>`/interning은 높은 중복률이 profile로 확인될 때
- lock guard나 DB row borrow를 async boundary 넘어 보관하지 않음
- self-referential 구조를 피하고 stable IDs/reference로 연결
- 큰 enum/struct의 크기를 점검하고 hot collection에서 boxing 여부를 benchmark

## 6. 할당과 컬렉션

- 예상 크기를 아는 경우 `with_capacity`
- 과대 preallocation 금지; 관측 percentile 기반
- 작은 벡터 최적화는 분포와 stack 크기 영향 benchmark 후
- HashMap을 기본 만능 구조로 사용하지 않음
- iteration/cache locality가 중요하면 contiguous collection 우선
- stable order가 필요하면 명시적 ordered structure
- string formatting을 loop 내부에서 반복하지 않음
- serialization 중간 `Value` tree를 피하고 typed/streaming 사용
- 대용량 결과는 Artifact spill
- pool/arena는 수명과 reset이 명확한 task-local 범위에서만
- object pool이 retained memory를 키우는지 계측
- cache는 maximum entries가 아니라 maximum bytes도 관리

## 7. Trait와 추상화

Trait 사용:
- 외부 Provider/Port
- 테스트 대체가 필요한 비결정 요소
- 독립적 구현이 실제 존재하거나 계획된 capability seam

Trait를 쓰지 않을 경우:
- Domain 내부 닫힌 상태 집합 → enum
- 한 구현뿐이고 교체 이유가 없음
- 단순 코드 재사용 → 함수/구성
- 테스트만을 위해 모든 struct를 trait로 감싸는 경우

규칙:
- 작은 응집된 trait
- async cancellation/deadline/error 계약 포함
- object safety 여부 의도
- dynamic dispatch vs monomorphization 비용 판단
- blanket impl로 의미가 흐려지지 않음
- `Repository<T>` 범용 CRUD 금지; domain-specific Store
- mega `Context`/service locator 금지
- capability dependency는 constructor/manifest로 명시

## 8. 오류 처리

- library/domain: typed error
- application boundary: context를 추가하되 stable code 보존
- `unwrap/expect`는 test, startup invariant, 증명된 내부 invariant로 제한
- 사용자/외부 입력에서 panic 금지
- error source chain 유지
- retryable/permanent/conflict/cancel 분류
- partial outcome을 error 하나로 소실하지 않음
- cleanup 오류를 원래 오류에 덮어쓰지 않고 함께 기록
- 로그하고 다시 반환하는 중복 logging 금지; 책임 경계에서 한 번
- secret/raw payload를 Display/Debug에 노출하지 않음
- errors를 문자열 parsing해 분기하지 않음

## 9. Async와 Concurrency

- async trait/function은 cancellation safety를 문서화
- spawn은 structured supervisor를 통해서만
- fire-and-forget 금지; background 작업 registry
- unbounded channel 금지
- send 실패/receiver close 의미 처리
- async mutex guard를 잡고 I/O await 금지
- blocking 작업은 bounded blocking pool
- select cancellation branch의 resource cleanup
- timeout wrapper가 실제 child 종료를 보장하는지 구분
- task-local state는 명시적으로 제한
- atomics는 memory ordering 근거를 주석/테스트
- lock-free 구조는 benchmark와 모델 검증 없이는 금지
- closure/callback panic을 dispatcher 경계에서 격리

## 10. 데이터 직렬화

- versioned envelope
- size/depth/string length 제한
- unknown field/variant 정책
- deterministic digest가 필요한 형식은 canonicalization
- secret field는 serialize 금지 또는 redacted wrapper
- zero-copy deserialization은 input buffer 수명 비용과 비교
- Domain struct를 그대로 persistence/wire schema로 derive하지 않음
- migration test fixture
- large payload streaming
- compressed data의 decompression bomb 제한
- binary format은 tooling/compatibility와 함께 ADR

## 11. 공통 코드와 중복 금지

금지:
- `common`, `utils`, `misc`, `helpers`에 무작위 함수 축적
- 같은 validation/policy를 CLI/API/Tool마다 구현
- 같은 DTO를 crate별 재정의
- duplicate retry/backoff implementation
- duplicate cache of same canonical data without owner
- copy-paste state machine
- 문자열 error code 복제

허용되는 공통화:
- 소유권과 의미가 명확한 value object
- 완전한 capability seam
- 반복되는 protocol envelope
- 동일 invariant를 강제하는 constructor
- 검증된 serializer/limit primitive

추상화 전 최소 두 사용 사례와 변형 축을 확인하되, 보안/정합성 규칙은 첫 사례부터 중앙화한다.

## 12. Unsafe와 FFI

- 기본 금지
- 별도 모듈/crate
- `# Safety` 계약
- invariant와 caller obligations
- safe wrapper 외부 노출
- Miri/sanitizer/fuzz/target test
- panic/unwind/ownership/thread safety
- FFI resource dispose
- benchmark로 필요성 증명
- upstream native dependency audit
- 대체 safe 구현과 fallback

## 13. Dependency 정책

- 유지보수·라이선스·보안·MSRV/플랫폼·binary size 평가
- 직접 구현보다 검증된 의존성이 owned code/tests를 실질적으로 줄이면 채택
- 작은 기능 하나를 위해 큰 dependency graph를 추가하지 않음
- default features 검토
- dependency pin/lockfile
- 중복 major version 감시
- abandoned/unmaintained signal
- build script/native code 별도 승인
- public API에 dependency type 노출 최소화
- 교체 Adapter와 conformance test

## 14. 코드 리뷰 체크리스트

- Canonical owner가 명확한가
- clone/allocation/channel capacity가 정당한가
- state duplication이 생겼는가
- async cancellation/teardown이 완결되는가
- error/retry/idempotency 의미가 있는가
- permission/resource guard를 우회하는가
- new abstraction이 실제 seam인가
- existing module로 재사용 가능한가
- cache invalidation/retention이 있는가
- tests가 race/failure를 포함하는가
- observability와 redaction이 있는가
- migration/compatibility가 필요한가

## 15. 예외상황

규칙 예외는 ADR 또는 코드 주석만으로 끝내지 않고:
- 측정치
- 대안
- 범위
- owner
- 만료/재검토 조건
- 테스트
를 가진다.

## 16. 확장성

외부 Plugin SDK가 생겨도 내부 trait를 그대로 ABI로 공개하지 않는다. 안정 wire protocol을 사용한다. 성능용 specialized path는 common path와 의미가 같음을 conformance test로 보장한다.

## 17. 구현 우선순위

- **P0:** lints, dependency rules, no-unbounded, typed IDs/errors, code review template
- **P1:** duplication/size/allocation gates, unsafe policy automation
- **P2:** public SDK/ABI policy, advanced profile-guided optimization

## 18. 검증 기준

- production code의 `unwrap/expect`, `unsafe`, unbounded channel이 allowlist 외 0건이다.
- duplicate policy/retry/state machine 검사가 CI 또는 리뷰에서 수행된다.
- hot path benchmark에 allocation count/bytes가 포함된다.
- Domain crate가 외부 framework type을 공개하지 않는다.
- 모든 spawned task가 supervisor/join/cancel owner를 가진다.
- 공개 API 변경이 semver/schema check를 통과한다.
