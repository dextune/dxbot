---
title: "Channel Collaboration 모델"
document_id: "DXB-DOM-029"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-GOV-002", "DXB-DOM-028", "DXB-DOM-027"]
---

# Channel Collaboration 모델

## 1. 목적

v0.5 Channel identity/lifecycle/membership/Role/Authority/Conversation relation을 유지하고 v0.6 Collaboration Run과 information-flow policy relation을 **Domain reference 수준**으로 추가한다. routing/termination은 `DXB-RUN-037`, cross-aggregate progress는 `DXB-RUN-038`이 소유한다.

## 2. 정의 비회귀

```text
Channel ≠ Bot
Channel ≠ Brain
Channel ≠ Core
Channel ≠ Task
Channel ≠ Thread
Channel ≠ Collaboration Run
```

Channel은 판단을 수행하지 않는다.

## 3. Canonical 구성 보강

기존 ChannelId/ProjectId/Revision, lifecycle, Membership, Role/Authority Binding, ParticipationPolicyRef, Channel Conversation, Shared Memory Scope, Thread/Task/Artifact refs를 유지한다.

필요 시 추가 relation:
- collaboration admission/termination policy ref
- information-flow/declassification policy ref
- active/recent ProcessRef/CycleId index
- resource policy ref

CoreLeaseId, MemoryReservation, processing state, current pressure state를 Membership Canonical State에 넣지 않는다.

## 4. Channel Memory / Information Flow

Channel Shared Memory는 독립 `Channel(ChannelId)` Scope다. target write authority와 source read authority가 있어도 Private/Sensitive source publication은 `DXB-RUN-032` information-flow/declassification을 별도 통과한다.

## 5. Participation / Collaboration Run

Membership은 participant eligibility source일 뿐 모든 메시지 실행을 의미하지 않는다. causal run의 participant selection, activation budget, progress/stall, terminal reason은 Runtime Orchestration이 소유한다.

Single-Bot-first 정책은 Channel Domain identity를 변경하지 않는다. Multi-Bot activation을 줄이거나 종료해도 Membership/Role revision이 변경되는 것은 아니다.

## 6. Manager / Reviewer

- Manager Role ≠ SupervisorRef ≠ Authority ≠ Authorization Decision.
- Reviewer Role 자체가 결과를 Verified로 만들지 않는다.
- 실제 validation은 Claim/Evidence/provenance/criteria를 Memory/Task owner와 연결한다.

## 7. Lifecycle / Late Result

Channel archive가 running Task/Process를 자동 cancel하지 않는다. terminal Collaboration Run에 늦게 도착한 result가 새 Cycle을 암묵 생성하지 않는다. late result는 audit/reconciliation 대상으로 유지할 수 있다.

## 8. 검증 기준

- 기존 AT-CHANNEL-001~003 유지.
- AT-COLLAB-002에서 Channel Membership 수와 무관하게 causal run이 bounded/terminal.
- Role 문자열/다수결만으로 Verified Memory 생성 0.
- Channel Memory가 Bot Global Memory replica가 아님.
