---
title: "DXBOT 최상위 기준선"
document_id: "DXB-BASE-000"
version: "0.7.0"
status: "Normative Baseline"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-SOURCE-000"]
---

# DXBOT 최상위 기준선

본 문서는 v0.6의 **Persistent Bot / Single Brain Semantics / Dynamic Core Lease / Persistent Main Conversation / Thread / Project / Channel / Scope-Aware Epistemic Memory / Durable Process / Live Control / Provider Independence / Authorization / Runtime Memory Safety**를 전부 유지하면서, 해당 Runtime을 안정된 **Headless Application Contract** 뒤에 노출하고 **CLI Reference Interface**를 첫 공식 운영 Interface로 완성한다.

v0.7의 핵심 문장은 다음과 같다.

> **Runtime 의미는 Backend가 소유하고, Application Contract는 그 의미를 안정적으로 노출하며, CLI는 그 계약을 소비할 뿐 재해석하지 않는다.**

## 0. v0.6 Normative Inheritance

v0.7은 `docs/plan/20260821-1544-v0-6-detailed-development-plan` 전체를 normative baseline으로 참조 상속한다.

- 동일 `document_id`의 v0.7 문서는 **명시적으로 변경한 의미만 supersede**한다.
- v0.7에서 반복하지 않은 v0.6/v0.5/v0.4 요구·failure semantic·race rule·resource rule·Acceptance/Risk/Open Question은 계속 유효하다.
- `생략 = deprecation`으로 해석하지 않는다.
- Backend Domain/Runtime/Provider/Memory/Security 의미는 Interface 개발을 이유로 축소하거나 재설계하지 않는다.
- v0.6 패키지의 검수 기록은 과거 evidence로 보존하며 v0.7은 Repository 규약에 따라 Structural/Consistency와 Cross-Layer Executability의 2회 독립 검수를 수행한다.

## 1. v0.7 Active Interface Scope Amendment

v0.7의 active implementation/release scope는 다음 두 계층으로 제한한다.

1. `DXB-IFC-040` — Headless Application Contract / Control Protocol
2. `DXB-IFC-041` — CLI Reference Interface

```text
Persistent DXBOT Runtime
        ↓
Application Layer
        ↓
Headless Application Contract
        ↓
Control Endpoint / Control Client
        ↓
      dxb CLI
```

### 1.1 TUI/Web explicit supersession

`DXB-IFC-042`과 `DXB-IFC-043`은 **v0.7 active implementation baseline에서 superseded**된다.

- 두 문서의 구현 milestone, release gate, acceptance requirement, interface-specific Risk/Open Question은 v0.7에 상속하지 않는다.
- 이는 Backend Domain semantic, Project/Channel/Memory/Task/Runtime 기능을 제거하는 결정이 아니다.
- Headless Application Contract를 제거하거나 CLI 전용 schema로 축소하는 결정이 아니다.
- 향후 Runtime과 CLI가 충분히 안정화된 뒤 새로운 Interface 계획을 별도 버전에서 도입하는 것을 금지하지 않는다.
- v0.7 구현은 아직 존재하지 않는 Interface를 위한 BFF, route/view schema, frontend state model, UI projection을 선행 구현하지 않는다.

따라서 v0.6의 `CLI/TUI/Web이 동일 schema를 사용한다` 또는 동등한 병렬 Interface 전제는 다음으로 supersede한다.

> **Headless Application Contract is the shared contract. The v0.7 Reference Consumer is CLI. No additional product Interface is part of the v0.7 implementation/release scope.**

## 2. Backend 비회귀 원칙

다음 v0.6 Foundation은 그대로 유지한다.

1. Bot은 Session과 독립된 Persistent Identity·Memory·State를 가진다.
2. 하나의 Bot은 하나의 logical Brain semantics를 유지한다.
3. Core는 Scheduler-owned Dynamic Lease이며 독립 Agent가 아니다.
4. Bot당 하나의 Persistent Main Conversation을 유지한다.
5. `Thread ≠ Task ≠ Execution ≠ Core Lease`다.
6. `Interface Session ≠ Provider Session ≠ Thread`다.
7. Project/Channel은 Collaboration/Knowledge/Resource boundary이며 Brain이 아니다.
8. Bot/Project/Channel/Thread Memory는 독립 Scope이고 History와 Memory는 구분된다.
9. running Execution의 Context Plan/Task revision/Provider binding은 immutable하다.
10. Common Provider Host/Stable Capability Contract/Conformance/Tier A-B 구조를 유지한다.
11. Durable Process는 cross-aggregate progress만 소유하며 Task/Memory/Directive 상태를 복제하지 않는다.
12. Common Authorization Decision Owner는 하나이며 Information Flow/Declassification과 ActionGrant 의미를 유지한다.
13. Collaboration Run은 bounded되고 explicit terminal reason을 가진다.
14. Runtime Memory는 process-wide envelope/reservation/headroom/pressure semantics로 bounded된다.
15. Bot-only path는 Project/Channel/Process 없이도 완전하게 동작한다.

