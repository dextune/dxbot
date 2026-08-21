---
title: "CLI Reference Interface"
document_id: "DXB-IFC-041"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-IFC-040", "DXB-DOM-026", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029"]
---

# CLI Reference Interface

## 1. 목적

GUI 없이 v0.5 Project/Channel/Scoped Memory/Multi-Bot collaboration을 자동화·검증하는 Reference Interface를 제공한다. CLI는 shared Control Client만 사용한다.

## 2. Command Tree 후보

```text
dxb
├─ runtime      start | status | stop | doctor | version
├─ bot          create | list | show | activate | deactivate | archive | restore | delete
├─ conversation show | send | history
├─ project      create | list | show | archive | restore | members | memory | channels
├─ channel      create | list | show | archive | restore | join | leave | members | roles | authorities | send | history | threads | memory | tasks | status
├─ thread       create | list | show | send | history | archive | restore | branch | tasks
├─ goal         create | list | show | pause | resume | close
├─ task         submit | list | show | graph | inspect | report | redirect | suspend | resume | cancel | retry | reprioritize | delegate | result
├─ core         list | show | watch
├─ memory       put | get | search | history | propose | promote | correct | archive | forget | compact | index
├─ routine      create | list | show | update | enable | disable | run-now | occurrences
├─ message      send | ask | delegate | inbox | outbox | trace
├─ capability   list | show | test
├─ provider     list | show | select-preview | deprecate | drain | detach
├─ plugin       list | show | install | validate | enable | disable | upgrade | rollback | uninstall
├─ reconcile    list | show | side-effect | waiting | suspension | directive | membership | memory
├─ approval     list | show | approve | deny
├─ config       show | validate | diff | reload
└─ trace        show | follow | export
```

최종 subcommand 이름은 API schema freeze에서 확정한다.

## 3. Project / Channel UX

- `project members`와 `channel members`는 분리한다.
- Role과 effective Authority를 별도 컬럼/JSON field로 표시한다.
- Channel presence는 `observed_at/stale`가 있는 Derived 상태로 표시한다.
- history는 cursor pagination이며 전체 transcript를 메모리에 적재하지 않는다.
- Channel join/leave가 Bot create/delete를 의미하지 않는다.

## 4. Memory UX

scope를 명시적으로 선택한다.
- Bot
- Project
- Channel
- Thread

`memory promote`는 source revision과 target Scope를 표시하고 blind copy처럼 표현하지 않는다.

## 5. Collaboration UX

- `delegate`는 target Bot/Task/Channel provenance를 명시
- `report/redirect/suspend/...`는 effective Authority/Supervisor conflict를 명시
- stale membership generation은 error로 보여주고 로컬에서 자동 재시도해 권한을 우회하지 않는다.
- “Core 추가”는 requested hint와 effective Scheduler allocation을 구분한다.

## 6. 검증 기준

- 신규 Acceptance를 CLI만으로 재현 가능.
- CLI가 Role label로 control 가능 여부를 로컬 판정하지 않음.
- CLI 종료가 Channel Task/Directive/Bot lifecycle을 변경하지 않음.
- `core` command에 direct Lease mutation API가 없음.
