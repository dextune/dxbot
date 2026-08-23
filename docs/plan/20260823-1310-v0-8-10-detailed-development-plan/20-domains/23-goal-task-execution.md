---
title: "Goal·Task·Execution·Side Effect 모델"
document_id: "DXB-DOM-023"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-DOM-020", "DXB-ARC-014"]
---
# Goal·Task·Execution·Side Effect 모델

Task는 durable work intent, Execution은 immutable attempt, Core Lease는 transient resource다.

```text
Submitted → Admitted → Running
Submitted/Admitted → Deferred | Rejected | Cancelled
Running → Suspended | Completed | Failed | Cancelled | RecoveryRequired
```

Task `Rejected/Deferred`는 Domain outcome이다. SubmitTask operation이 Task aggregate를 생성하고 이 상태를 commit했다면 Operation Receipt는 `Committed`다. Receipt `Rejected`는 Task를 생성하지 못한 operation rejection에만 사용한다.

Side Effect Ledger는 `Prepared → Dispatched → Confirmed | Failed | Unknown`, `Unknown → Reconciling → Confirmed | Failed | ManualResolutionRequired`다. Unknown을 blind retry하지 않는다.

Task result와 Memory promotion은 분리한다.
