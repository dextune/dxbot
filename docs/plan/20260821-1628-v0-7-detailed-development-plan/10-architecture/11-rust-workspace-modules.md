---
title: "Rust Workspace와 모듈 경계"
document_id: "DXB-ARC-011"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-GOV-003"]
---

# Rust Workspace와 모듈 경계

## 1. 목적

v0.6 Common architecture를 유지하면서 Application Contract/Control/CLI 경계와 Runtime bootstrap 경계를 추가한다. **crate 폭증, service locator, global mutable context, Wire/Domain type 공유, Interface별 Backend shortcut**을 금지한다.

## 2. 권장 Logical Boundary

```text
dxb-domain
├─ bot / project / channel
├─ conversation_thread
├─ memory
├─ goal_task
└─ execution

dxb-application
├─ command_query
├─ use_case
├─ unit_of_work
└─ authorization_port

dxb-runtime
├─ durable_process
├─ task_runtime / execution_supervisor
├─ dynamic_core_scheduler
├─ resource_governance
├─ live_control
└─ recovery

dxb-provider-host
└─ capability invocation/lifecycle

dxb-control
├─ contract
├─ server_adapter
├─ client
├─ event_subscription
└─ host_lifecycle_client

dxb-cli
├─ command
├─ rendering
├─ machine_output
└─ local_config
```

이는 logical responsibility 예시다. 독립 compile/test/dependency 이유가 입증되지 않으면 각 이름을 별도 crate로 기계적으로 분리하지 않는다.

## 3. Dependency 방향

```text
dxb-kernel
    ↑
dxb-domain
    ↑
dxb-application
    ↑
dxb-runtime-composition ← dxb-provider-host
    ↑
dxb-control adapter/client
    ↑
  dxb-cli
```

Host lifecycle client는 Runtime internal API가 아니라 OS/service-manager/launcher adapter를 좁게 감싼다. 공유 public value/DTO가 필요하면 Domain internal struct를 노출하지 않고 stable contract package/boundary로 제한한다.

## 4. CLI Dependency Firewall

`dxb-cli`가 직접 사용할 수 있는 것은 원칙적으로 다음뿐이다.

- public Application Contract DTO/value
- Control Client
- narrow Host Lifecycle Client
- CLI-specific parser/render/config

금지:

```text
dxb-cli → dxb-domain internals
dxb-cli → dxb-runtime internals
dxb-cli → dxb-storage internals
dxb-cli → dxb-provider-host concrete implementation
dxb-cli → Scheduler/Registry mutator
Host Lifecycle Client → Domain Store/Task/Memory mutator
```

architecture test로 금지 edge를 검출한다.

## 5. Type 분리

동일 struct를 다음 경계에서 공유하지 않는다.

- Domain state vs Persistence row vs public Wire DTO
- Command intent vs Domain Event
- Subscription event DTO vs internal Work/Event queue item
- Control Connection/CLI Session vs Thread/Task identity
- AuthorizationDecision vs CLI confirmation boolean
- Resource pressure DTO vs raw OS/cgroup metric
- Provider Capability metadata vs Application capability discovery DTO
- Runtime process handle/status vs Domain Runtime state DTO

## 6. Runtime / CLI Task Ownership

- Runtime spawned task는 Runtime owner/cancel/join을 가진다.
- CLI watch/follow spawned task는 CLI process local owner/cancel/join을 가진다.
- CLI local cancellation token을 Runtime Task cancellation token과 공유하지 않는다.
- slow stdout writer가 Runtime queue를 직접 점유하지 않도록 bounded buffer/backpressure를 둔다.
- CLI가 large result를 `Vec`/`String` 하나로 aggregate하지 않는다.

## 7. v0.7 Interface Workspace Scope

active implementation candidate는 Application/Runtime composition 위의 `dxb-control`과 `dxb-cli`까지다. TUI/Web 전용 crate/module/package 후보를 v0.7 workspace plan에 두지 않는다.

미래 Interface를 위해 미사용 abstraction crate, frontend schema crate, BFF crate를 선행 생성하지 않는다.

## 8. 검증 기준

- Domain→Interface/Provider forbidden edge 0.
- CLI→Domain/Runtime/Storage/Provider internal edge 0.
- Host Lifecycle Client의 Domain mutation edge 0.
- Wire DTO와 Domain/Persistence struct 직접 공유 0.
- 별도 crate 추가 없이도 contract/fault/E2E test가 가능하다.
- TUI/Web 전용 active crate/module 후보 0.
- strong ownership cycle/orphan task/large payload retention regression 0.
