---
title: "DXBOT 최상위 기준선"
document_id: "DXB-BASE-000"
version: "0.6.0"
status: "Normative Baseline"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-SOURCE-000"]
---

# DXBOT 최상위 기준선

본 문서는 v0.5의 **Persistent Bot / Single Brain Semantics / Dynamic Core Lease / Persistent Main Conversation / Thread / Project / Channel / Scope-Aware Memory / Live Control / Provider Independence**를 유지하면서, 확률적 판단이 영속 상태·기억·행동으로 전환되는 경계를 강화한다.

v0.6의 핵심은 다음 네 축이다.

> **Durable Process Semantics + Epistemic/Secure Memory Lifecycle + Runtime Memory Safety + Bounded and Measurable Collaboration**

## 0. v0.5 Normative Inheritance

v0.6은 `docs/plan/20260821-1409-v0-5-detailed-development-plan` 전체를 normative baseline으로 참조 상속한다.

- 동일 `document_id`의 v0.6 문서는 **명시적으로 변경한 의미만 supersede**한다.
- v0.6에서 반복하지 않은 v0.5/v0.4 요구·failure semantic·race rule·resource rule·Acceptance/Risk/Open Question은 계속 유효하다.
- `생략 = deprecation`으로 해석하지 않는다.
- 기존 계약 제거에는 명시적 deprecation/ADR, 영향 분석, migration/rollback, Acceptance/Risk 갱신이 필요하다.
- v0.5와 v0.6이 충돌할 때만 v0.6의 명시적 변경이 우선하며, 나머지는 strict-superset으로 유지한다.
- v0.5 패키지의 3회 문서 검수 기록은 과거 evidence로 보존한다. v0.6 문서 패키지는 현재 Repository 규약과 v0.6 고도화 계획에 따라 **Structural/Consistency와 Cross-Layer Executability의 2회 독립 검수**를 수행하며, v0.5의 compatibility 검사항목은 두 Review에 흡수해 축소하지 않는다.

## 1. 제품 정의

> DXBOT은 지속 Identity·Memory·State를 가진 Bot들이 Session과 독립적으로 존재하고, 하나의 logical Brain semantics를 유지한 채 Dynamic Core Lease로 병렬 실행하며, Project와 Channel을 통해 영속 협업하는 Rust 기반 AI Bot Runtime이다.

Bot의 연속성은 Provider Session, Interface Session, 특정 모델, 특정 Core가 아니라 `Identity + Canonical Memory + Persistent State + Conversation/Thread State`로 판단한다.

## 2. v0.5 비회귀 원칙

1. Bot은 Session과 독립된 Persistent Identity·Memory·State를 가진다.
2. 하나의 Bot은 하나의 logical Brain semantics를 유지한다.
3. Core는 독립 Agent가 아니라 Scheduler가 Execution에 발급하는 일시적 Lease다.
4. Bot당 하나의 Persistent Main Conversation을 유지한다.
5. `Thread ≠ Task ≠ Execution ≠ Core Lease`를 유지한다.
6. `Interface Session ≠ Provider Session ≠ Thread`를 유지한다.
7. Project와 Channel은 Collaboration/Knowledge/Resource boundary이며 Brain이 아니다.
8. Channel Coordinator는 routing/admission/backpressure owner이며 reasoning owner가 아니다.
9. Bot/Project/Channel/Thread Memory는 독립 Scope다.
10. Shared Memory는 participating Bot Global Memory의 union/replica가 아니다.
11. Conversation History와 Memory는 별도 Canonical 의미다.
12. 자연어 Message와 Canonical Command/Directive는 동일 사건이 아니다.
13. running Execution의 Context Plan/Task revision/Provider binding은 immutable하다.
14. Core Lease authority는 Scheduler에 유지한다.
15. Provider Session 손실은 Bot/Conversation/Thread/Memory identity를 손상시키지 않는다.
16. Common Provider Host/Stable SPI/Conformance/Tier A-B 구조를 유지한다.
17. Project/Channel을 사용하지 않는 Bot-only 경로는 완전하게 지원한다.

## 3. Durable Process 원칙

