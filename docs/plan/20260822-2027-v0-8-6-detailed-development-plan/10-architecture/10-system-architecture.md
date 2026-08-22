---
title: "전체 시스템 아키텍처와 구현 DAG"
document_id: "DXB-ARC-010"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-BASE-000", "DXB-GOV-002", "DXB-GOV-003"]
---
# 전체 시스템 아키텍처와 구현 DAG

```text
Shell / Automation
  ├─ Host Lifecycle Client
  └─ Control Client
        ↓ authenticated bounded UDS
Control Endpoint
        ↓ server-derived Principal
Headless Application Contract
        ↓ typed Use Case / Unit of Work
Domain + Atomic Persistence / Receipt / Outbox
        ↓
Runtime / Scheduler / Provider Host
        ├─ deterministic Reference Provider
        └─ real Harness Adapter canary
        ↓
Result / Evidence / Event / Projection
        ↓
Snapshot Page / Subscription / Safe CLI Writer
```

## Integrated DAG

```text
M0 active baseline + executable validator
→ M1A storage proof and crash model
→ M1B kernel/domain/application-contract minimum
→ M2 instance/bootstrap/principal/control endpoint
→ M3 Bot-only CLI + durable client submission
→ M4 Project/Channel membership + typed delegation
→ M5 recovery/streaming/export + real Harness canary
→ M6 compatibility/security/performance/release freeze
```

CLI는 Domain, Store, Scheduler, Provider concrete API를 직접 호출하지 않는다. Host Lifecycle Client는 process/service bootstrap만 수행하며 Domain state를 읽거나 변경하지 않는다.

## Vertical slices

- Bot-only: `start → create bot → send → submit/watch → receipt recovery → restart → same identity`
- Multi-Bot: `create bots/project/channel → set membership → delegated task → target Bot execution → result/evidence`
- Recovery: `pre-send journal → response loss → operation lookup → same effect`, cursor gap/resync, partial export cleanup
- Harness: `Task → Context Plan → Provider Host → real Adapter → bounded Result/Evidence`

## Non-goal

TUI/Web/BFF, generic transport/IDL/workflow framework, distributed consensus, crate-per-concept 구조는 포함하지 않는다.
