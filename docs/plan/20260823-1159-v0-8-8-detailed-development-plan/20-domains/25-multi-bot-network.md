---
title: "Multi-Bot Network와 Delegation"
document_id: "DXB-DOM-025"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-DOM-020", "DXB-DOM-023", "DXB-ARC-010"]
---
# Multi-Bot Network와 Delegation

Delegation은 recipient Bot의 durable Task intent다. sender가 recipient Scheduler/Core/Memory를 직접 조작하지 않는다.

- Bot-executed sender는 current Execution/ActionGrant에서 server-side 파생한다.
- operator-issued sender는 requested BotId를 act-as authorization으로 resolve한다.

Delegation request 자체가 pre-accept 단계에서 거부되면 Receipt 없는 request error 또는 Receipt Rejected다. recipient Task가 생성된 뒤 lifecycle/capability/admission에서 거부·지연되면 operation은 Committed이고 Task outcome이 `Rejected/Deferred`다. 이를 “delegation receipt rejected”라고 부르지 않는다.

fan-out, participant/task/bytes/deadline/budget은 bounded하고 duplicate delegation은 duplicate Task effect를 만들지 않는다.
