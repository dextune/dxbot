---
title: "성능·메모리·캐시 계획"
document_id: "DXB-ENG-051"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-017", "DXB-DOM-022", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-031", "DXB-RUN-034", "DXB-RUN-037", "DXB-ENG-050"]
---

# 성능·메모리·캐시 계획

## 1. 목적

Project/Channel/Scoped Memory 도입이 Runtime RSS, allocation, retrieval, fan-out, cache invalidation 비용을 선형 폭증시키지 않도록 실제 workload와 ownership 기반 benchmark를 정의한다.

## 2. Memory Ownership Map

| 영역 | Owner | 제한/회수 |
|---|---|---|
| Bot coordinator | Runtime | idle/deactivate |
| Project/Channel hot metadata | Runtime/Query | entries+bytes cap/rebuild |
| Membership/Role/Authority cache | Permission/Query | generation key/byte cap |
| Channel Presence | Orchestration/Projection | bounded/rebuild |
| Conversation/Thread hot view | Conversation Runtime | cursor/window/byte cap |
| Scoped Memory cache | Memory Runtime | entries+bytes eviction; canonical store 별도 |
| Semantic/ranking index | Derived provider/index | rebuildable/bounded candidate set |
| Context Plan/prefix cache | Brain/Context Runtime | digest/revision key/byte cap |
| Channel inbound/fan-out buffers | Orchestration | item+byte cap/backpressure |
| Core Working Context | Execution/Core | terminal/drop/spill |
| Canonical Conversation/Memory | durable store | retention/archive/forget only |

## 3. Cache Contract

모든 cache는 다음을 명시한다.
- Canonical source
- Scope key
- revision/generation
- max entries + max bytes
- TTL/eviction
- stale policy
- permission/sensitivity constraints
- invalidation trigger
- rebuild path
- metrics

Context/cache key는 필요한 범위에서 Bot/Project/Channel/Membership/Authority/Thread/Task/Memory/Policy/Provider generation을 반영한다.

## 4. Shared Immutable Prefix

Bot/Project/Channel identity/policy/tool schema 등 immutable prefix는 digest/revision 기준 shared reference를 사용할 수 있다.

```text
Core A ─┐
Core B ─┼─ shared immutable Project/Channel prefix
Core C ─┘
```

Core별 deep clone과 scope 전체 materialization을 금지한다.

## 5. Required Workloads

### Project / Channel Scale
- Project: 1 / 10 / 100
- Channel per Project: 1 / 10 / 100 / 1k
- Bot per Channel: 1 / 10 / 100
- Thread per Channel: 1 / 100 / 10k
- Shared Memory: 1k / 100k / 1M equivalent records

### Runtime Stress
- concurrent Channel turns
- Manager/Researcher delegation storm
- membership revoke under routing/load
- scoped retrieval cache hit/miss/stale index
- promotion conflict/dedup storm
- long-running Channel soak
- Provider Session loss/recreation
- control + routing saturation

기존 Bot/Core/Provider/Plugin/Side Effect/Routine/Conversation workloads도 유지한다.

## 6. 측정 지표

- RSS/allocations/retained bytes by owner
- Project/Channel cache bytes vs canonical storage bytes
- routing candidate/selected participant count
- fan-out queue wait/reject
- Context selected bytes/tokens vs total available history/Memory
- recall candidate/final-selected/authorization-filtered count
- cache hit/miss/invalidation cost
- promotion normalize/dedup/conflict cost
- revoke-to-denial latency for new operations
- delegation/result latency

## 7. Structural Performance Violations

다음은 benchmark로 정당화할 대상이 아니라 구조 위반이다.
- 전체 Channel history를 매 Context에 삽입
- 모든 member를 매 message마다 activate
- Core별 Project/Channel state deep copy
- unbounded semantic candidate list
- authorization을 위해 모든 Project/Channel Memory를 materialize
- canonical state를 RSS pressure로 삭제

## 8. Regression Gate

- steady soak에서 RSS/cache/Presence/routing queue 선형 누수 없음
- selected Context는 token+byte cap 유지
- member 수가 증가해도 activated participant가 policy cap 유지
- revoke 후 stale cache hit가 authorization bypass하지 않음
- cache/index 제거 후 canonical source에서 rebuild 가능
