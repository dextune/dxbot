---
title: "저장소와 데이터 모델"
document_id: "DXB-ARC-015"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-011", "DXB-ARC-017"]
---

# 저장소와 데이터 모델

## 1. 목적

v0.6의 Bot/Project/Channel/Conversation/Thread/Task/Execution/Memory/Durable Process/ActionGrant persistent model을 그대로 유지하고, v0.7 Interface 도입이 새 duplicate Canonical store를 만들지 않도록 한다.

## 2. v0.6 Durable State 유지

v0.6에서 정의된 다음 의미는 unchanged다.
- Durable Process identity/progress/command/outcome refs
- ActionGrant issue/consume/revoke
- Memory revision/epistemic/temporal/evidence/revalidation metadata
- existing Bot/Project/Channel/Conversation/Thread/Task/Execution/Directive/Side Effect state

v0.7 Interface를 위해 기존 ID/revision/content/provenance를 변경하지 않는다.

## 3. CLI / Control State Classification

### Persistent Canonical로 만들지 않음
- CLI Session
- Control Connection/socket ID
- terminal width/color
- local watch cursor cache
- rendering/progress state
- local completion cache
- endpoint reconnect backoff

필요한 local profile/config는 client configuration이지 Runtime Canonical truth가 아니다.

### Durable receipt가 필요한 경우
Command idempotency/reconciliation에 durable receipt가 필요하면 Application/Command owner의 기존 journal/inbox/outbox/idempotency boundary에 둔다. `cli-command-table` 같은 Interface-specific canonical 원본을 별도로 만들지 않는다.

## 4. DTO / Storage 분리

```text
Public Wire DTO ≠ Domain Struct ≠ Persistence Row
```

protocol/schema version field를 저장소 physical schema와 1:1로 강제하지 않는다. Runtime data schema migration과 CLI protocol compatibility는 별도로 관리한다.

## 5. Subscription State

Event cursor가 durable Canonical state를 참조할 수는 있으나 subscriber별 socket/buffer를 durable business truth로 저장하지 않는다.

- cursor retention/resync source는 event/projection owner가 제공
- disconnect 후 stale buffer를 replay source로 사용 금지
- gap이면 bounded resync/query 사용

## 6. Migration

v0.6→v0.7은 이상적으로 Backend Canonical data migration을 요구하지 않는다.

- BotId/ConversationId/ThreadId/ProjectId/ChannelId/TaskId/ExecutionId/MemoryId/Durable Process/Directive/Side Effect identity 유지
- Interface 개발을 위해 새 Domain ID 발급 금지
- protocol/schema compatibility migration과 data migration 분리

## 7. 검증 기준

- CLI-specific canonical table/state 원본 0.
- Control Connection/Session이 Domain identity로 저장되지 않는다.
- idempotency receipt가 CLI local state에만 의존하지 않는다.
- public DTO와 persistence row 직접 공유 0.
- v0.6 data migration requirement 증가 0이 기본이며 예외는 ADR/fixture로 증명한다.
