---
title: "Memory-Centric Architecture 상세"
document_id: "DXB-DOM-022"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-ARC-015", "DXB-ARC-014", "DXB-ARC-017"]
---

# Memory-Centric Architecture 상세

## 1. 목적

Memory를 채팅 로그나 cache가 아니라 Bot 지속성의 Canonical 지식 시스템으로 구현하고, Working/Cache pressure와 Long-term retention을 분리한다. MemoryIndex 등 Optional Capability를 사용해도 Provider가 Canonical Memory ownership을 취득하지 않는다.

## 2. Memory 분류

- Long-term: 장기 유지 사실/관계/프로젝트 정보
- Semantic: 정리된 지식
- Episodic: 수행 경험과 결과
- Procedural: 반복 가능한 절차 지식
- Working: 현재 Execution/Core 임시 상태
- Preference
- Safety/Policy reference

Routine 정의는 Procedural Memory가 아니라 별도 Bot-owned persistent automation state다.

## 3. Canonical Record

MemoryId, Revision, class, scope/owner, content 또는 Artifact ref, provenance, confidence/verification, sensitivity/trust, validity, retention, relations, supersedes/conflicts, creator, digest를 가진다. Embedding/ranking/cache는 Derived다.

## 4. Write Pipeline

`Candidate → Normalize → Policy/Scope → Dedup → Verify → Conflict → Commit Revision → Index Queue`

Working Memory는 자동 승격하지 않고 `MemoryProposal`을 통해 정책 검증 후 Commit한다. Provider가 Memory Store에 직접 write하지 않는다.

## 5. Recall

```text
Permission/Trust Filter
→ Exact/Metadata/Relationship
→ optional MemoryIndex Capability via Provider Host
→ Candidate IDs/Scores
→ Canonical Record Fetch
→ Dedup/Conflict Grouping
→ Ranking
→ Context Budget
```

Index Provider 결과는 candidate/ranking hint이며 Canonical truth가 아니다. Provider에게 raw Domain Store handle을 전달하지 않고 bounded request/response Contract를 사용한다.

## 6. MemoryIndex Provider 경계

MemoryIndex Provider가 소유할 수 있는 것:
- provider-specific derived index representation
- external index SDK/API integration
- index DTO/error mapping

Common/Memory subsystem이 소유하는 것:
- Canonical Memory Record/Revision
- permission/trust filter authority
- index rebuild source
- Provider selection/lifecycle/resource/deadline/cancellation
- stale/tombstone final filter
- retention/forget semantics

production index query/rebuild external call은 Provider Host를 거친다. Provider가 Canonical record mutation, retention decision, permission cache authority를 갖지 않는다.

## 7. Long-term Memory Growth Contract

Canonical Memory 성장과 Runtime memory pressure는 별도 문제다.

**금지:** RSS watermark, cache eviction, queue pressure를 이유로 Long-term Canonical Memory를 즉시/암묵 삭제하는 것.

관리 대상:
- canonical bytes
- item/revision count
- unused age
- retention class/expiry
- compaction/cold/archive eligibility
- provenance completeness
- user/operator cleanup requests
- tombstone/delete propagation

정책 수치/default는 Policy/Config SSOT가 소유한다.

## 8. Compaction / Archival / Forget

- duplicate merge
- episodic detail → verified summary + source Artifact
- cold tier 이동
- expiry/retention enforcement
- explicit user/operator forget
- tombstone + index/cache propagation

Compaction은 provenance/digest chain을 유지한다. Archival은 삭제가 아니다.

## 9. Runtime Memory Pressure

허용 순서:
1. Derived cache trim
2. prefetch/fan-out 감소
3. Working Context spill/evict according to Task policy
4. background index/compaction throttle
5. 신규 Execution/Provider Host admission 축소

Canonical Memory 삭제는 별도 retention/forget Command가 있어야 한다.

## 10. Provider Independence / Removal

Index Provider가 제거되어도 Canonical Memory schema와 exact/metadata lookup은 유지한다. provider-specific vector/raw metadata는 derived/operational namespace에 둔다.

제거:
1. 신규 Provider selection 차단/Draining
2. in-flight index call quiesce
3. Provider registry/config/dependency 제거
4. provider-owned derived index purge 또는 retain-for-migration
5. Canonical Memory 유지
6. 새 Provider가 Canonical source에서 rebuild

stale index가 존재해도 tombstone/revision final filter가 Canonical source를 기준으로 적용된다.

## 11. 동시성

두 Core가 같은 Memory revision을 갱신하면 expected revision conflict를 반환한다. update/delete race에서는 tombstone policy를 따른다. stale index/cache가 tombstone 항목을 반환해도 query-time final filter가 차단한다.

Index rebuild/query Provider generation이 교체되어도 Canonical Memory write revision 의미를 바꾸지 않는다.

## 12. 검증 기준

- 100k/1M workload에서 canonical bytes/cache/index bytes를 별도 계측한다.
- memory pressure가 cache/admission을 줄여도 Canonical item count가 자동 감소하지 않는다.
- Index Provider 제거 후 exact/metadata lookup과 rebuild가 가능하다.
- Index Provider call이 Provider Host/MemoryIndex Conformance를 통과한다.
- explicit forget가 Canonical/Index/Cache/backup retention ledger에 추적된다.
- 모든 recalled Canonical item에 provenance/revision이 있다.
