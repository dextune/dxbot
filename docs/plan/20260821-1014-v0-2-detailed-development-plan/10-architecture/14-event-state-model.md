---
title: "Command·Event·State·Projection 모델"
document_id: "DXB-ARC-014"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-GOV-002", "DXB-ARC-010"]
---

# Command·Event·State·Projection 모델

## 1. 목적

상태 변경의 단일 경로, 감사 가능성, 충돌 검출, 복구, 조회 성능을 확보하고 Routine/Waiting Continuation/Side Effect Ledger가 Canonical/Derived 경계를 위반하지 않게 한다.

## 2. 기본 모델

`Command → Handler → Aggregate Decide → Atomic Journal/State/Outbox Commit → Projection/Notification`

- Command는 의도다.
- Domain Event는 Commit된 제품 사실이며 immutable하다.
- Current State는 Aggregate의 Canonical materialization이다.
- Projection은 재구축 가능한 조회 모델이다.
- Runtime notification은 Canonical source가 아니다.
- Side Effect Ledger는 외부 변경의 내구 operation state이며 Domain Journal을 대체하지 않는다.

## 3. Aggregate 경계

초기 Aggregate/Canonical owner:
- Bot
- Goal
- Task
- Execution
- Routine
- Message/Conversation Link
- Memory Record/Revision
- Core Lease
- Runtime Policy Set

Task Continuation은 별도 독립 Agent/Aggregate가 아니라 Task Waiting 상태에 결합된 Canonical child state다.

## 4. Event Envelope

- event_id
- aggregate type/id/revision
- event type/schema version
- occurred/recorded time
- actor/principal
- correlation/causation
- command/idempotency
- payload or Artifact reference
- policy/config/provider metadata reference
- sensitivity/retention
- digest

시간은 ordering의 유일 근거가 아니며 Aggregate revision이 상태 순서를 정의한다.

## 5. Commit 규칙

일반 Command:
1. current revision read
2. auth/precondition
3. pure decision
4. expected revision append
5. Event + Current State + idempotency + Outbox atomic commit
6. projection/index async update

Waiting 전이:
- `TaskWaiting` 의미와 Task current status, Continuation body/reference, 필요한 correlation/outbox를 같은 Unit of Work에서 Commit한다.
- Waiting state만 또는 Continuation만 Commit되는 상태를 허용하지 않는다.

Routine occurrence:
- occurrence claim/dedup, generated Task command/result reference, Routine next/last occurrence를 일관된 transaction/application Unit of Work로 연결한다.

## 6. Routine Event 후보

- RoutineCreated/Updated
- RoutineEnabled/Disabled
- RoutineOccurrenceClaimed
- RoutineTaskGenerated
- RoutineOccurrenceSkipped/Coalesced

정확한 Event 이름/schema는 구현 ADR에서 결정하되 occurrence idempotency와 owner 의미를 유지한다.

## 7. Waiting Event 의미

- TaskEnteringWaiting은 Continuation revision/reference와 결합한다.
- child/delegation result는 event/message ID로 dedup한다.
- TaskResumed는 resume guard와 consumed Continuation revision을 기록한다.
- restart가 기존 child 완료 사실을 덮어쓰지 않는다.

## 8. Side Effect Ledger와 Event 관계

비멱등 외부 동작은 Domain Event만으로 원자성을 가장하지 않는다.

1. Side Effect Intent/Key/Action Digest를 durable ledger에 Commit
2. 외부 effect 수행
3. Confirmed/Failed/Unknown outcome 기록
4. 필요 시 reconciliation evidence 기록

Domain 의미가 바뀌는 최종 결과만 해당 Aggregate Event로 반영한다. raw provider request/response는 Artifact/trace에 둘 수 있다.

## 9. Snapshot / Projection

Snapshot은 최적화이며 Journal을 대체하지 않는다. Projection은 source event set, version, checkpoint/watermark, rebuild, stale policy, permission filter를 선언한다.

v0.2 Projection에는 필요에 따라:
- Routine next/last/health
- Waiting Task pending summary
- Side Effect reconciliation queue
- Provider/Plugin lifecycle operation
을 포함할 수 있다.

## 10. Idempotency / Delivery

- Command: command ID 또는 scoped idempotency key
- Event: event ID + aggregate revision
- subscriber/message: at-least-once + consumer dedup
- Routine: occurrence ID
- Waiting resume: continuation revision/resume guard
- Side Effect: external/stable idempotency key + action digest

`exactly-once` 표현은 실제 transaction boundary 밖에 확대하지 않는다.

## 11. 예외상황

- unknown Event version: quarantine/upgrade-required
- commit response loss: idempotency lookup
- Projection poison: source Journal은 계속, Projection degraded
- payload over cap: Artifact reference
- secret risk: schema/redaction gate
- Waiting without Continuation: corruption/recovery-required
- Side Effect outcome unknown: retry가 아니라 reconciliation

## 12. 검증 기준

- 동일 Command 재전송이 하나의 의미 결과를 만든다.
- Waiting transition failpoint에서 status/Continuation이 함께 commit/rollback된다.
- Routine occurrence replay가 duplicate Task를 만들지 않는다.
- Side Effect external crash window가 Unknown으로 표현된다.
- Projection 삭제 후 Canonical source로 rebuild된다.
