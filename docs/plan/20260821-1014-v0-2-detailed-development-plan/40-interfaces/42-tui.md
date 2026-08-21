---
title: "TUI Interactive Terminal"
document_id: "DXB-IFC-042"
version: "0.1.0"
status: "Draft"
normative: true
priority: "P1"
last_updated: "2026-08-21"
depends_on: ["DXB-IFC-040", "DXB-IFC-041", "DXB-DOM-026"]
---


# TUI Interactive Terminal

## 1. 목적

터미널에서 Bot 대화, Task/Core 진행, Tool 실행, Memory/Message 상태를 실시간으로 다루되 Runtime 로직을 중복하지 않는 Interactive Interface를 제공한다.

## 2. 책임 범위

- 화면 구조와 navigation
- local state/store
- event stream synchronization
- input/command palette
- approval
- degraded/offline/reconnect
- terminal 성능과 접근성

TUI는 Control API의 client이며 Bot Brain·Task Scheduler·Permission 로직을 소유하지 않는다.

## 3. 주요 화면

- Bot Selector/Overview
- Bot Conversation/Task Composer
- Task Board/Graph
- Active Core Inspector
- Tool/Model Execution Timeline
- Memory Search/Revision/Conflict
- Bot Network Inbox/Delegation
- Approval Queue
- Runtime Health/Resource
- Config/Capability Viewer
- Audit/Trace Viewer

MVP는 Bot 대화, Task status, Core/Tool timeline, approval, runtime health로 제한한다.

## 4. Layout

```text
┌ Bot/Navigation ┬ Main Workspace                         ┐
│ Bots           │ Conversation / Task / Memory / Trace   │
│ Tasks          │                                        │
│ Cores          │                                        │
│ Memory         │                                        │
├────────────────┼────────────────────────────────────────┤
│ Runtime state  │ Context/status/action bar              │
└────────────────┴────────────────────────────────────────┘
```

좁은 terminal에서는 single-pane stack과 modal drawer로 전환한다. 색상만으로 상태를 구분하지 않는다.

## 5. Client State

TUI local store는 Derived UI state다.
- selected Bot/view
- server snapshot + watermark
- optimistic pending commands
- event sequence/cursor
- scroll/filter/focus
- draft input
- cached display models

Domain state를 로컬에서 재계산하지 않는다. Command 결과와 Event가 authoritative다. optimistic state는 command_id로 표시하고 실패 시 되돌린다.

## 6. 동기화 흐름

1. 연결 및 capability/version negotiation
2. authorized initial snapshot
3. event stream subscribe
4. sequence 순서로 reducer 적용
5. gap 발견 시 delta fetch 또는 resync
6. user action은 Command API로 전송
7. accepted/committed 상태를 local pending과 결합
8. reconnect 시 cursor resume
9. 오래된 local state 폐기

TUI event reducer는 같은 event 중복에 멱등이어야 한다.

## 7. 입력 모델

- text composer
- slash command/command palette
- keyboard shortcuts
- multi-line editor
- file/Artifact attach
- Bot mention/target selection
- confirmation/approval modal
- fuzzy search

CLI command syntax를 TUI가 문자열로 실행하지 않는다. 같은 typed client method를 호출한다.

## 8. Conversation View

대화는 Bot의 한 Interface View다.
- 현재 Bot/Task/Session context 명시
- message와 Task 생성의 차이 표시
- model/tool/Core timeline
- Memory로 승격된 항목 표시
- Harness Session ID는 보조 trace로만 표시
- UI 종료가 Bot/Task를 종료하지 않음
- 여러 Session/View 간 동일 Bot Event를 공유

## 9. Core/Task 시각화

- queued/admitted/running/quiescing/terminal
- Task graph와 work slices
- provider/tool/usage
- queue delay와 deadline
- cancellation state
- result fragments/conflicts
- stale heartbeat
- Core가 독립 Bot처럼 보이지 않도록 Bot 아래 실행 흐름으로 표현

## 10. Approval UX

- action, exact arguments digest, resource, Bot/Task/Core
- risk/side effect
- allow once / allow constrained / deny
- expiry
- policy source
- keyboard accidental approval 방지
- approval 후 변경된 request는 재승인
- non-interactive pending과 timeout 표시

## 11. 성능·메모리

- visible rows만 render
- virtualized lists
- bounded event history
- old detail은 server query
- model token delta coalescing
- frame rate와 input latency 분리
- expensive syntax highlight/background parsing 제한
- resize debounce
- string/line buffer reuse는 benchmark 후 적용
- local Artifact/content 전체 preload 금지

## 12. 예외상황

- connection loss: read-only cached view + reconnect banner
- cursor invalid: full resync
- terminal resize/zero size: render suspend
- color/Unicode 미지원: fallback
- slow event flood: coalesce UI-only events, Domain event는 cursor resync
- server command accepted 후 disconnect: command status lookup
- permission 변경: sensitive panes 즉시 clear/reload
- TUI crash: Bot/Task는 계속 실행
- pasted huge text: size cap와 Artifact 전환

## 13. 확장성

새 pane은 Query/View Model과 Command mapping으로 추가한다. UI-specific plugin은 Control API capability catalog를 사용하고 Runtime plugin을 직접 import하지 않는다. remote multi-node topology는 P2에서 view만 확장한다.

## 14. 구현 우선순위

- **P1:** Bot/task conversation, active Core, event sync, approval, health
- **P2:** Memory graph, Bot network, advanced trace, remote fleet
- **P3:** customizable layouts/plugin panes

## 15. 검증 기준

- TUI 종료/재시작이 Bot/Task 상태에 영향을 주지 않는다.
- event duplicate/gap/reconnect 테스트가 correct view state를 만든다.
- 10k Task/Memory row에서 UI 메모리가 bounded이고 input latency가 유지된다.
- TUI crate가 domain/storage/harness provider를 직접 의존하지 않는다.
- approval request digest가 변경되면 기존 승인 UI를 재사용하지 않는다.
- 좁은 terminal과 non-color 환경에서 핵심 상태를 식별할 수 있다.
