---
title: "Rust Workspace와 모듈 경계"
document_id: "DXB-ARC-011"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-GOV-003"]
---

# Rust Workspace와 모듈 경계

## 1. 목적

v0.5 Common architecture를 유지하면서 Durable Process, Authorization Decision, Runtime Memory Governance를 기존 crate의 명확한 module/service boundary로 추가한다. crate 폭증, service locator, global mutable RuntimeContext, 상태 원본 중복을 금지한다.

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

dxb-application
├─ command_query
├─ unit_of_work
└─ authorization

dxb-runtime
├─ bot_coordinator
├─ conversation_runtime
├─ channel_orchestration
├─ durable_process
├─ task_runtime
├─ execution_supervisor
├─ dynamic_core_scheduler
├─ resource_governance
│  └─ memory_pressure
├─ live_control
└─ recovery

dxb-control
├─ project_channel_api
├─ conversation_thread_api
├─ process_status_api
├─ command_query
└─ event_stream
```

실제 Rust source module은 `snake_case`, 문서/디렉터리 파일명은 lowercase kebab-case를 사용한다.

## 3. 책임 경계

- `durable_process`: Process identity/progress/step/outcome refs/reconciliation만 소유한다. Task/Memory/Channel state를 복제하지 않는다.
- `authorization`: Common Authorization Decision semantic을 제공한다. Project/Channel/Memory/Task/Provider Host는 이를 호출하는 PEP다.
- `resource_governance`: hierarchical budget/reservation/pressure state를 소유한다. Scheduler는 consumer/enforcement point다.
- `memory_pressure`: platform-neutral pressure decision과 optional host probe adapter boundary를 분리한다.
- `memory`: Scope/Epistemic/Retraction relation을 소유한다.
- `channel_orchestration`: routing/Collaboration Run admission/termination을 소유하되 Brain이 아니다.

## 4. 신규 crate 금지 기본값

`durable-process`, `authorization`, `memory-governor`라는 별도 crate를 먼저 만들지 않는다. 다음이 실제로 입증될 때만 분리를 검토한다.
- 독립 compile/test boundary
- 다수 consumer 간 stable public contract
- platform adapter 격리 필요
- dependency direction 단순화
- measurable build/runtime benefit

## 5. Type 분리

동일 struct로 공유하지 않는다.
- Process Domain/Runtime state vs DB row vs Wire DTO
- Activity outcome ref vs Execution/Provider internal result
- AuthorizationDecision vs Membership/Role/Authority binding
- ActionGrant vs secret/token/permission policy
- InformationLabel vs MemoryScopeRef
- RuntimeMemoryReservation vs Task/Execution Canonical state
- PressureState vs OS/cgroup raw metric

## 6. Ownership / Allocation

- large immutable Context/Artifact/prefix는 `Arc` 등 shared immutable ownership 또는 durable reference를 우선한다.
- back-reference는 ID/`Weak` 등 non-owning relation을 우선하고 strong `Arc` cycle을 architecture review 대상으로 둔다.
- spawned task는 owner/cancel/join과 retained payload upper bound를 가진다.
- permit/reservation/subscriber/stream은 RAII 또는 동등 terminal cleanup을 가진다.
- resource admission hot path는 장시간 global mutex나 scope 전체 map scan을 피한다.

## 7. Dependency 방향

```text
dxb-kernel
   ↑
dxb-domain
   ↑
dxb-application
   ↑
dxb-runtime composition ← dxb-provider-host
   ↑
dxb-control
   ↑
interfaces
```

Domain이 Runtime/Provider/UI를 import하지 않는다. `DXB-RUN-038`이 Task/Memory/Channel owner API를 호출하더라도 lower layer의 state machine을 재구현하지 않는다.

## 8. 검증 기준

- Domain→Provider/Plugin/UI forbidden edge 0.
- Durable Process/Authorization/Resource module이 기존 owner state를 복제하지 않는다.
- 별도 crate 추가 없이도 unit/contract/fault test가 가능하다.
- strong ownership cycle/orphan task가 large payload를 영구 retain하지 않는다.
- Channel/Process 기능을 제거해도 Bot-only path를 컴파일·실행할 수 있다.
