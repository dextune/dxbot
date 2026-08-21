---
title: "Memory-Centric Architecture 상세"
document_id: "DXB-DOM-022"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-ARC-015", "DXB-ARC-014", "DXB-ARC-017", "DXB-DOM-027"]
---

# Memory-Centric Architecture 상세

## 1. 목적

Memory를 채팅 로그나 cache가 아니라 Bot 지속성의 Canonical 지식 시스템으로 구현하고, **Bot-global / Thread-scoped / Execution Working** scope를 분리한다. Thread transcript와 Memory를 혼합하지 않으며 Thread-local 정보의 Bot-global 승격을 명시적으로 통제한다.

## 2. Memory 분류와 Scope

Class:
- Long-term
- Semantic
- Episodic
- Procedural
- Working
- Preference
- Safety/Policy reference

Scope:
- **Bot-global Memory**: Bot 전체에서 장기 재사용할 검증된 기억
- **Thread-scoped Memory**: 특정 Thread 문맥에 유효한 기억
- **Execution/Working Memory**: 현재 Execution/Core의 임시 상태

Routine 정의와 Conversation History는 Memory가 아니다.

## 3. Canonical Record

MemoryId, Revision, class, scope/owner(BotId/optional ThreadId), content 또는 Artifact ref, provenance, confidence/verification, sensitivity/trust, validity, retention, relations, supersedes/conflicts, creator, digest를 가진다. Embedding/ranking/cache는 Derived다.

Thread-scoped Memory는 ThreadId를 provenance만이 아니라 scope owner로 명시한다. Bot-global Memory에 ThreadId provenance가 있을 수 있으나 global scope로 승격된 결과임을 구분한다.

## 4. Write / Promotion Pipeline

```text
Execution Working State
  ↓ MemoryProposal
Normalize / Policy / Scope / Dedup / Verify / Conflict
  ↓
Thread-scoped Memory Commit
  ↓ PromotionProposal (필요 시)
Global Policy / Provenance / Confidence / Conflict / Retention
  ↓
Bot-global Memory Commit
```

Working Memory는 자동 승격하지 않는다. Thread에서 생성된 정보도 자동 global 승격하지 않는다.

명시적 user global-memory command는 Thread stage를 반드시 거쳐야 한다는 의미가 아니라 Memory subsystem의 scope-aware 정상 Commit 경로와 정책을 거쳐야 한다는 뜻이다. Provider/Thread/Core가 Bot-global store를 직접 write하지 않는다.

## 5. Conversation History와 Memory

`Conversation History ≠ Memory`.

- History는 발화/actor/order/Thread/provenance를 보존한다.
- Memory는 검증·정규화된 재사용 지식이다.
- 대화 메시지 저장만으로 Memory Record를 만들지 않는다.
- Memory provenance가 Conversation Message reference를 가질 수 있다.
- transcript compaction/archive가 Memory forget를 암묵 수행하지 않는다.
- Memory forget가 원본 conversation retention을 자동 삭제하는지도 별도 policy가 결정한다.

## 6. Recall

```text
Scope + Permission/Trust Filter
→ Exact/Metadata/Relationship
→ optional MemoryIndex Capability via Provider Host
→ Candidate IDs/Scores
→ Canonical Record Fetch
→ Thread/global scope merge + Conflict Grouping
→ Ranking
→ Context Budget
```

Index Provider는 candidate/ranking hint만 제공한다. Thread A query가 authorization/context 없이 Thread B local Memory를 반환하지 못하게 final Canonical filter를 적용한다.

## 7. Promotion 정책

Thread→Bot promotion은 최소 다음을 평가한다.
- 명시적 user/Bot intent
- provenance completeness
- confidence/verification
- cross-thread 재사용 가치
- sensitivity/trust
- conflict with global revision
- retention class
- duplicate/supersedes 관계

실패한 가설, 임시 조사, 특정 Thread에만 유효한 intermediate state는 기본적으로 Thread scope에 남긴다.

Promotion은 source Thread Memory를 삭제하거나 scope를 in-place mutation하는 대신 provenance를 가진 새 global revision/relationship로 표현하는 방식을 우선한다.

## 8. Long-term Growth / Runtime Pressure

Runtime RSS/cache pressure 때문에 Bot-global 또는 Thread-scoped Canonical Memory를 암묵 삭제하지 않는다.

pressure 순서:
1. Derived cache trim
2. prefetch/fan-out 감소
3. Working Context spill/evict
4. background index/compaction throttle
5. 신규 Execution/Provider Host admission 축소

Canonical retention/archive/forget는 별도 policy/command다.

Thread-local Memory retention의 정확한 기본값은 Open Question이며, unlimited hot-memory를 의미하지 않는다. canonical storage는 cold/archive/compaction을 가질 수 있다.

## 9. Provider Independence

MemoryIndex Provider 제거 시 canonical Bot-global/Thread Memory schema와 exact/metadata lookup이 유지된다. provider-specific vector/raw metadata는 derived namespace다. Provider Session/Index session이 사라져도 Memory owner는 변하지 않는다.

## 10. 동시성

- 같은 Memory revision update는 expected revision conflict
- Thread A/B의 local revision은 scope key로 격리
- promotion vs source update는 source revision/digest를 pin
- forget/tombstone vs stale index는 final Canonical filter
- simultaneous promotions는 dedup/conflict policy

## 11. 검증 기준

- AT-MEM-002: Working Memory가 정책 없이 Canonical Memory로 자동 승격되지 않는다.
- AT-MEM-003: Runtime pressure가 retention/forget 없이 Canonical item/revision을 감소시키지 않는다.
- **AT-MEM-004**: Thread-local Memory가 명시 Promotion policy 없이 Bot-global Memory로 들어가지 않는다.
- AT-THREAD-001: Thread A/B/C local Memory가 서로 누출되지 않는다.
- Index Provider 제거 후 global/thread exact lookup과 rebuild가 가능하다.
- 모든 recalled Canonical item에 scope/provenance/revision이 있다.
