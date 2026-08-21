---
title: "DXBOT 상세 개발 기획 문서 집합"
document_id: "DXB-INDEX"
version: "0.3.0"
status: "Reviewed Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000"]
---

# DXBOT 상세 개발 기획 문서 집합 v0.3

이 디렉터리는 v0.2의 Persistent Bot, Memory-Centric Runtime, Single Brain/Dynamic Core, Capability/Provider/Plugin 분리, Removal/Recovery 규범을 유지하면서 **Common Framework + Stable SPI + Provider Host + Executable Conformance**를 프로젝트 전체에 강제하는 신규 버전이다.

> Common이 어려운 correctness 문제를 소유하고, Extension은 정해진 계약을 구현하게 만들어 기능 추가·교체·제거를 쉽고 정형화하며 안전하게 한다.

## 1. 기준 우선순위

1. `00-governance/00-source-concept-original.md`의 제품 철학
2. `00-governance/00-normative-baseline.md`의 비협상 기준
3. 승인된 ADR 및 `03-architecture-decision-baseline.md`
4. Domain 문서의 불변조건과 Canonical Owner
5. Architecture/Runtime/Engineering의 세부 계약
6. 구현 코드와 테스트

## 2. v0.3 핵심 강화점

### Common Framework / Extension Zone
Domain과 Runtime의 어려운 문제를 Common이 소유하고 Provider는 provider-specific integration만 구현한다.

### Mandatory Provider Host
모든 production Provider 호출은 `Consumer → Provider Host → Provider SPI → Provider`를 따른다. Host가 selection binding, permission, resource, deadline/cancellation, side-effect guard, telemetry/audit, normalization/accounting을 공통 집행한다.

### Standard Provider Lifecycle
`Declared → Validated → Starting → Ready → Draining → Stopped`와 `Degraded/Failed/Quarantined/Incompatible` 의미를 Common이 소유한다. Provider가 임의 idle/drain 상태를 결정하지 않는다.

### Complete Capability Contract
각 Capability는 Request/Response/Event/Error/Config/Cancel/Deadline/Resource/Idempotency/Side Effect/Version/Conformance를 가진다.

### Capability Binding Pinning
하나의 Execution은 여러 Capability를 사용할 수 있다. 각 Capability binding별 `ProviderSelectionRef`를 Host가 확정하며, 같은 binding은 해당 Execution 동안 Provider/version/config/Registry generation을 바꾸지 않는다.

### Thin Provider / Minimum Surface
Provider는 Provider-specific Config, 외부 SDK/API Adapter, Capability Logic, DTO/Error Mapping만 소유한다. `RuntimeContext`, Domain Store, Scheduler, Registry mutator를 받지 않는다.

### Provider SDK / Skeleton / Scaffold
허용된 구현 surface와 표준 package 구조를 제공하고 P1에서 Provider 생성 자동화를 도입한다.

### Executable Conformance / Reference Provider
Provider별 테스트 설계를 반복하지 않고 Common suite를 실행한다. 주요 Capability에는 deterministic Reference Provider와 negative fixture를 둔다.

### Dependency Firewall
Provider는 Capability Contract/Provider SDK/공개 Kernel/승인 external SDK만 기본 의존한다. Domain/Application/Runtime/Storage/다른 Provider 의존을 CI에서 차단한다.

### Common Contract Stability
변경을 Additive Compatible / Behavior Clarification / Deprecation / Breaking Semantic Change로 분류하고 Breaking에는 ADR·Migration·Provider inventory·Conformance·Reference·SDK·Acceptance·Risk 갱신을 의무화한다.

### Quality Tier / AI Delegation
Common/Contract/Host/Security/Recovery/Conformance는 Tier A/Frontier-quality 작업으로, 기존 Stable Contract의 Provider/Adapter는 Tier B/Extension 작업으로 분리한다. Tier B가 Contract 변경을 요구하면 즉시 Tier A로 재분류한다.

## 3. Canonical 문서 분리

`10-architecture/12-shared-contracts-extension-model.md`는 Capability의 의미와 Contract Package를 소유한다.

