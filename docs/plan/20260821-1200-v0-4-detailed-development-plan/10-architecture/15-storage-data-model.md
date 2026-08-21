---
title: "저장소와 데이터 모델"
document_id: "DXB-ARC-015"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-011", "DXB-ARC-017"]
---

# 저장소와 데이터 모델

## 1. 목적

Persistent Bot의 Canonical State를 crash-safe하게 보존하고 Main Conversation, Thread Graph, Conversation History, Thread-local Memory scope, Control Directive/Suspension을 기존 Task/Continuation/Side Effect/Provider operational data와 중복 원본 없이 저장한다.

## 2. 저장 계층

Canonical:
- Domain Journal / Current State
- Bot/Main Conversation/Thread identities and revisions
- Conversation Message history/lineage metadata
- Memory Records/Revisions with Bot/Thread scope
- Task Specification revisions / Task Continuations / Suspension checkpoint refs
- Control Directive durable state
- Routine occurrences
- Side Effect Ledger
- Inbox/Outbox
- Artifact Store
- Provider/Plugin operational intent/version metadata

Derived/Ephemeral:
- Projection/Index/Cache
- Interface Session/WebSocket state
- Provider Session/live connection/process/activity handle
- Runtime Control queue item

## 3. 논리 데이터 모델

### Core
- bots
- goals
- tasks / task-revisions / task-edges
- executions / execution-capability-bindings
- core-leases
- bot-network-messages
- inbox-receipts / outbox

### Conversation / Thread
- main-conversations (BotId 1:1 invariant)
- conversation-messages
- threads
- thread-lineage/edges
- thread-message-link 또는 message scope fields
- thread-task-links
- thread-artifact/checkpoint references where needed

실제 DB table naming은 storage ADR을 따르며 repository kebab rule과 동일 개념이 아니다.

### Memory Scope
Memory item/revision에 scope owner를 표현한다.
- `bot-global: BotId`
- `thread: BotId + ThreadId`

Thread→global promotion은 source MemoryId/revision/provenance link를 보존한다. embedding/index는 Derived다.

### Control Directive / Suspension
최소:
- DirectiveId
- target Bot/Thread/Task/Execution refs
- command kind
- actor/authority scope
- expected revision/control epoch
- payload/Artifact ref
- idempotency/correlation/causation
- committed/ack/superseded/conflict/terminal status
- safe-point/progress metadata ref

Suspension:
- Task/spec revision
- suspension reason/directive ref
- checkpoint/Artifact refs
- resume guard/idempotency
- compatibility checksum/version

정확한 schema는 ADR 대상이나 in-memory pause flag만 저장하는 것은 금지한다.

## 4. Conversation History ≠ Memory

Conversation Message row와 Memory Record를 같은 table/entity로 공유하지 않는다. Message는 transcript retention/order, Memory는 knowledge scope/provenance/verification/retention을 가진다. 둘은 reference로 연결한다.

## 5. Provider Session 저장 금지/제한

raw provider connection/session object는 저장하지 않는다. resume token이 필요하면 Capability Contract가 version/compatibility를 정의하고 Execution checkpoint/Artifact metadata로 저장한다.

Provider Session ID를 ThreadId/ConversationId column의 source로 사용하지 않는다.

## 6. Transaction Boundaries

- Command: Event + Current State + idempotency + Outbox
- Conversation append: Message + Conversation/Thread revision + Outbox
- Waiting: Task state + Continuation
- Thread branch: new Thread + source lineage + idempotency
- Memory promotion: global Memory revision + source provenance/link
- Redirect: Directive + Task Specification revision; runtime yield/schedule는 idempotent follow-up
- Suspension: durable suspend state + checkpoint/Continuation linkage
- Side Effect: Intent → external → Outcome

외부 시스템과 DB 사이 분산 원자성을 가장하지 않는다.

## 7. Recovery

startup 시:
1. Bot/Main Conversation 1:1 invariant 검증
2. Thread identities/lineage/message watermark 복원
3. Thread-local Memory scope/link 검증
4. Task/Execution/Continuation/Suspension 복원
5. pending Directive dedup/reconcile
6. Side Effect reconcile
7. Provider config/lifecycle 재구축
8. Provider Session은 optional compatibility optimization으로만 재검증

redirect가 commit됐지만 old Execution yield/new Execution 시작이 완료되지 않은 crash window를 Directive/Task revision으로 복구한다.

## 8. Growth / Retention

Main Conversation/Thread transcript가 수년 누적될 수 있으므로 hot-memory materialization을 금지하고 pagination, cold/archive, compaction/summary projection, Artifact reference를 사용한다. raw history retention/compaction 정확한 정책은 Open Question/Policy가 소유한다.

Thread branch/lineage 성장도 depth/node/count/archival metric을 갖고 resource limit은 Policy SSOT에서 관리한다.

## 9. Migration

v0.3 legacy data에 Thread identity가 없으면 기존 Session/Provider Session ID를 추측 변환하지 않는다. 안전한 default Main Conversation과 명시적 legacy/unassigned Thread mapping 또는 migration strategy를 사용하고 provenance를 남긴다.

## 10. 검증 기준

- Bot당 Main Conversation 중복 row가 생성되지 않는다.
- Session/Provider Session 삭제가 Conversation/Thread rows를 삭제하지 않는다.
- Waiting/Suspension/Directive crash failpoint에서 재개 정보가 손상되지 않는다.
- Thread-local Memory가 promotion 없이 global scope로 rewrite되지 않는다.
- Provider Session loss 후 persisted Thread/Context source로 복구한다.
- cache/index 삭제가 Conversation History/Canonical Memory를 손상시키지 않는다.
