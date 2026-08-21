---
title: "Web Control Center"
document_id: "DXB-IFC-043"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P2"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-026", "DXB-IFC-040", "DXB-RUN-032"]
---

# Web Control Center

## 1. 목적

다수 Bot/Task/Core/Memory/Routine/Provider/Plugin을 통합 관찰·운영하는 Control Center를 제공한다. Web은 Control API만 사용하고 Domain/DB/Provider/Plugin runtime에 직접 접근하지 않는다.

## 2. 화면 구조

### Overview
Bot health, Task/Core queues, Routine trigger health, resource/cost, approvals/incidents, Provider/Plugin status.

### Bot
Identity/Goal/Task/Memory/Communication/Routine/Permission/Resource timeline.

### Execution
Task DAG, Waiting dependencies, Core lanes, selected Provider, Tool trajectory, result/artifact, retry/cancel, Side Effect reconciliation summary.

### Memory
search/revision/provenance/relations/retention/archive/forget, canonical vs derived index health.

### Network
Bot topology, delegation chain, failed/expired messages.

### Extensions
Capability catalog, Provider health/deprecate/drain, Plugin install/enable/disable/upgrade/uninstall, permission/data policy.

### Administration
Policy/config/version/migration/audit/backup/maintenance.

## 3. Frontend 경계

- generated/shared API client
- local store는 UI Projection
- Domain/selector/Routine/Plugin lifecycle 재구현 금지
- BFF는 read composition만
- mutation은 Command API
- optimistic state는 command ID와 pending 표시

## 4. Control UX

- Bot/Core/Provider/Plugin을 서로 다른 개념으로 표현
- Session 종료와 Bot delete 분리
- deprecated/draining/removed와 unavailable 구분
- long-running operation의 progress/rollback 표시
- Plugin purge/Provider detach는 dry-run/impact/approval
- projection stale/watermark 표시

## 5. Real-time / Scale

initial snapshot + event cursor, per-view filter, reconnect/resync, virtualized list, graph neighborhood budget, Artifact lazy load, bounded client history를 사용한다. browser hidden state에서 high-frequency stream을 감쇠할 수 있다.

## 6. Security

server-side authorization, CSRF/origin/CSP, XSS-safe model/tool/plugin output rendering, sensitive cache invalidation, short-lived Artifact access, approval digest, admin audit, browser secret storage 최소화.

Plugin UI extension은 별도 sandboxed/signed surface가 설계되기 전에는 지원하지 않는다.

## 7. 검증 기준

- Web이 DB/Runtime internal Port에 직접 접근하지 않는다.
- Provider/Plugin 상태를 Brain/Bot identity로 오표현하지 않는다.
- permission revoke 후 cached sensitive data가 재노출되지 않는다.
- large list/graph에서 client memory가 bounded다.
- frontend/backend version mismatch가 명시 차단된다.