18. 둘 이상의 Canonical Owner를 횡단하며 crash 후 진행 복원이 필요한 long-running flow는 하나의 Durable Process Canonical Owner를 가져야 한다.
19. Durable Process는 Task/Execution/Channel/Memory의 상태를 복제하지 않고 해당 Canonical 결과와 진행 상태의 reference만 소유한다.
20. Durable Process는 다른 Aggregate를 직접 mutate하지 않고 기존 Application/Command boundary를 통해 typed Command를 제출한다.
21. replay-safe transition은 committed state/outcome만으로 결정하며, replay 중 LLM/Tool/HTTP/외부 Side Effect를 다시 실행하지 않는다.
22. 비결정적 작업은 기존 Execution/Provider Host/Tool/Side Effect Ledger를 재사용한 durable Activity outcome boundary로 격리한다. Activity는 새 Brain/Core/Agent를 뜻하지 않는다.
23. transport delivery는 at-least-once일 수 있으며 idempotency/revision/fencing으로 semantic duplicate를 제거한다. DXBOT 전체에 추상적인 exactly-once execution을 약속하지 않는다.
24. Process cancel/timeout은 신규 step admission을 중단하지만 이미 실행 중인 child Task/Execution/Side Effect를 암묵 cancel하지 않는다. 필요한 중지는 기존 Task/Live Control Command를 사용한다.
25. terminal Process에 늦게 도착한 결과는 audit/reconciliation 대상으로 보존할 수 있으나 Process를 암묵 재개하지 않는다.

## 4. Epistemic Memory 원칙

26. Memory는 Scope뿐 아니라 지식의 종류와 주장 상태를 표현할 수 있어야 한다.
27. Observation/Evidence, Claim/Assertion, Hypothesis/Inference, Decision, Policy, Preference, Procedure의 권위 규칙을 동일하게 취급하지 않는다.
28. factual assertion은 Proposed/Unverified, Verified, Disputed, Superseded, Retracted, Stale/RevalidationRequired, Quarantined에 해당하는 의미를 구분할 수 있어야 한다.
29. 모델·Tool·외부 문서 결과는 Canonical Truth가 아니라 provenance와 trust를 가진 Proposal/Evidence다.
30. 모델이 생성한 citation/verification 주장만으로 `Verified`를 만들지 않는다.
31. Memory relation은 evidence/derived/contradiction/supersession/invalidation/decision dependency를 추적할 수 있어야 한다.
32. source Memory가 retract되면 dependent Memory를 blind delete하지 않고 stale/revalidation 대상으로 만들며 새 Context에서 Verified knowledge처럼 무조건 사용하지 않는다.
33. legacy v0.5 Memory는 migration만으로 Verified가 되지 않으며 ID/revision/content/provenance를 보존한다.

## 5. Information Flow / Authorization 원칙

34. `Read Authority + Write Authority ≠ Information Flow Authority`다.
35. Private/Sensitive source를 Channel/Project 같은 Shared Scope에 publish할 때 target information-flow policy와 필요한 declassification/approval을 별도로 검증한다.
36. Authorization 의미는 Common Authorization Decision Owner 하나가 소유하고 각 subsystem은 enforcement point로 동작한다.
37. Project/Channel Membership·Authority Binding·Permission Policy를 대체하는 새 권한 체계를 만들지 않는다.
38. approval/high-risk action은 필요한 경우 특정 Canonical Action, target scope, budget/use, expiry, policy revision에 묶인 Durable ActionGrant로 제한할 수 있다.
39. retry/replan/delegation/provider fallback/resume/duplicate delivery가 ActionGrant의 허용 semantic execution budget을 우회하지 못한다.
40. 외부 instruction-like content, Prompt, Model/Provider/Tool output은 authority object가 아니다.

## 6. Collaboration 원칙

41. Channel participant routing의 Canonical Runtime 의미는 `DXB-RUN-037`이 유지한다.
42. 메시지 단위 cap뿐 아니라 하나의 causal Collaboration Run 전체가 activation/delegation/round-hop/token-cost/deadline/concurrency budget으로 bounded되어야 한다.
43. Collaboration Run은 Succeeded/Partial/NoProgress/LoopDetected/BudgetExhausted/TimedOut/Cancelled/AuthorizationRevoked/Failed·RecoveryRequired에 해당하는 명시적 terminal reason을 가져야 한다.
44. progress는 LLM self-report 문자열이 아니라 신규 evidence, pending task 변화, acceptance coverage, blocker 변화, duplicate/same-state 신호 같은 Runtime 관측으로 평가한다.
45. 기본 실행은 Single-Bot-first다. Multi-Bot은 병렬 decomposition, 독립 검증, 전문 Context, 사용자 요청, 측정된 cost/latency 대비 기대 효용이 있을 때 승격한다.
46. Reviewer Role 또는 동일 모델 Bot의 다수결만으로 Memory Claim을 Verified로 만들지 않는다.

