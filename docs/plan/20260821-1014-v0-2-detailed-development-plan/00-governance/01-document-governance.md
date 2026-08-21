---
title: "문서 거버넌스와 변경 통제"
document_id: "DXB-GOV-001"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000"]
---

# 문서 거버넌스와 변경 통제

## 1. 목적

설계·코드·테스트가 서로 다른 의미로 분기하지 않도록 문서 소유권, 기능 분류, 변경 세트, 제거 영향, 이중 재검수, 추적성을 정의한다.

## 2. 문서 상태

- `Draft`: 검토 중
- `Accepted`: 구현·테스트가 따라야 하는 승인 계약
- `Normative Baseline`: 최상위 제품 기준
- `Superseded`: 새 문서/ADR로 대체
- `Deprecated`: 신규 사용을 금지하고 제거 계획 보유

## 3. 요구사항 식별자

- 제품 원칙 `PR-*`
- 기능 요구 `FR-*`
- 비기능 요구 `NFR-*`
- 불변조건 `INV-*`
- 수용 테스트 `AT-*`
- ADR `ADR-*`
- Risk `R-*`

## 4. Architecture Feature Classification

새 기능 또는 기존 기능의 의미 변경은 구현 전에 다음 중 하나로 분류한다.

`Core Domain | Optional Capability | Provider | Interface | Plugin`

각 기능 문서에는 최소 다음 metadata/section이 있어야 한다.

- Classification
- Canonical Owner
- Lifecycle Owner
- Persistent State 존재 여부와 owner
- Capability/Provider/Plugin 여부
- Dependency 방향
- disable/remove 가능 여부
- remove 시 data/config/schema 영향
- Conformance/Acceptance Test

분류가 불가능하거나 두 범주를 암묵적으로 겸하는 기능은 설계 검토 대상으로 올린다.

## 5. 변경 세트

의미 변경은 최소 다음을 하나의 change set으로 묶는다.

1. 기준/소유 문서
2. 영향 API/Event/Config/Persistence 계약
3. Migration 또는 Feature Removal 판단
4. Acceptance/Traceability
5. Security/Resource/Observability 영향
6. Rollback/Compatibility/Deprecation
7. Risk 및 Open Question 갱신

Provider나 Plugin 변경을 이유로 Core Domain 의미를 함께 수정하는 경우 별도 ADR로 정당화한다.

## 6. Feature Removal 영향 분석

Optional Capability/Provider/Interface/Plugin 제거 PR은 다음을 명시한다.

- 신규 사용 차단 지점
- in-flight drain/quiescence
- stale config 처리
- API/Event 상태(`deprecated`, `unsupported`, version negotiation)
- persistent data의 retain/export/migrate/purge
- Derived cache/index/projection cleanup
- dependency/build feature cleanup
- 마지막 호환 버전과 rollback
- 제거된 구성 없이 수행하는 acceptance suite

단순 code delete로 완료 처리하지 않는다.

## 7. ADR 필요 조건

다음은 ADR 없이 변경할 수 없다.

- Core Domain 경계
- Canonical Owner 이동
- Provider/Plugin 계약의 호환성 파괴
- Repository naming/layout 규칙
- 영속 상태 또는 side-effect crash semantics
- Routine/Task/Execution lifecycle 의미
- public wire protocol 또는 migration irreversibility
- 새로운 `unsafe`, FFI, process isolation 방식

## 8. 2단계 재검수

### Review 1 — Structural / Consistency

- lowercase kebab-case 및 허용 예외
- document_id 중복
- `depends_on` 존재성
- Canonical/Lifecycle Owner 충돌
- Capability/Provider/Plugin 용어 혼용
- 중복 Policy/Limit
- 링크·Manifest·Acceptance·Risk 연결
- Superseded/Deprecated 대체·제거 경로

### Review 2 — Cross-Layer Executability

대표 시나리오를 다음 흐름으로 끝까지 추적한다.

`Command → Application → Domain → Persistence → Scheduler → Provider → Recovery → Projection → Control API`

정상, Provider 없음/교체/제거/복수, crash, cancellation, timeout, partial success, migration, feature disable/removal을 검토한다. 계층 사이에 owner 없는 상태가 생기면 완료로 보지 않는다.

## 9. 자동 검사

CI는 최소 다음을 검증한다.

- 깨진 상대 링크/문서 ID/depends_on
- naming/layout
- requirement↔acceptance 미연결
- schema version 변경↔migration 부재
- provider concrete dependency 침투
- removal 대상 orphan dependency/config
- policy/default 중복 후보
- manifest drift

## 10. 검증 기준

- 모든 Accepted 문서가 소유 crate/module과 Acceptance를 가진다.
- 모든 신규 기능이 5개 분류 중 하나다.
- Provider/Plugin 제거 변경이 Removal 영향 분석 없이 merge되지 않는다.
- 두 단계 재검수 결과가 release evidence에 남는다.
