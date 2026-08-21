# DXBOT Agent Instructions

이 파일은 저장소 전체에서 작업하는 AI Agent와 자동화 도구가 따라야 하는 공통 규약이다. 특정 구현 버전, 특정 Provider, 특정 UI 또는 특정 배포 방식에 종속되지 않는다.

하위 디렉터리에 별도의 `AGENTS.md`가 존재하면 해당 범위의 추가 규칙으로 적용한다. 단, 명시적인 승인 없이 이 루트 문서의 비협상 원칙을 완화해서는 안 된다.

## 1. 필수 참조 문서

작업 성격에 따라 다음 문서를 반드시 함께 확인한다.

- [아키텍처 및 코드 규칙](docs/agent/architecture-and-code-rules.md)
- [문서 작성 및 변경 규칙](docs/agent/documentation-rules.md)
- [테스트 및 재검수 규칙](docs/agent/testing-and-review-rules.md)
- [Repository 및 Git 규칙](docs/agent/repository-and-git-rules.md)

Agent 작업을 위해 추가적인 장기 참조 문서가 필요하면 저장소 루트에 임의 파일을 늘리지 말고 `docs/agent/` 아래에 작성한 뒤 이 파일에 링크한다.

## 2. 최우선 원칙

1. Correctness, 명확한 책임 경계, 장기 유지보수성을 단기 편의보다 우선한다.
2. 공통성·재사용성·메모리 효율·캐시 효율·동시성 안전성을 설계 단계부터 고려한다.
3. 동일 로직, 동일 정책, 동일 상태, 동일 데이터의 Canonical Owner를 중복시키지 않는다.
4. 불필요한 복사·할당·직렬화·상태 복제·무제한 큐·무제한 캐시를 금지한다.
5. 기능은 추가하기 쉬운 것뿐 아니라 제거·교체하기 쉬워야 한다.
6. 외부 Provider, Framework, Storage, UI의 구체 타입이나 생명주기가 Core Domain을 오염시키지 않아야 한다.
7. 실패, 취소, timeout, crash, restart, partial success, duplicate delivery를 정상 설계 범위로 취급한다.
8. Common Framework가 공통 correctness 문제를 소유하고 Extension은 안정된 계약을 구현한다.
9. 확장 기능 편의를 위해 Common Contract를 오염시키거나 Runtime 내부를 Extension에 노출하지 않는다.
10. 변경 후 코드와 문서의 관계성을 별도의 두 차례 재검수한다. 사용자 요청 또는 범위별 규범이 더 엄격한 검수 횟수/목적을 요구하면 그 기준을 추가로 따른다.

## 3. 기능 분류

새 기능이나 변경은 구현 전에 다음 중 하나로 분류한다.

- **Core Domain**: 제품 자체의 지속적 의미와 불변조건
- **Optional Capability**: 선택적으로 소비되는 안정된 기능 계약
- **Provider**: Capability의 구체 구현
- **Interface**: CLI/TUI/Web/API 등의 접근 표면
- **Plugin**: 안정된 외부 확장 계약을 사용하는 독립 패키지

분류가 불명확하면 구현을 서두르지 말고 Canonical Owner와 dependency 방향부터 확정한다. Plugin과 Provider를 같은 개념으로 취급하지 않는다.

## 4. Common / Extension 작업 등급

분류와 별도로 변경의 **품질 등급**을 판단한다.

### Tier A — Common / Contract 변경

다음은 높은 변경 비용을 갖는 공통 설계 작업이다.

- 새로운 Capability 정의 또는 기존 Capability semantic 변경
- Domain invariant/state machine
- Provider Host/Lifecycle/Registry/Selector
- Error taxonomy와 retry disposition
- Security/permission/approval
- Resource/admission/concurrency
- Persistence/recovery/migration
- Conformance/Testkit/Provider SDK
- public compatibility contract

Tier A는 설계 영향, 기존 Provider inventory, property/model/concurrency/fault test, compatibility/removal/rollback을 함께 검토한다. 단순 구현 편의를 위해 범위를 축소하지 않는다.

### Tier B — Extension 구현

다음은 이미 안정된 계약을 구현하는 작업이다.

- 기존 Capability의 Provider
- 외부 API/SDK Adapter
- Provider-specific Config
- DTO/Error Mapping
- Provider fixture/reference가 아닌 단순 통합 fixture