`10-architecture/17-extension-framework.md`는 다음을 소유한다.
- Common vs Extension 책임
- Provider Host
- Standard Provider Lifecycle
- Thin Provider / Minimum Provider Surface
- Provider SDK
- Standard Provider Skeleton / Scaffold
- Executable Conformance
- Reference Provider
- Dependency Firewall
- Contract Stability
- Quality Tier / AI Delegation
- Extension Task Template

## 4. 권장 읽기 순서

| 순서 | 문서군 | 목적 |
|---:|---|---|
| 1 | `00-governance` | 제품 의미, 용어, 비협상 결정 |
| 2 | `10-architecture/10~12` | 전체 구조와 Capability semantic |
| 3 | `10-architecture/17-extension-framework.md` | Provider 구현 Framework/SPI |
| 4 | `20-domains` | Bot·Brain·Memory·Task·Core·Network·Control |
| 5 | `30-runtime` | Host와 연결되는 동시성·자원·보안·복구·관측·구성 |
| 6 | `40-interfaces` | 동일 Runtime Control/API/UI |
| 7 | `50-engineering` | Rust/Conformance/Compatibility/CI |
| 8 | `60-delivery` | 구현 단계, Acceptance, Risk, Open Question |

## 5. 공통 설계 규칙

- Bot이 Identity·Memory·Persistent State를 소유한다. Session/UI/Provider는 소유자가 아니다.
- 하나의 Bot에는 하나의 Brain이 있고 Core는 일시적 실행 Lease다.
- Canonical State와 Derived State를 분리한다.
- Consumer는 concrete Provider가 아니라 Capability Contract/Provider Host에 의존한다.
- Scheduler는 Core/Execution fairness를, Host는 Provider selection/Provider-specific admission을 소유한다.
- Provider는 Common security/resource/retry/lifecycle/telemetry를 재구현하지 않는다.
- 같은 Execution의 같은 Capability binding은 Provider generation을 바꾸지 않는다.
- Provider/Plugin 제거가 Core Domain schema 변경을 요구하지 않아야 한다.
- 모든 queue/cache는 bounded다.
- 불필요한 복사·할당·상태·정책·재시도·캐시 중복을 금지한다.

## 6. 개발 순서 원칙

대규모 Provider 확장은 다음 P0 Foundation이 닫힌 뒤 시작한다.

1. Common vs Extension 책임
2. Standard Provider Lifecycle
3. Complete Capability Contract
4. Provider Host
5. Thin Provider / Minimum Surface
6. Dependency Firewall
7. Quality Tier / AI Delegation
8. Executable Conformance 기준

그 다음 Provider SDK/Reference/Skeleton을 완성하고, 이후 기존 Capability의 Provider 구현을 반복한다.

## 7. v0.3 완료 정의

1. 새로운 Capability를 Tier A에서 Contract/Host/Conformance/Reference까지 정의할 수 있다.
2. 이후 Tier B 구현 Agent에는 표준 skeleton의 TODO와 허용 dependency만 주어 Provider를 구현시킬 수 있다.
3. Provider 내부 구현 품질과 무관하게 Input/Output/Error/Security/Resource/Lifecycle/Recovery semantics가 Common Framework로 보호된다.
4. AT-SPI-001~010이 자동화 가능한 Acceptance로 연결된다.
5. Provider replacement/removal/multi-selection 기존 Acceptance도 유지된다.
6. Plugin Provider도 동일 Host/Conformance/Dependency Firewall을 따른다.
7. Structural / Contract-Traceability / Development-Flow 관계성 검토 3회를 모두 통과한다.

## 8. 관계성 검토 상태

문서 작성 후 총 3회의 독립 검토와 발견 사항 수정을 완료했다.

- Review 1 — Structural / Canonical Ownership: **PASS**
- Review 2 — Contract / Traceability: **PASS**
- Review 3 — Development Flow / Cross-Layer Executability: **PASS**

세부 발견 사항, 수정 내용, 재확인 결과는 `manifest.md`가 검증 이력의 Canonical 문서다.
