---
title: "DXBOT 최상위 기준선"
document_id: "DXB-BASE-000"
version: "0.3.0"
status: "Normative Baseline"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-SOURCE-000"]
---

# DXBOT 최상위 기준선

본 문서는 「DXBOT 1차 컨셉 기획안」의 제품 의미를 개발 기준으로 고정하고 v0.3의 **Common Framework / Stable SPI / Thin Extension** 규범을 추가한다. 원문 제품 철학을 바꾸지 않으며 아래 원칙은 별도 ADR, Migration/Compatibility 영향, Acceptance 갱신 없이 완화할 수 없다.

## 1. 제품 정의

> DXBOT은 기억을 중심으로 지속적으로 존재하는 AI Bot들이 하나의 Brain을 유지한 채 필요한 수의 Core를 동적으로 사용하고, 다른 Bot과 협업하며, Common Framework가 강제하는 안정된 Capability SPI 위의 교체 가능한 Provider를 통해 실행되는 Rust 기반 AI Bot Runtime이다.

Bot의 연속성은 Session이나 특정 모델/Provider가 아니라 `Identity + Memory + Persistent State`로 판단한다.

## 2. 비협상 핵심 원칙

1. DXBOT은 Session 기반 Agent가 아니다.
2. Bot은 요청·UI·프로세스 수명을 넘어 지속한다.
3. 하나의 Bot에는 하나의 논리적 Brain이 있다.
4. Brain은 특정 LLM 또는 Harness Provider가 아니다.
5. Core는 독립 Agent가 아니라 동일 Brain이 Execution을 수행하기 위한 일시적 Lease다.
6. Bot 생성 시 Core 개수를 지정하지 않는다. 시스템은 자원 정책으로 최대 동시성만 제한한다.
7. Core는 Identity·Goal·Permission·확정 Memory를 공유하되 Working Context는 격리한다.
8. 공유 상태 쓰기는 명령·revision·merge 계약을 거친다.
9. Bot은 다른 Bot에 Task를 위임할 수 있으나 상대 Bot의 내부 Memory/권한을 직접 공유하지 않는다.
10. Control Plane은 관찰·운영 표면이며 Domain Store를 직접 변경하지 않는다.
11. Interface는 CLI/TUI/Web/API 모두 동일 Runtime 계약을 사용한다.
12. Harness/Model/Tool/Sandbox 등 선택 기능은 Domain과 분리된 Capability 계약 뒤에 둔다.
13. DeepSeek는 필수 구성요소가 아니라 최초 Harness Provider 후보 중 하나다.
14. 특정 Provider의 제거·교체가 Bot/Brain/Memory/Task/Core의 Domain 의미를 바꾸지 않아야 한다.
15. 모든 queue/channel/cache는 item 및 byte 상한과 backpressure/eviction 정책을 가진다.
16. 동일 사실의 Canonical Owner는 하나이며 상태·정책·재시도·캐시를 중복 소유하지 않는다.
17. 불필요한 복사·할당·직렬화·상태 중복을 금지하고 성능 최적화는 측정 근거를 가진다.
18. async 작업은 owner, cancellation, join/quiescence 경로를 가진다.
19. Working Memory와 Canonical Long-term Memory를 분리한다.
20. Runtime memory pressure나 cache eviction을 이유로 Canonical Memory를 암묵 삭제하지 않는다.
21. `Waiting` Task를 영속하면 재개에 필요한 Continuation도 같은 논리적 Commit으로 보존한다.
22. 비멱등 Side Effect는 실행 전에 Intent/Idempotency 정보를 영속하며 결과 불명확 상태를 임의 Retry하지 않는다.
23. Routine은 Bot이 소유하는 영속 자동 실행 정의이며 Trigger/Schedule에 따라 일반 Task를 생성한다.
24. Plugin은 내부 모듈/Provider의 동의어가 아니라 별도 보안·패키징·버전·Lifecycle을 가진 외부 확장 단위다.
25. Plugin 공개 계약은 내부 Rust trait ABI가 아니라 안정된 Manifest/Wire/SDK 경계를 사용한다.
26. 모든 기능은 Core Domain / Optional Capability / Provider / Interface / Plugin 중 하나로 분류한다.
27. Optional Capability/Provider/Interface/Plugin은 추가뿐 아니라 비활성화·교체·제거 경로를 가진다.
28. 반복되는 limit/default/policy 값은 하나의 Canonical Policy Owner에서 정의하고 다른 문서는 참조한다.
29. Repository 구조와 naming은 Normative Rule이며 CI가 검증한다.
30. production Provider 호출은 Capability별 예외 없는 `Provider Host` 경계를 거친다.
31. Provider lifecycle state, registration generation, selection, drain/quiescence 판정은 Common Framework가 소유한다.
32. 각 Capability는 Request/Response/Event/Error/Config/Cancellation/Deadline/Resource/Idempotency/Side Effect/Version/Conformance를 포함하는 완전한 계약 패키지를 가진다.
33. Provider에는 최소 `ProviderCallContext`만 제공하며 `RuntimeContext`/service locator/Domain Store/Scheduler/Registry mutator를 제공하지 않는다.
34. Provider가 global retry, permission, resource admission, scheduling, domain persistence, global telemetry/audit를 자체 구현하거나 우회하지 않는다.
35. 모든 동등 Provider는 Common이 제공하는 Executable Conformance Suite와 실제 Provider Host 경로 검증을 통과한다.
36. Provider dependency는 Capability Contract, Provider SDK, 공개 Kernel 값 타입, 승인된 external SDK로 제한하고 Domain/Runtime/Storage/다른 Provider 의존을 CI가 차단한다.
37. Common Contract, Provider Host, Lifecycle, Error taxonomy, Security, Resource, Recovery, Conformance, SDK 변경은 Tier A/Common 설계 변경으로 취급한다.
38. 기존 Capability Provider 구현은 Tier B/Extension 작업으로 위임할 수 있으나 Common Contract 변경이 필요해지는 즉시 Tier A로 재분류한다.
39. 주요 Capability는 deterministic한 Reference Provider를 가지며 Conformance/SDK 사용의 실행 가능한 기준으로 삼는다.
40. 확장 편의를 위해 Common Contract를 오염시키지 않는다. Breaking semantic change에는 ADR, migration, Provider 영향, Conformance/Reference/SDK/Acceptance/Risk 갱신이 필요하다.

