---
title: "구현 로드맵과 단계별 Gate"
document_id: "DXB-DEL-060"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-ARC-010", "DXB-ARC-016", "DXB-ARC-017", "DXB-ENG-052"]
---

# 구현 로드맵과 단계별 Gate

## 1. 목적

기능을 화면이나 Provider 수 중심이 아니라 **Common correctness boundary의 완성도 → 정형화된 Extension 개발 표면 → 자동화 → 생태계** 순으로 구현한다. 기간보다 exit gate와 의존성을 기준으로 한다.

## 2. 선행 원칙 — Provider 확장 Freeze

다음 P0 Foundation이 닫히기 전 production Provider를 대규모로 늘리지 않는다.

1. Common vs Extension 책임 확정
2. Standard Provider Lifecycle
3. Complete Capability Contract Package
4. Mandatory Provider Host
5. Thin Provider / Minimum Provider Surface
6. Extension Dependency Firewall
7. Common/Extension Quality Tier와 AI Delegation Boundary
8. Executable Conformance의 기본 구조

Provider를 먼저 늘리고 나중에 공통화하는 순서를 금지한다. 동일 보안·재시도·자원·lifecycle 로직이 두 Provider에 나타나는 순간 Common ownership 결함으로 취급한다.

## 3. Phase 1 — Common Contract Freeze Foundation / P0

### 구현 범위
- `DXB-ARC-012` Capability Contract Package 규범 확정
- `DXB-ARC-017` Provider Host/Lifecycle/Thin Provider/Minimum Surface 확정
- Registry generation/Selector/Draining ownership 확정
- Provider Call Context 최소 권한 surface 확정
- Error/Retry/Side Effect ownership 확정
- Dependency Firewall architecture rule
- Tier A/Tier B 작업 분류와 escalation 규칙

### Exit Gate
- AT-SPI-001 Contract completeness
- AT-SPI-002 Lifecycle
- AT-SPI-003 Host enforcement
- AT-SPI-004 Thin Provider boundary
- AT-SPI-005 Forbidden dependency
- AT-SPI-008 Contract compatibility의 분류 규칙
- Provider Host를 우회하는 production path 설계 0건
- Domain/Runtime 내부를 Provider에 제공하는 service locator 설계 0건

이 Gate가 닫힌 뒤 Common Contract의 P0 surface를 구현 freeze 후보로 본다.

## 4. Phase 2 — Provider Development Framework / P0-P1

### 구현 범위
- Provider SDK 최소 안정 surface
- Capability별 Executable Conformance Framework
- Host-path integration harness
- deterministic Golden Reference Provider
- negative/broken Provider fixtures
- Standard Provider Package Skeleton
- Provider removal/minimal build fixture
- CI dependency/Host/contract gates

### Exit Gate
- AT-SPI-007 Conformance Auto Suite
- AT-SPI-009 Reference Provider Equivalence
- AT-SPI-010 Provider Removal
- 기존 AT-MOD-001/002/003 Provider replacement/removal/multi-selection 유지
- Reference Provider가 실제 Host 경로에서 통과
- intentionally broken Provider가 예상 gate에서 실패
- Provider SDK가 Runtime/Domain/Storage mutable internals를 노출하지 않음

Phase 2 이후 기존 Capability의 새 Provider는 Tier B 작업으로 반복 구현할 수 있어야 한다.

## 5. Phase 3 — Extension Automation / P1

### 구현 범위
- `dxb-dev new-provider <capability> <provider>` Scaffold Generator
- config/error/mapping/provider/conformance skeleton
- fixture/benchmark/docs template
- Capability/SDK version pinning
- generated template drift validation
- static forbidden dependency check 강화

### Exit Gate
- AT-SPI-006 Provider Scaffold
- 새 Provider skeleton이 생성 직후 compile 가능
- 허용 dependency 외 internal dependency 0
- Conformance 연결점 누락 0
- 동일 입력에서 deterministic generated tree
- obsolete SDK/contract template drift가 CI에서 탐지됨

## 6. Phase 4 — Extension Ecosystem / P2

- remote Provider transport
- public/third-party Extension SDK 후보
- Plugin Provider certification
- compatibility certification evidence
- signed/isolated package 연계
- third-party Provider/Plugin ecosystem

원격화·외부화해도 `Provider Host`의 permission/resource/deadline/cancellation/error/telemetry semantics를 약화시키지 않는다.

## 7. 기존 Milestone 연계

