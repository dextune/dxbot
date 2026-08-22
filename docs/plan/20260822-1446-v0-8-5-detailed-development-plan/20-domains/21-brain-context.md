---
title: "Brain과 Context Plan"
document_id: "DXB-DOM-021"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-020", "DXB-ARC-014"]
---

# Brain과 Context Plan

## 1. 목적

하나의 Bot이 여러 Core를 사용해 병렬 작업하더라도 하나의 logical Brain semantics와 Memory continuity를 유지하고, 각 Execution의 사고 흐름·입력 snapshot은 분리한다.

## 2. Brain 의미

Brain은 특정 LLM process나 model session이 아니다. 다음을 연결하는 논리적 정책 중심이다.

- Bot Identity와 역할
- Memory retrieval/promotion policy
- Goal/Task 판단
- Context Plan 구성
- Capability/Provider 선택 요청
- 결과 검증과 Memory proposal

Brain은 별도 mutable mega-state를 두지 않고 각 Canonical Owner의 ID/revision을 참조한다.

## 3. Context Plan

Execution 시작 전에 Application/Runtime이 다음을 pin한다.

```text
BotId / IdentityRevision
TaskId / TaskRevision
Conversation/Thread/Project/Channel scope refs
Memory assertion/revision refs
Capability requirements
Provider binding generation
Tool/Permission/ActionGrant refs
Resource/Deadline budget
Prompt/Context schema version
```

running Execution의 Context Plan은 immutable하다. 새 Memory, authority revoke, provider generation 교체가 발생하면 해당 정책에 따라 신규 Execution/Attempt 또는 explicit preemption/revalidation을 사용하지 기존 snapshot을 in-place rewrite하지 않는다.

## 4. Shared Brain / Isolated Working Context

공유:
- Identity·long-term/semantic Memory refs
- Goal/Task Canonical state
- permission/resource policy refs

분리:
- execution-local message window
- model/tool call state
- temporary artifact refs
- partial output and checkpoints
- cancellation/deadline state

Core 간 temporary chain을 global Brain state로 복사하지 않는다. 공유할 결과는 typed Evidence/Result/Memory Proposal로 owner validation을 거친다.

## 5. Context 구성과 boundedness

Context Builder는 full history/full Memory를 materialize하지 않는다. relevance, scope, revision, temporal validity, epistemic state, token/byte budget을 적용하고 selected references와 omission reason을 기록한다. large artifact는 reference/stream으로 전달한다.

Cache는 `(BotId, scope, source revision/watermark, policy generation, schema version)`에 결박하며 canonical source가 아니다. invalidation이나 eviction이 Identity/Memory를 손실시키지 않는다.

## 6. Concurrency

두 Execution이 같은 Memory proposal/Task result를 제출할 때 expected revision과 provenance를 검증한다. stale Context Plan은 silent overwrite하지 않고 conflict/revalidation으로 처리한다. Core Lease와 Brain ownership을 혼합하지 않는다.

## 7. 검증 기준

- 한 Bot의 병렬 Core가 서로 다른 Working Context를 가지면서 동일 BotId/Brain policy를 사용한다.
- running Execution의 Provider/Task/Context snapshot in-place mutation 0.
- Context cache eviction 뒤 canonical Memory/Task 의미 손실 0.
- full transcript/Memory materialization 없이 bounded Context Plan을 구성한다.
- Core 결과가 validation 없는 global Memory write로 이어지는 경로 0.
