---
title: "DXBOT v0.3 개발 기획 Manifest"
document_id: "DXB-MANIFEST"
version: "0.3.0"
status: "Reviewed Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-INDEX", "DXB-GOV-001", "DXB-ENG-052"]
---

# DXBOT v0.3 개발 기획 Manifest

## 1. 패키지 정보

- Package: `docs/plan/20260821-1120-v0-3-detailed-development-plan/`
- Source package: `docs/plan/20260821-1014-v0-2-detailed-development-plan/`
- Upgrade focus: Common Framework / Stable SPI / Provider Host / Executable Conformance
- Document count: **42개** (`manifest.md` 제외, `readme.md` 포함)
- New Canonical Owner: `10-architecture/17-extension-framework.md`
- Previous package preservation: v0.2는 수정하지 않고 별도 버전으로 보존

v0.3은 v0.2의 Persistent Bot, Single Brain/Dynamic Core, Memory-Centric, Routine/Continuation, Side Effect, Capability/Provider/Plugin, Feature Removal 원칙을 유지하면서 Provider 개발을 강한 Common Framework의 정형화된 SPI 슬롯으로 제한한다.

## 2. 문서 버전 정책

v0.3에서 의미가 변경된 문서는 `0.3.0`으로 승격했다. 제품 원문, 외부 참조 snapshot, 또는 v0.3 변경과 의미 충돌이 없어 그대로 계승한 문서는 기존 개별 문서 버전을 유지할 수 있다. 패키지 버전과 개별 문서 버전은 동일 개념이 아니다.

## 3. 문서 구성

### Governance — 6
- `00-governance/00-normative-baseline.md` — v0.3 강화
- `00-governance/00-source-concept-original.md` — 원문 계승
- `00-governance/01-document-governance.md` — v0.3 강화
- `00-governance/02-glossary-domain-model.md` — v0.3 강화
- `00-governance/03-architecture-decision-baseline.md` — v0.3 강화
- `00-governance/04-document-template.md` — v0.3 강화

### Architecture — 8
- `10-architecture/10-system-architecture.md` — v0.3 강화
- `10-architecture/11-rust-workspace-modules.md` — v0.3 강화
- `10-architecture/12-shared-contracts-extension-model.md` — v0.3 강화
- `10-architecture/13-harness-integration.md` — v0.3 강화
- `10-architecture/14-event-state-model.md` — 계승
- `10-architecture/15-storage-data-model.md` — v0.3 강화
- `10-architecture/16-plugin-system.md` — v0.3 강화
- `10-architecture/17-extension-framework.md` — **신규 Canonical 문서**

### Domains — 7
- `20-domains/20-bot-identity-lifecycle.md` — 계승
- `20-domains/21-brain-context.md` — v0.3 강화
- `20-domains/22-memory-system.md` — v0.3 강화
- `20-domains/23-goal-task-execution.md` — v0.3 강화
- `20-domains/24-dynamic-core-scheduler.md` — v0.3 강화
- `20-domains/25-multi-bot-network.md` — 계승
- `20-domains/26-control-plane.md` — v0.3 강화

### Runtime — 6
- `30-runtime/30-concurrency-consistency.md` — v0.3 강화
- `30-runtime/31-resource-governance.md` — v0.3 강화
- `30-runtime/32-security-permissions-sandbox.md` — v0.3 강화
- `30-runtime/33-resilience-recovery.md` — v0.3 강화
- `30-runtime/34-observability-audit.md` — v0.3 강화
- `30-runtime/35-configuration-deployment.md` — v0.3 강화

### Interfaces — 4
- `40-interfaces/40-api-protocols.md` — v0.3 강화
- `40-interfaces/41-cli.md` — 계승
- `40-interfaces/42-tui.md` — 계승
- `40-interfaces/43-web-control-center.md` — 계승

### Engineering — 5
- `50-engineering/50-rust-engineering-rules.md` — v0.3 강화
- `50-engineering/51-performance-memory-cache.md` — v0.3 강화
- `50-engineering/52-testing-verification.md` — v0.3 강화
- `50-engineering/53-versioning-migration.md` — v0.3 강화
- `50-engineering/54-repository-ci-release.md` — v0.3 강화

### Delivery — 5
- `60-delivery/60-implementation-roadmap.md` — v0.3 강화
- `60-delivery/61-acceptance-traceability.md` — v0.3 강화
- `60-delivery/62-risk-register.md` — v0.3 강화
- `60-delivery/63-open-questions.md` — v0.3 강화
- `60-delivery/64-external-reference-snapshot.md` — 계승

### Index — 1
- `readme.md` — v0.3 패키지 진입점

