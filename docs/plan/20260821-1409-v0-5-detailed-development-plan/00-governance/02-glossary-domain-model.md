---
title: "용어집과 도메인 모델"
document_id: "DXB-GOV-002"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000"]
---

# 용어집과 도메인 모델

## 1. 목적

v0.4 용어를 유지하면서 Project/Channel/Scope-Aware Memory/Collaboration 관련 용어를 수명·Canonical Owner까지 포함해 고정한다.

## 2. 핵심 용어

| 용어 | 정의 | 수명 | Canonical Owner |
|---|---|---|---|
| Bot | 지속 Identity·Memory·Goal·Permission·State를 가진 논리 엔티티 | 장기 | Bot Aggregate |
| Brain | 한 Bot의 판단·조정·Context 정책 | Bot과 동일 | Bot Runtime |
| Project | 장기 Collaboration/Knowledge/Resource Boundary | 장기 | Project Domain |
| Project Membership | Project 접근 principal/participant의 durable grant | membership 수명 | Project Domain |
| Channel | Project-scoped Persistent Multi-Bot Collaboration Space | 장기 | Channel Domain |
| Channel Membership | Bot의 Channel 참여, role/authority relation의 기준 grant | membership 수명 | Channel Domain |
| Channel Role | Manager/Researcher/Reviewer 등 협업상의 의미 | binding 수명 | Channel Domain |
| Channel Authority | delegate/inspect/report/redirect/suspend/resume/cancel/reprioritize 등 실제 Runtime 권한 binding | binding 수명 | Channel/Permission Domain |
| Channel Conversation | Channel 안의 영속 conversation surface | Channel 수명 | Channel/Conversation relation |
| Main Conversation | Bot당 하나의 영속 Communication + Supervisor Control Surface | Bot 수명 | Conversation Domain |
| Thread | DXBOT 소유의 영속 작업·문맥 경계와 lineage | 장기 | Conversation/Thread Domain |
| Conversation Parent Ref | Thread가 어느 Bot Main Conversation 또는 Channel Conversation에 속하는지 나타내는 Canonical relation | Thread 수명 | Conversation/Thread Domain |
| Memory Scope Reference | Memory Record의 owner scope를 `Bot/Project/Channel/Thread` 중 하나로 식별하는 안정 의미 | Memory revision 수명 | Memory subsystem |
| Shared Scope Memory | Project 또는 Channel이 소유하는 독립 Canonical Memory | scope retention | Memory subsystem |
| Bot-global Memory | Bot 전체에서 재사용되는 Canonical Memory | 장기 | Memory subsystem |
| Thread Memory | 특정 Thread에 한정된 Canonical Memory | Thread/retention | Memory subsystem |
| Working Context | Execution/Core 임시 상태; Canonical Memory가 아님 | Execution | Execution/Core Runtime |
| Memory Proposal | candidate를 Canonical Memory로 commit하기 전 scope/policy 검증 입력 | 짧음 | Memory subsystem |
| Memory Promotion | source revision을 보존하며 다른 Scope에 새 Canonical revision/reference를 만드는 명시적 publication | 감사 보존 | Memory subsystem |
| Context Scope Set | 현재 Execution이 읽을 수 있는 Bot/Project?/Channel?/Thread?/Task/Execution scope 집합 | Execution | Brain/Context Runtime |
| Channel Coordinator | participant routing/turn admission/bounded fan-out/backpressure runtime component; Brain이 아님 | Runtime | Channel Orchestration |
| Channel Presence Runtime | speaking/processing/active execution 등 Derived activity projection | Runtime | Channel Orchestration/Projection |
| SupervisorRef | Task control authority를 가리키는 authoritative relation | Task revision | Task Domain |
| Control Directive | redirect/suspend/reprioritize 등 durable live control 지시 | 감사 보존 | Control/Task Domain |
| Provider Session | Provider-private optimization handle | 임시/Derived | Provider/Adapter |
| Interface Session | client connection/view | 임시 | Interface |
| Core Lease | Scheduler가 Execution에 발급한 일시 실행 권한 | 짧음 | Scheduler |

v0.4의 Goal, Task, Task Specification Revision, Execution, Continuation, Side Effect Ledger, Routine, Bot Message, Artifact, Capability, Provider, Provider Host, Provider SDK, Conformance Suite, Reference Provider, Interface, Plugin, Policy, Quality Tier A/B 정의는 그대로 유지한다.

## 3. 구분 규칙

```text
Project ≠ Bot ≠ Brain
Channel ≠ Bot ≠ Brain ≠ Core
Channel ≠ Thread
Project Membership ≠ Channel Membership
Channel Role ≠ Channel Authority
Conversation History ≠ Memory
Bot Memory ≠ Project Memory ≠ Channel Memory ≠ Thread Memory ≠ Working Context
Thread ≠ Task ≠ Execution ≠ Core Lease
Interface Session ≠ Provider Session ≠ Thread
```

## 4. Scope 관계

Project→Channel은 ownership relation이지만 Memory Scope는 단순 계층 inheritance가 아니다. Channel Memory가 자동 Project Memory가 되지 않고 Project Memory가 자동 Bot Memory가 되지 않는다.

Shared Scope Memory는 Bot Memory의 공용 view가 아니라 독립 Canonical Record 집합이다.

## 5. Conversation Parent

Bot-only path:
`Bot → Main Conversation → Thread`.

Collaboration path:
`Project → Channel → Channel Conversation → Thread`.

Thread의 Parent가 Channel이라고 Thread가 특정 Manager Bot 소유가 되지 않는다.

## 6. Authority

Role은 UX/협업 의미다. Authority는 authenticated membership/permission binding이 생성한다. Prompt에 `Manager`가 포함되어도 authority가 생기지 않는다.

## 7. 검증 기준

- 문서/API에서 Project/Channel/Bot/Thread가 alias로 사용되지 않는다.
- `scope`라는 단어가 Memory Scope와 Permission Scope를 혼동할 경우 구체 타입/명칭을 사용한다.
- Role 문자열을 permission check로 사용하지 않는다.
- Shared Scope Memory를 participating Bot Memory replica로 표현하지 않는다.
