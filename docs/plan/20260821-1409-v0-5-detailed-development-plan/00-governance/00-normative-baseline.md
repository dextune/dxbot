---
title: "DXBOT 최상위 기준선"
document_id: "DXB-BASE-000"
version: "0.5.0"
status: "Normative Baseline"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-SOURCE-000"]
---

# DXBOT 최상위 기준선

본 문서는 v0.4의 Persistent Bot/Main Conversation/Thread/Live Control/Common Provider Framework를 모두 유지하면서 v0.5의 **Project · Channel · Scope-Aware Memory · Persistent Multi-Bot Collaboration**을 비협상 기준으로 추가한다. 아래 원칙은 ADR, Migration, Security, Acceptance 영향 검토 없이 완화할 수 없다.

## 0. v0.4 Normative Inheritance

v0.5는 `docs/plan/20260821-1200-v0-4-detailed-development-plan` 전체를 **normative baseline으로 참조 상속**한다.

- v0.5 문서가 v0.4 문서의 상세 문장을 반복하지 않았다는 이유만으로 해당 계약이 폐기·완화·비규범화되지 않는다.
- 동일 `document_id`의 v0.5 문서는 **명시적으로 변경한 의미만 supersede**하며 나머지 v0.4 요구·failure semantic·race rule·resource rule·Acceptance/Risk/Open Question은 계속 유효하다.
- `생략 = deprecation`으로 해석하는 것을 금지한다. 기존 계약 제거에는 명시적 deprecation/ADR, 영향 분석, migration/rollback, Acceptance/Risk 갱신이 필요하다.
- v0.4와 v0.5가 충돌하는 경우에만 v0.5의 명시적 변경 사항이 우선한다. 충돌하지 않는 세부는 v0.4 계약을 유지한다.
- 이 규칙은 문서 중복 복사를 줄이면서 strict-superset 호환성을 보장하기 위한 versioned SSOT 규칙이다.

## 1. 제품 정의

> DXBOT은 지속 Identity·Memory·State를 가진 Bot들이 Session과 독립적으로 존재하고, 하나의 logical Brain semantics를 유지한 채 Dynamic Core Lease로 병렬 실행하며, Project와 Channel을 통해 다른 Bot과 영속 협업할 수 있는 Rust 기반 AI Bot Runtime이다.

Bot의 연속성은 Provider Session, Interface Session, 특정 모델, 특정 Core가 아니라 `Identity + Canonical Memory + Persistent State + Conversation/Thread State`로 판단한다.

## 2. v0.4 보존 원칙

1. 하나의 Persistent Bot은 하나의 지속 Identity를 가진다.
2. 하나의 Bot은 하나의 logical Brain semantics를 가진다.
3. Core는 Identity/Memory의 owner가 아니며 Scheduler가 발급한 Lease로 Execution을 수행한다.
4. Bot당 하나의 Persistent Main Conversation을 유지한다.
5. `Thread ≠ Task ≠ Execution ≠ Core Lease`를 유지한다.
6. `Interface Session ≠ Provider Session ≠ Thread`를 유지한다.
7. Conversation History와 Memory는 서로 다른 Canonical 의미다.
8. 전체 transcript를 매 Provider Context로 사용하지 않는다.
9. Working Context는 정책 없이 Canonical Memory가 되지 않는다.
10. running Execution의 Context Plan, Task revision, Provider binding을 mutation하지 않는다.
11. 자연어 Message와 Canonical Control Directive를 동일 사건으로 취급하지 않는다.
12. Core Lease의 authoritative owner는 Scheduler다.
13. Provider Session 손실은 Bot/Conversation/Thread/Memory identity를 손상시키지 않는다.
14. Provider/Plugin/UI/Cache/Projection이 Core Domain Canonical identity를 소유하지 않는다.
15. 기존 Stable Capability/Provider Host/Conformance/Tier A-B 계약을 유지한다.

## 3. Project / Channel 원칙

16. Project는 Bot/Brain/Channel/Thread/Session이 아닌 영속 Collaboration/Knowledge/Resource Boundary다.
17. Project 자체는 Brain을 갖지 않으며 판단이나 LLM 실행을 수행하지 않는다.
18. Channel은 Project 안의 영속 Multi-Bot Collaboration Space다.
19. Channel 자체는 Brain/Core/Task/Thread가 아니다.
20. Channel의 Canonical participant는 Persistent Bot이며 CoreLeaseId를 Membership Canonical State에 저장하지 않는다.
21. Project Membership과 Channel Membership은 서로 다른 grant이며 Channel grant는 Project permission ceiling을 초과할 수 없다.
22. Project 사용 여부는 기존 Bot/Main Conversation 사용 경로의 필수 조건이 아니다.
23. Channel Role은 협업 의미이고 Channel Authority는 Runtime 권한이다. Role 문자열만으로 delegate/redirect/suspend 등의 권한을 얻지 못한다.
24. Channel Role은 Bot Global Identity를 변경하지 않는다.
25. Channel event 실행도 `Target Bot → Brain → Scheduler → Core Lease` 경로를 따른다.
26. Channel/Manager 전용 Core를 영구 고정하지 않는다.
27. Channel Coordinator는 routing/admission/backpressure를 담당하며 LLM reasoning/Brain 역할을 수행하지 않는다.

## 4. Scope-Aware Memory 원칙

