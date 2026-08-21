---
title: "Durable Cross-Aggregate Process Orchestration"
document_id: "DXB-RUN-038"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-015", "DXB-DOM-023", "DXB-DOM-025", "DXB-RUN-030", "DXB-RUN-031", "DXB-RUN-032", "DXB-RUN-033", "DXB-RUN-036"]
---

# Durable Cross-Aggregate Process Orchestration

## 1. 목적

v0.6의 Durable Process identity/progress/replay/reconciliation owner 의미를 그대로 유지하고, v0.7 CLI watch/control이 Process의 **observer/command consumer**일 뿐 lifecycle owner가 아님을 명시한다.

Durable Process는 새 Brain/Agent/Task engine이 아니며 CLI process가 Process execution을 호스팅하지 않는다.

## 2. Canonical Owner 비회귀

`DXB-RUN-038`이 계속 단독 소유한다.
- Process identity/lifecycle/revision
- ProcessDefinitionVersion
- committed progress/step
- command/outcome refs
- retry/reconciliation
- budget/deadline refs
- terminal reason/replay boundary

Task/Execution/Memory/Directive/Core Lease/Provider/Side Effect/MemoryReservation은 각 기존 owner가 소유한다.

## 3. Application Contract 경계

```text
CLI mutation
→ DXB-IFC-040 Command
→ Application use case
→ Durable Process or target owner
→ typed child Command
→ committed outcome
→ Process deterministic advance
```

CLI가 Process DB row/step runner를 직접 mutate하지 않는다. Process는 다른 Aggregate를 직접 mutate하지 않고 기존 Application/Command boundary를 재사용한다.

## 4. CLI Watch / Inspect

```text
Process Canonical State / Projection
→ authorized Query/Subscription
→ cursor/watermark
→ CLI watch/render
```

- CLI process 종료가 Process cancel/timeout/failure를 생성하지 않는다.
- watch SIGINT는 local subscription만 종료한다.
- process cancel/suspend/redirect가 필요하면 explicit Command를 제출한다.
- late result/replay semantics는 CLI 상태가 아니라 Process Canonical revision으로 결정한다.

## 5. Lost Response / Duplicate Command

Process 관련 mutation도 v0.7 Command idempotency를 따른다.
- command commit 후 response loss에서 동일 idempotency identity retry
- duplicate process create/advance/control effect 0
- child Side Effect Unknown은 기존 reconciliation 사용
- client가 local timeout을 Process failure로 commit하지 않음

## 6. Waiting / Resource

Process waiting은 active CLI connection, Core, Provider Context, Runtime MemoryReservation을 요구하지 않는다. CLI가 연결되지 않은 상태에서도 Process/child Task는 Runtime Host에서 계속 진행한다.

## 7. Recovery / Reconnect

Runtime restart 후 기존 v0.6 recovery를 수행하고 CLI는 새 connection에서 same ProcessId/revision/status를 조회한다. Control Connection/CLI Session ID는 Process identity가 아니다.

## 8. 검증 기준

- AT-PROC-001 v0.6 crash/replay 의미 유지.
- CLI 종료/crash/SIGINT로 Process terminal state 변화 0.
- Process control이 direct internal mutation 없이 `DXB-IFC-040` Command path를 통과.
- duplicate/retry response-loss window에서 Process/child semantic duplicate 0.
- CLI 연결 없이도 waiting/running Process가 Runtime Host 수명에 따라 지속 가능.
