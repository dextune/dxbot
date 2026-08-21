---
title: "구현 로드맵과 단계별 Gate"
document_id: "DXB-DEL-060"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-ARC-010", "DXB-ARC-016", "DXB-ARC-017", "DXB-DOM-022", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-037", "DXB-ENG-052"]
---

# 구현 로드맵과 단계별 Gate

## 1. 목적

v0.4의 Common Provider Framework/Persistent Conversation/Live Control foundation을 유지하면서 v0.5를 **Persistent Collaboration + Scope-Aware Knowledge + Dynamic Execution** 순서로 구현한다. 기간보다 dependency와 exit gate를 우선한다.

## 2. 유지되는 Foundation

다음은 재설계하지 않는다.
- Common Capability Contract/Provider Host/Lifecycle/SDK/Conformance/Tier A-B
- Persistent Bot/Main Conversation/Thread
- Conversation History ≠ Memory
- immutable Execution + durable Live Control
- Waiting/Suspension/Side Effect recovery
- Dynamic Core Scheduler ownership
- Provider Session independence

## 3. M0 — Contract / Scope Foundation

Exit:
- Project/Channel/Memory Scope/Conversation Parent 용어 freeze
- Canonical Owner freeze
- Role/Authority 분리
- generic Memory ScopeRef semantic freeze
- ConversationParentRef semantic freeze
- Project/Channel permission boundary
- dependency DAG 검증
- v0.4 migration fixture 초안
- physical enum/schema/threshold는 ADR 전 과고정하지 않음

## 4. M1 — Persistent Project / Channel

Exit:
- Project persistence/lifecycle
- Project Membership
- Channel persistence/lifecycle
- Channel Membership/Role/Authority persistence
- Channel Conversation relation
- restart recovery
- basic Project/Channel API
- Bot-only v0.4 regression 0
- Project 없는 기존 Bot path 정상 동작

## 5. M2 — Scope-Aware Memory / Context

Exit:
- Bot/Project/Channel/Thread Memory isolation
- generic ScopeRef
- proposal/write pipeline
- Thread→Channel→Project/Shared→Bot explicit promotion path
- source revision/provenance preservation
- Scope-aware Context Plan
- final canonical authorization after candidate/index
- bounded retrieval/context
- v0.4 Thread→Bot promotion compatibility

## 6. M3 — Channel Runtime Orchestration

Exit:
- Channel Coordinator
- recipient/mention/role-aware routing
- manager-first routing
- bounded participant fan-out
- response concurrency/backpressure
- membership/authority generation fencing
- Presence projection
- Dynamic Core/Scheduler integration
- Work Queue/Control Channel ownership 유지
- AT-CHANNEL-002 통과

## 7. M4 — Multi-Bot Collaboration

Exit:
- Manager delegation
- Researcher/Reviewer workflow
- Task SupervisorRef
- durable Bot Network reuse
- report/redirect/suspend integration
- Shared Memory publication/promotion
- Manager가 다른 Bot Brain/Core/Memory 직접 mutate하지 않음
- membership revoke under collaboration
- AT-COLLAB-001 통과

## 8. M5 — API / CLI / TUI

Exit:
- Project/Channel management
- membership/Role/Authority
- Channel message/history/thread/memory
- collaborative task/control
- scoped Memory search/promotion
- stale/revoked generation error semantics
- headless/CLI/TUI shared schema

## 9. M6 — Web

Exit:
- Project navigator
- Channel persistent conversation
- Thread workspace
- member Role/effective Authority view
- Derived Presence/status
- Shared Memory view
- virtualized history/member/thread lists
- bounded client state
- Bot Main Conversation 독립 접근 유지

## 10. M7 — Distributed / HA

Exit:
- local/remote deployment에 따라 Project/Channel/Membership/Scope semantic이 변하지 않음
- remote transport가 identity/authority owner가 아님
- durable delegation/Directive/ScopeRef semantics 유지
- Provider Session independence 유지

## 11. Cross-Cutting Gate

모든 Milestone에서:
- v0.4 Acceptance regression
- Provider Host/SPI/Conformance regression
- naming/dependency/Canonical Owner check
- resource item+byte cap
- security current authorization
- migration/rollback compatibility
- RSS/allocation/cache evidence
- deterministic concurrency/fault evidence

## 12. Scope Control

P0에서 임의 확정하지 않는다.
- exact Project/Channel lifecycle enum
- MemoryScopeRef DB physical representation
- exact per-message Bot activation number
- Manager failover algorithm
- multi-supervisor merge
- automatic scope inheritance
- Project/Channel Memory retention 숫자

## 13. 검증 기준

- 각 M0~M7에 Acceptance/Risk/OQ가 연결됨.
- v0.4 Bot-only path를 Project/Channel implementation이 막지 않음.
- M3 이전에 UI workaround로 all-member wake/direct Core control을 만들지 않음.
- M4에서 협업이 기존 Bot Network/Task/Live Control을 재사용함.
