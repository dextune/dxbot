---
title: "Web Control Center"
document_id: "DXB-IFC-043"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P2"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-026", "DXB-DOM-027", "DXB-IFC-040", "DXB-RUN-032", "DXB-RUN-036"]
---

# Web Control Center

## 1. 목적

Bot당 하나의 Persistent Main Conversation을 사용자 중심 진입점으로 제공하고, 그 아래 Thread Graph와 병렬 Task/Execution/Core를 관찰·개입하는 Web Control Center를 정의한다. Web은 shared Control API만 사용한다.

## 2. 화면 구조

### Overview
Bot health, Task/Core queues, control queue health, Routine, resource/cost, approvals/incidents, Provider/Plugin status.

### Bot / Main Conversation
- persistent chat surface
- Identity/Goal/Memory summary
- Thread Navigator
- active Task/Core summary
- supervisor control input/status

### Thread
- conversation history with pagination
- lineage/branch/archive
- Thread-local Memory vs Bot-global Memory
- linked Tasks/Executions/Artifacts
- report/redirect/suspend/resume/cancel/reprioritize/fork

### Execution
Task/spec revision, Directive timeline, Waiting/Suspended, Core lanes, selected Provider binding, Tool trajectory, results/checkpoints, Side Effect reconciliation.

### 기존 화면
Memory, Network, Extensions(Capability/Provider/Plugin), Administration은 v0.3 의미를 유지한다.

## 3. Session UX

browser tab/window/WebSocket reconnect는 Interface Session일 뿐이다. UI route/tab 생성이 Thread create를 자동 의미하지 않는다. Session 종료와 Bot delete/Thread archive를 명확히 분리한다.

## 4. Supervisor UX

- 일반 자연어와 명시 control action 모두 shared server Command로 처리
- target Thread/Task를 명시/확인 가능
- expected revision/conflict 표시
- Directive accepted/committed/awaiting-safe-point/acked/superseded 표시
- report freshness/stale 표시
- “Core 추가”는 요청 hint와 effective allocation을 구분
- Provider native steering 존재 여부를 사용자에게 execution authority처럼 노출하지 않음

## 5. Frontend 경계

- generated/shared API client
- local store는 Projection
- Domain/Thread routing/control precedence/Scheduler/selector 재구현 금지
- BFF는 read composition만
- mutation은 Command API
- optimistic state는 pending CommandId/DirectiveId로만 표현

## 6. Real-time / Scale

initial snapshot + cursor, per-view filter, reconnect/resync, virtualized history/list, graph neighborhood budget, Artifact lazy load, bounded client history. hidden browser state에서는 high-frequency progress stream 감쇠 가능하나 server Directive state를 잃지 않는다.

## 7. Security

server-side authorization, CSRF/origin/CSP, XSS-safe untrusted output, sensitive cache invalidation, short-lived Artifact access, approval digest, control audit를 적용한다. Prompt/Provider output을 UI action authority로 변환하지 않는다.

## 8. 검증 기준

- AT-CONV-001 reconnect 후 동일 Main Conversation/Thread Graph 표시.
- AT-CTRL-005 Thread별 intervention isolation을 UI가 정확히 반영.
- Web이 DB/Runtime worker/Provider Session에 직접 접근하지 않음.
- large transcript/Thread graph에서 client memory bounded.
- frontend/backend version mismatch 명시 차단.
