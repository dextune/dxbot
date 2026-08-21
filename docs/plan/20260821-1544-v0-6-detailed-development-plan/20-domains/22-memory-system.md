---
title: "Scope-Aware Epistemic Memory Architecture"
document_id: "DXB-DOM-022"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-ARC-015", "DXB-ARC-014", "DXB-ARC-017", "DXB-DOM-027"]
---

# Scope-Aware Epistemic Memory Architecture

## 1. 목적

Memory를 Bot 지속성과 협업 지식의 Canonical 시스템으로 유지하면서, v0.6에서 **무엇을 기억하는가**뿐 아니라 **그 정보가 어떤 종류의 지식이며 현재 어떤 검증 상태인가**를 명시한다. 새 Memory subsystem을 만들지 않고 `DXB-DOM-022`가 Scope/Epistemic/Relation/Retraction 의미를 단독 소유한다.

## 2. Canonical Scope 비회귀

```text
Memory Scope
├─ Bot(BotId)
├─ Project(ProjectId)
├─ Channel(ChannelId)
└─ Thread(ThreadId)

Runtime-only
└─ Execution Working Context
```

Scope는 독립 knowledge owner이며 단순 부모→자식 namespace inheritance가 아니다. Shared Scope Memory는 participating Bot Memory의 union/replica가 아니다.

## 3. Canonical Record 확장

기존 MemoryId/Revision/Class/ScopeRef/content-or-Artifact/provenance/visibility/confidence/sensitivity/validity/retention/relations/creator/digest 의미를 유지하고 다음을 additive하게 표현한다.

### Epistemic Kind
최소 의미:
- Observation / Evidence
- Claim / Assertion
- Hypothesis / Inference
- Decision
- Policy
- Preference
- Procedure

정확한 enum 세분화는 ADR 대상이다. factual Claim과 Decision/Policy/Preference/Procedure가 동일 verification/authority 규칙을 사용하지 않는 것은 P0다.

### Assertion State
factual assertion은 최소 다음 의미를 구분한다.
- Proposed / Unverified
- Verified
- Disputed
- Superseded
- Retracted
- Stale / RevalidationRequired
- Quarantined

정확한 transition table과 physical enum은 ADR 대상이다.

## 4. Temporal Semantics

하나의 timestamp로 모든 의미를 표현하지 않는다.
- `observed_at`: source가 관찰된 시점
- `recorded_at`: DXBOT에 기록된 시점
- `valid_from / valid_until`: 사실 유효 기간
- `effective_from`: Decision/Policy 적용 시점

시간 metadata가 없다고 현재 사실임을 자동 추론하지 않는다.

## 5. Evidence / Dependency Relation

최소 semantic:

```text
evidence-for
derived-from
contradicts
supersedes
invalidates
decision-based-on
```

Graph DB를 요구하지 않는다. 중요한 것은 source revision을 잃지 않고 dependency를 bounded하게 조회·감사·재검증할 수 있는 Canonical relation이다.

## 6. Write Pipeline

```text
Message / Execution Result / Tool / Artifact / External Content
→ MemoryCandidate
→ MemoryProposal
→ Scope Classification
→ current Authorization
→ Information-Flow / Declassification check
→ Normalize / Dedup
→ Epistemic Kind classification
→ Evidence / Provenance validation
→ Conflict Detection
→ Assertion State / Confidence
→ Sensitivity / Trust
→ Retention
→ Canonical Commit
```

LLM/Tool/외부 content는 Proposal/Evidence 입력일 뿐 Canonical Truth가 아니다. 모델이 스스로 “검증됨”이라고 말하거나 citation을 생성했다는 이유만으로 Verified를 부여하지 않는다.

## 7. Promotion / Publication / Internalization

`promote ≠ move ≠ blind copy`를 유지한다.

```text
Source Scope/Revision + Information Label
→ Target Scope
→ Principal/Authority
→ Information-Flow Policy
→ Declassification/Approval if required
→ Epistemic/Conflict Validation
→ Target Canonical Revision/Relation
```

Private/Sensitive → Channel/Project publication은 source read와 target write가 모두 허용되더라도 자동 허용되지 않는다. source revision/digest/provenance/epistemic state를 보존한다.

## 8. Retraction / Correction / Revalidation

```text
Memory A
→ promoted/derived B
→ promoted/derived C

A Retracted
→ A는 감사 가능한 상태로 보존
→ dependent B/C 조회
→ B/C를 Stale/RevalidationRequired 또는 policy-equivalent로 표시
→ 새 Context에서 Verified knowledge처럼 무조건 사용하지 않음
→ bounded revalidation
→ 유지 / 수정 / Superseded / Retracted 결정
```

금지:
- blind cascade delete
- source retract를 무시하고 dependent를 계속 Verified로 사용
- promotion provenance 손실로 dependent를 찾을 수 없는 구조

## 9. Quarantine

Quarantine candidate:
- external untrusted source가 만든 high-authority Policy/Procedure proposal
- Retracted evidence에 의존하는 item
- poisoning 의심 source에서 대량 생성된 Memory
- provenance chain이 끊긴 promoted Memory

Quarantine는 삭제가 아니라 normal recall/authority path에서 격리하는 semantic이다. 해제에는 current policy와 필요한 revalidation을 사용한다.

## 10. Recall

```text
Authorized Scope Set
→ bounded candidate retrieval
→ canonical record fetch
→ FINAL current authorization
→ information-label/trust filter
→ assertion-state filter
→ evidence/conflict grouping
→ relevance/authority/freshness ranking
→ Context byte/token budget
```

Index/cache/ranking score는 permission이나 Verified truth의 authority가 아니다.

## 11. Legacy v0.5 Migration

- MemoryId/revision/content/provenance 유지
- metadata 부재를 `legacy-unclassified` 또는 동등한 explicit migration state로 표현 가능
- 과거 Memory를 일괄 Verified로 승격 금지
- background reclassification이 Canonical content를 rewrite하지 않음
- legacy recall compatibility는 별도 policy/fixture로 검증

## 12. Resource / Concurrency

- dependency traversal/revalidation queue는 item+byte/work budget을 가진다.
- source retract vs promotion, retract vs Context build, revalidation vs concurrent update는 expected revision과 immutable snapshot으로 결정한다.
- Canonical Memory는 Runtime memory pressure만으로 삭제하지 않는다.
- large content는 Artifact/durable blob ref를 우선하고 hot heap 전체 materialization을 피한다.

## 13. 검증 기준

- 기존 AT-MEM-002~006 유지.
- AT-MEM-007: Unverified factual Claim과 Verified/Decision/Policy authority가 분리됨.
- AT-MEM-008: A→B→C retraction dependency가 보존되고 dependent가 revalidation 상태로 전환됨.
- AT-SEC-003: source read + target write만으로 Private→Shared publication이 commit되지 않음.
- legacy Memory migration이 Verified를 임의 생성하지 않음.
- index 제거 후 canonical exact/metadata/dependency lookup/rebuild 가능.
