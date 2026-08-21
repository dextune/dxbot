---
title: "TUI Interactive Terminal"
document_id: "DXB-IFC-042"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P1"
last_updated: "2026-08-21"
depends_on: ["DXB-IFC-040", "DXB-IFC-041", "DXB-DOM-026", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029"]
---

# TUI Interactive Terminal

## 1. 목적

Bot Main Conversation과 Project→Channel→Thread 협업 구조를 하나의 terminal UI에서 운영하되 Runtime logic, routing, permission, Memory conflict를 재구현하지 않는다.

## 2. Navigation

```text
Bots
  └─ Main Conversation / Threads
Projects
  └─ Channels
      ├─ Conversation
      ├─ Threads
      ├─ Members / Role / Authority
      ├─ Shared Memory
      └─ Tasks / Presence
```

Bot Main Conversation은 Project navigator와 독립적으로 계속 접근 가능하다.

## 3. Channel Workspace

- message stream/history pagination
- participant list
- Role와 effective Authority 별도 표시
- Derived Presence: speaking/processing/background/stale
- Thread navigator
- Channel/Project/Bot/Thread Memory scope 구분
- linked Task/delegation/Supervisor status
- report/redirect/suspend/resume/cancel/reprioritize

## 4. Client State

selected Bot/Project/Channel/Thread, viewport, event cursor, display cache, presence는 Derived client state다. UI pane/tab ID를 ChannelId/ThreadId로 사용하지 않는다.

snapshot → event stream → gap resync → typed command 패턴을 유지한다.

## 5. Performance

virtualized history/member/thread list, bounded event/presence cache, Artifact lazy load, server pagination을 사용한다. Project/Channel 전체 state를 client heap에 복제하지 않는다.

## 6. Security UX

- Role만 있고 Authority가 없으면 control action을 권한 있음처럼 표시하지 않는다.
- revoke/stale generation conflict를 명확히 표시한다.
- Prompt/Provider output을 자동 action으로 실행하지 않는다.

## 7. 검증 기준

- reconnect 후 동일 Project/Channel/Thread state를 server source에서 복원.
- 100-member Channel에서도 client state가 bounded.
- presence stale이 membership truth로 오표현되지 않음.
- TUI가 routing/Scheduler/Memory promotion policy를 계산하지 않음.