## 3. 아키텍처 분류

| 범주 | 의미 | 기본 제거성 | 예 |
|---|---|---|---|
| Core Domain | DXBOT 자체의 제품 의미 | 제거 불가 | Bot, Brain, Memory, Task, Execution, Core Lease |
| Optional Capability | 교체 가능한 기능의 안정 계약 | 제거/교체 가능 | ModelGateway, SandboxExecutor, MemoryIndex |
| Provider | Capability 구현 | 자유 교체/복수 등록 | DeepSeek Harness, Native Harness, 외부 Model Adapter |
| Interface | Runtime 접근 표면 | Runtime과 독립 | CLI, TUI, Web, API |
| Plugin | 외부 패키지 확장 단위 | 설치/비활성/업그레이드/제거 | third-party extension |

Domain은 Provider나 Plugin API에 맞춰 의미를 변경하지 않는다.

## 4. Common Framework 불변조건

`10-architecture/17-extension-framework.md`가 상세 Canonical Owner다.

```text
Consumer → Provider Host → Stable Provider SPI → Provider
```

Common Framework는 lifecycle, registry/selection, permission, resource, deadline/cancellation, retry disposition, side-effect guard, telemetry/audit, config/version, recovery, conformance를 중앙 소유한다. Provider는 provider-specific config, 외부 SDK/API adapter, capability-specific logic, DTO/error mapping을 소유한다.

