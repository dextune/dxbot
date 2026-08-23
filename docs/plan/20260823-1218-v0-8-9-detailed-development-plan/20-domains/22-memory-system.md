---
title: "Scope-Aware Epistemic Memory"
document_id: "DXB-DOM-022"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-020", "DXB-ARC-014", "DXB-ARC-015"]
---

# Scope-Aware Epistemic Memory

## 1. 목적

DXBOT Memory를 채팅 로그나 vector index와 구분하고, Bot·Project·Channel·Thread scope별 지식의 provenance, epistemic state, revision, publication을 안전하게 관리한다.

## 2. Memory 종류와 Scope

```text
MemoryScopeRef
├─ BotMemory(BotId)
├─ ProjectMemory(ProjectId)
├─ ChannelMemory(ChannelId)
└─ ThreadMemory(ThreadId)
```

Long-term/Semantic/Episodic/Procedural 분류는 의미 tag이며 Canonical Scope owner를 대체하지 않는다. Working Memory는 Execution-local이고 durable promotion 전에는 Canonical Memory가 아니다.

Conversation/Channel History는 Message 기록이며 Memory와 동일하지 않다.

## 3. Assertion 모델

최소 metadata:

```text
MemoryId / Revision
ScopeRef
EpistemicKind
AssertionState
Statement or ArtifactRef
Provenance / EvidenceRelation
Temporal validity
InformationLabel
CreatedBy / ValidatedBy
Supersedes / Retracts refs
ObservedAt / IndexedWatermark
```

상태 예:

```text
Proposed → Validating → Accepted
Proposed/Validating → Rejected
Accepted → Superseded | Retracted | Stale/RevalidationRequired
```

index/cache/search result가 AssertionState를 생성하거나 변경하지 않는다.

## 4. Write와 Promotion

- `memory propose`는 untrusted proposal을 생성한다.
- `memory promote`는 target scope, source revision, evidence, authorization, information-flow, expected revision을 검증한다.
- Private/Sensitive → Project/Channel Shared는 Declassification decision과 audit를 요구한다.
- 동일 사실을 scope마다 원본처럼 복제하지 않고 provenance/supersession relation을 유지한다.
- correction은 기존 revision을 덮어쓰기보다 새 revision/supersession/retraction으로 추적한다.

## 5. Query와 Cursor

`get/search/history`는 `DXB-IFC-040`의 snapshot page contract를 사용한다. 기본 stable order는 owner가 명시한 score+MemoryId 또는 revision+MemoryId이며 cursor는 scope/filter/principal/snapshot/schema에 결박된다. total count를 위해 전체 collection을 materialize하지 않는다.

Index는 candidate retrieval을 제공할 뿐 final scope authorization, current revision, epistemic/temporal validity를 canonical owner가 재검증한다.

## 6. Context 사용

Context Builder는 accepted/current assertion만 무조건 사용하지 않는다. task semantics에 따라 stale/retracted/conflicting evidence를 표시할 수 있으나 상태와 provenance를 보존한다. index result를 current truth로 단정하지 않는다.

## 7. Recovery·Retention·Removal

- Journal/revision/provenance를 먼저 복구하고 index/cache는 rebuild 가능하다.
- partial index rebuild는 source watermark를 노출한다.
- forget/purge는 P0 CLI 범위가 아니며 법적·보존·reference 영향과 irreversible step을 별도 승인해야 한다.
- receipt/local CLI journal에 raw Memory payload를 복사하지 않는다.

## 8. Resource와 보안

query page, candidate set, decoded payload, artifact stream, context budget은 item+byte cap을 가진다. secret/raw sensitive statement를 metric label, doctor, terminal diagnostics에 노출하지 않는다.

## 9. 검증 기준

- History append가 자동 Memory promotion을 만들지 않는다.
- revoked principal의 stale cursor가 Shared/Private Memory를 읽지 못한다.
- duplicate proposal/retry가 duplicate accepted assertion을 만들지 않는다.
- index 삭제/재구축 후 canonical revision/provenance가 유지된다.
- Private→Shared promotion이 Declassification/Authorization을 우회하지 않는다.
- large search/history가 bounded page로 실행된다.
