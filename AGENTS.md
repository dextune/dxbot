# DXBOT Agent Instructions

이 파일은 저장소 전체에서 작업하는 AI Agent와 자동화 도구가 항상 읽는 최상위 규약이다. 상세 규칙을 이 파일에 반복하지 않고, 필요한 범위만 단계적으로 읽도록 `docs/agent/`의 Guide Router를 사용한다.

## 1. 비협상 원칙

1. Correctness, 명확한 책임 경계, 장기 유지보수성을 단기 편의보다 우선한다.
2. 공통성·재사용성·메모리 효율·캐시 효율·동시성 안전성을 설계 단계부터 고려한다.
3. 동일 로직·정책·상태·데이터의 Canonical Owner를 중복시키지 않는다.
4. 불필요한 복사·할당·직렬화·상태 복제와 unbounded queue/cache를 금지한다.
5. 기능은 추가뿐 아니라 제거·교체가 쉬워야 한다.
6. Provider, Plugin, Interface, Storage 구현 세부가 Core Domain 의미를 소유하거나 오염시키지 않는다.
7. failure, cancellation, timeout, crash, restart, partial success, duplicate delivery를 정상 설계 범위로 취급한다.
8. Common Framework가 공통 correctness를 소유하고 Extension은 Stable Contract를 구현한다.
9. 사용자 정의 파일/디렉터리는 lowercase kebab-case, Rust module source(`*.rs`)는 snake_case를 따른다. 외부 도구 고정 이름만 명시적 예외다.
10. 의미 있는 변경 후 Structural/Consistency와 Cross-Layer Executability 재검수를 별도로 2회 수행한다.

## 2. 작업 시작 — 필요한 Guide만 읽기

모든 작업은 먼저 [DXBOT Agent Guide Index](docs/agent/guide-index.md)에서 작업 유형을 찾는다. 한 번에 `docs/agent/` 전체를 읽는 것을 기본 절차로 삼지 않는다.

대표 경로:

- Repository 구조·파일·crate 배치 → [Repository Guide](docs/agent/repository/index.md)
- Domain/Capability/Provider/Plugin/책임 경계 → [Architecture Guide](docs/agent/architecture/index.md)
- Rust 타입·ownership·memory·concurrency·cache → [Rust Guide](docs/agent/rust/index.md)
- runtime lifecycle·resource·recovery·security → [Runtime Guide](docs/agent/runtime/index.md)
- test·fixture·fault·benchmark·완료 판정 → [Quality Guide](docs/agent/quality/index.md)
- 문서 작성 자체 → [Documentation Rules](docs/agent/documentation-rules.md)

## 3. 변경 분류

구현 전에 변경을 `Core Domain | Optional Capability | Provider | Interface | Plugin` 중 하나로 분류하고 Canonical Owner를 확인한다.

또한 다음 중 하나로 품질 등급을 판정한다.

- **Tier A — Common / Contract**: Domain invariant, Stable Capability Contract, Provider Host/Lifecycle, security, resource, recovery, migration, public compatibility, Conformance 의미 변경.
- **Tier B — Extension**: 이미 안정된 Capability를 구현하는 Provider/Adapter/config/DTO/error mapping.

Tier B 작업 중 Common semantic 변경이 필요해지면 workaround로 숨기지 말고 Tier A로 승격한다.

## 4. 기본 작업 절차

1. 관련 코드·문서·test·config·migration을 탐색한다.
2. Classification, Quality Tier, Canonical Owner, 기존 abstraction을 확인한다.
3. `guide-index.md`에서 필요한 상세 Guide만 추가로 읽는다.
4. 새 abstraction보다 기존 Stable Contract와 책임 경계 재사용을 먼저 검토한다.
5. 가장 작은 완결된 의미 단위로 수정한다.
6. 영향받는 code/docs/schema/config/migration/conformance를 같은 변경 단위에서 갱신한다.
7. 적용 가능한 format/lint/build/test/docs 검증을 수행한다.
8. 재검수 1: Structural / Consistency Review.
9. 재검수 2: Cross-Layer Executability Review.
10. 발견 사항을 수정한 뒤 관련 범위를 다시 검증한다.

## 5. Canonical 문서와 Guide의 관계

`docs/agent/`는 **어떻게 설계·구현·검증할지**를 안내한다. 제품의 현재 의미, 상태기계, public contract, milestone 같은 버전별 규범은 활성 `docs/plan/` package의 Canonical Owner가 소유한다.

Guide가 Plan의 제품 의미를 복제하거나 override해서는 안 된다. 충돌 시 먼저 Owner를 식별하고 문서·코드·schema·test를 같은 변경에서 수렴시킨다.

## 6. Git 안전성

사용자가 만든 unrelated 변경을 되돌리지 않는다. 명시적 요청 없이 force push, hard reset, 공유 history rewrite, branch protection 우회, 기존 commit amend를 하지 않는다. secret·credential·개인 데이터·로컬 runtime artifact를 commit하지 않는다.
