---
title: "문서 거버넌스와 변경 통제"
document_id: "DXB-GOV-001"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000"]
---

# 문서 거버넌스와 변경 통제

## 1. 목적

설계·코드·테스트가 서로 다른 의미로 분기하지 않도록 문서 소유권, 기능 분류, Common/Extension 품질 등급, 변경 세트, 제거 영향, **2회 독립 관계성 검토**와 추적성을 정의한다.

과거 패키지의 3단계 Review evidence는 역사 기록으로 보존한다. v0.7부터는 검사항목을 축소하지 않고 `Structural / Consistency`와 `Cross-Layer Executability / Compatibility` 두 Review로 통합해 프로젝트 공통 규칙과 일치시킨다.

## 2. 문서 상태

- `Draft`: 검토 중
- `Accepted`: 구현·테스트가 따라야 하는 승인 계약
- `Normative Baseline`: 최상위 제품 기준
- `Superseded`: 새 문서/ADR로 대체
- `Deprecated`: 신규 사용을 금지하고 제거 계획 보유

문서 집합 버전과 개별 문서 버전은 구분한다. 새 패키지가 이전 패키지의 변경 없는 문서를 동일 blob으로 계승하는 경우 개별 문서 버전은 그대로 유지할 수 있으며 Manifest가 계승 사실과 범위를 명시한다.

## 3. 요구사항 식별자

- 제품 원칙 `PR-*`
- 기능 요구 `FR-*`
- 비기능 요구 `NFR-*`
- 불변조건 `INV-*`
- 수용 테스트 `AT-*`
- ADR `ADR-*`
- Risk `R-*`, SPI 위험은 `R-SPI-*`

## 4. Architecture Feature Classification

새 기능 또는 기존 기능의 의미 변경은 구현 전에 다음 중 하나로 분류한다.

`Core Domain | Optional Capability | Provider | Interface | Plugin`

각 기능 문서에는 적용 가능한 범위에서 다음을 명시한다.

- Classification
- Quality Tier `A(Common/Contract) | B(Extension Implementation)`
- Canonical Owner
- Lifecycle Owner
- Persistent State 존재 여부와 owner
- Capability/Provider/Plugin 여부
- Dependency 방향
- disable/remove 가능 여부
- remove 시 data/config/schema 영향
- Conformance/Acceptance Test

분류가 불가능하거나 두 범주를 암묵적으로 겸하는 기능은 설계 검토 대상으로 올린다.

## 5. Quality Tier 변경 통제

### Tier A
새 Capability, Domain invariant/state, Application/Public Contract, Provider Host/Lifecycle/Registry/Selector, Security/Resource/Recovery, Error taxonomy, Conformance/Testkit/Provider SDK, Migration/Public compatibility를 포함한다.

### Tier B
기존 Stable Capability를 구현하는 Provider/Adapter/config/DTO/error mapping을 포함한다.

Tier B 작업이 Common Contract 또는 Host/SDK semantic 변경을 요구하면 Provider 내부 우회로 해결하지 않고 Tier A 변경으로 재분류한다.

## 6. 변경 세트

의미 변경은 최소 다음을 하나의 change set으로 묶는다.

1. 기준/소유 문서
2. 영향 API/Event/Config/Persistence/Capability/Application 계약
3. Migration 또는 Feature Removal 판단
4. Acceptance/Traceability/Conformance
5. Security/Resource/Observability 영향
6. Rollback/Compatibility/Deprecation/Supersession
7. affected Provider/Plugin/Interface inventory
8. Risk 및 Open Question 갱신

Provider나 Plugin 변경을 이유로 Core Domain/Common Contract 의미를 함께 수정하는 경우 Tier A 재분류와 ADR 판단을 수행한다.

## 7. Feature Removal / Scope Withdrawal 영향 분석

Optional Capability/Provider/Interface/Plugin 제거 또는 active scope withdrawal은 다음을 명시한다.

- 신규 사용 차단 지점
- in-flight drain/quiescence 적용 여부
- stale config/reference 처리
- API/Event 상태(`deprecated`, `unsupported`, version negotiation)
- persistent data의 retain/export/migrate/purge
- Derived cache/index/projection cleanup
- dependency/build feature/registry/doc inventory cleanup
- 마지막 호환 버전과 rollback/re-entry condition
- 제거된 구성 없이 수행하는 acceptance suite
- `omission != deprecation/supersession`이므로 baseline에서 명시적 상태 선언

