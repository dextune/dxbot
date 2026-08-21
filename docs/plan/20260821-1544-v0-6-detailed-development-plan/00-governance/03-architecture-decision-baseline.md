---
title: "초기 아키텍처 결정 기준선"
document_id: "DXB-GOV-003"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-GOV-002"]
---

# 초기 아키텍처 결정 기준선

## 1. 목적

ADR-0001~0059의 v0.5 의미를 유지하고, v0.6의 Durable Process, Epistemic Memory, Information Flow/Authorization, Collaboration Termination, Runtime Memory Safety에 필요한 기본 결정을 추가한다. 정확한 enum·DB representation·numeric threshold·allocator 선택은 증거 없이 고정하지 않는다.

## 2. v0.5 결정 유지

Persistent Main Conversation, Thread/Session 분리, History≠Memory, Scope-Aware Memory, source-preserving promotion, bounded Context, Work/Control queue 분리, immutable redirect, durable suspend/resume, cooperative preemption, Scheduler Core ownership, Provider Session non-ownership, Project/Channel Brain 금지, Role/Authority 분리, final current authorization, bounded Channel routing 의미를 모두 유지한다.

## 3. v0.6 신규 결정 후보

| ADR | 기본 결정 | 재검토 트리거 |
|---|---|---|
| ADR-0060 | 둘 이상의 Canonical Owner를 횡단하고 crash-resume이 필요한 흐름만 Durable Process를 사용하며 Process는 child state를 복제하지 않음 | 단일 Aggregate transaction으로 충분함이 입증 |
| ADR-0061 | replay-safe transition과 non-deterministic Activity를 분리하고 DXBOT 전체에 global exactly-once execution을 약속하지 않음 | 비협상 |
| ADR-0062 | Memory에 Epistemic Kind와 Assertion State 의미를 추가하고 factual claim과 Decision/Policy/Preference/Procedure authority를 분리 | 비협상 |
| ADR-0063 | source correction/retraction은 dependency relation으로 dependent Memory를 stale/revalidation 처리하며 blind cascade delete 금지 | 비협상 |
| ADR-0064 | access authorization과 information-flow/declassification을 분리 | 비협상 |
| ADR-0065 | Common Authorization Decision Owner를 `DXB-RUN-032` 하나로 유지하고 각 subsystem은 PEP로 동작 | 비협상 |
| ADR-0066 | approval/high-risk action은 필요 시 bounded ActionGrant로 Canonical Action digest/target/budget/use/expiry에 결박 | 더 단순한 기존 Side Effect/approval semantic만으로 동일 안전성 입증 |
| ADR-0067 | Channel 협업은 causal Collaboration Run 단위 total budget과 explicit terminal reason을 가짐 | 비협상 |
| ADR-0068 | collaboration admission은 Single-Bot-first deterministic policy로 시작 | workload evidence가 다른 default를 지지 |
| ADR-0069 | Runtime Memory는 process-wide hierarchical byte envelope와 reserve-before-admit를 사용하며 Core count를 memory ceiling으로 사용하지 않음 | 비협상 |
| ADR-0070 | safety/control/recovery headroom과 Normal/Constrained/Critical/Emergency pressure semantic, hysteresis를 둠 | threshold/algorithm은 benchmark로 조정 가능 |
| ADR-0071 | large payload/Provider stream은 cumulative+in-flight byte cap, backpressure, bounded spill/reference를 사용 | 비협상 |
| ADR-0072 | Rust allocator OOM을 normal recoverable control event로 가정하지 않고 proactive admission/pressure/fail-stop으로 다룸 | 비협상 |
| ADR-0073 | cgroup/PSI/native host pressure 신호는 optional Infrastructure Adapter로 격리하고 Core Runtime semantic은 동일 | 플랫폼 요구 변화 |

## 4. ADR로 반드시 닫을 항목

- Durable Process lifecycle/state representation과 Process Definition Version
- Activity outcome/reference와 existing Execution/Provider/Tool/Side Effect Ledger 재사용 경계
- Epistemic Kind와 Assertion State exact enum/transition
- temporal metadata physical representation
- evidence/dependency relation representation 및 bounded traversal
- Information Label taxonomy와 declassification/approval flow
- Authorization Decision input/output schema와 audit retention
- ActionGrant digest, remaining use/budget, revocation/attenuation model
- Collaboration Run progress/stall detector와 terminal enum
- Single→Multi admission policy
- Runtime Memory envelope source와 reservation granularity/estimation
- pressure threshold/hysteresis와 safety headroom policy
- large payload spill threshold/temp-storage quota/cleanup
- allocator telemetry adapter 여부
- cgroup/PSI/native host pressure adapter
- leak/retained RSS release gate

## 5. ADR 증거

각 ADR은 Context/Decision/Alternatives, Canonical Owner, invariants, compatibility/migration/removal, security/information-flow, resource/memory/cache, concurrency/race, recovery, API/schema, Provider independence, deterministic fixture, benchmark/fault evidence, rollout/rollback을 포함한다.

## 6. Dependency 원칙

Governance/Architecture가 후행 Runtime 문서에 `depends_on`하여 cycle을 만들지 않는다. Domain의 stable reference가 Runtime owner를 가리킬 수 있어도 문서 dependency는 계층 방향을 유지한다. 신규 crate는 reuse/compile/test 경계가 입증되기 전 만들지 않는다.

## 7. 검증 기준

- ADR-0060~0073이 Acceptance/Risk/OQ 중 하나 이상에 연결된다.
- physical representation 미결정을 이유로 Canonical semantic을 구현팀이 임의 결정하지 않는다.
- v0.5 ADR 의미를 v0.6 convenience implementation이 약화하지 않는다.
- Runtime Memory/Authorization/Durable Process가 기존 Scheduler/Task/Memory owner를 복제하지 않는다.
