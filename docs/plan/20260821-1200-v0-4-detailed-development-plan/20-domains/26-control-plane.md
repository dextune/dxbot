---
title: "Bot Control Plane과 Main Conversation Supervisor Surface"
document_id: "DXB-DOM-026"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-ARC-016", "DXB-ARC-017", "DXB-DOM-020", "DXB-DOM-025", "DXB-DOM-027"]
---

# Bot Control Plane과 Main Conversation Supervisor Surface

## 1. 목적

Bot/Main Conversation/Thread/Task/Core/Memory/Routine/Capability/Provider/Plugin을 중앙에서 관찰·운영하고, running Task를 실시간으로 report/redirect/suspend/resume/reprioritize할 수 있게 하되 Domain/Persistence/Scheduler/Provider Host를 우회하지 않는다.

## 2. 원칙

- 모든 mutation은 Application Command + Canonical Owner를 통한다.
- Query는 projection watermark/freshness/stale 상태를 표시한다.
- Main Conversation은 사용자-facing Communication + Supervisor Control Surface이나 authority는 authenticated Control Command가 가진다.
- Control Plane은 Brain/Thread/Task/Scheduler/Provider lifecycle owner가 아니다.
- DB/worker/process/Core handle 직접 조작 금지.
- Runtime Control Channel과 live semantics는 `DXB-RUN-036`이 소유한다.

## 3. Main Conversation 기능

하나의 Bot surface에서:
- 일반 Bot communication
- Thread create/list/select/archive/restore/branch
- Thread message/history
- Thread↔Task/Execution status
- active Core/Provider binding 관찰
- inspect/report
- redirect/suspend/resume/cancel/reprioritize/fork
- Memory provenance/promotion interaction

Main Conversation message 자체가 control authority가 아니며 intent routing 후 authorization/expected revision/idempotency를 거친다.

## 4. 기능 영역

### Bot / Conversation / Thread
create/activate/deactivate/archive/restore/delete, Main Conversation 조회/메시지, Thread create/list/get/archive/restore/branch, Thread lineage/local Memory summary, history pagination.

### Task / Execution / Core
submit/inspect/report/cancel/retry/redirect/suspend/resume/reprioritize/fork, DAG/delegation, Waiting/Suspended Continuation, active Core/Lease, result/trace, side-effect reconciliation.

### Routine
create/update/enable/disable/run-now, occurrence/restart reconciliation.

### Memory
search/revision/provenance/scope/conflict/retention/compaction/archive/forget, Thread→Bot promotion proposal/decision, canonical vs derived bytes.

### Capability / Provider / Plugin
v0.3의 Contract/version/lifecycle/health/generation/config/conformance/drain/removal/permission/data lifecycle 관리 의미를 그대로 유지한다.

## 5. Live Control Result

Control response는 최소 다음을 제공한다.
- command/directive ID
- target Bot/Thread/Task/Execution + expected/resulting revision
- accepted/committed/rejected/conflict/superseded
- acknowledgement state
- current canonical/runtime summary
- safe-point/awaiting-provider status
- operation ID if long-running
- freshness/stale timestamp
- stable error/warning

Provider가 hard cancellation을 지원하지 않으면 `accepted`를 `completed`로 오표현하지 않는다.

## 6. Query / Report Freshness

`inspect/report`는 last committed state, live runtime state, heartbeat/progress, observation time, stale/degraded flag를 구분한다. Control queue가 일반 Work Queue 뒤에서 starvation되지 않아야 한다.

## 7. 장기 Operation

Provider/Plugin lifecycle, Memory archival, Side Effect reconciliation과 함께 Thread 대량 archive/export 또는 destructive Bot deletion도 Operation으로 표현할 수 있다. client disconnect가 Operation을 취소하지 않는다.

## 8. 안전장치

- destructive/bulk dry-run
- expected revision/generation
- structured approval
- Thread/Task scoped authorization
- control rate/resource limit
- audit reason
- migration/compatibility/rollback preflight
- Prompt/Provider output이 control authority를 생성하지 못함

## 9. 검증 기준

- AT-CTRL-001: Control Plane 종료가 Runtime 의미를 바꾸지 않는다.
- AT-CTRL-002: Work Queue 포화에서도 report/inspect starvation이 없다.
- AT-CTRL-003: redirect가 immutable Execution을 mutation하지 않는다.
- AT-CTRL-004: suspend/resume가 crash 후 정확히 한 번 재개된다.
- AT-CTRL-005: Thread A control이 Thread B/C Execution에 영향을 주지 않는다.
- CLI/TUI/Web이 같은 Conversation/Thread/Control schema를 사용한다.
- Provider/Plugin 상태는 기존 Common Lifecycle semantics를 유지한다.
