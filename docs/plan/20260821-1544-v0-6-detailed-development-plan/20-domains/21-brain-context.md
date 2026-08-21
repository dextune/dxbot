---
title: "Brain과 Scope-Aware Context 조립"
document_id: "DXB-DOM-021"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-020", "DXB-DOM-022", "DXB-DOM-027", "DXB-ARC-013", "DXB-ARC-017"]
---

# Brain과 Scope-Aware Context 조립

## 1. 목적

v0.5 authorized Scope-Aware Context를 유지하면서 Memory의 epistemic/trust/confidentiality/provenance 상태와 process-wide byte budget을 Context selection/rendering에 반영한다. Brain은 여전히 특정 Provider가 아니며 하나의 Bot에서 동일 logical semantics를 유지한다.

## 2. Context Scope Set

기존 `BotId + ProjectId? + ChannelId? + ThreadId? + TaskId? + ExecutionId` 의미를 유지한다. ProcessId/CycleId는 필요 시 provenance/correlation reference로 포함될 수 있으나 read authority나 Brain identity가 아니다.

## 3. Context Candidate Metadata

candidate는 content/reference 외 필요한 범위에서 다음을 유지한다.
- ScopeRef / source revision / provenance chain
- Epistemic Kind / Assertion State
- Evidence relation refs
- observed/recorded/valid/effective 시간 의미
- origin/source class
- trust class
- confidentiality/sharing class
- sensitivity
- current authorization/policy generation
- content/artifact byte size estimate

## 4. Claim / Evidence / Decision 분리

- Claim과 Evidence를 동일 item으로 간주하지 않는다.
- Proposed/Unverified Claim은 Verified Claim과 동일 authority score를 받지 않는다.
- Retracted/Quarantined item은 normal verified recall path에서 제외한다.
- Stale/RevalidationRequired item은 정책에 따라 제외·감점·conflict 표시한다.
- Decision/Policy/Preference/Procedure는 factual Claim과 별도 authority/conflict 규칙을 사용한다.
- legacy-unclassified 또는 동등 상태는 migration만으로 Verified 취급하지 않는다.

## 5. 조립 순서

```text
Resolve current scope IDs
→ resolve membership/authority generation
→ retrieve bounded candidate IDs/refs
→ canonical record fetch
→ FINAL current authorization
→ information-label/trust/confidentiality filter
→ assertion/evidence/conflict grouping
→ relevance/authority/freshness ranking
→ byte-first + token budget allocation
→ immutable Context Plan + selected revision/digest
→ Provider-neutral bounded render
```

semantic index/cache는 permission, epistemic state, information-flow policy를 대체하지 않는다.

## 6. Byte-First Allocation

Context candidate/prefix/history는 token 변환 전에도 byte budget을 적용한다.
- 전체 Project/Channel history/Memory materialization 금지
- immutable Bot/Project/Channel prefix는 digest/revision shared ref
- large Artifact/history는 reference/stream/pagination 우선
- Core별 deep clone 금지
- Provider render가 전체 Canonical object graph나 대형 intermediate `String`/`Vec`를 중복 생성하지 않도록 bounded render를 사용
- Context build에 필요한 Runtime Memory capacity가 없으면 partial selection/defer/reject 등 Resource Policy semantic을 따른다.

## 7. Revocation / Retraction Race

- membership revoke 후 새 Context Plan은 해당 Scope를 제외한다.
- Memory retract/revalidation이 concurrent하면 Context Plan은 선택된 immutable revision과 assertion state를 pin한다.
- 이미 생성된 Execution Context를 retroactive mutation하지 않는다.
- high-risk effect는 필요 시 current authorization/Grant를 별도 preflight한다.

## 8. Cache Key / Invalidation

기존 Bot/Project/Channel/Membership/Thread/Task/Memory/Policy/Provider generation에 필요한 범위에서 assertion-state revision, information-label policy revision을 반영한다. cache가 stale item을 반환해도 canonical fetch/final filter가 제거한다.

## 9. 검증 기준

- AT-CTX-001/003 기존 isolation/boundedness 유지.
- AT-MEM-007/008에 따라 Unverified/Retracted/Quarantined item이 Verified context로 무조건 선택되지 않음.
- AT-SEC-003에 따라 Private source가 Shared Context publication authority로 오해되지 않음.
- Context render의 cumulative bytes가 Resource budget을 우회하지 않음.
- 동일 authorized canonical snapshot/input이 동일 Context Plan digest/order를 생성.
- Bot-only v0.5 Context fixture semantic 유지.
