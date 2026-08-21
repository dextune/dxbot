---
title: "TUI Interactive Terminal"
document_id: "DXB-IFC-042"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P1"
last_updated: "2026-08-21"
depends_on: ["DXB-IFC-040", "DXB-IFC-041", "DXB-DOM-026", "DXB-DOM-027"]
---

# TUI Interactive Terminal

## 1. 목적

터미널에서 Bot의 Main Conversation을 중심으로 Thread/Task/Core/Memory/Live Control과 Provider/Plugin/Recovery 상태를 실시간 운영하되 Runtime logic을 재구현하지 않는다.

## 2. 주요 화면

- **Bot Main Conversation**: message stream + supervisor input
- **Thread Navigator**: list/lineage/archive/branch/current Thread
- **Thread Workspace**: local history/Memory/Task/Artifact summary
- **Task Board/Graph**: Waiting/Suspended/Directive 상태
- **Active Execution/Core Timeline**: Provider binding + safe-point/control progress
- Routine / Memory / Bot Network
- Capability/Provider / Plugin lifecycle
- Side Effect reconciliation
- Approval/Audit/Trace / Runtime health

## 3. Main Conversation UX

하나의 Bot pane에 여러 Interface reconnect가 붙어도 같은 Main Conversation을 표시한다. TUI tab/pane ID를 ThreadId로 사용하지 않는다.

Supervisor input은 일반 대화와 control intent를 표현할 수 있으나 실제 mutation은 shared typed Command로 변환되어 authorization/revision/idempotency를 거친다.

## 4. Live Control UX

- running Thread/Task 옆에 inspect/report/redirect/suspend/resume/cancel/reprioritize/fork action
- Directive committed/acknowledged/awaiting-safe-point/superseded 상태 표시
- report의 `observed_at`/stale 표시
- reprioritize requested vs effective Core allocation 구분
- Side Effect Unknown이면 control UI가 retry처럼 표현하지 않음

## 5. Client State / Synchronization

local selected Bot/Thread/view, snapshot watermark, pending command, event cursor, display cache는 Derived다. snapshot → event stream → gap resync → typed command 패턴을 유지한다.

TUI가 Thread lifecycle, Task revision, selector, Scheduler, control precedence를 로컬에서 계산하지 않는다.

## 6. 성능

virtualized history/thread list, bounded local event/history cache, server pagination, Artifact lazy load, graph neighborhood query를 사용한다. 장기 Conversation transcript 전체를 local memory에 유지하지 않는다.

## 7. 검증 기준

- reconnect/duplicate/gap 후 같은 Main Conversation/Thread state 복원.
- Thread A redirect/B report/C suspend가 UI local state가 아니라 server state로 판정됨.
- 10k Thread/message view에서도 local memory bounded.
- TUI 종료가 Bot/Task/Directive를 취소하지 않음.