28. Canonical Memory Scope는 `Bot | Project | Channel | Thread`의 독립 Scope로 일반화한다.
29. Execution Working Context는 Runtime-only이며 Canonical Memory Scope가 아니다.
30. `Bot Memory ≠ Project Memory ≠ Channel Memory ≠ Thread Memory ≠ Working Context`다.
31. Shared Scope Memory는 참여 Bot Global Memory의 복제본이 아니다.
32. Scope는 단순 부모→자식 inheritance namespace가 아니다. Project Memory가 자동 Bot Memory가 되지 않는다.
33. Conversation Message/Execution Result/Artifact는 MemoryCandidate가 될 수 있지만 그 자체가 Memory Commit은 아니다.
34. Canonical write는 `Candidate → Proposal → Scope Classification → Authorization → Normalize → Dedup → Conflict → Verification/Trust → Retention → Commit`을 거친다.
35. Provider/Core/Channel Runtime/Thread Runtime이 Canonical Memory Store를 직접 mutate하지 않는다.
36. Scope 간 promotion/publication/internalization은 source Memory scope의 in-place mutation이 아니라 source revision/provenance를 보존한 새 Canonical revision/reference 관계다.
37. Bot A와 Bot B의 Canonical Memory를 직접 합치거나 공유하지 않는다. 공동 지식은 독립 Project/Channel Scope가 소유한다.
38. Recall은 candidate/index 결과 뒤에도 **current Canonical authorization filter**를 최종 적용한다.
39. semantic/vector index, summary, cache는 Derived이며 permission authority가 아니다.
40. Membership revoke 후 신규 Context/Memory/history/artifact/control 접근은 current grant/generation 기준으로 차단한다.
41. 이미 생성된 immutable Execution Context를 retroactive mutation하지 않으며 high-risk Side Effect는 실행 직전 current authorization을 재평가할 수 있다.
42. Memory conflict는 단순 `Thread > Channel > Project > Bot` 우선순위로 결정하지 않고 Class/Scope/Authority/Provenance/Verification/Validity/Freshness/Relevance/Policy를 사용한다.

## 5. Conversation / Thread 일반화

43. Bot Main Conversation 모델은 유지한다.
44. Channel은 자체 Channel Conversation을 가질 수 있으며 Thread의 Conversation Parent는 Bot Main Conversation 또는 Channel Conversation이 될 수 있다.
45. Thread identity는 특정 Bot에 종속된 `BotId + ThreadId` pair로만 정의하지 않는다.
46. Thread가 Memory Scope owner로 사용될 때 Thread canonical identity로 충분히 식별 가능해야 한다.
47. Main Conversation을 Channel로, Channel을 Thread 0으로 단순화하지 않는다.
48. Channel Message가 참여 Bot Global Memory로 자동 복제되지 않는다.

## 6. Collaboration / Control 원칙

49. Manager/Supervisor Bot은 다른 Bot의 Brain/Core/Canonical Memory를 직접 조작하지 않는다.
50. Bot 간 위임은 기존 durable Bot Network/Task Delegation/Live Control semantic을 재사용한다.
51. Task의 Channel Role과 authoritative SupervisorRef는 별도 의미다.
52. P0에서는 Task당 authoritative Supervisor 1명을 기본으로 하며 변경 가능 여부는 ADR로 결정한다.
53. Channel Message의 자연어 지시가 Runtime Command committed를 의미하지 않는다. Intent Interpretation → Authorization → Canonical Command/Directive를 거친다.
54. Membership/Role/Authority stale revision으로 control mutation을 commit하지 않는다.
55. Channel archive가 running Task를 암묵 cancel하지 않는다.

## 7. Resource / Recovery 원칙

56. Channel inbound events, participant fan-out, response concurrency, Bot Network delegation, history/memory retrieval, Provider stream, event subscriber는 모두 bounded다.
57. Project/Channel/Memory scope 증가를 이유로 전체 state를 Runtime에 상주시켜서는 안 된다.
58. 모든 cache는 canonical source, scope key, revision/generation, max entries+bytes, eviction/stale policy, permission constraint, rebuild path, metrics를 가진다.
59. immutable Project/Channel context prefix는 digest/revision 기반 shared reference를 사용하고 Core별 deep copy를 피한다.
60. restart 후 Bot, Project, Channel, Membership/Role/Authority, Conversation/Thread, scoped Memory, Task/Execution/Continuation/Directive를 Canonical State에서 복구한다.
61. Provider Session/runtime presence 손실이 Project/Channel Membership을 소실시키지 않는다.
62. v0.4→v0.5 migration은 기존 Bot/Main Conversation/Thread/Memory identity/revision/provenance를 보존하며 임의 Project/Channel을 생성하지 않는다.

## 8. Architecture Classification

| 범주 | v0.5 예 |
|---|---|
| Core Domain | Bot, Project, Channel, Conversation, Thread, Memory Scope, Task, Execution, Core Lease |
| Runtime Common | Channel Orchestration, Scheduler, Live Control, Recovery, Permission/Resource |
| Optional Capability | ModelGateway, SandboxExecutor, MemoryIndex |
| Provider | Capability 구현 |
| Interface | CLI/TUI/Web/API |
| Plugin | 외부 확장 package |

## 9. 검증 기준

- v0.4 baseline의 미명시 세부가 생략만으로 deprecate되지 않는다.
- Project/Channel이 Brain을 소유하지 않는다.
- Bot-only v0.4 fixture가 Project 없이 동일하게 동작한다.
- Shared Memory scope가 Bot Global Memory와 물리·논리적으로 분리된다.
- revoke 후 stale cache/index candidate가 final authorization에서 제거된다.
- Channel 참여자 수 증가가 무제한 Execution fan-out으로 이어지지 않는다.
- 동일 Manager Bot의 연속 발언이 서로 다른 Core Lease에서 실행되어도 같은 Bot Identity로 관측된다.
- migration 후 기존 ThreadId/MemoryId/revision/provenance가 유지된다.
