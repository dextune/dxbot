---
title: "CLI Reference Interface"
document_id: "DXB-IFC-041"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-IFC-040", "DXB-DOM-026", "DXB-DOM-027"]
---

# CLI Reference Interface

## 1. 목적

GUI 없이 Persistent Main Conversation/Thread/Live Control을 포함한 P0 Runtime 기능을 검증·자동화하는 Reference Interface를 제공한다. CLI는 shared Control Client만 사용한다.

## 2. Command Tree 후보

```text
dxb
├─ runtime      start | status | stop | doctor | version
├─ bot          create | list | show | activate | deactivate | archive | restore | delete
├─ conversation show | send | history
├─ thread       create | list | show | send | history | archive | restore | branch | tasks
├─ goal         create | list | show | pause | resume | close
├─ task         submit | list | show | graph | inspect | report | redirect | suspend | resume | cancel | retry | reprioritize | fork | result
├─ core         list | show | watch
├─ memory       put | get | search | history | promote | correct | archive | forget | compact | index
├─ routine      create | list | show | update | enable | disable | run-now | occurrences
├─ message      send | ask | delegate | inbox | outbox | trace
├─ capability   list | show | test
├─ provider     list | show | select-preview | deprecate | drain | detach
├─ plugin       list | show | install | validate | enable | disable | upgrade | rollback | uninstall
├─ reconcile    list | show | side-effect | waiting | suspension | directive | routine
├─ approval     list | show | approve | deny
├─ config       show | validate | diff | reload
└─ trace        show | follow | export
```

최종 subcommand 이름은 API schema freeze와 함께 확정한다.

## 3. Conversation / Thread UX

- `conversation`은 Bot당 하나의 Main Conversation을 대상으로 한다.
- `thread create`가 Interface Session 생성과 연결되지 않는다.
- `thread branch`는 source Thread/revision을 표시한다.
- history는 cursor pagination이며 전체 transcript를 stdout heap에 한 번에 materialize하지 않는다.
- Thread-local Memory와 Bot-global Memory를 scope로 구분해 표시한다.

## 4. Live Control UX

- `report`: committed/runtime/freshness를 구분
- `redirect`: expected revision/idempotency 지원
- `suspend`: accepted→safe-point→suspended 진행 상태 표시
- `resume`: consumed checkpoint/revision 표시
- `reprioritize`: requested priority/parallelism hint와 effective scheduler state를 구분
- `fork`: 새 Thread/Task IDs와 source lineage 출력

CLI가 `redirect`를 cancel+submit으로 로컬 합성하지 않는다.

## 5. Output / Scripting

Human/JSON/NDJSON의 v0.3 원칙을 유지한다. stdout schema와 stderr diagnostics를 분리하고 stale/degraded/awaiting-safe-point/reconciliation-required를 숨기지 않는다.

Ctrl-C는 기본적으로 client wait/subscription만 중단한다. running Task cancel은 명시 command/flag가 필요하다.

## 6. Recovery / Doctor

기존 Waiting/Side Effect/Provider/Routine/Plugin 진단에 다음을 추가한다.
- Bot without/duplicate Main Conversation
- orphan Thread/lineage
- Thread Memory scope orphan
- pending/stuck Directive
- suspended Task checkpoint invalid
- Provider Session stale token diagnostic

## 7. 검증 기준

- AT-CONV-001, AT-THREAD-001, AT-CTRL-002~005를 GUI 없이 재현 가능.
- non-TTY prompt/hang 없음.
- CLI crate가 Domain/Storage/Provider internals를 직접 의존하지 않음.
- `core` command가 direct Core Lease mutation API를 제공하지 않음.
