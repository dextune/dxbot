---
title: "Harness 통합 경계와 실제 실행 canary"
document_id: "DXB-ARC-013"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-ARC-010", "DXB-ARC-012"]
---
# Harness 통합 경계와 실제 실행 canary

## 1. 경계

```text
Application/Runtime
→ Capability Contract
→ Provider Host
→ Harness Adapter
→ external Harness / Model / Tool / Sandbox
```

외부 Session, trajectory, agent, subagent, plugin tree를 Bot, Thread, Core, Memory, Domain Event로 자동 매핑하지 않는다.

## 2. 이중 검증 경로

- **Reference Provider**: fake clock/seed/result로 Core correctness, cancellation, error, bounded output을 deterministic하게 검증한다.
- **Real Harness Adapter**: 실제 external harness/model/tool 경로가 Provider Host를 통과하여 Task 하나를 완료하는 canary를 제공한다.

Reference Provider만 통과하고 실제 Adapter 경로가 전혀 없는 상태를 P0 release-ready로 표시하지 않는다. 실제 canary의 외부 비결정성은 Domain state machine의 unit acceptance를 대체하지 않는다.

## 3. Canary 최소 계약

```text
Task/Execution snapshot
→ bounded Context Plan
→ Provider selection generation
→ permission/resource/deadline/cancellation
→ model or tool activity
→ partial/terminal output accounting
→ typed Result/Evidence
→ no direct Memory/Permission mutation
```

Provider unavailable, timeout, cancellation, partial output, process loss, stale generation late callback을 포함한다. credential은 scoped handle로만 전달하며 log/receipt/schema에 복사하지 않는다.

## 4. Release evidence

- adapter exact version/commit/license
- Host-path trace
- bounded input/output
- cancel/deadline propagation
- one successful and one failure canary
- Adapter 제거 build
