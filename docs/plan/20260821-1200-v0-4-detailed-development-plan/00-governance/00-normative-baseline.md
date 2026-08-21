---
title: "DXBOT 최상위 기준선"
document_id: "DXB-BASE-000"
version: "0.4.0"
status: "Normative Baseline"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-SOURCE-000"]
---

# DXBOT 최상위 기준선

본 문서는 「DXBOT 1차 컨셉 기획안」의 제품 의미와 v0.3 Common Framework / Stable SPI 규범을 유지하면서 v0.4의 **Persistent Main Conversation + Thread + Live Control**을 비협상 제품 원칙으로 추가한다. 아래 의미는 ADR·Migration/Compatibility·Acceptance 영향 검토 없이 완화할 수 없다.

## 1. 제품 정의

> DXBOT은 기억을 중심으로 지속적으로 존재하는 AI Bot들이 하나의 Brain을 유지한 채 영속 Main Conversation과 독립 Thread들을 통해 장기 문맥을 보존하고, 필요한 수의 Core를 동적으로 사용하며, 실행 중 작업을 관찰·개입할 수 있고, Common Framework의 안정된 Capability SPI 위 교체 가능한 Provider를 통해 실행되는 Rust 기반 AI Bot Runtime이다.

Bot의 연속성은 Interface Session, Provider Session, 특정 모델/Provider가 아니라 `Identity + Memory + Persistent State + Main Conversation/Thread State`로 판단한다.

## 2. 비협상 핵심 원칙

### Persistent Bot / Brain / Core
1. DXBOT은 Session 기반 Agent가 아니다.
2. Bot은 요청·UI·프로세스 수명을 넘어 지속한다.
3. 하나의 Bot에는 하나의 논리적 Brain이 있다.
4. Brain은 특정 LLM 또는 Harness Provider가 아니다.
5. Core는 독립 Agent가 아니라 동일 Brain이 Execution을 수행하기 위한 일시적 Lease다.
6. Bot 생성 시 Core 개수를 지정하지 않는다. 시스템은 자원 정책으로 최대 동시성만 제한한다.
7. Core는 Identity·Goal·Permission·확정 Memory를 공유하되 Working Context는 격리한다.
8. 공유 상태 쓰기는 Command·revision·merge 계약을 거친다.
9. Bot은 다른 Bot에 Task를 위임할 수 있으나 상대 Bot의 내부 Memory/권한을 직접 공유하지 않는다.

### Control / Interface
10. Control Plane은 관찰·운영 표면이며 Domain Store를 직접 변경하지 않는다.
11. Interface는 CLI/TUI/Web/API 모두 동일 Runtime 계약을 사용한다.
12. Interface Session 종료가 Bot/Conversation/Thread lifecycle을 변경하지 않는다.

### Capability / Provider / Plugin
13. Harness/Model/Tool/Sandbox 등 선택 기능은 Domain과 분리된 Capability 계약 뒤에 둔다.
14. DeepSeek는 필수 구성요소가 아니라 Harness Provider 후보 중 하나다.
15. 특정 Provider 제거·교체가 Bot/Brain/Memory/Task/Core/Conversation/Thread의 Domain 의미를 바꾸지 않아야 한다.
16. production Provider 호출은 예외 없는 Provider Host 경계를 거친다.
17. Provider lifecycle/registration generation/selection/drain/quiescence는 Common Framework가 소유한다.
18. 각 Capability는 Request/Response/Event/Error/Config/Cancellation/Deadline/Resource/Idempotency/Side Effect/Version/Conformance를 포함하는 완전한 Contract Package를 가진다.
19. Provider에는 최소 scoped Call Context만 제공하며 RuntimeContext/service locator/Domain Store/Scheduler/Registry mutator를 제공하지 않는다.
20. Provider가 global retry/permission/resource admission/scheduling/domain persistence/global telemetry를 재구현하거나 우회하지 않는다.
21. 동등 Provider는 Common Executable Conformance와 실제 Host 경로 검증을 통과한다.
22. Provider dependency는 Capability Contract/Provider SDK/공개 Kernel/승인 external SDK로 제한한다.
23. Plugin은 내부 모듈/Provider의 동의어가 아니라 별도 패키징·보안·Lifecycle을 가진 외부 확장 단위다.
24. Plugin 공개 계약은 내부 Rust trait ABI가 아니라 안정된 Manifest/Wire/SDK를 사용한다.
25. 모든 기능은 Core Domain / Optional Capability / Provider / Interface / Plugin 중 하나로 분류한다.
26. Optional Capability/Provider/Interface/Plugin은 추가뿐 아니라 비활성화·교체·제거 경로를 가진다.

### State / Resource / Recovery
27. 모든 queue/channel/cache는 item+byte 상한과 backpressure/eviction 정책을 가진다.
28. 동일 사실의 Canonical Owner는 하나이며 상태·정책·재시도·캐시를 중복 소유하지 않는다.
29. 불필요한 복사·할당·직렬화·상태 중복을 금지하고 성능 최적화는 측정 근거를 가진다.
30. async 작업은 owner, cancellation, join/quiescence 경로를 가진다.
31. Working Memory와 Canonical Memory를 분리한다.
32. Runtime pressure/cache eviction을 이유로 Canonical Memory를 암묵 삭제하지 않는다.
33. Waiting Task를 영속하면 재개 Continuation도 같은 논리적 Commit으로 보존한다.
34. 비멱등 Side Effect는 외부 실행 전 Intent/Idempotency를 영속하고 결과 불명확 상태를 임의 Retry하지 않는다.
35. Routine은 Bot 소유의 영속 자동 실행 정의이며 일반 Task를 생성한다.
36. 반복되는 limit/default/policy 값은 하나의 Canonical Policy Owner에서 정의한다.
37. Repository 구조/naming은 Normative Rule이며 CI 검증 대상이다.
38. Execution snapshot과 동일 Capability binding은 해당 Execution 동안 immutable하다.
39. Common Contract/Host/Lifecycle/Error/Security/Resource/Recovery/Conformance/SDK 변경은 Tier A로 취급한다.
40. 기존 Stable Capability Provider 구현은 Tier B이며 Common semantic 변경이 필요하면 Tier A로 승격한다.