## 7. Runtime Memory Safety 원칙

47. queue/cache/context의 local cap만으로 process-wide memory safety를 주장하지 않는다.
48. Runtime Memory의 Canonical Resource Owner는 `DXB-RUN-031`이며 Scheduler/Context Runtime/Provider Host/Channel Orchestration은 enforcement point다.
49. Deployment/process Runtime Memory는 hierarchical byte envelope와 **safety/control/recovery headroom**을 가진다.
50. Core Lease 수는 Memory ceiling이 아니다. memory-heavy work는 가능한 범위에서 estimate/reserve 후 admission한다.
51. reservation은 runtime-only permit이며 Task/Core identity나 Canonical persistent state가 아니다.
52. Waiting/Suspended/외부 이벤트 대기 상태는 large transient Context/Provider buffer reservation을 계속 보유하지 않고 durable reference/checkpoint만 남긴 뒤 resume 시 재-admission한다.
53. Provider/Tool/Artifact/serialization stream은 cumulative bytes, in-flight bytes, downstream backpressure를 제한한다.
54. large immutable payload/history/prefix는 shared reference 또는 durable blob/artifact reference를 우선하며 copy amplification을 제한한다.
55. encoded size뿐 아니라 decoded/decompressed size, element count, nested expansion을 allocation 전에 제한한다.
56. Normal/Constrained/Critical/Emergency에 해당하는 memory pressure semantic과 hysteresis를 둔다. 정확한 threshold는 Policy/ADR/benchmark가 소유한다.
57. Critical/Emergency에서도 일반 work가 safety/control/recovery headroom을 소진하지 못한다.
58. Rust allocation failure/OOM을 catch 기반 정상 recovery API로 가정하지 않는다. OOM prevention은 admission/reservation/pressure/bounded streaming/spill/host limit 조합으로 달성한다.
59. `Arc` cycle, orphan async task, subscriber/stream registration, cache pin, reservation/permit의 retention leak을 lifecycle/drop/reconnect/soak에서 검증한다.
60. RSS 하나로 leak을 판정하지 않고 accounted in-flight bytes, owner retained bytes/object count, cache bytes, allocator stats(가능 시), process/child-process resident memory를 구분한다.
61. restart/rebuild에서 모든 Bot/Project/Channel/Memory/index를 동시에 eager hot-load하지 않고 staged/lazy/bounded recovery를 사용한다.
62. cgroup/PSI/native host pressure integration은 optional Infrastructure Adapter이며 Core Domain semantic을 변경하지 않는다.

## 8. Architecture Classification

| 범주 | v0.6 예 |
|---|---|
| Core Domain | Bot, Project, Channel, Conversation, Thread, Memory Scope/Epistemic Lifecycle, Task, Execution, Core Lease |
| Runtime Common | Durable Process, Authorization Decision, Resource Governance, Channel Orchestration, Live Control, Recovery |
| Optional Capability | ModelGateway, SandboxExecutor, MemoryIndex |
| Provider | Capability 구현 |
| Interface | CLI/TUI/Web/API |
| Plugin | 외부 확장 package |

## 9. 검증 기준

- v0.5/v0.4의 미명시 세부가 생략만으로 deprecate되지 않는다.
- Durable Process가 Brain/Task/Execution/Channel Coordinator의 owner 역할을 복제하지 않는다.
- replay 과정에서 이미 완료된 Provider/Tool Activity가 재실행되지 않는다.
- unverified/retracted/quarantined Memory가 Verified knowledge처럼 사용되지 않는다.
- Private→Shared publication은 별도 information-flow/declassification 검사를 통과한다.
- ActionGrant가 retry/replan/restart에서 semantic replay를 제한한다.
- Collaboration Run이 terminal reason 없이 무한 지속되지 않는다.
- process-wide Runtime Memory admission을 Scheduler/Core 수 또는 local cap이 우회하지 않는다.
- overload/restart 이후 reservation/task/subscriber/permit retained trend가 bounded하다.
- Bot-only v0.5 path, Provider Framework, Side Effect, Live Control, Scheduler ownership이 회귀하지 않는다.
