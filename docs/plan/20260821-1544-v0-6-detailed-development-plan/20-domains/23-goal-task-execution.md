---
title: "Goal·Task·Execution·Supervisor 모델"
document_id: "DXB-DOM-023"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-015", "DXB-ARC-017", "DXB-DOM-020", "DXB-DOM-021", "DXB-DOM-027"]
---

# Goal·Task·Execution·Supervisor 모델

## 1. 목적

v0.5 Goal/Task/immutable Execution/Continuation/Side Effect/Supervisor semantics를 유지하고 Durable Process·ActionGrant·Runtime Resource linkage를 reference 수준으로 추가한다. Task state의 Canonical Owner는 계속 `DXB-DOM-023`이다.

## 2. Task Reference 확장

기존 ProjectId?/ChannelId?/ThreadId/SupervisorRef/delegation/authority refs에 필요 시 다음을 추가한다.
- ProcessRef?
- Collaboration CycleId?
- ActionGrantRef?
- causation/activity outcome refs
- ResourceHint? / memory class or estimated bytes?

이 reference가 Process/Grant/Resource state를 Task Aggregate 안에 복제하지 않는다.

## 3. Durable Process 관계

- Durable Process는 Task 생성/변경을 기존 typed Command/Application boundary로 요청한다.
- Task는 Process step state를 소유하지 않는다.
- Process terminal state가 child Task terminal state를 덮어쓰지 않는다.
- Process cancel/timeout만으로 running Task를 암묵 cancel하지 않는다.
- Task result는 committed outcome/reference로 Process에 관측된다.

## 4. Activity / Execution 경계

Activity는 새 실행 Aggregate가 아니다. 기존 Execution, Provider Host call, Tool execution, Side Effect Ledger가 replay-safe durable outcome을 제공할 수 있으면 이를 재사용한다.

replay 중:
- 완료된 Execution/Tool/Provider outcome ref를 재사용
- outcome이 불명확한 외부 Side Effect는 기존 Ledger/Reconciliation semantic 사용
- 새 Provider call을 deterministic transition 내부에서 직접 수행 금지

## 5. ActionGrant

Task/Execution이 grant-required action을 수행할 경우:
- current authorization과 GrantRef/action digest/budget을 preflight
- retry/replan/provider fallback이 grant use를 무한 재소비/증폭하지 않도록 semantic receipt를 사용
- revoke/exhaust 이후 신규 grant-required effect를 시작하지 않음

ActionGrant는 SupervisorRef/Membership/Authority를 대체하지 않는다.

## 6. Runtime Memory Resource Hint

Task/Execution은 admission을 위해 coarse memory class/estimated bytes를 제공할 수 있다. 그러나:
- MemoryReservation은 runtime-only permit
- Task row/state에 active permit을 Canonical truth로 저장하지 않음
- Waiting/Suspended 시 transient Context/Provider reservation을 release
- resume/new Execution에서 재-admission
- cancel/timeout/error/panic/drop에서 permit이 회수되어야 함

## 7. Immutable Execution / Race

Execution 시작 시 Task revision, selected Memory revisions/assertion states, ProcessRef?, ActionGrantRef?, policy generation, Context Plan을 pin한다. running snapshot은 Process step/Grant/retraction/pressure change 때문에 mutation하지 않는다.

추가 race:
- Process cancel vs Task completion
- Grant revoke/exhaust vs effect start
- Memory retract vs Execution Context build
- MemoryReservation acquire vs cancel/lease expiry

winner는 expected revision/generation + existing Execution fencing + current preflight policy가 결정한다.

## 8. 검증 기준

- Process가 Task state를 복제/직접 mutate하지 않음.
- replay가 이미 완료된 Execution/Provider Activity를 다시 호출하지 않음.
- AT-SEC-004 grant budget보다 많은 Canonical action/Side Effect가 발생하지 않음.
- Waiting/Suspended Task가 large transient reservation을 무기한 보유하지 않음.
- v0.5 Task/Side Effect/Live Control fixture regression 0.
