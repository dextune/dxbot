---
title: "Command·Event·State·Projection 모델"
document_id: "DXB-ARC-014"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-GOV-002", "DXB-ARC-010"]
---

# Command·Event·State·Projection 모델

## 1. 목적

상태 변경의 단일 경로와 crash recovery를 유지하면서 Main Conversation/Thread/Control Directive/Suspension을 1급 Canonical state로 추가한다.

## 2. 기본 모델

`Command → Handler → Aggregate Decide → Atomic Journal/State/Outbox Commit → Projection/Notification`.

Domain Event는 immutable committed fact, Current State는 canonical materialization, Projection은 rebuildable query model, Runtime signal은 non-canonical이다. Side Effect Ledger는 external operation state다.

## 3. Aggregate / Canonical Owner

- Bot
- Main Conversation
- Thread + Thread Lineage
- Goal
- Task + Task Specification Revision
- Execution
- Routine
- Bot Network Message
- Memory Record/Revision
- Core Lease
- Runtime Policy Set
- Control Directive / Suspension metadata where modeled as child state

Task Continuation/Suspension checkpoint는 Task에 결합된 Canonical child state이며 Runtime queue item이 아니다.

## 4. Event Envelope

기존 event_id, aggregate/id/revision, schema version, times, actor, correlation/causation, command/idempotency, payload/Artifact, policy/config/provider metadata, sensitivity/retention, digest를 유지한다.

Conversation/Thread event에는 필요 시 ConversationId, ThreadId, parent/source Thread revision, MessageId를 포함한다. Control event에는 DirectiveId, target revision/control epoch를 포함한다.

## 5. Conversation / Thread Event 의미

후보 semantic:
- MainConversationProvisioned
- ConversationMessageAppended
- ThreadCreated
- ThreadArchived / Restored
- ThreadBranched
- ThreadTaskLinked / Unlinked

정확한 event name/schema는 ADR에서 조정 가능하나 다음은 고정한다.
- Bot당 Main Conversation 하나
- Message append가 Memory commit을 의미하지 않음
- branch는 source revision lineage를 남김
- Session connect/disconnect를 Domain event로 남발하지 않음

## 6. Memory Promotion Event 의미

Thread Memory commit과 Bot-global promotion을 별도 event/revision으로 표현한다. Promotion은 source Thread Memory revision/provenance를 pin하며 in-place scope mutation으로 source history를 잃지 않는다.

## 7. Live Control Event 의미

후보:
- ControlDirectiveCommitted
- DirectiveAcknowledged / Superseded / Rejected
- TaskSpecificationRevised
- ExecutionYieldRequested / Yielded
- TaskSuspensionCommitted
- TaskResumed
- TaskReprioritized

`redirect`의 durable fact가 Runtime signal만으로 존재하지 않는다. Directive + Task revision이 commit된 뒤 supervisor signal을 보낸다.

## 8. Commit 규칙

### Conversation Message
Message append + Conversation/Thread revision + 필요한 Outbox를 atomic commit한다. Memory extraction/promotion은 별도 Command/Proposal이다.

### Waiting
Task Waiting + Continuation + correlation/outbox same UoW.

### Suspension
Suspend semantic state + checkpoint/Continuation reference + Directive acknowledgement에 필요한 durable linkage를 일관되게 Commit한다. exact transaction split은 store design에 따라 달라도 crash 후 ambiguous pause flag가 남지 않아야 한다.

### Redirect
Directive + new Task Specification revision을 먼저 durable commit한다. old Execution yield/new Execution schedule은 후속 idempotent runtime operation이다.

### Side Effect
Intent/key durable → external effect → outcome; Unknown이면 reconciliation.

## 9. Projection

Projection 후보:
- Bot Main Conversation summary
- Thread list/lineage/archive state
- Thread Task/Execution/Core summary
- Message history pagination watermark
- Thread/global Memory promotion status
- pending Directive/ack/freshness
- suspended Task summary
- Side Effect reconciliation
- Provider/Plugin lifecycle

Projection은 control authority가 아니다.

## 10. Idempotency / Race

- Conversation Message: command/message id + target revision
- Thread branch: source revision + idempotency
- redirect/suspend/resume: DirectiveId + expected Task/spec revision + resume guard
- duplicate control signal: durable Directive status로 dedup
- Session reconnect: Domain state mutation 없이 cursor/resync

## 11. 검증 기준

- duplicate message command가 하나의 append 의미를 만든다.
- Thread branch가 source state를 mutation하지 않는다.
- Thread Memory promotion이 source revision provenance를 유지한다.
- redirect commit 후 crash가 old Execution prompt mutation 없이 복구된다.
- suspend failpoint에서 durable state/checkpoint 관계가 손상되지 않는다.
- Projection 삭제 후 Conversation/Thread/Directive view를 Canonical source로 rebuild한다.
