---
title: "전체 시스템 아키텍처"
document_id: "DXB-ARC-010"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-GOV-002", "DXB-GOV-003"]
---

# 전체 시스템 아키텍처

## 1. 목적

v0.5의 `Persistent Bot → Main Conversation/Channel Conversation → Thread → Task → Execution → Core Lease`와 Project/Channel/Scope-Aware Memory를 보존하면서, cross-aggregate durable flow와 epistemic/security validation, process-wide Runtime Memory admission을 Common Runtime에 추가한다.

## 2. v0.6 목표 구조

```text
Persistent Bot / Project / Channel / Thread / Task
            ↓
    Canonical Command Boundary
            ↓
 Durable Process Orchestration? ────────────────┐
            ↓                                   │
Task / Delegation / Activity / Directive        │ committed refs only
            ↓                                   │
 Resource Memory Admission / Reservation        │
            ↓                                   │
 Scheduler / Dynamic Core Lease                 │
            ↓                                   │
 Provider Host / Tool / External Effect         │
            ↓                                   │
 Result / Evidence / Memory Proposal            │
            ↓                                   │
Authorization + Information Flow                │
            ↓                                   │
Epistemic Validation / Conflict / Retraction    │
            ↓                                   │
       Canonical State Commit ←──────────────────┘
```

Durable Process는 항상 존재하는 wrapper가 아니다. 둘 이상의 Canonical Owner를 횡단하며 crash 후 진행 복원이 필요한 흐름에만 사용한다.

## 3. 책임 경계

| 의미 | Owner | 금지 |
|---|---|---|
| Reasoning | Persistent Bot Brain | Process/Channel/Project가 reasoning owner가 됨 |
| Cross-Aggregate Progress | `DXB-RUN-038` Durable Process | child Task/Memory/Directive state 복제·직접 mutation |
| Task State | `DXB-DOM-023` | Process가 Task lifecycle 소유 |
| Execution Attempt | Execution owner | replay engine이 Provider를 직접 재호출 |
| Core Lease | Scheduler | Process/Coordinator가 Lease mint/revoke |
| Runtime Memory Budget | `DXB-RUN-031` | Scheduler가 별도 memory truth 보유 |
| Authorization/Info Flow | `DXB-RUN-032` | 서비스별 policy semantic 재구현 |
| Knowledge Meaning | `DXB-DOM-022` | Provider/Process가 Verified truth 직접 생성 |
| Channel Routing | `DXB-RUN-037` | Coordinator가 Brain/Task/Memory owner가 됨 |
| Provider Call | Provider Host | Provider Session이 Domain identity 소유 |

## 4. Normal Bot Path

```text
Interface
→ Bot Main Conversation / Thread
→ Brain Context Plan
→ Task
→ Runtime Memory admission if needed
→ Scheduler/Core Lease
→ Provider Host
→ Result / MemoryProposal
→ validation / Canonical commit
```

Project/Channel/Durable Process가 없어도 이 경로는 완전하게 동작한다.

## 5. Durable Cross-Aggregate Path

예시:

```text
Manager Task
→ Process step commits delegation command ref
→ Researcher Task / Activity outcome
→ Process observes committed outcome ref
→ Reviewer Task / validation
→ Memory Proposal
→ information-flow + epistemic validation
→ Project Memory commit
→ Process terminal reason
```

Process transition은 committed Event/State/Activity outcome만 읽는다. LLM/Tool/HTTP/외부 Side Effect는 replay-safe transition 안에서 직접 실행하지 않는다.

## 6. Scope-Aware Context / Epistemic Path

```text
Authorized Scope Set
→ candidate retrieval
→ canonical record fetch
→ current authorization
→ information label/trust/confidentiality filter
→ assertion state / evidence / conflict evaluation
→ deterministic selection
→ token + byte budget
→ immutable Context Plan
→ provider-neutral bounded render
```

Retracted/Quarantined/Stale Memory는 policy 없이 Verified knowledge처럼 선택하지 않는다. Decision/Policy/Preference/Procedure와 factual Claim은 authority 규칙을 분리한다.

## 7. Runtime Memory Admission Path

```text
Work Candidate
→ bounded memory class/byte estimate
→ DXB-RUN-031 hierarchical reserve
→ Scheduler / Context / Provider admission
→ actual/incremental byte accounting
→ ownership transfer or shared reference
→ terminal/cancel/drop release
```

Runtime Memory ceiling은 Core 수와 별도다. Waiting/Suspended work는 large transient reservation을 release하고 resume 시 다시 admission한다.

## 8. Pressure / Large Payload

- Normal/Constrained/Critical/Emergency state는 Resource Governance가 소유한다.
- Provider output은 cumulative+in-flight byte cap과 downstream backpressure를 가진다.
- 큰 Artifact/Tool output은 bounded streaming/spooling 후 reference로 전환할 수 있다.
- 일반 work가 safety/control/recovery headroom을 소비하지 못한다.
- OOM 발생을 정상 제어 신호로 기다리지 않는다.

## 9. Persistence / Recovery

새 Durable 대상:
- Process identity/definition version/progress/step/outcome refs/reconciliation/terminal reason
- ActionGrant issuance/consumption/revocation
- Memory epistemic/temporal metadata와 evidence/dependency/revalidation/quarantine relation

Runtime-only:
- MemoryReservation/permit
- pressure probe sample
- active Core handle
- Provider/Interface Session
- Presence/cache/index

restart 시 active memory reservation을 durable truth로 복원하지 않고 Canonical Task/Execution/Process 상태에서 재-admission/reconcile한다. heavy history/index/Provider Context는 lazy/bounded rebuild한다.

## 10. 검증 기준

- Project/Channel/Process→Brain 직접 소유 edge가 없다.
- replay path에 Provider/Tool invocation edge가 없다.
- Task/Execution/Scheduler/Memory owner가 Process에 복제되지 않는다.
- Private→Shared publication이 Authorization만으로 bypass되지 않는다.
- Scheduler/Core admission이 `DXB-RUN-031` memory capacity를 우회하지 않는다.
- Bot-only v0.5 end-to-end path가 동일하다.