Tier B는 Common Contract를 수정하지 않는다. 구현 중 Contract/Host/SDK semantic 변경이 필요해지면 임시 우회하지 말고 Tier A 변경으로 재분류한다.

## 5. 작업 절차

모든 의미 있는 변경은 다음 순서로 수행한다.

1. 관련 코드, 문서, 테스트, 설정, migration을 먼저 탐색한다.
2. 변경 대상의 Classification, Quality Tier, Canonical Owner와 기존 abstraction을 확인한다.
3. 새 abstraction보다 기존 책임 경계와 Stable Contract의 재사용 가능성을 우선 검토한다.
4. 가장 작은 **완결된 의미 단위**로 수정한다.
5. 영향받는 문서·테스트·schema·config·migration·conformance를 같은 변경 단위에서 갱신한다.
6. 적용 가능한 format/lint/build/test/docs 검증을 수행한다.
7. **재검수 1: Structural / Consistency Review**를 수행한다.
8. **재검수 2: Cross-Layer Executability Review**를 수행한다.
9. 범위 규범이나 사용자 요청이 추가 review를 요구하면 서로 다른 검사 목적과 증거로 수행한다.
10. 모든 재검수에서 발견된 문제를 수정한 뒤 관련 범위를 다시 확인한다.

파일이나 저장소에서 확인할 수 있는 사실을 사용자에게 다시 질문하지 않는다. 불확실한 경우 기존 소스와 규범 문서를 우선 조사한다.

## 6. Common Framework / Provider 핵심 규칙

- Consumer는 concrete Provider가 아니라 Stable Capability Contract와 공통 invocation boundary에 의존한다.
- production Provider 호출은 프로젝트가 정의한 Provider Host 또는 동등한 Common enforcement boundary를 거친다.
- Common이 lifecycle, registration/generation, selection, permission, resource admission, deadline/cancellation, semantic retry, side-effect guard, telemetry/audit, compatibility, recovery를 소유한다.
- Provider는 provider-specific config, external SDK/API adapter, Capability-specific logic, DTO/error mapping에 집중한다.
- Provider에 거대한 `RuntimeContext`, service locator, Domain Store, Scheduler, Registry mutator, raw Secret Store를 제공하지 않는다.
- Provider가 자체 global retry, permission, scheduler/admission, Domain persistence, global telemetry system을 만들어 Common을 우회하지 않는다.
- Provider-local transport retry는 Stable Contract가 허용하고 호출 의미·idempotency·deadline을 변경하지 않는 범위에 한한다.
- 모든 동등 Provider는 Common Conformance Suite를 통과하고 최소 한 검증 경로는 실제 Common invocation boundary를 사용한다.
- 주요 Capability는 가능한 범위에서 deterministic Reference Provider 또는 동등한 executable reference를 유지한다.

세부 규칙은 [아키텍처 및 코드 규칙](docs/agent/architecture-and-code-rules.md)을 따른다.

## 7. Extension Dependency Firewall

Provider의 기본 허용 dependency는 다음으로 제한한다.

- 해당 Capability의 Stable Contract
- 프로젝트가 승인한 Provider SDK
- 공개 허용된 Kernel/value types
- 실제 통합에 필요한 승인된 external SDK/library

기본 금지:

- Domain internals
- Application internals
- Runtime internals
- Storage internals
- 다른 Provider
- Interface/UI
- private application module

공통 helper가 필요하다는 이유로 금지 dependency를 추가하지 않는다. 필요한 기능을 Stable SDK surface로 승격할지 Tier A에서 검토한다.

## 8. Provider 구현 작업 규격

Provider 작업을 Agent에 위임할 때 가능한 한 다음을 명시한다.

```text
Classification: Provider
Quality Tier: B
Capability: <stable capability>
Implement: <provider package>
Allowed dependencies: <contract/sdk/external sdk>
Forbidden dependencies: <domain/runtime/storage/other providers>
Required: config/provider/error mapping/dto mapping/conformance
Do not implement: global retry/permission/resource admission/scheduler/domain persistence/global telemetry
Required verification: compile/lint/conformance/common-host path/removal/dependency gate
```

Agent는 작업 규격 밖의 Common Contract를 편의상 변경하지 않는다.

## 9. 코드 변경 핵심 규칙

