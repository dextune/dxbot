---
title: "Web Control Center"
document_id: "DXB-IFC-043"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P2"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-026", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-IFC-040", "DXB-RUN-032", "DXB-RUN-036", "DXB-RUN-037"]
---

# Web Control Center

## 1. 목적

기존 Bot Persistent Main Conversation UX를 유지하면서 Project navigator와 Channel collaboration UX를 추가한다. Web은 shared Control API만 사용한다.

## 2. 화면 구조

### Bots
- Bot Main Conversation
- Bot Identity/Goal/Memory
- Bot Threads/Tasks/Core status

### Projects
- Project identity/membership/resources/Memory
- Channel navigator

### Channel
- persistent conversation
- Thread navigator
- member list
- Role / effective Authority
- Presence projection
- Shared Memory
- delegated/managed Tasks
- report/redirect/suspend/resume/cancel/reprioritize

### Execution / Existing Operations
v0.4 Task/Execution/Core/Provider/Directive/Side Effect/Recovery와 Capability/Provider/Plugin/Admin 화면을 유지한다.

## 3. Identity / Session UX

browser tab/route/WebSocket은 Interface Session/view다. route 생성이 Project/Channel/Thread create를 의미하지 않는다. Provider Session을 Channel/Thread identity로 노출하지 않는다.

## 4. Presence / Role UX

Presence는 `observed_at`, processing/speaking/background 등의 Derived 상태이며 member truth와 분리한다. Role과 Authority를 별도 표시한다.

## 5. Supervisor UX

- natural-language input과 explicit action 모두 server Command로 변환
- target Bot/Thread/Task/Supervisor scope 확인
- membership/authority revision conflict 표시
- Directive accepted/acked/awaiting-safe-point/stale 표시
- requested participant/Core hint와 effective admission을 구분

## 6. Frontend 경계

- generated/shared API client
- local store는 Projection
- BFF는 read composition만
- mutation은 Command API
- routing/permission/Scheduler/Memory conflict policy 재구현 금지
- optimistic state는 pending CommandId/DirectiveId만 표현

## 7. Scale

initial snapshot + cursor, virtualized Channel history/thread/member lists, per-view filter, Artifact lazy load, bounded client state, reconnect/resync를 사용한다. hidden tab에서는 high-frequency Presence를 감쇠할 수 있으나 Canonical Directive/Message state를 잃지 않는다.

## 8. 검증 기준

- AT-PROJECT/CHANNEL/COLLAB 상태를 shared API와 동일하게 표시.
- revoke 후 stale UI cache가 action authority를 유지하지 않음.
- 10k-thread/long-history project에서 client memory bounded.
- Web 제거가 Project/Channel Domain lifecycle을 변경하지 않음.
