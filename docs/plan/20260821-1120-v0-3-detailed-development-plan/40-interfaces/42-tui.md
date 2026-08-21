---
title: "TUI Interactive Terminal"
document_id: "DXB-IFC-042"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P1"
last_updated: "2026-08-21"
depends_on: ["DXB-IFC-040", "DXB-IFC-041", "DXB-DOM-026"]
---

# TUI Interactive Terminal

## 1. 목적

터미널에서 Bot/Task/Core/Memory/Network뿐 아니라 Routine/Provider/Plugin/Recovery 상태를 실시간 운영하되 Runtime 로직을 중복하지 않는다.

## 2. 주요 화면

- Bot Overview / Conversation
- Task Board/Graph + Waiting status
- Active Core/Provider timeline
- Routine list/next occurrence/history
- Memory search/revision/retention
- Bot Network/delegation
- Capability/Provider catalog + drain status
- Plugin lifecycle/permission/resource
- Side Effect reconciliation queue
- Approval/Audit/Trace
- Runtime health/config

MVP는 Bot/Task/Core/approval/health를 우선하며 Routine/Provider는 P1, Plugin 관리 pane은 Plugin runtime maturity에 맞춰 추가한다.

## 3. Client State

TUI local state는 selected view, server snapshot/watermark, pending command, event cursor, display cache 등 Derived state다. Domain state machine, selector, Plugin lifecycle, Routine schedule 계산을 로컬에서 재구현하지 않는다.

## 4. Synchronization

snapshot → event stream → sequence reducer → gap resync → typed Command. duplicate event는 idempotent reducer로 처리한다. Provider/Plugin generation 변경과 permission revoke 시 관련 cached pane을 invalidate한다.

## 5. UX 원칙

- Bot과 Core를 시각적으로 구분
- Provider는 Brain/Identity처럼 표현하지 않음
- Plugin은 Provider와 별도 lifecycle entity로 표시
- deprecated/draining/removed, stale projection, reconciliation required를 명시
- destructive detach/uninstall/data purge는 impact+approval
- UI 종료가 Bot/Task/Routine을 종료하지 않음

## 6. 성능

virtualized list, bounded event history, server pagination, token/event coalescing, Artifact lazy load, large graph neighborhood query를 사용한다. Plugin/Provider raw logs를 무제한 유지하지 않는다.

## 7. 예외

connection loss는 cached read-only + reconnect. cursor invalid는 resync. auth/permission revoke는 sensitive state clear. TUI crash는 Runtime을 중지시키지 않는다.

## 8. 검증 기준

- duplicate/gap/reconnect 후 correct view state를 만든다.
- 10k rows에서도 local memory가 bounded다.
- TUI crate가 Domain/Storage/Harness Provider/Plugin internal API를 직접 의존하지 않는다.
- Plugin disable/Provider drain이 server state로만 결정되고 UI local state가 authority가 아니다.