단순 code/file delete로 완료 처리하지 않는다.

## 8. ADR 필요 조건

다음은 ADR 없이 변경할 수 없다.

- Core Domain 경계/Canonical Owner 이동
- Capability/Common/Application Contract breaking semantic change
- Provider Host/Lifecycle의 enforcement 의미
- Provider/Plugin public contract의 호환성 파괴
- Repository naming/layout 규칙
- 영속 상태 또는 side-effect crash semantics
- Routine/Task/Execution lifecycle 의미
- public wire protocol 또는 migration irreversibility
- 새로운 `unsafe`, FFI, process isolation 방식
- Runtime Host / Interface process boundary의 semantic 변경

## 9. 2회 독립 관계성 검토

모든 의미 변경은 완료 후 아래 두 Review를 **독립적으로** 수행한다. 각 Review에서 발견한 문제는 즉시 수정하고 해당 Review 범위를 처음부터 재확인한다.

### Review 1 — Structural / Consistency

과거 Structural + Contract/Traceability 검사항목을 통합한다.

- lowercase kebab-case 및 허용 예외
- document_id 중복/파일 누락/inventory drift
- `depends_on` 존재성·순환/역방향 의존
- Canonical/Lifecycle/Persistent Owner 충돌
- Capability/Provider/Provider Host/Plugin/Interface 용어 혼용
- 중복 Policy/Limit/State/Contract
- 링크·Manifest·Acceptance·Risk·Open Question 연결
- Superseded/Deprecated 대체·제거 경로
- Capability/Application Contract completeness
- same-rule SSOT와 중복 문구 의미 일치
- Breaking change의 ADR/Migration/affected inventory/SDK/Conformance/Reference/Acceptance/Risk 영향
- Provider SDK/Dependency Firewall/Scaffold와 Contract 일치
- 이전 baseline inheritance와 explicit supersession의 충돌 여부

### Review 2 — Cross-Layer Executability / Compatibility

실제 개발·실행 경로를 끝까지 추적한다.

```text
Command / Client Intent
→ Application Contract
→ Application
→ Domain/Persistence
→ Scheduler/Runtime
→ Provider Host
→ Provider/Plugin
→ Recovery
→ Projection/Event
→ Control Client / Interface Output
```

Provider/Extension 변경이면 다음도 추적한다.

```text
Capability Definition
→ Common Host/SDK/Testkit
→ Scaffold/Provider
→ Conformance/CI
→ Runtime Invocation
→ Failure/Recovery
→ Removal
```

정상 경로와 함께 다음 fault/compatibility를 대입한다.
- Provider 없음/교체/제거/복수
- lifecycle start/drain/failure
- crash/restart/reconnect
- cancellation/SIGINT/timeout
- partial success/response loss/duplicate delivery
- stale revision/authorization revoke
- version mismatch/migration
- feature disable/removal/supersession
- resource pressure/slow consumer/large payload

계층 사이에 owner 없는 상태, Host/Application Contract 우회, duplicate semantic effect, unbounded resource, identity drift가 생기면 완료로 보지 않는다.

## 10. 자동 검사

CI는 최소 다음을 검증한다.

- 깨진 상대 링크/문서 ID/depends_on
- naming/layout
- requirement↔acceptance 미연결
- schema/Contract version 변경↔migration/compatibility 부재
- Provider concrete/forbidden dependency 침투
- production Provider Host/Application boundary bypass
- removal/supersession 대상 orphan dependency/config/registry/doc gate
- policy/default 중복 후보
- Scaffold/Conformance/Reference drift
- manifest drift

## 11. 검증 기준

- 모든 Accepted 문서가 소유 crate/module과 Acceptance를 가진다.
- 모든 신규 기능이 5개 분류와 적용 가능한 Quality Tier를 가진다.
- Provider/Plugin/Interface 제거 또는 scope withdrawal이 영향 분석 없이 merge되지 않는다.
- Review 1/2 결과와 발견·수정·재확인 evidence가 Manifest/release evidence에 남는다.
- 과거 3-review evidence는 보존하되 신규 변경의 필수 Review 수로 재상속하지 않는다.
