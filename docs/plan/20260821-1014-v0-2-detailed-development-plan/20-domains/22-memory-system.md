---
title: "Memory-Centric Architecture 상세"
document_id: "DXB-DOM-022"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-ARC-015", "DXB-ARC-014"]
---

# Memory-Centric Architecture 상세

## 1. 목적

Memory를 채팅 로그나 cache가 아니라 Bot 지속성의 Canonical 지식 시스템으로 구현하고, Working/Cache pressure와 Long-term retention을 분리한다.

## 2. Memory 분류

- Long-term: 장기 유지 사실/관계/프로젝트 정보
- Semantic: 정리된 지식
- Episodic: 수행 경험과 결과
- Procedural: 반복 가능한 절차 지식
- Working: 현재 Execution/Core 임시 상태
- Preference
- Safety/Policy reference

Routine 정의는 Procedural Memory가 아니라 별도 Bot-owned persistent automation state다. Memory는 Routine에 참고 절차를 제공할 수 있다.

## 3. Canonical Record

MemoryId, Revision, class, scope/owner, content or Artifact ref, provenance, confidence/verification, sensitivity/trust, validity, retention, relations, supersedes/conflicts, creator, digest를 가진다. Embedding/ranking/cache는 Derived다.

## 4. Write Pipeline

`Candidate → Normalize → Policy/Scope → Dedup → Verify → Conflict → Commit Revision → Index Queue`

Working Memory는 자동 승격하지 않고 `MemoryProposal`을 통해 정책 검증 후 Commit한다.

## 5. Recall

permission/trust filter → exact/metadata/relationship → lexical/vector candidate → Canonical Record fetch → dedup/conflict grouping → ranking → context budget. Index 결과를 Canonical truth로 사용하지 않는다.

## 6. Long-term Memory Growth Contract

Canonical Memory 성장과 Runtime memory pressure는 별도 문제다.

**금지:** RSS watermark, cache eviction, queue pressure를 이유로 Long-term Canonical Memory를 즉시/암묵 삭제하는 것.

관리 대상:
- canonical bytes
- item/revision count
- unused age
- retention class/expiry
- compaction eligibility
- cold/archive eligibility
- provenance completeness
- user/operator cleanup requests
- tombstone/delete propagation

정책 수치와 default는 Policy/Config SSOT가 소유한다. 이 문서에서 동일 값을 복제하지 않는다.

## 7. Compaction / Archival / Forget

- duplicate merge
- episodic detail → verified summary + source Artifact
- cold tier 이동
- expiry/retention enforcement
- explicit user/operator forget
- tombstone + index/cache propagation

Compaction은 provenance/digest chain을 유지한다. 요약이 원본 의미를 보존하지 못하면 원본 삭제 근거가 아니다. Archival은 삭제가 아니며 recall latency/availability class가 달라질 수 있다.

## 8. Runtime Memory Pressure

memory pressure 시 허용되는 조치:
1. Derived cache trim
2. prefetch/fan-out 감소
3. Working Context spill/evict according to task policy
4. background index/compaction throttle
5. 신규 execution admission 축소

Canonical Memory 삭제는 별도 retention/forget command가 있어야 한다.

## 9. Provider Independence

Embedding/Search Index Provider가 제거되어도 Canonical Memory schema는 그대로 유지한다. Provider-specific vector/raw metadata는 derived/operational namespace에 둔다. 새 Provider는 rebuild queue를 통해 재생성할 수 있어야 한다.

## 10. 동시성

두 Core가 같은 Memory revision을 갱신하면 expected revision conflict를 반환한다. update/delete race에서 tombstone이 우선한다. stale index/cache가 tombstone 항목을 반환해도 query-time filter가 즉시 차단한다.

## 11. 검증 기준

- 100k/1M Memory workload에서 canonical bytes와 cache bytes를 별도 계측한다.
- memory pressure test가 cache/Core admission을 줄여도 Canonical item count가 자동 감소하지 않는다.
- Index Provider 제거 후 exact/metadata lookup과 rebuild가 가능하다.
- explicit forget가 Canonical/Index/Cache/backup retention ledger에 추적된다.
- 모든 recalled item에 provenance/revision이 있다.
