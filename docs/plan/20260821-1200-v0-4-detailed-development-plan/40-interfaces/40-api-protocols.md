---
title: "Control API와 프로토콜"
document_id: "DXB-IFC-040"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-026", "DXB-DOM-027", "DXB-RUN-036", "DXB-ARC-014", "DXB-ARC-016", "DXB-ARC-017", "DXB-RUN-032"]
---

# Control API와 프로토콜

## 1. 목적

CLI/TUI/Web/IDE/자동화가 하나의 Runtime 계약으로 Bot의 Persistent Main Conversation, Thread Graph, Task/Execution/Core, Memory, Provider/Plugin과 Live Control을 관리하게 한다. Interface Session이나 내부 Rust/DB/Provider handle을 public Domain identity로 노출하지 않는다.

## 2. Resource 표면

### Bot / Main Conversation
- Bot get/list/lifecycle
- Main Conversation get
- Main Conversation message append/history query
- Bot-level supervisor intent submit

Bot당 Main Conversation은 하나이며 API에서 `conversation create`를 일반 사용자 operation으로 반복 제공하지 않는다. provisioning/recovery가 owner다.

### Thread
- create/list/get
- archive/restore
- branch/fork
- message append/history pagination
- lineage/source revision
- linked Goal/Task/Execution/Core projection
- Thread-local Memory summary/promotion status

### Task / Execution / Live Control
- submit/get/list/graph
- inspect/report
- redirect
- suspend/resume
- cancel/retry
- reprioritize
- fork
- result/checkpoint/continuation/control-directive query

### 기존 Resource
Routine, Memory, Capability, Provider, Plugin, Approval, Policy/Config, Side Effect reconciliation, Operation, Audit/Trace는 v0.3 semantics를 유지한다.

## 3. Session 경계

API schema에서 세 의미를 분리한다.

- `interface_session_*`: connection/subscription/cursor 범위의 ephemeral metadata
- `provider_session_*`: 기본 public Domain API에 노출하지 않으며 diagnostics가 필요하면 provider-scoped opaque metadata
- `thread_id`: persistent DXBOT Domain identity

Provider Session ID를 ThreadId/ConversationId alias로 직렬화하지 않는다. Interface reconnect는 Thread 생성 API를 자동 호출하지 않는다.

## 4. Message Contract

Conversation Message는 최소 message ID, Bot/Conversation ID, optional ThreadId, actor/principal class, sequence/revision, content 또는 Artifact ref, correlation/causation, trust/sensitivity, retention metadata를 가진다.

Conversation Message와 Bot Network Message는 별도 schema/type을 사용한다. Message append가 Memory commit을 의미하지 않는다.

## 5. Thread Contract

Thread response는 최소:
- ThreadId / BotId / ConversationId
- revision
- lifecycle/archive state
- parent/source ThreadId + source revision where applicable
- title/summary projection refs
- linked Task/Execution summary
- local Memory summary
- message watermark/pagination cursor
- created/updated provenance

정확한 lifecycle enum은 ADR 이전에 wire major semantic으로 과고정하지 않는다.

## 6. Live Control Command Envelope

기존 command envelope의 command ID/version, principal, target, expected revision/generation, idempotency, correlation/causation, deadline, dry-run, payload를 유지하고 live control에는 필요 시 다음을 포함한다.

- target ThreadId/TaskId/ExecutionId
- expected Task/spec revision
- control kind
- reason
- requested priority/parallelism/resource hint
- fork target metadata

응답:
- accepted/committed/rejected/conflict/superseded
- DirectiveId
- resulting Task/spec revision
- acknowledgement state
- safe-point/awaiting-provider state
- OperationId if long-running
- observation timestamp/freshness/stale
- stable error/warnings

`accepted`와 `completed`를 구분한다.

## 7. Inspect / Report

Query/report response는 다음을 구분한다.
- last committed Canonical state/revision
- live Runtime state
- active Execution/Core Lease/Provider binding
- last heartbeat/progress marker
- pending Directive/ack
- observed_at
- freshness/stale/degraded

Provider가 progress를 지원하지 않는 경우 이를 임의 추측하지 않는다.

## 8. Redirect / Suspend / Resume Semantics

- redirect: new Task Specification revision + Directive; old Execution mutation 금지
- suspend: durable suspension/checkpoint 진행을 반환
- resume: expected suspended revision + idempotency, new Execution semantic
- reprioritize: Scheduler input revision이며 direct Core count mutation API가 아님
- fork: new Thread/Task lineage, source mutation 없음

API client가 이를 cancel/new-task 조합으로 임의 재구현하지 않는다.

## 9. Event Stream

추가 이벤트/Projection:
- Conversation message appended
- Thread created/archived/restored/branched
- Thread Task linkage
- Memory promotion status
- Directive committed/delivered/acknowledged/superseded
- Execution yield/suspend/resume progress
- report freshness/progress

Event stream은 at-least-once/cursor/resync이며 Runtime Control Channel 자체가 아니다.

## 10. Capability / Provider / Plugin

v0.3의 Capability contract, Provider lifecycle/health/generation/config/SDK/Conformance, Plugin lifecycle/data/permission API를 그대로 유지한다. Provider native steering/cancellation 지원은 Capability metadata로 discovery할 수 있으나 Domain control command를 대체하지 않는다.

## 11. Unsupported / Error Semantics

기존 `capability-unavailable`, `provider-not-ready/degraded/quarantined/incompatible/deprecated/removed`, `plugin-disabled/incompatible`, `configuration-stale`, `reconciliation-required`를 유지한다.

v0.4 추가 stable semantic 후보:
- `thread-archived`
- `thread-revision-conflict`
- `control-conflict`
- `control-superseded`
- `awaiting-safe-point`
- `suspension-checkpoint-invalid`
- `provider-session-unavailable` (diagnostic only)

정확한 code registry는 schema ADR에서 freeze한다.

## 12. Backpressure / Performance

- request/response size cap
- message/history cursor pagination
- bounded event/control client buffers
- large content Artifact streaming
- slow-client resync
- no full Thread transcript materialization
- control retry는 idempotency key 사용, client-side busy loop 금지

## 13. 검증 기준

- AT-IFC-001/002 headless/interface separation 유지.
- AT-CONV-001/AT-THREAD-001 resource semantics가 CLI/TUI/Web 동일 schema로 표현됨.
- AT-CTRL-002~005가 public command/query/event로 관찰 가능.
- Provider Session/Interface Session/Thread field가 혼용되지 않음.
- slow history/event client가 server memory를 무제한 증가시키지 않음.