- Domain은 Infrastructure, UI, 특정 Provider를 직접 의존하지 않는다.
- Provider 간 직접 의존성을 만들지 않는다.
- concrete Provider 선택은 Registry/Selector/Policy 경계를 통해 수행한다.
- `common`, `utils`, `helpers`, `misc` 같은 무소유 dumping ground를 만들지 않는다.
- Domain-specific invariant는 한 소유자에서만 구현한다.
- 외부 입력 DTO, Domain type, Persistence schema, Wire schema를 무분별하게 동일 타입으로 공유하지 않는다.
- 비동기 task는 owner, cancellation, shutdown/join 경로를 가진다.
- queue/channel/cache는 item 수뿐 아니라 필요 시 byte 상한과 backpressure 정책을 가진다.
- 비멱등 Side Effect는 retry 가능성을 추측하지 말고 durable intent/idempotency/reconciliation 의미를 설계한다.
- `unsafe`는 별도 안전 계약, 테스트, 측정 근거 없이 추가하지 않는다.
- 성능 최적화는 profile/benchmark 근거 없이 복잡도를 늘리지 않는다.

## 10. 파일 및 문서 규칙

- 사용자 정의 Repository 파일/디렉터리명은 원칙적으로 **lowercase kebab-case**를 사용한다.
- Rust source module 파일(`*.rs`)은 Rust 생태계 규약에 따라 **snake_case**를 사용한다.
- `AGENTS.md`처럼 외부 도구가 정해진 이름을 요구하는 파일은 명시적 예외다.
- `AGENTS.md`와 `docs/agent/`의 장기 Agent 지침은 특정 기획·릴리스 버전이나 일회성 구현 상태에 종속시키지 않는다.
- 특정 시점의 상세 설계, migration 기록, 실험 결과는 해당 책임을 가진 별도 프로젝트 문서가 소유한다.
- 같은 개념의 규범을 여러 문서에 복사하지 말고 Canonical 문서를 참조한다.
- 문서만 바꾸는 작업이라도 관련 코드/API/schema/test와 모순되는지 확인한다.
- 코드만 바꾸고 관련 설계/운영 문서를 방치하지 않는다.

세부 규칙은 [문서 작성 및 변경 규칙](docs/agent/documentation-rules.md)을 따른다.

## 11. 테스트 및 완료 기준

변경은 happy path만으로 완료되지 않는다. 변경 성격에 따라 다음을 검증한다.

- 정상 경로와 금지 상태 전이
- timeout/cancellation/retry
- concurrency/race/backpressure
- crash/restart/recovery
- Provider lifecycle/교체·미가용·제거·복수 선택
- Provider Host/Common boundary enforcement와 forbidden dependency
- Capability Conformance와 compatibility
- Plugin enable/disable/upgrade/uninstall
- stale config/schema/data
- memory/resource leak
- migration/rollback
- permission/security boundary

테스트는 가능한 한 deterministic하게 작성하고 sleep이나 실제 외부 모델의 비결정성에 핵심 correctness를 의존시키지 않는다.

세부 규칙은 [테스트 및 재검수 규칙](docs/agent/testing-and-review-rules.md)을 따른다.

## 12. Git 및 변경 안전성

- 사용자가 요청하지 않은 기존 변경을 되돌리거나 덮어쓰지 않는다.
- 관련 없는 파일을 정리한다는 이유로 함께 수정하지 않는다.
- 파괴적 Git 명령, 강제 push, history rewrite는 명시적 요청 없이 수행하지 않는다.
- commit은 가능한 한 하나의 논리적 변경 목적을 가진다.
- dependency 추가는 실제 소유 코드와 복잡도를 줄이는 경우에만 허용하고 영향 범위를 검토한다.
- secret, credential, 개인 데이터, 로컬 runtime data를 commit하지 않는다.

세부 규칙은 [Repository 및 Git 규칙](docs/agent/repository-and-git-rules.md)을 따른다.

## 13. Agent의 판단 기준

충돌하는 선택지가 있을 때 다음 순서를 우선한다.

1. 제품/Domain 의미의 정확성
2. 데이터 보존과 복구 가능성
3. 보안과 권한 경계
4. 명확한 소유권과 Common/Extension 책임 분리
5. 동시성 안전성과 bounded resource 사용
6. Contract 안정성과 테스트 가능성
7. 관측 가능성
8. 성능과 캐시 효율
9. 구현 편의

임시 편의를 위해 장기적인 상태 중복, Provider 종속, Common Contract 오염, 숨은 fallback, 무제한 자원 사용, 테스트 불가능한 전역 상태를 도입하지 않는다.
