---
title: "Command·Event·State·Projection 모델"
document_id: "DXB-ARC-014"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-GOV-002", "DXB-ARC-010"]
---

# Command·Event·State·Projection 모델

## 1. 목적

v0.5의 `Command → Aggregate Decide → Atomic Journal/State/Outbox Commit → Projection`을 유지하고 Durable Process, Epistemic Memory lifecycle, ActionGrant를 Canonical event/state 의미로 연결한다.

## 2. Canonical Aggregate / State Owner

기존 Bot/Project/Channel/Conversation/Thread/Task/Execution/Memory/Directive/Side Effect 의미를 유지한다. v0.6에서 추가되는 Canonical owner는 **Durable Cross-Aggregate Process 하나**다.

Runtime MemoryReservation/pressure probe/Presence/cache/index는 Canonical Aggregate가 아니다.

## 3. Event 의미 후보

### Durable Process
- ProcessCreated
- ProcessStepAdvanced
- ProcessWaiting
- ProcessCompleted / Failed / RecoveryRequired
- ActivityScheduled / ActivityOutcomeLinked

정확한 이름보다 `process definition version + stable step/command causation + committed outcome ref + revision` 의미가 중요하다.

### Memory
- MemoryAssertionStateChanged
- MemoryRetracted
- MemoryRevalidationRequired
- MemoryQuarantined / ReleasedFromQuarantine
- EvidenceRelationRecorded / Invalidated

### Authorization
- ActionGrantIssued
- ActionGrantConsumed
- ActionGrantExhausted
- ActionGrantRevoked

Channel Collaboration Run의 routing/termination event는 `DXB-RUN-037` 의미를 따르고 필요 시 ProcessRef/CycleId를 연계한다.

## 4. Commit 규칙

- Process step은 child Aggregate의 state copy가 아니라 typed Command ref와 committed outcome ref를 저장한다.
- 동일 logical step이 replay되어도 stable idempotency/causation key로 semantic duplicate를 제거한다.
- Activity outcome이 commit된 뒤 replay가 동일 Provider/Tool call을 다시 실행하지 않는다.
- Memory Assertion State 변경은 content rewrite와 동일 사건으로 취급하지 않는다.
- retraction은 dependent Memory를 삭제하지 않고 revalidation work를 생성/표시할 수 있다.
- ActionGrant consume은 target action digest/use budget과 원자적 또는 reconciliation 가능한 경계를 가진다.
- Runtime Memory reservation acquire/release는 runtime telemetry/event일 수 있으나 durable business truth로 복원하지 않는다.

## 5. Idempotency / Fencing

- Process: ProcessId + DefinitionVersion + StepRef + expected revision + semantic command key
- Activity: Activity/Execution outcome ref + attempt/fencing
- Memory retraction: MemoryId/revision + expected assertion state
- Revalidation: source dependency revision + target Memory revision
- ActionGrant: GrantRef + action digest + consumption revision
- Collaboration Run: CycleId/ProcessRef + causal hop/activation budget

## 6. Projection

재생성 가능:
- process status/timeline
- cycle status/progress/terminal summary
- Memory evidence graph/index
- revalidation/quarantine queue view
- ActionGrant remaining-use status view
- Runtime Memory pressure/owner metrics

Projection은 authority, Memory truth, Process progress source가 아니다.

## 7. 검증 기준

- duplicate activity outcome이 Process step을 두 번 진행시키지 않는다.
- replay가 Provider/Tool side effect를 재실행하지 않는다.
- Memory retract가 source content를 삭제하지 않고 dependent state를 추적할 수 있다.
- grant duplicate consume가 허용 budget을 초과하지 않는다.
- projection 삭제 후 Canonical source에서 process/memory/grant view를 rebuild할 수 있다.
