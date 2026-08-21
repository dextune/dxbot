---
title: "Command·Event·State·Projection 모델"
document_id: "DXB-ARC-014"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-GOV-002", "DXB-ARC-010"]
---

# Command·Event·State·Projection 모델

## 1. 목적

v0.4의 `Command → Aggregate Decide → Atomic Journal/State/Outbox Commit → Projection`을 유지하고 Project/Channel/Membership/Scoped Memory를 Canonical State로 추가한다.

## 2. Canonical Aggregate 후보

- Bot
- Project / Project Membership
- Channel / Channel Membership / Role Binding / Authority Binding
- Main Conversation / Channel Conversation relation / Thread
- Goal / Task / Task Specification / SupervisorRef
- Execution / Core Lease
- Memory Record / Revision / ScopeRef
- Control Directive / Suspension child state
- Routine / Bot Network Message / Side Effect Ledger

Runtime Presence, summaries, semantic index, cache는 Canonical이 아니다.

## 3. Event 후보

### Project
- ProjectCreated / Archived / Restored
- ProjectMembershipChanged

### Channel
- ChannelCreated / Archived / Restored
- ChannelMembershipChanged
- ChannelRoleBindingChanged
- ChannelAuthorityBindingChanged

### Memory
- ScopedMemoryCommitted
- MemoryPromotionProposed
- MemoryPromoted
- MemorySuperseded / ConflictRecorded

### 기존 Conversation/Control
v0.4의 Message/Thread/TaskSpec/Directive/Suspension/Resume event semantics를 유지한다.

정확한 이름과 payload는 ADR 대상이지만 의미는 변경하지 않는다.

## 4. Commit 규칙

- Membership 변경은 durable revision/generation을 가진다.
- Channel Message append와 Memory commit은 별도 Command/Event다.
- Memory promotion은 source Memory revision/digest를 pin한다.
- Role과 Authority 변경은 별도 state/revision으로 추적 가능해야 한다.
- Runtime Presence update를 Domain Event로 남발하지 않는다.
- Control Directive commit 전에 자연어 Message만으로 Task state를 변경하지 않는다.

## 5. Revocation / Fencing

membership/authority revoke event가 commit되면 신규 authorization은 current generation을 사용한다. Runtime routing snapshot이 과거 generation이어도 control/memory write commit 직전에 current authorization을 재검증한다.

## 6. Projection

재생성 가능 후보:
- Project/Channel summary
- member/role/authority view
- Channel Presence Runtime view
- Channel/Thread history summary
- scoped Memory index/ranking
- managed Task status

Projection은 permission authority가 아니다.

## 7. Idempotency / Race

- Project/Channel create: idempotency key + owner scope
- membership/role/authority: expected revision/generation
- message routing: message ID + routing snapshot
- promotion: source revision/digest + target scope + dedup key
- stale revoke race: final authorization before Canonical commit

## 8. 검증 기준

- Message append만으로 Shared Memory revision이 생기지 않는다.
- duplicate promotion이 knowledge explosion을 만들지 않는다.
- stale Authority revision으로 Directive commit 불가.
- Projection 삭제 후 Project/Channel/Membership/Scoped Memory view를 rebuild할 수 있다.
