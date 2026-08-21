---
title: "용어집과 도메인 모델"
document_id: "DXB-GOV-002"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000"]
---

# 용어집과 도메인 모델

## 1. 목적

v0.5의 Bot/Project/Channel/Thread/Memory/Task/Core 용어를 그대로 유지하면서 Durable Process, Epistemic Memory, Information Flow, Authorization, Collaboration Run, Runtime Memory Safety 용어를 Canonical Owner와 함께 고정한다.

## 2. v0.6 핵심 용어

| 용어 | 정의 | 수명 | Canonical Owner |
|---|---|---|---|
| Durable Process | 둘 이상의 Canonical Owner를 횡단하는 장기 흐름의 진행·step·outcome reference·reconciliation을 crash 후 복원하는 Runtime 의미 | workflow 수명 | `DXB-RUN-038` |
| Process Definition Version | replay 시 어떤 deterministic process definition을 사용했는지 식별하는 버전 | Process 수명 | `DXB-RUN-038` |
| Activity | replay 중 다시 실행하면 안 되는 비결정적 작업의 durable outcome boundary. 새 Agent/Core/범용 Aggregate를 의미하지 않음 | 실행/결과 수명 | 기존 Execution/Provider/Tool/Side Effect owner + Process reference |
| Epistemic Kind | Memory가 Observation/Evidence, Claim, Inference, Decision, Policy, Preference, Procedure 중 어떤 지식 의미인지 나타내는 분류 | Memory revision 수명 | `DXB-DOM-022` |
| Assertion State | factual assertion의 Proposed/Unverified/Verified/Disputed/Superseded/Retracted/Stale·RevalidationRequired/Quarantined 상태 의미 | Memory revision/state 수명 | `DXB-DOM-022` |
| Evidence Relation | `evidence-for`, `derived-from`, `contradicts`, `supersedes`, `invalidates`, `decision-based-on`에 해당하는 Memory 간 의미 relation | relation 수명 | `DXB-DOM-022` |
| Retraction | source assertion을 더 이상 유효한 근거로 사용하지 않음을 Canonical하게 기록하는 상태 변화 | 감사 보존 | `DXB-DOM-022` |
| Revalidation | dependency 변경/retraction 이후 dependent Memory의 신뢰 상태를 다시 평가하는 절차 | 작업/결과 수명 | Memory owner + Task/Process orchestration |
| Information Label | origin/source class, trust, confidentiality/sharing class, scope provenance, sensitivity를 표현하는 metadata | item/revision 수명 | `DXB-RUN-032` policy semantic, 각 record에 metadata |
| Declassification | 더 제한적인 source 정보를 Shared/낮은 제한 target으로 publish할 수 있도록 명시적으로 허용하는 policy/approval 결정 | 승인/audit 수명 | `DXB-RUN-032` |
| Authorization Decision | Principal/Resource/Action/PolicyRevision을 평가한 Allow/Deny/ApprovalRequired 결과 | 결정/audit 수명 | `DXB-RUN-032` |
| Security Domain | P0에서 local/workspace owner를 기준으로 principal의 최상위 security boundary를 표현하는 의미 | deployment/workspace 수명 | `DXB-RUN-032` |
| Action Grant | 이미 허용된 권한/승인을 특정 Canonical Action, target, budget/use, expiry에 묶는 bounded authorization artifact | grant 수명 | `DXB-RUN-032` |
| Collaboration Run / Interaction Cycle | 하나의 root intent에서 파생된 Channel 협업의 causal boundary | run 수명 | routing/termination=`DXB-RUN-037`, cross-aggregate progress=`DXB-RUN-038` |
| Terminal Reason | Collaboration/Process가 종료된 원인을 명시하는 bounded terminal semantic | terminal 이후 감사 보존 | Process/Channel Runtime owner |
| Runtime Memory Envelope | deployment/process가 일반 work에 사용할 수 있는 hierarchical Runtime Memory byte ceiling | runtime/config 수명 | `DXB-RUN-031` |
| Memory Reservation | memory-heavy work admission 전에 확보하는 runtime-only capacity permit | work 수명 | `DXB-RUN-031` |
| Safety Headroom | cancel/reconcile/recovery/shutdown 등 제어 경로를 위해 일반 work와 분리해 보존하는 Runtime Memory 여유 | runtime 수명 | `DXB-RUN-031` |
| Memory Pressure State | Normal/Constrained/Critical/Emergency에 해당하는 감압 상태 의미 | runtime | `DXB-RUN-031` |
| Accounted Bytes | Resource Governance가 논리적으로 예약/사용 중이라고 추적하는 in-flight byte | runtime | `DXB-RUN-031` |
| Retained Bytes | 특정 owner/object/cache가 quiescence 이후에도 보유하는 논리/측정 byte | runtime/측정 | `DXB-ENG-051` 측정 |
| Resident RSS | OS가 관측하는 process resident memory. leak과 동일 의미가 아님 | runtime/측정 | `DXB-ENG-051` 측정 |
| Spill | 큰 transient payload를 bounded temporary storage/Artifact reference로 전환하는 실행 전략 | transient | Resource/Artifact owner |

## 3. 기존 용어 유지

v0.5의 Bot, Brain, Project, Project Membership, Channel, Channel Membership, Channel Role, Channel Authority, Channel Conversation, Main Conversation, Thread, Conversation Parent Ref, Memory Scope Reference, Shared Scope Memory, Bot-global Memory, Thread Memory, Working Context, Memory Proposal/Promotion, Context Scope Set, Channel Coordinator/Presence, SupervisorRef, Control Directive, Provider Session, Interface Session, Core Lease와 v0.4의 Goal/Task/Execution/Continuation/Side Effect/Artifact/Capability/Provider/Plugin/Policy 정의는 그대로 유지한다.

## 4. 구분 규칙

```text
Durable Process ≠ Brain ≠ Agent ≠ Task ≠ Execution
Activity ≠ Agent ≠ Core Lease
Authorization Decision ≠ Role ≠ Membership ≠ ActionGrant
Read Authority + Write Authority ≠ Information Flow Authority
Epistemic Kind ≠ Memory Scope
Assertion State ≠ Retention State
Knowledge Memory ≠ Runtime Memory
Core Lease count ≠ Runtime Memory capacity
Memory Reservation ≠ Canonical Task/Core state
Collaboration Run ≠ Channel ≠ Conversation ≠ Thread
```

## 5. Memory 시간 의미

하나의 timestamp가 모든 의미를 표현하지 않는다.

- `observed_at`: 언제 관찰했는가
- `recorded_at`: 언제 DXBOT에 기록했는가
- `valid_from / valid_until`: factual assertion이 유효한 기간
- `effective_from`: Decision/Policy가 적용되는 시점

정확한 필드 구성과 storage enum은 ADR 대상이다.

## 6. 검증 기준

- Process/Activity를 새 Brain/Core/Agent로 표현하지 않는다.
- ActionGrant를 Membership/Authority의 대체 권한 체계로 표현하지 않는다.
- Knowledge Memory와 Runtime Memory를 동일 `Memory` subsystem으로 취급하지 않는다.
- Role/Authority/Authorization/Information Flow가 문서/API에서 구분된다.
- exact enum/physical schema/numeric threshold는 ADR/benchmark 전에 과고정하지 않는다.
