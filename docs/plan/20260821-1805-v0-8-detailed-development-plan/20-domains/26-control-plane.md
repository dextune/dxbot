---
title: "DXBOT Control Plane과 Typed Use-Case Surface"
document_id: "DXB-DOM-026"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-020", "DXB-DOM-025", "DXB-ARC-010"]
---

# DXBOT Control Plane과 Typed Use-Case Surface

## 1. 목적

Bot/Conversation/Thread/Task/Memory/Project/Channel/Process를 하나의 Headless Application boundary로 운영하면서 Domain rule, Authorization, Scheduler, Provider Host를 Interface에 복제하지 않는다.

## 2. Typed Operation Ownership

각 외부 use-case는 하나의 Application operation에 귀결된다.

| Use-case family | Canonical Application owner |
|---|---|
| Bot lifecycle | `CreateBot`, `ChangeBotLifecycle`, `Get/ListBot` |
| Message send | `SendMessage` |
| Conversation/Thread history | `GetConversation`, `ListMessages`, `BranchThread` |
| Task | `SubmitTask`, `ControlTask`, `Get/List/WatchTask` |
| Memory | `Get/Search/HistoryMemory`, `Propose/PromoteMemory` |
| Project/Channel | typed lifecycle/membership/message operations |
| Process | `Get/WatchProcess`; typed control은 child owner operation 재사용 |
| Operation recovery | `GetOperation`, `ReconcileOperation` |

`conversation send`, `thread send`, `channel send`는 서로 다른 Backend mutation을 만들지 않고 parent selector를 가진 하나의 `SendMessage`를 재사용한다. CLI alias가 Application operation을 복제하지 않는다.

## 3. 금지 Surface

- `control <json>` 또는 arbitrary method/payload dispatch
- `process control <verb>`가 typed Task/Memory/Approval operation을 우회
- CLI가 Domain method·Store·Scheduler·Provider concrete API 호출
- free-form 자연어를 authorization된 mutation으로 바로 commit
- local Role/profile/last-used target을 authority나 canonical target으로 사용

## 4. Selector와 Authority

Interface input은 `ResourceSelector`일 수 있으나 Application은 `ResolvedResourceRef`로 확정한 뒤 current authorization, scope, expected revision/generation을 owner에서 재검증한다. 후보 해석은 authority를 생성하지 않는다.

## 5. Query freshness

status/list/show는 canonical revision, projection watermark, `observed_at`, stale/degraded/source를 구분한다. Presence, index, provider health를 membership/Task/Memory truth로 표시하지 않는다.

## 6. Process independence

Control Connection/CLI 종료가 Bot membership, Task, Directive, Durable Process를 변경하지 않는다. explicit typed Command와 Operation Receipt만 mutation을 추적한다.

## 7. 검증 기준

- P0 CLI command마다 Application operation 하나가 존재한다.
- `SendMessage` semantic의 중복 구현 0.
- generic mutation escape hatch 0.
- selector resolve 후 Runtime authorization/revision 재검증.
- direct Domain/Store/Scheduler/Provider handle 노출 0.
