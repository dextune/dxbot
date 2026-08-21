---
title: "저장소와 데이터 모델"
document_id: "DXB-ARC-015"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-011", "DXB-ARC-017"]
---

# 저장소와 데이터 모델

## 1. 목적

v0.4 persistent identity/revision을 보존하면서 Project/Channel과 Generic Memory Scope를 중복 원본 없이 저장한다.

## 2. 논리 저장 대상

```text
bots
projects
project-memberships
channels
channel-memberships
channel-role-bindings
channel-authority-bindings
main-conversations
channel-conversation-relations
threads
conversation-messages
thread-lineage
thread-task-links
memory-records
memory-revisions
memory-scope-references
memory-relations
tasks / task-revisions / task-supervisor-links
executions / capability-bindings
core-leases
control-directives / suspension-state
artifacts
events
inbox / outbox
side-effect-ledger
```

실제 table naming/normalization/polymorphic reference 방식은 Storage ADR이 소유한다.

## 3. Generic Memory Scope

v0.4의 `bot-global: BotId`, `thread: BotId + ThreadId` 특화를 다음 논리 계약으로 일반화한다.

```text
ScopeRef = Bot(BotId) | Project(ProjectId) | Channel(ChannelId) | Thread(ThreadId)
```

Rust enum, discriminant, FK schema는 ADR 전 고정하지 않는다. Canonical Memory를 Scope별 별도 table에 복제 저장하는 구조는 기본안으로 사용하지 않는다.

## 4. Thread Parent

Thread는 `ConversationParentRef`를 가진다.
- BotMainConversation
- ChannelConversation

기존 Bot-owned Thread의 ThreadId는 변경하지 않는다. migration은 기존 Bot relation을 ParentRef로 재구성한다.

## 5. Membership / Authority

Project/Channel membership과 Role/Authority binding은 durable revision/generation을 가진다. Runtime presence/CoreLeaseId/ProviderSessionId를 membership row의 Canonical state에 넣지 않는다.

## 6. Transaction Boundary

- Project/Channel mutation: event + current state + idempotency + outbox
- membership/authority: expected revision/generation + event/state
- conversation append: message + parent/thread revision
- scoped Memory commit: record/revision + ScopeRef + provenance
- promotion: target revision + source relation/digest
- redirect/suspend/side effect: v0.4 atomic meaning 유지

## 7. Derived Storage

- Project/Channel/Thread summaries
- semantic/ranking index
- presence/activity view
- UI caches
- Provider Session/live handle

Derived 삭제 후 Canonical source로 rebuild 가능해야 한다.

## 8. Migration

v0.4 Bot-only data에 Project/Channel을 강제 생성하지 않는다. 기존 Thread-scoped Memory는 실제 Thread Canonical Record를 찾아 `Thread(ThreadId)` ScopeRef에 연결한다.

금지:
- 새 ThreadId 발급
- Provider Session ID로 Thread/Scope 보정
- Memory content 재생성
- provenance 삭제
- Bot Global Memory scope 임의 변경

## 9. 검증 기준

- migration 전/후 BotId/ThreadId/MemoryId/revision/provenance 동일.
- Project/Channel disable 시 기존 rows 없이 Bot-only path 동작.
- membership revoke 후 derived index/cache가 stale여도 canonical auth가 누출을 차단.
- physical schema가 Role과 Authority를 한 문자열 field로 합치지 않는다.
