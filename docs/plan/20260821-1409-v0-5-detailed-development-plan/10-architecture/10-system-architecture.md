---
title: "전체 시스템 아키텍처"
document_id: "DXB-ARC-010"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-GOV-002", "DXB-GOV-003"]
---

# 전체 시스템 아키텍처

## 1. 목적

v0.4의 `Persistent Bot → Main Conversation → Thread → Task → Execution → Core Lease`를 보존하면서 Project/Channel collaboration path와 Scope-Aware Memory를 공통 Runtime에 추가한다.

## 2. 목표 구조

```text
Persistent Bot
├─ Identity / Brain / Bot Memory
└─ Main Conversation
   └─ Thread
      └─ Task → Execution → Core Lease

Project
├─ Project Membership / Policy / Resources / Project Memory
└─ Channel
   ├─ Channel Membership / Role / Authority
   ├─ Channel Memory
   └─ Channel Conversation
      └─ Thread
         └─ Task / Delegation
             ↓
      Target Persistent Bot Brain
             ↓
      Dynamic Scheduler / Core Lease
```

Project/Channel은 intelligence owner가 아니다. 실제 판단과 Context 조립은 대상 Bot Brain이 수행한다.

## 3. 계층 책임

| 계층 | 책임 | 금지 |
|---|---|---|
| Core Domain | Bot/Project/Channel/Conversation/Thread/Memory/Task/Execution 의미 | Provider/UI/DB 구체 구현 참조 |
| Application | auth, command, UoW, idempotency, side-effect orchestration | Domain rule 복제 |
| Channel Orchestration | recipient resolution, turn admission, bounded fan-out, membership fencing, presence projection | LLM reasoning, Bot Brain/Task state 직접 소유 |
| Conversation Runtime | message/thread routing, parent/lineage service | Memory/Task state machine 소유 |
| Brain/Context | authorized Scope Set 기반 Context Plan | concrete Provider 선택 |
| Scheduler | Core Lease/admission/fairness | Role/Channel membership 판단, Provider selection |
| Provider Host | Capability binding, permission/resource/deadline/cancel enforcement | Project/Channel/Thread identity 소유 |
| Persistence | Canonical journal/state/history/scoped Memory/directive/artifact | runtime presence/Provider Session을 truth로 저장 |
| Interface | 표현/상호작용 | UI room/tab을 Domain identity로 사용 |

## 4. Normal Bot Path

```text
Interface
→ Bot Main Conversation / Thread
→ Brain Context Plan
→ Task
→ Scheduler/Core Lease
→ Provider Host
→ Result/MemoryProposal
```

이 경로는 Project 없이 완전하게 동작한다.

## 5. Channel Collaboration Path

```text
Channel Message
→ Project/Channel authorization
→ Channel Coordinator
→ recipient/role-aware routing + fan-out budget
→ Target Bot
→ Brain / Scope-Aware Context Plan
→ Task / durable Bot Network delegation
→ Scheduler/Core Lease
→ Provider Host
→ result/message/artifact
→ MemoryProposal / explicit scoped commit
```

Coordinator가 직접 Provider를 호출하거나 Core Lease를 mint하지 않는다.

## 6. Scope-Aware Context Path

```text
Current Input
+ Bot Identity/Policy + relevant Bot Memory
+ Project Identity/Policy + relevant Project Memory (authorized only)
+ Channel Identity/Role/Authority + Channel Memory (if present)
+ Current Thread + Thread Memory
+ Task/Execution/Supervisor status
+ Recent/Retrieved History
+ Artifact/Checkpoint
→ Trust/Authorization
→ Deterministic Selection
→ Token+Byte Budget
→ Immutable Context Plan
→ Provider-neutral Render
```

모든 scope를 materialize하지 않는다. Context Plan은 selected revision/digest를 pin한다.

## 7. Memory Path

Message/Execution/Artifact는 Candidate일 뿐이다.

```text
Candidate
→ Proposal
→ Scope Classification
→ Authorization
→ Normalize/Dedup/Conflict/Verification/Trust/Retention
→ Scoped Canonical Commit
→ optional Promotion Proposal
```

Project/Channel Memory는 참여 Bot의 global store에 자동 복제하지 않는다.

## 8. Control Path

Channel 자연어 Message와 Runtime Directive는 분리한다.

```text
Message / explicit action
→ intent resolution
→ current membership/authority check
→ expected revision/generation
→ Canonical Command/Directive
→ Runtime Control Channel
→ Execution Supervisor
→ safe point / Scheduler
```

Manager도 Supervisor scope 밖 Task를 직접 조작하지 못한다.

## 9. Persistence / Recovery

Durable:
- Bot/Main Conversation/Thread
- Project/Project Membership
- Channel/Channel Membership/Role/Authority
- Project/Channel/Thread Memory revision/provenance
- Task/Execution/Continuation/Directive
- Message/Artifact/Event/Outbox/Inbox

Derived/Ephemeral:
- Provider Session
- Interface Session
- Channel Presence Runtime
- summaries/index/cache
- active Core handle

## 10. Modularity

Project/Channel 기능이 disabled여도 Bot-only path와 Provider Framework가 유지된다. Provider/Plugin/UI 제거가 Project/Channel/Thread/Memory identity migration을 요구해서는 안 된다.

## 11. 검증 기준

- Project/Channel→Brain 직접 소유 edge가 없다.
- Channel message 하나가 participant 수만큼 무제한 Execution을 만들지 않는다.
- Shared Scope read가 final current authorization을 통과한다.
- Core Lease ownership이 Scheduler에 유지된다.
- Bot-only v0.4 end-to-end path가 동일하다.
