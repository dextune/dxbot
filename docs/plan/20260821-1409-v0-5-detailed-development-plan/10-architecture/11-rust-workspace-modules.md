---
title: "Rust Workspace와 모듈 경계"
document_id: "DXB-ARC-011"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-GOV-003"]
---

# Rust Workspace와 모듈 경계

## 1. 목적

Project/Channel/Scoped Memory를 추가하되 crate 폭증, service locator, shared mutable RuntimeContext 없이 기존 Common abstraction을 재사용한다.

## 2. 권장 logical module

```text
dxb-domain
├─ bot
├─ project
├─ channel
├─ conversation_thread
├─ memory
├─ goal_task
└─ execution

dxb-runtime
├─ bot_coordinator
├─ conversation_runtime
├─ channel_orchestration
├─ task_runtime
├─ execution_supervisor
├─ dynamic_core_scheduler
├─ live_control
└─ recovery

dxb-control
├─ project_channel_api
├─ conversation_thread_api
├─ command_query
├─ live_control_protocol
└─ event_stream
```

실제 Rust source module은 snake_case를 사용한다. 문서/디렉터리 이름은 lowercase kebab-case 규칙을 따른다.

## 3. 책임 경계

- `project`: identity/lifecycle/membership/resource relation만 소유하고 Memory internals를 소유하지 않는다.
- `channel`: identity/membership/role/authority/conversation relation을 소유한다.
- `memory`: 범용 ScopeRef, Record/Revision, Recall/Promotion을 소유한다.
- `conversation_thread`: ParentRef/Thread identity/lineage/history를 소유한다.
- `channel_orchestration`: routing/admission/fan-out/presence runtime을 소유한다.
- `bot_coordinator/brain`: 판단과 Context Plan을 수행한다.
- `scheduler`: Core Lease owner다.

## 4. 재사용 우선

신규 Project/Channel 때문에 별도 Task engine, 별도 Memory engine, 별도 Scheduler, 별도 Control stack을 만들지 않는다. Channel collaboration은 기존 Task/Network/Live Control/Provider Host를 조합한다.

## 5. Dependency 방향

```text
dxb-kernel
   ↑
dxb-domain ← Capability Contracts
   ↑
dxb-application
   ↑
dxb-runtime composition ← dxb-provider-host
   ↑
dxb-control
   ↑
interfaces
```

Channel Runtime이 Domain internals를 bypass하거나 Provider crate를 직접 import하지 않는다.

## 6. Type 분리

동일 struct로 공유하지 않는다.
- Project/Channel Domain type vs DB row vs Wire DTO
- Channel Membership vs Runtime Presence
- Role Binding vs Authority Binding
- MemoryScopeRef vs permission grant object
- ConversationParentRef vs UI route
- Channel Message vs Control Directive vs Bot Network Message

## 7. 메모리 효율

Stable immutable Bot/Project/Channel policy/context prefix는 digest/revision 기반 shared ref를 사용할 수 있다. Scope별 전체 state를 Core별 clone하지 않는다. large history/artifact는 reference/stream/pagination을 우선한다.

## 8. 검증 기준

- Domain→Provider/Plugin/UI forbidden edge 0.
- Project/Channel module이 Memory/Task/Scheduler owner를 복제하지 않는다.
- channel_orchestration module이 LLM/Provider invocation을 직접 수행하지 않는다.
- dependency cycle 0.
- 제거 가능한 Channel 기능이 Bot-only path를 컴파일/실행 불가능하게 만들지 않는다.