### Persistent Conversation / Thread
41. **Bot 하나는 하나의 영속 Main Conversation을 가진다.** 새 Interface Session이 새 Bot 대화를 만들지 않는다.
42. 작업 문맥의 영속 독립 단위는 Session이 아니라 **Thread**다.
43. `Interface Session`, `Provider Session`, `Thread`를 서로 다른 수명·소유권 개념으로 분리한다.
44. Main Conversation은 `Thread 0`이 아니라 Bot Communication + Supervisor Control Surface다.
45. `Thread ≠ Task ≠ Execution ≠ Core Lease ≠ Provider Session`을 유지한다.
46. Conversation History는 Memory가 아니다. transcript 저장이 Memory commit을 의미하지 않는다.
47. Thread-local Memory는 명시 Promotion 정책 없이 Bot-global Memory로 승격되지 않는다.
48. 전체 Thread transcript 크기와 모델 Context 크기는 독립적이며 Context Plan은 항상 bounded다.
49. Provider Session 손실은 Bot/Main Conversation/Thread/Memory 손실을 의미하지 않는다.

### Live Control
50. 일반 Work Queue와 Runtime Control Channel을 분리하며 둘 다 bounded다.
51. inspect/report/cancel/safety control이 일반 backlog에서 무기한 starvation되지 않아야 한다.
52. redirect는 running Execution snapshot을 mutation하지 않고 durable Directive + Task Specification revision + 새 Execution으로 전환한다.
53. suspend는 dependency Waiting과 구분되며 safe checkpoint/Continuation을 보존하고 resume는 idempotent한 새 Execution으로 수행한다.
54. interruption은 cooperative preemption/safe point를 기본으로 하며 외부 Provider에 Hard Real-Time 중단을 약속하지 않는다.
55. Control request가 Core Lease를 직접 생성·회수하지 않는다. Scheduler가 최종 admission/rebalance authority를 유지한다.
56. Prompt/Provider/Tool output이 Control authority를 획득할 수 없다.
57. Side Effect safety와 immutable history는 live steering 편의보다 우선한다.

## 3. Architecture Classification

| 범주 | 의미 | 제거성 | 예 |
|---|---|---|---|
| Core Domain | 제품 자체의 지속 의미 | 제거 불가 | Bot, Brain, Memory, Conversation, Thread, Task, Execution, Core Lease |
| Optional Capability | 교체 가능한 안정 semantic contract | 제거/교체 가능 | ModelGateway, SandboxExecutor, MemoryIndex |
| Provider | Capability 구현 | 교체/복수 등록 | DeepSeek/Native Harness 등 |
| Interface | Runtime 접근 표현 | Runtime과 독립 | CLI/TUI/Web/API |
| Plugin | 외부 패키지 확장 | 설치/업그레이드/제거 | third-party extension |

Conversation/Thread/Control Directive는 Provider가 아니라 Core Domain/Runtime 계약이다.

## 4. Canonical Owner

- Conversation/Thread: `DXB-DOM-027`
- Thread-aware Context: `DXB-DOM-021`
- Memory scope/promotion: `DXB-DOM-022`
- Task/Execution revision: `DXB-DOM-023`
- Scheduler/Core Lease: `DXB-DOM-024`
- Control Plane semantics: `DXB-DOM-026`
- Live Control/Preemption: `DXB-RUN-036`
- Provider Framework: `DXB-ARC-017`

같은 규칙을 다른 문서가 반복 정의할 때는 위 Canonical 문서의 의미를 참조한다.

## 5. 영속·복구 불변조건

- Command의 Event/Current State/idempotency/Outbox는 Unit of Work로 Commit한다.
- Waiting/Suspension/Directive의 durable meaning이 Runtime queue에만 존재하지 않는다.
- Side Effect Intent는 외부 변경 전에 영속한다.
- restart는 persisted Canonical State로 복구하며 Provider Session/live process를 Truth로 신뢰하지 않는다.
- Thread/Conversation history와 Memory scope는 각각의 owner/retention으로 복원한다.
- redirect가 commit된 뒤 old Execution yield와 new Execution 시작 사이 crash해도 idempotent recovery가 가능해야 한다.

## 6. 검증 기준

- DeepSeek/Provider Session 없이도 Core Domain/Conversation/Thread restore test가 통과한다.
- 여러 Interface Session이 같은 Main Conversation에 접속하고 Session 종료가 Bot을 종료하지 않는다.
- Thread A/B/C 병렬 실행이 local context/Memory/control을 격리한다.
- Work Queue 포화에서도 Control Channel starvation이 없다.
- redirect/suspend가 immutable Execution/Side Effect 불변조건을 깨지 않는다.
- Provider 제거/교체가 Conversation/Thread schema 의미 변경을 요구하지 않는다.
- naming/dependency/Acceptance/Risk 관계가 CI/문서 Review로 검증 가능하다.
