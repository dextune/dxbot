---
title: "버전·호환성·Migration"
document_id: "DXB-ENG-053"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-015", "DXB-ARC-016", "DXB-ARC-017", "DXB-DOM-022", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-036", "DXB-IFC-040"]
---

# 버전·호환성·Migration

## 1. 목적

v0.4 Persistent Bot/Main Conversation/Thread/Memory/Task identity를 보존하면서 Project/Channel/Generic ScopeRef를 additive하게 도입한다. v0.4 fixture가 v0.5에서 동일 의미로 동작하는 것이 P0 gate다.

## 2. 신규 버전 대상

- Project/Project Membership schema/event
- Channel/Membership/Role/Authority schema/event
- ConversationParentRef
- MemoryScopeRef
- Memory promotion relation
- Task Project/Channel provenance/SupervisorRef
- Channel Orchestration wire/projection metadata
- Project/Channel API resources

Provider Contract/SDK/Plugin versioning discipline은 기존 규칙을 유지한다.

## 3. v0.4 → v0.5 Migration 원칙

기존:
```text
Bot
└─ Main Conversation
   └─ Thread
      └─ Thread Memory
```

는 그대로 유효하다.

Migration:
- 기존 Bot에 Project/Channel을 강제 생성하지 않는다.
- 기존 Main Conversation identity 유지.
- 기존 ThreadId/revision/lineage 유지.
- 기존 Bot Global Memory는 `Bot(BotId)` 의미 유지.
- 기존 Thread-scoped Memory는 actual Thread Canonical Record를 기준으로 `Thread(ThreadId)` ScopeRef에 연결.
- MemoryId/revision/content/provenance를 재생성하지 않는다.
- 기존 Thread→Bot promotion relation은 Generic Promotion의 특수 사례로 보존.

## 4. 금지 Migration

- 새 ThreadId 발급
- Provider Session ID로 Thread/Scope mapping 보정
- legacy Session을 Project/Channel로 자동 승격
- 기존 Memory content 재요약/재생성하여 새 record로 대체
- Bot Global Memory를 Project Memory로 자동 이동
- provenance/conflict/supersedes relation 삭제

## 5. Idempotency / Crash Safety

migration marker와 source/target schema version을 durable하게 관리한다. partial migration crash 후 동일 fixture를 재실행해 duplicate Project/Channel/Memory relation을 만들지 않는다.

## 6. API Compatibility

Project/Channel fields/resources는 additive를 우선한다. Bot-only API command에 ProjectId/ChannelId를 필수화하지 않는다. 기존 Thread/Memory/Task identifiers와 error semantics를 유지한다.

## 7. Rollback

- migration 전 backup/checkpoint와 compatibility preflight
- v0.4 binary가 신규 Project/Channel records를 이해하지 못하면 downgrade-blocked를 명시
- rollback이 기존 v0.4 data를 삭제하거나 새 Project/Channel data를 v0.4 Bot Memory로 섞지 않음
- external Side Effect와 schema rollback을 분리

## 8. Verification

AT-MIG-001:
Given representative v0.4 Bot/Main Conversation/Thread/Memory/Task/Directive fixtures,
When v0.5 migration 후 restart/recovery를 수행하면,
Then:
- BotId/ConversationId/ThreadId/MemoryId/revisions/provenance unchanged
- Bot-only workflow/Acceptance semantic unchanged
- Project/Channel rows are not fabricated for legacy path
- Thread Memory ScopeRef resolves to same Thread
- Provider Session is irrelevant to mapping

## 9. 검증 기준

- migration/rollback fixture가 deterministic/idempotent.
- v0.4 Acceptance suite가 v0.5 migrated data에서 통과.
- exact physical ScopeRef representation이 ADR 전 migration semantic을 왜곡하지 않음.
