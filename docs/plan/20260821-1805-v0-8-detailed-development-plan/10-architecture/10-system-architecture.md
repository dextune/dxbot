---
title: "전체 시스템 아키텍처"
document_id: "DXB-ARC-010"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-GOV-002", "DXB-GOV-003"]
---

# 전체 시스템 아키텍처

## 1. 목표 구조

```text
Shell / Automation
  ├─ Host Lifecycle Client ─→ Linux user service manager (bootstrap only)
  └─ Control Client
        ↓ authenticated bounded local IPC
Control Endpoint
        ↓
Headless Application Contract
        ↓
Application Use Cases / Unit of Work
        ↓
Domain Aggregates + Persistence/Journal
        ↓
Runtime/Scheduler/Durable Process/Provider Host
        ↓
Result/Event/Receipt/Projection
        ↓
Pagination/Subscription → CLI writer
```

Runtime Ready 이후 Domain use-case는 Application Contract 하나로 귀결된다. Host Lifecycle Client는 process/service start, host status, readiness discovery, graceful-stop bootstrap만 수행한다.

## 2. Integrated implementation DAG

```text
Workspace/Kernel
→ Domain identity/state minimum
→ Persistence/Journal/Recovery minimum
→ Provider Host + deterministic Reference Provider
→ Bot Main Conversation / Task minimum
→ Application Contract Kernel
→ Runtime Instance / Control Endpoint / Client
→ Bot-only CLI Vertical Slice
→ Project/Channel Vertical Slice
→ Durable Process / Recovery / Streaming
→ CLI hardening / compatibility / release
```

존재하지 않는 Backend fixture를 전제로 CLI부터 구현하지 않는다.

## 3. Runtime Instance 경계

`RuntimeInstance = InstanceId + DataRoot + ConfigProfile + current HostGeneration`. 동일 DataRoot에 active HostGeneration은 하나뿐이다. endpoint descriptor와 process handle은 Domain identity가 아니다.

## 4. Canonical Owner와 금지 edge

| 의미 | Owner | 금지 |
|---|---|---|
| Bot/Memory/Task/Project/Channel | Domain | CLI/Control replica |
| Runtime scheduling/resource/recovery | Runtime | client policy |
| Operation Receipt | Application command journal | CLI-only truth |
| Selector/cursor/public schema | IFC-040 | transport별 schema |
| CLI output/exit/file | IFC-041 | Domain mutation |

```text
CLI -X-> Domain/Storage/Scheduler/Provider internals
Host Lifecycle Client -X-> Bot/Task/Memory state
Control adapter -X-> Domain Store direct mutation
Public DTO -X-> Persistence row identity
```

## 5. Vertical Slice 순서

### Bot-only
`runtime start → bot create → conversation send → task submit/watch → operation show → memory inspect → restart → same identity/result`

### Project/Channel
`project create → channel create → membership/authority → send → task/delegation/process → scoped promotion → result/evidence`

### Recovery/Automation
`response loss → receipt reconcile`, `page/--all`, `JSONL resume/gap`, `partial file cleanup`, `endpoint recreation`.

## 6. Resource·security boundary

모든 request/page/chunk/subscriber/export는 item+byte cap과 owner cleanup을 가진다. local IPC directory와 peer identity는 RUN-032/035, machine writer와 terminal/file는 IFC-041이 소유한다.

## 7. 비범위

TUI/Web/BFF, 범용 transport framework, distributed consensus, generic Workflow DSL, crate-per-concept 구조는 포함하지 않는다.

## 8. 검증 기준

- 각 milestone이 실행 가능한 vertical slice를 만든다.
- CLI direct internal dependency 0
- Runtime start race에서 active instance 1개
- Bot-only path에 Project/Channel/Process 필수화 0
