---
title: "Control API와 프로토콜"
document_id: "DXB-IFC-040"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-026", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-036", "DXB-RUN-037", "DXB-ARC-014", "DXB-ARC-016", "DXB-ARC-017", "DXB-RUN-032"]
---

# Control API와 프로토콜

## 1. 목적

CLI/TUI/Web/자동화가 동일 API/schema로 Bot과 Project/Channel/Scoped Memory/Collaboration을 관리하게 한다. Interface Session/UI room/Provider Session/runtime worker를 public Domain identity로 노출하지 않는다.

## 2. Project Resource

- create/list/get/archive/restore
- membership list/get/change
- policy/resource/artifact references
- Memory search/proposal/promotion/history
- Channel list

Project delete/export는 retention/approval/operation semantics를 가진 장기 operation일 수 있다.

## 3. Channel Resource

- create/list/get/archive/restore
- membership join/leave/change
- Role Binding get/change
- Authority Binding get/change
- message append/history pagination
- Thread create/list/get/archive/restore/branch
- Channel Memory search/proposal/promotion
- participant/status/presence query
- linked Task/delegation/status

Role과 Authority를 동일 필드로 합치지 않는다.

## 4. Collaboration Control

공통 command:
- delegate
- inspect/report
- redirect
- suspend/resume
- cancel
- reprioritize

Command envelope는 기존 command ID/version/principal/target/idempotency/correlation/deadline/dry-run에 필요 시 다음을 포함한다.
- ProjectId / ChannelId / ThreadId / TaskId / ExecutionId
- expected Project/Channel/Membership/Authority/Task revision or generation
- SupervisorRef
- requested routing/parallelism/resource hint
- reason/approval metadata

응답은 accepted/committed/rejected/conflict/superseded, resulting revisions, DirectiveId, acknowledgement, observed_at/stale를 구분한다.

## 5. Message Contract

Channel Conversation Message는 Conversation Message schema 계열을 사용하되 ChannelId/ProjectId/optional ThreadId와 actor Bot/User principal을 명확히 한다.

다음은 별도 schema다.
- Conversation Message
- Bot Network Message
- Control Directive
- Memory Proposal

Message append가 Command/Memory commit을 자동 의미하지 않는다.

## 6. Memory Scope API

Memory resource는 `scope_ref`를 명시한다.

```text
Bot / Project / Channel / Thread
```

read/search 결과는 Canonical Memory revision/provenance/ScopeRef를 제공한다. semantic score/index result만 반환해 authorization 근거로 사용하지 않는다.

Promotion API는 source MemoryId/revision, target ScopeRef, expected target revision/policy generation, reason을 가진다.

## 7. Membership / Authority API

- Project Membership과 Channel Membership을 별도 resource로 표현한다.
- effective Authority와 Role을 별도 표시한다.
- generation/revision을 mutation precondition으로 제공한다.
- revoke 이후 stale client command는 stable conflict/forbidden semantic으로 실패한다.

정확한 error code registry는 schema ADR에서 freeze한다.

## 8. Session / Identity 경계

- `interface_session_*`: ephemeral connection/subscription
- `provider_session_*`: provider diagnostic opaque metadata
- `thread_id`, `project_id`, `channel_id`: persistent DXBOT Domain identity

UI tab/room/Provider session ID를 Domain ID alias로 직렬화하지 않는다.

## 9. Event Stream

추가 event/projection:
- Project/Channel lifecycle
- membership/Role/Authority changes
- Channel message/thread activity
- Memory proposal/promotion/conflict
- routing/participant selection summary
- Presence projection
- delegation/Supervisor/control status

Event stream은 at-least-once/cursor/resync이며 Runtime routing/control queue 자체가 아니다.

## 10. Backpressure

- request/response bytes cap
- history cursor pagination
- participant/presence list pagination or bounded window
- bounded event client buffers
- large Artifact streaming
- slow-client gap/resync
- no full Project/Channel history materialization

## 11. 검증 기준

- AT-PROJECT/CHANNEL/MEM/CTX/SEC/COLLAB Acceptance를 headless API로 재현 가능.
- Role와 Authority가 wire에서 분리됨.
- revoked generation의 stale client mutation이 실패함.
- Bot-only v0.4 API path가 Project/Channel 필수 field 없이 유지됨.