## 4. v0.3 핵심 계약

1. `12-shared-contracts-extension-model.md`가 Capability semantic과 완전한 Contract Package를 소유한다.
2. `17-extension-framework.md`가 Provider Host/Lifecycle/SDK/Conformance/Reference/Scaffold/Firewall/Quality Tier를 소유한다.
3. production 호출은 `Consumer → Provider Host → Stable SPI → Provider`를 따른다.
4. Scheduler는 Core/Execution fairness와 coarse admission을, Provider Host는 Provider selection과 Provider-specific permit/admission을 소유한다.
5. Execution은 여러 Capability binding을 가질 수 있고 각 binding의 `ProviderSelectionRef`는 한 번 확정되면 해당 Execution 동안 immutable하다.
6. Provider는 provider-specific config, external SDK/API adapter, capability logic, DTO/error mapping만 기본 소유한다.
7. Provider가 Common retry/security/resource/scheduler/persistence/telemetry를 중복 구현하거나 우회하지 않는다.
8. Provider dependency는 Contract/Provider SDK/공개 Kernel/승인 external SDK로 제한한다.
9. Common Contract breaking change는 ADR/Migration/Provider inventory/SDK/Conformance/Reference/Acceptance/Risk 갱신을 요구한다.
10. Tier A(Common/Contract)와 Tier B(Extension Implementation)를 분리하고 Tier B가 Contract 변경을 요구하면 Tier A로 승격한다.

## 5. 관계성 검토 결과 — 3/3 PASS

문서 작성 완료 후 `DXB-GOV-001`과 `DXB-ENG-052`의 목적이 다른 세 Review를 수행했다. 각 회차에서 발견 사항을 수정한 후 동일 범위를 재확인했다.

### Review 1 — Structural / Canonical Ownership — PASS

검사:
- file/path naming
- document ID / `depends_on`
- Canonical/Lifecycle/Persistent Owner
- Capability/Provider/Provider Host/Plugin 용어
- 중복 Policy/State
- 기존 v0.2 Acceptance/Risk/Open Question 보존

발견 및 수정:
1. v0.2에서 계승된 문서 거버넌스가 2회 검수만 정의 → v0.3 패키지에 **3단계 관계성 검토**를 Normative 규칙으로 승격.
2. Provider skeleton의 `cargo.toml` 표기가 tool-mandated `Cargo.toml` 예외 규칙과 충돌 → `Cargo.toml`로 통일.
3. 초기 병합본에서 v0.2의 상세 Acceptance/Risk/Open Question 일부가 요약됨 → AT-MOD/ROUTINE/TASK/SFX/PLUGIN/REPO/MEM/POL, R-001~039, OQ-001~015/OQ-101~113 세부 의미를 복원하고 SPI 항목과 병합.
4. Governance template에 Quality Tier가 없음 → Tier A/B와 escalation을 추가.
5. 패키지 버전과 계승 문서 개별 버전 관계가 모호 → inheritance/version 정책을 명문화.

재확인 결과: 신규 Canonical Owner와 기존 Domain Owner가 중복되지 않고, 계승 요구의 의미 회귀가 없음을 확인.

### Review 2 — Contract / Traceability — PASS

검사:

```text
Capability Contract
→ Provider Host
→ Security / Resource / Recovery / Observability / Config
→ Provider SDK / Conformance / CI
→ Acceptance / Risk
```

발견 및 수정:
1. Performance 문서에 Provider Host/SDK의 allocation/copy/latency 비용 측정이 없음 → Host pipeline/Call Context/DTO mapping/stream/telemetry/generation pin benchmark를 추가.
2. Control Plane/API가 새 Lifecycle/Contract/SDK/Conformance evidence를 충분히 노출하지 않음 → full lifecycle vocabulary, health 분리, generation/config/SDK/Conformance/removal diagnostics를 추가.
3. Rust module 예시가 kebab 표기로 실제 Rust path와 불일치 → `dxb_runtime::provider_host`로 교정.
4. Workspace Contract 설명에 Side Effect semantic이 누락 → complete Contract 의미와 일치시킴.
5. Application↔Provider Host crate dependency cycle 가능성이 문서상 모호 → Application은 Port를 정의하고 Runtime composition이 Host 구현을 연결하도록 명시.

재확인 결과: Contract의 security/resource/deadline/cancel/error/side-effect/version 의미가 Runtime, API, Testing, CI, Acceptance/Risk까지 끊김 없이 연결됨을 확인.

### Review 3 — Development Flow / Cross-Layer Executability — PASS

검사 경로 A:

```text
Capability Definition
→ Common Host/SDK/Testkit
→ Scaffold/Provider
→ Conformance/CI
→ Runtime Invocation
→ Failure/Recovery
→ Removal
```

검사 경로 B:

```text
Command
→ Application/Domain/Persistence
→ Scheduler/Core Lease
→ Provider Host
→ Provider/Plugin
→ Outcome/Recovery
→ Projection/Control API
```

대입 시나리오:
- 정상 실행
- 여러 Capability/Provider 사용
- permission/resource deny
- timeout/cancellation
- Provider Draining/Failed
- Contract/SDK version mismatch
- crash/restart
- Side Effect outcome unknown
- Provider replacement/removal
- Plugin 제공 Provider disable/uninstall

발견 및 수정:
1. 기존 Scheduler가 Provider selection과 Provider permit까지 일부 소유하여 Host와 책임 중복 → Scheduler는 provider-independent Core/Execution admission만, Host는 selection/binding/Provider-specific permit을 소유하도록 분리.
2. Brain routing이 concrete Provider 선택으로 오해될 수 있음 → Brain은 Capability requirement/selection hint만 만들고 actual selection은 Host가 수행하도록 명시.
3. Task/Execution의 Provider snapshot owner가 불명확 → Host-created opaque `ProviderSelectionRef`로 고정.
4. 하나의 Execution이 Model/Tool/MemoryIndex 등 여러 Capability를 쓸 수 있는데 “Execution당 Provider 하나”로 해석될 여지 → **Capability binding별 pin**으로 정교화. 동일 binding은 immutable, 같은 binding의 fallback은 새 Execution/Attempt.
5. MemoryIndex가 Derived Provider임은 정의됐지만 실제 Host 호출 경로가 약함 → recall/rebuild Provider 호출을 Host/Conformance 경로로 고정하고 Canonical Memory owner를 Memory subsystem에 유지.
6. persisted Provider lifecycle state가 live `Ready/activity` authority로 오해될 수 있음 → Operational Catalog에는 config/admin intent/last known diagnostics만 보존하고 restart 시 lifecycle/compatibility를 재검증하도록 명시.
7. Scheduler가 Resource Governance를 선행 `depends_on`으로 두고 Resource Governance가 Scheduler를 의존해 문서 DAG 순환 가능 → Scheduler의 선행 dependency에서 제거하고 Policy SSOT는 의미 참조로 유지.
8. 시스템 mermaid에서 `Scheduler → Selector` 경로가 남아 Host ownership과 충돌 → `Task/Memory → Provider Host → Selector → Registry`로 교정.

재확인 결과: Scheduler/Host/Provider/Recovery 사이에 중복 selection/permit/retry authority가 없고, crash/version mismatch/removal 시 기존 Execution을 silent mutation하지 않음을 확인.

## 6. 최종 개발 흐름

### 새로운 Capability — Tier A

```text
Capability semantic 정의
→ Complete Contract Package
→ Provider Host integration
→ Lifecycle/Resource/Security/Error ownership 연결
→ Provider SDK surface
→ Executable Conformance
→ Reference Provider + negative fixture
→ Acceptance/Risk/Compatibility
→ CI gate
```

### 새로운 Provider — Tier B

```text
Standard Scaffold
→ Provider Config
→ External SDK/API Adapter
→ DTO/Error Mapping
→ Capability Logic
→ Common Conformance
→ Host-path Integration
→ Dependency Firewall
→ Removal/Leak Check
```

Provider 작업 중 Common Contract 변경이 필요해지면 구현 우회 없이 Tier A로 재분류한다.

## 7. 완료 판정

- [x] v0.2 패키지 보존
- [x] v0.3 새 패키지 생성
- [x] 신규 `17-extension-framework.md` Canonical Owner 생성
- [x] Common/Extension 책임 분리
- [x] Provider Host / Lifecycle / SDK / Conformance / Reference / Scaffold 규범화
- [x] Dependency Firewall / Contract Stability / Quality Tier 규범화
- [x] AT-SPI-001~010 추가
- [x] R-SPI-001~010 추가
- [x] AGENTS 및 장기 Agent 규칙과 정합화
- [x] Review 1 Structural / Canonical Ownership
- [x] Review 2 Contract / Traceability
- [x] Review 3 Development Flow / Cross-Layer Executability
- [x] 각 Review 발견 사항 수정 후 재확인

v0.3 기획 단계의 관계성 검토 결과는 **3/3 PASS**다. 실제 구현 단계에서는 각 문서의 Acceptance/CI Gate가 실행 가능한 코드·테스트 증거로 전환되어야 한다.