Provider가 Common 기능을 중복 구현하여 동작이 같더라도 위반이다. 목적은 코드 중복 제거뿐 아니라 **보안·복구·자원·오류 의미의 단일 강제 지점**을 유지하는 것이다.

## 5. Provider 선택·Lifecycle 불변조건

- Consumer는 Stable Capability Contract에만 의존한다.
- Provider가 둘 이상이면 명시적 Selector/Policy가 결정한다.
- 등록 순서, 마지막 등록 승리, hash iteration order, 암묵 fallback을 금지한다.
- 하나의 Execution에서는 Provider selection/config generation을 고정한다.
- Provider 변경은 신규 Execution부터 적용한다. 보안 제한 강화는 별도 control signal로 즉시 적용할 수 있다.
- Provider 상태는 `Declared → Validated → Starting → Ready → Draining → Stopped`를 기본 축으로 하고 `Degraded/Failed/Quarantined/Incompatible`를 공통 상태로 해석한다.
- Provider health observation과 lifecycle state는 구분한다.
- `Draining` Provider는 신규 selection 대상이 아니다.
- quiescence는 Common activity/reference 상태로 판정한다.

## 6. 제거 불변조건

선택 기능 제거는 최소 다음을 만족한다.

`Deprecated → 신규 사용 차단 → in-flight Drain → Detach → API/Config/Event 호환 처리 → 데이터 Migration/Purge → Projection/Cache/Index 정리 → Dependency 제거 → Code 제거`

Provider가 완전히 제거된 빌드에서도 Domain/Core Runtime이 compile되고, 기존 Bot Identity/Canonical Memory가 복원되며, 관련 없는 Task가 정상 실행되어야 한다. stale config는 silent ignore하지 않는다.

## 7. 영속·복구 불변조건

- Command의 Domain Event, Current State, idempotency, Outbox는 하나의 Unit of Work로 Commit한다.
- `Waiting` 전이와 Continuation 저장을 분리 Commit하지 않는다.
- Side Effect Intent는 외부 변경 전에 영속한다.
- 외부 변경 후 Outcome 영속 전에 crash하면 `Unknown/Reconciliation Required` 의미를 보존한다.
- Runtime 재시작은 persisted state로 복구 판단하며 외부 Provider의 세션 상태를 Canonical Truth로 신뢰하지 않는다.
- Routine trigger occurrence는 고유 ID와 idempotency로 재시작 후 중복 Task 생성을 방지한다.
- Provider Host는 위 Side Effect/Recovery 계약을 우회할 수 있는 raw execution path를 Provider에 제공하지 않는다.

## 8. Repository 및 구현 기준

- 사용자 정의 repository path는 lowercase kebab-case를 기본으로 한다.
- Rust module/source `*.rs`는 language-native `snake_case`를 허용한다.
- `Cargo.toml`, `Cargo.lock`, `.github` 등 tool-mandated 이름은 명시적 allowlist로 관리한다.
- Provider concrete type의 Domain/Application 침투, Provider 간 직접 dependency, 제거 후 orphan dependency를 금지한다.
- Provider 표준 skeleton과 Scaffold는 `DXB-ARC-017`/`DXB-ENG-054`가 소유한다.

## 9. 검증 기준

- DeepSeek Provider 없이도 핵심 Domain/Runtime 테스트가 통과한다.
- 두 Harness Provider를 동시에 등록하고 Bot/Task별로 명시 선택할 수 있다.
- 모든 production Provider call path가 Provider Host enforcement를 거친다.
- Provider가 forbidden dependency 또는 자체 permission/retry/admission 경로를 추가하면 CI가 실패한다.
- Waiting Task crash/restart가 완료된 child를 재실행하지 않고 한 번만 resume한다.
- Side Effect crash window가 Unknown으로 복구되어 reconciliation 없이 재실행되지 않는다.
- Routine이 restart 후 missed/next trigger 정책에 따라 정확히 Task를 생성한다.
- naming/layout, dependency 방향, feature classification, SPI Acceptance 연결이 CI에서 검증 가능하다.
