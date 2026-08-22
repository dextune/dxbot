---
title: "Typed Control Plane Use-Case Surface"
document_id: "DXB-DOM-026"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-DOM-020", "DXB-DOM-025", "DXB-ARC-010"]
---
# Typed Control Plane Use-Case Surface

모든 external use-case는 하나의 Application operation에 귀결된다.

| Family | Typed owner |
|---|---|
| Bot | `CreateBot`, `ChangeBotLifecycle`, `Get/ListBot` |
| Message | parent selector를 가진 `SendMessage` |
| Task/Delegation | `SubmitTask`, `ControlTask`, `Get/List/WatchTask` |
| Memory | `Get/Search/History`, `Propose/PromoteMemory` |
| Project/Channel membership | `SetScopeMembership`, `RemoveScopeMembership`, `ListScopeMemberships` |
| Project/Channel lifecycle | typed create/get/list/archive/restore |
| Operation recovery | `GetOperation`, `ReconcileOperation` |
| Side effect recovery | `ReconcileSideEffect` |

`SetScopeMembership`은 Project/Channel scope kind, member BotId, RoleRef, AuthorityBindingRef, expected scope revision과 expected membership generation/absent 조건을 받는다. Project와 Channel canonical membership row는 별도 owner가 유지하되 Application operation metadata와 validation pipeline을 공유한다.

`SubmitTask`는 optional `DelegationEnvelope { sender_bot, recipient_bot, correlation, deadline, budget }`를 가진다. recipient Bot의 current lifecycle, authorization, capability, admission을 검증하고 sender가 recipient Core/Scheduler를 직접 조작하지 않는다.

generic `control <json>`, free-form natural-language mutation, Store/Scheduler/Provider handle 노출을 금지한다.