| Milestone | 핵심 결과 | v0.3 추가 Gate |
|---|---|---|
| M0 Foundation & Harness | workspace/testkit/storage/Harness spike | Provider Host/Lifecycle/Contract skeleton, Reference Harness, dependency firewall |
| M1 Persistent Single Bot | restart Bot + CLI | Common config/lifecycle generation, Routine persistence |
| M2 Memory/Task/Harness | Memory/Task/single Core | Harness Host-path Conformance, Waiting/Side Effect, Provider replacement/removal |
| M3 Dynamic Core | bounded parallel/fairness | multi-provider Host admission, activity/drain race, provider resource metrics |
| M4 Multi-Bot | durable delegation | delegated capability calls도 동일 Host enforcement |
| M5 Control API & TUI | management plane | Provider lifecycle/conformance/compatibility evidence 조회 |
| M6 Web | control UI | shared API only; Extension policy duplication 금지 |
| M7 Distributed | remote/HA candidates | remote Provider envelope가 local Host semantics 유지 |

## 8. M0 Exit 핵심

- Domain no-I/O test
- Event atomic append/replay
- Capability Contract Package 템플릿/검사
- Provider Host 기본 invocation pipeline
- Standard Lifecycle generation/drain model
- deterministic Reference Harness Provider
- DeepSeek는 optional feasibility candidate
- DeepSeek 없이 minimal build
- dependency firewall/Host bypass/naming/docs CI
- AT-SPI-001~005 기본 자동화 가능

## 9. M1 Exit 핵심

- BotId/Identity/Memory namespace restart restore
- Session independence
- Routine create/enable/disable persistence
- Provider config generation/reload validation
- Lifecycle clean quiescence
- Host activity/permit leak 0

## 10. M2 Exit 핵심

- Working Memory 자동 승격 금지
- Task semantic retry가 새 Execution/Attempt
- Waiting + Continuation atomic commit
- Side Effect Unknown/reconcile
- Harness Provider Host-path Executable Conformance
- Reference + 실제 후보 Provider 동일 semantic suite
- Provider A 제거 후 core acceptance
- incompatible Contract/SDK Provider Ready 진입 차단

## 11. M3 Exit 핵심

- core count가 Bot schema에 없음
- deterministic priority/tie-break
- global/per-Bot/provider resource limits
- Provider A/B/C multi-selection
- Draining vs new call, cancellation/expiry/late-result race
- queue/admission/activity/starvation metrics

## 12. Quality Tier Workstream

### Tier A — Common / Frontier Quality
새 Capability, Domain invariant/state, Provider Host/Lifecycle/Registry/Selector, Security/Resource/Recovery, Error taxonomy, Conformance/Testkit/SDK, Migration/Public compatibility를 담당한다.

필수: 설계/ADR 영향, property/model/concurrency/fault, API stability, removal/rollback, 필요 시 benchmark/profile.

### Tier B — Extension / Implementation Quality
기존 Capability Provider, 외부 API adapter, DTO/error mapping, provider-specific config/fixture를 담당한다.

필수: compile/lint + Contract compliance + Conformance + Host-path + Dependency Firewall + Removal cleanliness.

Tier B 작업이 Common Contract 변경을 필요로 하면 즉시 Tier A로 재분류한다.

## 13. Cross-Cutting Workstream

- docs/ADR/traceability/risk
- security threat model
- RSS/allocation/cache
- migration fixture
- fault injection
- Provider/Plugin removal cleanliness
- observability
- temporary shim cleanup
- naming/layout
- Scaffold/Reference/Conformance drift
- Contract compatibility inventory

## 14. Scope Control

- DeepSeek가 불안정해도 Reference/Fake/Native Provider로 Core 개발을 지속한다.
- 새 Provider 요구가 생겨도 Common Foundation을 우회해 임시 direct call을 만들지 않는다.
- Provider 편의를 위해 `RuntimeContext`/DB/Scheduler 접근을 허용하지 않는다.
- Plugin SDK와 Provider SDK를 동일 API로 합치지 않는다.
- 분산화는 단일 노드 Host/lifecycle/idempotency/recovery가 검증된 후 진행한다.

## 15. 검증 기준

- 각 Phase와 Milestone이 구현 결과가 아니라 자동화 가능한 exit gate를 가진다.
- Phase 1 이전 대규모 Provider 확장이 계획에 포함되지 않는다.
- Tier B 작업이 Tier A Contract를 암묵 수정할 수 없다.
- AT-SPI-001~010과 기존 AT-MOD/REC/SEC가 구현 단계에 연결된다.
