---
title: "저장소와 데이터 모델"
document_id: "DXB-ARC-015"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-011", "DXB-ARC-017"]
---

# 저장소와 데이터 모델

## 1. 목적

v0.5 persistent identity/revision/ScopeRef를 보존하면서 Durable Process, epistemic/dependency Memory metadata, ActionGrant를 중복 원본 없이 저장한다. physical table/enum/index 선택은 ADR이 소유한다.

## 2. 논리 저장 대상

기존 v0.5 대상에 다음 의미를 추가한다.

```text
durable-processes
process-steps / process-progress
process-command-links
process-activity-outcome-links

action-grants
grant-consumption / revocation

memory-records / memory-revisions
memory-epistemic-metadata
memory-temporal-metadata
memory-evidence-dependency-relations
memory-revalidation-quarantine-state
```

별도 Graph DB나 Workflow DB를 Canonical 필수 storage로 고정하지 않는다.

## 3. Durable Process 저장 원칙

최소 논리 의미:
- ProcessId / Revision
- ProcessDefinitionVersion
- RootIntentRef / RootMessageRef
- CurrentStepRef
- Correlation/Causation
- participating Aggregate references
- committed outcome references
- pending/issued command references
- budget/deadline refs
- recovery/reconciliation state
- terminal reason

Process row/document가 child Task/Memory/Directive/Execution의 상태 snapshot을 복제하지 않는다.

## 4. Memory 저장 확장

기존 MemoryId/revision/content/ScopeRef/provenance를 유지하며 다음을 additive하게 표현한다.
- Epistemic Kind
- Assertion State
- observed/recorded/valid/effective 시간 의미
- origin/trust/confidentiality/sensitivity metadata
- evidence/derived/contradict/supersede/invalidate/decision dependency relation
- revalidation/quarantine status

legacy v0.5 record는 metadata 부재를 명시적으로 표현하며 migration만으로 Verified로 쓰지 않는다.

## 5. ActionGrant 저장 원칙

논리적으로:
- GrantRef
- Principal/Audience
- Canonical Action Digest
- Target Resource/Scope
- Allowed Operation
- execution/cost/use budget
- expiry
- delegation attenuation
- Policy/Approval revision
- consumption/revocation state

secret/token 문자열 자체를 authorization truth로 만들지 않는다.

## 6. Runtime-only State

다음은 durable business truth로 저장하지 않는다.
- active MemoryReservation/permit
- current RSS sample
- allocator arena state
- OS/cgroup pressure sample
- active Core handle
- Provider Session

crash 후 permit은 복원하지 않고 Task/Execution/Process를 재평가해 새 admission을 수행한다.

## 7. Transaction / Reconciliation Boundary

- Process step advance: process revision + command/outcome reference
- Memory assertion/retraction: Memory revision/state + relation/revalidation marker
- ActionGrant consume: grant revision/use budget + action digest receipt
- Side Effect: 기존 write-ahead Intent/Ledger 의미 유지

Cross-Aggregate atomic transaction을 억지로 만들지 않는다. outbox/inbox/idempotency/expected revision/reconciliation을 사용한다.

## 8. Migration

v0.5→v0.6에서:
- Bot/Project/Channel/Conversation/Thread/Task/Execution/Memory ID를 변경하지 않는다.
- Memory content/provenance를 rewrite하지 않는다.
- epistemic metadata가 없던 Memory를 임의 Verified로 승격하지 않는다.
- 과거 active collaboration을 근거 없이 새 Durable Process로 조작하지 않는다.
- 필요한 active state만 explicit mapping하거나 recovery-required로 표시한다.

## 9. 검증 기준

- Process state와 child Aggregate state 중복 원본 0.
- grant consume/revoke가 restart 후 동일 semantic budget을 유지한다.
- source retraction에서 dependent Memory를 relation으로 찾을 수 있다.
- runtime MemoryReservation이 persistent Task/Core row에 저장되지 않는다.
- physical schema가 exact enum/graph/storage engine을 ADR 전에 과고정하지 않는다.
