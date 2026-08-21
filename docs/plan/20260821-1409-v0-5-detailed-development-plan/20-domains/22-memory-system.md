---
title: "Scope-Aware Memory-Centric Architecture"
document_id: "DXB-DOM-022"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-ARC-015", "DXB-ARC-014", "DXB-ARC-017", "DXB-DOM-027"]
---

# Scope-Aware Memory-Centric Architecture

## 1. 목적

Memory를 Bot 지속성 및 협업 지식의 Canonical 시스템으로 정의한다. v0.4 Bot-global/Thread Memory 의미를 보존하면서 Project/Channel을 독립 Canonical Scope로 추가한다.

## 2. Canonical Scope

```text
Memory Scope
├─ Bot(BotId)
├─ Project(ProjectId)
├─ Channel(ChannelId)
└─ Thread(ThreadId)

Runtime-only
└─ Execution Working Context
```

각 Scope는 독립 knowledge owner다. 단순 namespace inheritance가 아니다.

## 3. Canonical Record

최소 논리 필드:
- MemoryId / Revision / Class
- ScopeRef
- content 또는 Artifact ref
- provenance
- visibility/access policy ref
- confidence/verification
- sensitivity/trust
- validity/retention
- relations
- supersedes/conflicts
- creator
- digest

Embedding/ranking/index/cache/summary는 Derived다.

## 4. Write Pipeline

```text
Message / Execution Result / Artifact
→ MemoryCandidate
→ MemoryProposal
→ Scope Classification
→ current Authorization
→ Normalize
→ Dedup
→ Conflict Detection
→ Verification / Confidence
→ Sensitivity / Trust
→ Retention
→ Canonical Commit
```

Core, Provider, Channel Runtime, Thread Runtime이 Canonical store를 직접 write하지 않는다.

## 5. Promotion / Publication

허용 흐름 예:

```text
Working Result
→ Thread Memory
→ Channel Memory
→ Project Memory
```

필요한 경우 Shared Scope→Bot Global internalization도 별도 Proposal/Policy를 거친다.

`promote ≠ move ≠ blind copy`.

source revision/digest/provenance를 보존하고 target Scope에 새 Canonical revision/reference relation을 만든다. source를 삭제하거나 in-place scope mutation하지 않는다.

## 6. Shared Memory 원칙

Bot A/B가 Channel Memory를 읽을 수 있지만 A/B의 Global Memory는 직접 공유하지 않는다. Channel Memory는 participating Bot Memory의 union/materialized copy가 아니다.

## 7. Recall

```text
Authorized Scope Set
→ Permission/Membership/Trust filter
→ exact/metadata/relationship search
→ optional semantic index
→ candidate IDs
→ canonical record fetch
→ FINAL current authorization
→ conflict grouping
→ relevance/authority/freshness ranking
→ Context budget
```

Index가 revoked item을 반환해도 final canonical filter가 제거해야 한다.

## 8. Conflict Resolution

결정 입력:
- Memory Class
- Scope
- Authority
- Provenance
- Verification/Confidence
- Validity
- Freshness
- Current Task relevance
- Policy revision

scope depth만으로 winner를 정하지 않는다.

## 9. Revocation / Deletion

Membership revoke는 신규 recall/write/promotion을 차단한다. Canonical retention/delete는 해당 Scope owner/policy를 따른다. Project/Channel export/delete와 이미 Bot이 policy를 거쳐 internalize한 Memory의 관계는 별도 ADR/OQ다.

## 10. Growth / Cache

Runtime pressure 순서:
1. Derived cache/index trim
2. prefetch/fan-out 감소
3. Working Context spill/evict
4. background index/compaction throttle
5. 신규 Execution admission 축소

Canonical Memory는 pressure만으로 삭제하지 않는다.

## 11. Concurrency

- same revision update: expected revision
- promotion vs source update: source revision/digest pin
- simultaneous promotion: dedup + expected revision + conflict policy
- revoke vs stale index: final authorization
- Context creation vs Memory update: selected immutable Memory revision pin

## 12. v0.4 Compatibility

기존 Bot Global Memory는 `Bot(BotId)`로 의미 보존한다. 기존 Thread Memory는 content/revision을 재생성하지 않고 `Thread(ThreadId)`로 연결한다. 기존 Thread→Bot promotion은 Generic Promotion의 특수 사례다.

## 13. 검증 기준

- AT-MEM-002/003/004 기존 Acceptance 유지.
- AT-MEM-005 unauthorized scope leakage 0.
- AT-MEM-006 promotion chain에서 source revision/provenance 보존.
- Shared Memory commit이 participating Bot Global Memory revision을 자동 생성하지 않음.
- Index Provider 제거 후 canonical exact/metadata lookup/rebuild 가능.
