---
title: "Dynamic Core Scheduler"
document_id: "DXB-DOM-024"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-DOM-023", "DXB-ARC-010"]
---
# Dynamic Core Scheduler

Core는 Bot 생성 시 고정하는 worker 수가 아니라 Runtime이 필요·정책·자원에 따라 발급하고 회수하는 논리적 실행 Lease다.

```text
CoreLeaseId
BotId / TaskId / ExecutionId
SchedulerGeneration
ResourceGrantRef
AcquiredAt / Deadline
State
```

## P0 병렬 실행 단위

P0 병렬성은 **서로 다른 admitted Execution** 간 병렬성이다. 같은 Execution에는 authoritative active Core Lease 하나만 존재한다. 한 Execution 내부 subtask fan-out, anonymous work item, sub-agent hierarchy는 P0에서 정의하지 않는다. 필요해지면 explicit Task/Execution identity를 가진 Tier A 변경으로 도입한다.

## Admission

```text
Task ready
→ authorization/capability validation
→ memory/cost/concurrency admission
→ fair scheduling
→ Core Lease + Provider activity acquisition
→ Execution run
→ release/join/accounting
```

global/scope/Bot ceiling, fairness, starvation prevention, deadline, recovery headroom을 함께 고려한다. Core count만으로 memory safety를 주장하지 않는다.

```text
Requested → Granted → Active → Releasing → Released
Requested → Deferred | Rejected
Active → Preempting → Released
```

stale SchedulerGeneration/Lease write를 fencing하고, waiting/suspended/shutdown 뒤 permit·reservation·activity leak가 없어야 한다.
