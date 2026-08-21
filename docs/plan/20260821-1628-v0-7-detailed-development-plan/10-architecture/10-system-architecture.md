---
title: "전체 시스템 아키텍처"
document_id: "DXB-ARC-010"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-GOV-002", "DXB-GOV-003"]
---

# 전체 시스템 아키텍처

## 1. 목적

v0.6의 Persistent Bot/Project/Channel/Thread/Task/Execution/Durable Process/Epistemic Memory/Authorization/Runtime Memory 구조를 보존하고, 이를 **Headless Application Contract**와 **CLI Reference Interface**까지 하나의 단방향 호출 경로로 연결한다.

## 2. v0.7 목표 구조

```text
Persistent DXBOT Runtime Host
├─ Domain / Canonical State
├─ Durable Process / Memory / Security
├─ Scheduler / Core / Provider Host
└─ Recovery / Resource Governance
          ↑
     Application Layer
          ↑
Command / Query / Subscription Contract
          ↑
 Control Endpoint / Control Client
          ↑
       dxb CLI
```

정상 Domain request 흐름은 외부에서 아래 방향으로 진입한다.

```text
dxb CLI
→ Control Client
→ Control Endpoint
→ Application use case
→ Domain/Runtime Canonical Owner
→ Persistence/Scheduler/Provider Host
→ committed result/event/projection
→ Application Contract
→ CLI render/output
```

## 3. Canonical Ownership

| 의미 | Owner | 금지 |
|---|---|---|
| Bot/Project/Channel/Thread/Task/Memory | 기존 v0.6 Domain Owner | CLI/Control이 state machine 재구현 |
| Execution/Scheduling/Core Lease | 기존 Runtime/Scheduler | CLI가 Core/Execution 직접 mutate |
| Durable Process | `DXB-RUN-038` | CLI/process view가 child state 복제 |
| Authorization/Information Flow | `DXB-RUN-032` | CLI local Role→Authority inference |
| Runtime Memory Budget | `DXB-RUN-031` | client가 pressure policy 계산 |
| Application use case orchestration | Application Layer | Domain rule duplicate |
| public Command/Query/Subscription | `DXB-IFC-040` | transport/CLI별 divergent schema |
| CLI interaction/rendering | `DXB-IFC-041` | Canonical state local replica |
| Runtime Host process lifecycle | Runtime composition + Host Lifecycle Adapter | Domain state 직접 mutation |
| transport connection | Control adapter | Domain identity 소유 |

## 4. 금지 Dependency Edge

```text
CLI ─X→ Domain Store
CLI ─X→ Scheduler/Core Lease internals
CLI ─X→ Provider concrete API
CLI ─X→ Memory store/promotion policy
CLI ─X→ Authorization policy implementation
Control DTO ─X→ persistence row as shared type
```

CLI에서만 필요한 편의 기능을 이유로 Domain method를 추가하지 않는다. 필요한 use case가 공통 의미라면 Application Contract에 먼저 정의하고 Backend owner를 재사용한다.

## 5. Runtime Host / Process Boundary

```text
                 ┌─ Host Lifecycle Adapter ─→ OS service manager / launcher
                 │
dxb CLI ──────────┤
                 │
                 └─ Control Client ─→ Control Endpoint ─→ Running Runtime Host
```

불변조건:
- CLI process lifetime ≠ Runtime lifetime
- shell/session lifetime ≠ Bot/Task/Process lifetime
- CLI SIGINT/disconnect/crash ≠ target Task cancel
- Runtime stop/cancel/suspend 등은 해당 owner의 explicit command/lifecycle operation만 상태를 바꾼다.
- CLI reconnect는 Canonical state를 다시 조회하고 cursor가 유효하면 stream을 resume한다.

### 5.1 Bootstrap 예외 경계

`dxb runtime start`는 Runtime이 아직 실행되지 않아 Control Endpoint가 존재하지 않을 수 있다. 따라서 좁은 **Host Lifecycle Adapter**가 platform service manager/launcher와 통신하는 것을 허용한다.

Host Lifecycle Adapter는 오직:
- Runtime process/service start
- process/service status discovery
- graceful stop 요청 경로 bootstrap
- endpoint readiness discovery

만 담당한다. Bot/Project/Task/Memory/Scheduler/Provider state를 직접 읽거나 mutate하지 않는다. Runtime이 Ready가 된 이후 모든 Domain use-case는 `DXB-IFC-040` Control path를 사용한다.

exact service manager/IPC/install 방식은 ADR 대상이다.

## 6. Bot-only Path

```text
dxb CLI
→ Application Contract
→ Bot Main Conversation / Thread
→ Brain Context Plan
→ Task
→ Runtime Memory admission
→ Scheduler/Core Lease
→ Provider Host
→ Result / Memory Proposal
→ validation / Canonical commit
```

Project/Channel/Durable Process는 필요하지 않다. v0.6 Bot-only path의 의미를 변경하지 않는다.

## 7. Collaboration / Durable Process Path

```text
CLI command
→ Application Contract
→ Project/Channel command
→ Durable Process when required
→ Task/Delegation/Activity
→ Provider Host
→ Evidence/Result
→ Authorization + Information Flow
→ Epistemic validation
→ Canonical commit
→ subscription/projection
→ CLI watch/output
```

CLI watch가 Process owner가 아니며 CLI 종료가 Process terminal transition을 만들지 않는다.

## 8. Resource / Streaming Boundary

Application Contract/Control/CLI도 v0.6 Runtime Memory 정책의 consumer다.

- paginated query
- bounded request/response bytes
- bounded event buffers
- cumulative + in-flight stream cap
- slow consumer backpressure/gap policy
- large Artifact reference/streaming
- no full-history/full-project materialization

## 9. Future Interface Re-entry

v0.7에서는 TUI/Web/BFF/frontend state/view route를 설계·구현하지 않는다. 새 product Interface 계획은 **v0.7 Runtime Host + Application Contract + CLI의 M0~M5/DoD와 compatibility/resource/security evidence가 안정화된 뒤 별도 버전에서 다시 승인**한다.

미래 Interface가 도입될 때도 `DXB-IFC-040`을 실제로 재사용해야 하며, 그 가능성을 이유로 v0.7에 speculative abstraction을 추가하지 않는다.

## 10. 검증 기준

- CLI Domain request는 `DXB-IFC-040`/Application path 하나로 연결된다.
- Runtime bootstrap은 narrow Host Lifecycle Adapter만 예외이며 Domain mutation 0.
- CLI direct Domain/Store/Scheduler/Provider edge 0.
- CLI exit/crash/disconnect가 Bot/Task/Process lifecycle을 변경하지 않는다.
- v0.6 Durable Process/Security/Memory/Resource owner regression 0.
- Bot-only end-to-end path가 Project/Channel 없이 동작한다.
