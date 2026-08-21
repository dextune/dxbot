---
title: "Brain과 Scope-Aware Context 조립"
document_id: "DXB-DOM-021"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-020", "DXB-DOM-022", "DXB-DOM-027", "DXB-ARC-013", "DXB-ARC-017"]
---

# Brain과 Scope-Aware Context 조립

## 1. 목적

v0.4 Thread-aware Context를 **authorized Scope-Aware Context**로 일반화한다. Brain은 특정 Provider가 아니며 하나의 Bot에서 Project/Channel/Thread가 달라도 동일 logical semantics를 유지한다.

## 2. Context Scope Set

논리적으로 다음 식별자를 가진다.

```text
ContextScopeSet
├─ BotId
├─ ProjectId?
├─ ChannelId?
├─ ThreadId?
├─ TaskId?
└─ ExecutionId
```

정확한 Rust type명은 ADR 대상이다. 존재하는 ID가 곧 read authority를 의미하지 않는다.

## 3. Context 후보

1. current input
2. Bot Identity/Policy + relevant Bot Memory
3. Project identity/policy/resources + relevant Project Memory
4. Channel identity/Role/Authority + relevant Channel Memory
5. current Thread/ParentRef + Thread Memory
6. active Goal/Task/spec/SupervisorRef/Execution
7. managed Task status if authorized
8. recent Channel/Thread messages
9. retrieved historical context
10. Artifact/checkpoint/workspace refs
11. Capability metadata
12. token+byte budget
13. trust/sensitivity labels
14. selected source revisions/digests

Bot-only path에서는 Project/Channel section이 존재하지 않는다.

## 4. 조립 순서

```text
Resolve current scope IDs
→ resolve current membership/authority generation
→ retrieve candidates
→ canonical record fetch
→ FINAL authorization/trust filter
→ conflict grouping
→ relevance/authority/freshness ranking
→ deterministic budget allocation
→ immutable Context Plan + digest
→ Provider-neutral render
```

semantic index/cache가 permission filter를 대체하지 않는다.

## 5. Conflict Handling

단순 `Thread > Channel > Project > Bot` precedence를 금지한다. Memory Class별 authority와 provenance/verification/validity/current task relevance를 사용한다.

Project ADR/결정은 프로젝트 기술 정책 class에서 authoritative할 수 있지만 Bot Identity/Preference class를 임의 덮어쓰지 못한다.

## 6. Revocation

Membership revoke 후 새 Context Plan은 해당 Project/Channel scope를 제외한다. 이미 생성된 immutable Context Plan은 retroactive mutation하지 않는다. high-risk side effect는 current grant 재평가 policy를 사용할 수 있다.

## 7. Cache / Allocation

- immutable Bot/Project/Channel policy prefix는 digest/revision shared ref
- Core별 deep copy 금지
- Context cache key에 Bot/Project/Channel/Membership generation/Thread/Task/Memory/Policy/Provider generation을 필요한 범위에서 반영
- token+byte cap
- 전체 Project/Channel history/Memory materialization 금지

## 8. 검증 기준

- AT-CTX-001 기존 Core Working Context isolation 유지.
- AT-CTX-003 history/shared Memory 증가에도 Context budget bounded.
- revoked Channel Memory가 stale index/cache를 통해 Context에 나타나지 않음.
- 동일 authorized snapshot/input이 동일 Context Plan digest/order를 생성.
- Bot-only v0.4 Context fixture가 동일 semantic을 유지.
