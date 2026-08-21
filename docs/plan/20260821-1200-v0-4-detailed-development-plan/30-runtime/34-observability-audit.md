---
title: "관측성·Trace·Audit"
document_id: "DXB-RUN-034"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-017", "DXB-RUN-032", "DXB-RUN-033", "DXB-RUN-036"]
---

# 관측성·Trace·Audit

## 1. 목적

`Bot → Main Conversation → Thread → Task → Directive → Execution → Core Lease → Provider Host → Provider → Memory/Recovery`를 하나의 공통 correlation으로 추적한다.

## 2. Correlation

가능한 범위에서:
- trace/span, command, correlation/causation
- BotId / ConversationId / ThreadId / MessageId
- Goal/Task/TaskSpecRevision
- DirectiveId/control epoch/ack
- Execution/CoreLease
- Routine/Occurrence
- SideEffectEntry
- Capability/Provider ID/version/generation
- policy/config generation

Metric label에 unbounded user content/ID를 직접 넣지 않는다.

## 3. Conversation / Thread Metrics

- active/archived Thread count
- Thread lineage node/depth/branch rate
- message append rate/history bytes/cold-archive bytes
- history retrieval latency/bytes
- Thread-local vs Bot-global Memory bytes/items
- promotion proposal/accepted/rejected/conflict
- Context Plan selected history/memory/token/byte budget

Conversation raw content는 metric label이 아니다.

## 4. Live Control Metrics

- control queue items/bytes/latency/reject/coalesce
- inspect/report freshness/stale age
- Directive commit→delivery→ack latency
- redirect commit→old yield→new Execution start latency
- suspend request→quiesced latency
- resume duplicate/conflict/supersede count
- safe-point/provider cancellation capability
- control-induced Scheduler yield/rebalance

## 5. Provider Host / Scheduler / Memory

v0.3 automatic telemetry를 유지한다: selection, lifecycle, admission, deadline/cancel, call outcome, error/retry disposition, output/usage, side-effect state, drain/quiescence. Scheduler queue/fairness와 Memory canonical/derived metrics를 분리한다.

## 6. Report Freshness

report 응답은 observation timestamp, last committed revision, last heartbeat/progress, Provider call state, stale/degraded를 기록해 Hard Real-Time처럼 오해되지 않게 한다.

## 7. Audit

Bot lifecycle, Conversation/Thread destructive archive/delete/export, Memory promotion/forget, permission/policy, high-risk Tool/Side Effect, delegation, Provider/Plugin lifecycle와 함께 redirect/suspend/resume/cancel/reprioritize/fork를 감사한다.

## 8. Retention

Domain/Audit/Conversation History/Trace/Trajectory/Profile을 구분한다. duplicate payload를 피하고 Artifact digest/reference를 사용한다. UI/Provider diagnostic/control report buffer는 bounded다.

## 9. 검증 기준

- CommandId 또는 Conversation Message에서 Thread→Task→Directive→Execution→Host outcome까지 추적 가능.
- report stale/freshness가 명시됨.
- control queue/Directive ack gap을 탐지 가능.
- secret canary가 conversation/log/trace/audit/provider diagnostic에 나타나지 않음.
