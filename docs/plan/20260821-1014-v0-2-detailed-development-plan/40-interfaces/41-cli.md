---
title: "CLI Reference Interface"
document_id: "DXB-IFC-041"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-IFC-040", "DXB-DOM-026"]
---

# CLI Reference Interface

## 1. 목적

GUI 없이 Runtime의 핵심 기능과 Routine/Provider/Plugin/Recovery 상태를 검증·자동화하는 Reference Interface를 제공한다. CLI는 Control Client만 사용하며 Domain/Storage/Provider를 직접 import하지 않는다.

## 2. Command Tree

```text
dxb
├─ runtime      start | status | stop | doctor | version
├─ bot          create | list | show | activate | deactivate | archive | restore | delete
├─ goal         create | list | show | pause | resume | close
├─ task         submit | list | show | graph | cancel | retry | wait | result
├─ routine      create | list | show | update | enable | disable | run-now | occurrences
├─ core         list | show | watch | cancel
├─ memory       put | get | search | history | correct | archive | forget | compact | index
├─ message      send | ask | delegate | inbox | outbox | trace
├─ capability   list | show | test
├─ provider     list | show | select-preview | deprecate | drain | detach
├─ plugin       list | show | install | validate | enable | disable | upgrade | rollback | uninstall
├─ reconcile    list | show | side-effect | waiting | routine
├─ approval     list | show | approve | deny
├─ config       show | validate | diff | reload
├─ trace        show | follow | export
└─ completion
```

실제 subcommand naming은 Control API schema와 함께 freeze한다. CLI 문자열 parsing으로 Domain semantics를 별도 구현하지 않는다.

## 3. Output

- Human: stale/degraded/deprecated/removed/reconciliation-required를 숨기지 않음
- JSON: stdout 단일 schema document, logs/progress stderr
- NDJSON: watch/stream, sequence/cursor 포함
- secret/raw sensitive payload redaction
- large content는 Artifact reference

## 4. Exit / Error

success, usage, authorization/approval, conflict, unavailable/degraded, timeout, partial, incompatible version, interrupt를 stable 범주로 관리한다. 제거된 Provider/disabled Plugin은 일반 “not found”로 뭉개지 않고 API stable error를 보존한다.

## 5. Routine UX

- schedule/trigger preview
- next/last occurrence
- missed/overlap policy
- `run-now`는 일반 Task 생성 결과를 표시
- restart reconciliation/duplicate prevention 상태 확인

## 6. Provider / Plugin UX

Provider detach와 Plugin uninstall은 기본 dry-run/impact summary를 제공한다. in-flight count, dependent capability/config, data policy, rollback 가능성을 보여준다. destructive plugin data purge는 structured approval을 우회하지 않는다.

## 7. Recovery UX

`doctor`와 `reconcile`은 Waiting without Continuation, Side Effect Unknown, stale Provider config, Routine duplicate/missed occurrence, Plugin migration marker를 조회할 수 있다. 자동 repair는 safe/idempotent 항목만 수행한다.

## 8. Scripting

- non-TTY prompt 금지
- idempotency key 지원
- command commit과 wait timeout 분리
- Ctrl-C는 기본 wait 구독만 중단; Task cancel은 명시 flag
- secret는 argv에 직접 받지 않음
- output file atomic write

## 9. 검증 기준

- 모든 P0 use case와 AT-MOD/ROUTINE/TASK/SFX/REPO 진단을 GUI 없이 실행할 수 있다.
- JSON stdout에 로그가 섞이지 않는다.
- non-TTY hang이 없다.
- CLI crate가 Domain/Storage/Provider/Plugin internal crate를 직접 의존하지 않는다.