## 3. v0.7 Application / CLI 원칙

16. Application Layer는 Backend use-case orchestration을 소유하되 Domain rule을 복제하지 않는다.
17. `DXB-IFC-040`은 public Command/Query/Subscription contract와 protocol/schema compatibility의 Canonical Owner다.
18. Command, Query, Subscription, Domain Event, internal Work Queue를 하나의 generic envelope로 뭉개지 않는다.
19. CLI는 Domain Store, Scheduler/Core Lease, Provider, Memory policy, Authorization policy를 직접 read/write/호출하지 않는다.
20. CLI convenience를 위해 Backend Domain API나 duplicate canonical state를 만들지 않는다.
21. Runtime Host process lifetime과 CLI/shell/session lifetime을 분리한다.
22. CLI 정상 종료/crash/SIGINT/control disconnect는 explicit Runtime command가 없는 한 Bot/Task/Process lifecycle을 변경하지 않는다.
23. mutation retry는 CommandId/IdempotencyKey/expected revision 또는 동등 stable contract로 duplicate semantic effect를 만들지 않는다.
24. command commit 후 response loss는 retry/reconcile로 기존 outcome을 회수할 수 있어야 한다.
25. Query는 bounded pagination/window를 기본으로 하며 full materialization을 요구하지 않는다.
26. Subscription은 stable event identity/cursor/watermark/gap detection/resume-resync와 bounded buffer/backpressure를 가진다.
27. protocol/schema/runtime/client/data-schema version을 동일 개념으로 취급하지 않으며 incompatible pair는 silent fallback하지 않는다.
28. CLI machine output은 human rendering과 분리하고 JSON/JSONL, stdout/stderr, stable exit semantics를 제공한다.
29. CLI `--all`, history/export/watch는 total dataset size에 비례한 heap aggregation을 기본 구현으로 사용하지 않는다.
30. CLI principal/credential/approval은 Common Security owner를 소비하며 local Role label/ambient shell authority로 권한을 생성하지 않는다.
31. Runtime/Provider/Projection 상태와 current/stale/degraded를 구분해 진단한다.

## 4. Canonical Ownership

| 의미 | Canonical Owner |
|---|---|
| Bot/Project/Channel/Thread/Task/Memory 의미 | 기존 v0.6 Domain Owner |
| Execution/Scheduling/Recovery/Resource | 기존 v0.6 Runtime Owner |
| Authorization/Information Flow | `DXB-RUN-032` |
| Durable Process | `DXB-RUN-038` |
| Application use-case orchestration | Application Layer |
| public Command/Query/Subscription contract | `DXB-IFC-040` |
| CLI parsing/rendering/interaction | `DXB-IFC-041` |
| Runtime Host lifecycle/composition | Runtime Host/Application composition |
| Control connection/session | Infrastructure/Control adapter |

## 5. Interface-neutral Runtime 계약 유지

TUI/Web active scope를 제거해도 다음 계약은 삭제하지 않는다.

- cursor pagination
- event stream
- reconnect/resume/resync
- request/response cumulative byte cap
- authorization/current revision
- idempotency
- version compatibility/negotiation
- streaming/backpressure
- headless control
- stale/degraded/watermark

## 6. Architecture Classification

| 범주 | v0.7 예 |
|---|---|
| Core Domain | Bot, Project, Channel, Conversation, Thread, Memory, Task, Execution, Core Lease |
| Runtime Common | Durable Process, Authorization Decision, Resource Governance, Live Control, Recovery |
| Application Contract | Command/Query/Subscription, public DTO/error/version/cursor semantics |
| Reference Interface | `dxb` CLI |
| Optional Capability | ModelGateway, SandboxExecutor, MemoryIndex |
| Provider | Capability 구현 |
| Plugin | 외부 확장 package |

## 7. 검증 기준

- v0.6 Backend Canonical Owner/Acceptance 의미 회귀 0.
- active Interface normative document는 `DXB-IFC-040/041`만 존재한다.
- `DXB-IFC-042/043`이 active dependency/release gate/milestone로 남지 않는다.
- CLI→Domain Store/Scheduler/Provider 직접 edge 0.
- CLI disconnect/SIGINT가 Runtime lifecycle mutation을 만들지 않는다.
- duplicate retry/lost response가 duplicate Canonical effect를 만들지 않는다.
- large output/watch가 Runtime Memory Safety를 우회하지 않는다.
- incompatible protocol/schema pair가 명시적으로 실패한다.
- 미래 Interface를 이유로 speculative frontend/BFF state를 구현하지 않는다.
