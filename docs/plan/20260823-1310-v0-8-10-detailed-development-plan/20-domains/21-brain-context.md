---
title: "Brain과 Context Plan"
document_id: "DXB-DOM-021"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-DOM-020", "DXB-ARC-014"]
---
# Brain과 Context Plan

Brain은 특정 LLM process/session이 아니라 Bot Identity, Memory retrieval/promotion policy, Goal/Task 판단, Context 구성, Capability selection 요청, 결과 검증을 연결하는 logical semantics다. 별도 mutable mega-state를 만들지 않는다.

## Context Plan 단일 Owner

- `application-context-plan`이 current canonical revisions를 읽어 candidate Context Plan을 구성하고 검증한다.
- `Task/Execution` owner가 Execution 생성 commit에서 immutable Context Plan snapshot/ref를 영속화한다.
- Runtime과 Provider Host는 committed snapshot을 소비할 뿐 수정하지 않는다.
- revalidation은 기존 snapshot을 rewrite하지 않고 새 TaskRevision/Execution을 만든다.

```text
BotId / IdentityRevision
TaskId / TaskRevision
Conversation/Thread/Project/Channel refs
Memory assertion/revision refs
Capability requirements
Provider binding generation
ActionGrant/Permission refs
Resource/Deadline budget
Prompt/Context schema version
```

## Shared Brain / isolated execution

공유하는 것은 Identity·long-term Memory refs·Goal/Task Canonical state·policy refs다. execution-local message window, model/tool call state, temporary artifact, partial output/checkpoint, cancellation/deadline는 분리한다.

Core 간 temporary chain을 global Brain state로 복사하지 않는다. 공유 결과는 typed Evidence/Result/Memory Proposal로 owner validation을 거친다.

Context Builder는 full history/full Memory를 materialize하지 않고 relevance, scope, revision, temporal validity, epistemic state, token/byte budget을 적용한다. Cache는 source revision/watermark/policy generation/schema에 결박된 Derived State다.
