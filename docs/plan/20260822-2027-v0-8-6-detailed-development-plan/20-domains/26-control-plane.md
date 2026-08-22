---
title: "Typed Control Plane Use-Case Surface"
document_id: "DXB-DOM-026"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-DOM-020", "DXB-DOM-023", "DXB-DOM-025", "DXB-RUN-032", "DXB-ARC-010"]
---
# Typed Control Plane Use-Case Surface

모든 external use-case는 하나의 typed Application operation 또는 Host Action에 귀결된다.

| Family | Typed owner |
|---|---|
| Bot | `CreateBot`, `ChangeBotLifecycle`, `Get/ListBot` |
| Message/Thread | `SendMessage`, `Create/Branch/Get/ListThread`, `ListMessages` |
| Task/Delegation | `SubmitTask`, `ControlTask`, `Get/List/WatchTask` |
| Memory | `Get/Search/History`, `Propose/PromoteMemory` |
| Project/Channel membership | `SetScopeMembership`, `RemoveScopeMembership`, `ListScopeMemberships` |
| Operation recovery | `GetOperation`, `ReconcileOperation` |
| Approval | `List/GetApproval`, `DecideApproval` |
| Side Effect | `ReconcileSideEffect` |
| Runtime shutdown | `RequestRuntimeShutdown` Application Command |
| Host stop | `StopRuntimeHost` Host Action |

## Membership operation

`SetScopeMembership`은 ScopeId/revision, member BotId, RoleRef, expected membership generation/absent를 받는다. `AuthorityBindingRef`는 client가 발명하지 않는다. Security owner가 current scope policy와 RoleRef에서 binding generation을 생성·resolve하고 result에 반환한다.

## Approval creation

고위험 operation이 approval을 필요로 하면 Runtime은 action digest에 결박된 `ApprovalId`를 생성하고 `approval-required` error/receipt hint를 반환한다. `approval approve/deny`는 existing Approval revision만 결정한다. CLI가 ActionGrant를 직접 만들지 않는다.

## Delegation

`SubmitTask`의 operator request는 optional `requested_sender_bot_id`를 받을 수 있으나 resolved sender는 server-side authorization 결과다. Bot-executed delegation은 current Execution에서 sender를 파생한다.

`generic control <json>`, natural-language mutation, Store/Scheduler/Provider handle 노출을 금지한다.
