---
title: "테스트·검증 전략"
document_id: "DXB-ENG-052"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-013", "DXB-ARC-016", "DXB-ARC-017", "DXB-DOM-022", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-036", "DXB-RUN-037", "DXB-RUN-038", "DXB-ENG-050"]
---

# 테스트·검증 전략

## 1. 목적

v0.5 deterministic state/concurrency/recovery/Provider/Project/Channel 검증을 유지하면서 Durable Process replay, Epistemic/Information-Flow Memory, ActionGrant semantic replay, Collaboration termination, Runtime Memory/OOM resilience를 자동화 가능한 evidence로 검증한다.

## 2. Test Layer

Unit, Property/Model, Concurrency Model, Component, Contract/Conformance, Host Integration, E2E, Fault/Recovery, Performance, Soak, Security, Architecture/Docs를 유지한다. Core correctness는 deterministic fake/reference Provider를 사용하며 AI utility benchmark와 분리한다.

## 3. v0.6 Deterministic Fixture

- Durable Process fake store/step runner/outcome receipt
- replay에서 Provider/Tool call count를 검출하는 probe
- crash failpoint: child create/Waiting/result/review/promotion/target commit/process advance/ack
- Memory epistemic/assertion state store
- evidence dependency/retraction/revalidation fixture
- information label/declassification policy fixture
- Common AuthorizationDecision/ActionGrant consume/revoke fixture
- Collaboration Cycle loop/stall/budget fixture
- fake hierarchical Runtime Memory budget/reservation
- fake pressure probe Normal/Constrained/Critical/Emergency
- large stream + slow consumer fixture
- encoded-small/decoded-large payload fixture
- subscriber/reconnect/cache-pin/permit lifecycle fixture
- staged recovery/rebuild fixture

기존 virtual clock/ID/RNG/barrier/Scheduler/Provider Session/Side Effect/Control fixture를 재사용한다.

## 4. Durable Process Suite

AT-PROC-001:
- 8개 crash window 각각에서 restart/replay
- ProcessId/revision/progress 동일
- committed Activity outcome 재사용
- replay-time LLM/Tool call count 0
- duplicate Task/delegation/promotion/Side Effect 0
- pending step만 resume/reconcile
- terminal Process late result implicit reopen 0

## 5. Epistemic / Information Flow Suite

AT-MEM-007:
- factual Unverified/Verified와 Decision/Policy/Preference/Procedure authority 분리
- legacy-unclassified migration이 Verified를 생성하지 않음

AT-MEM-008:
- `A → B → C` relation 생성 후 A retract
- B/C dependency lookup 가능
- blind delete 0
- B/C Stale/RevalidationRequired
- 새 Context에서 Verified knowledge처럼 무조건 사용되지 않음

AT-SEC-003:
- Bot Private read allow + Channel write allow 상태 구성
- declassification/approval 없이는 publication denied
- valid approval 후 target commit 가능
- stale/revoked approval로 commit 불가

## 6. Authorization Replay Suite

AT-SEC-004:
- bounded-use ActionGrant 발급
- retry/replan/delegation/restart/provider fallback/duplicate delivery 주입
- allowed semantic execution budget 초과 0
- grant revoke/exhaust와 effect preflight barrier 검증
- existing Side Effect Ledger idempotency와 중복/충돌 없음

## 7. Collaboration Termination / Utility

AT-COLLAB-002:
- A→B→C→A 반복을 유도
- total activation/round/hop/cost/deadline cap 유지
- explicit terminal reason
- late result implicit Cycle 생성 0

AT-COLLAB-003(P1):
- Single-Bot baseline과 Multi-Bot 반복 비교
- quality/evidence/correction/duplicate/stall/latency/token/cost/activation/Runtime overhead 측정
- default 승격은 측정 근거가 있을 때만 허용

## 8. Runtime Memory Suite

AT-RMEM-001:
- heterogeneous Context/Provider/Tool burst
- local cap은 정상이나 global budget 초과
- reservation 없는 memory-heavy admission 0
- Waiting/Suspended transient reservation release
- nested wait permit deadlock/장기점유 0
- safety headroom starvation 0

AT-RMEM-002:
- fake budget으로 pressure 단계 전환
- large Provider stream + slow consumer
- cache trim/background throttle/new admission stop 확인
- cumulative/in-flight byte cap 유지
- allocator OOM을 기다리는 정상 control path 0
- optional real cgroup/native integration에서 external kill 후 recovery 확인

AT-RMEM-003:
- create/use/cancel/archive/reconnect/restart 반복
- task/subscriber/stream/pin/Reservation/permit/Lease count baseline 회귀
- retained bytes unbounded trend 0
- RSS/allocator/owner bytes를 분리 판정

## 9. Combined Cross-Layer E2E

```text
User request
→ Single-Bot or Collaboration admission
→ Channel/Task Durable Process
→ Manager delegation
→ Researcher Activity/Execution
→ crash after result before process advance
→ restart/replay without Provider re-call
→ Reviewer validation
→ Claim/Evidence commit
→ Thread→Channel→Project publication
→ source evidence retraction
→ dependent revalidation
→ private publication denied
→ ActionGrant issue
→ duplicate retry rejected/bounded
→ Cycle explicit terminal
→ many Bot/Core memory pressure
→ memory-heavy admission throttle
→ large stream bounded/spilled/cancelled
→ cancel/reconnect releases runtime resources
→ staged recovery
```

동시에 v0.5 Bot-only Thread/Provider/Live Control fixture를 실행한다.

## 10. Document Relationship Review — 2 Passes

v0.5 패키지에 기록된 3-review 방식은 과거 evidence로 유지한다. v0.6은 현재 Repository 규약과 고도화 계획에 맞춰 두 개의 독립 review로 통합한다. 검사항목은 축소하지 않는다.

### Review 1 — Structural / Consistency
- file/path/ID/depends_on/inventory/naming
- Canonical Owner duplication
- Process vs Task/Channel/Memory owner boundary
- Authorization/Memory/Resource owner boundary
- dependency cycle
- Event/Storage/API/Test/Acceptance/Risk/OQ relation
- v0.5 strict-superset inheritance

### Review 2 — Cross-Layer Executability / Compatibility
- 위 combined E2E를 Input→Application→Domain→Persistence→Runtime/Scheduler→Provider→Recovery→Interface까지 추적
- replay-time Provider call, duplicate semantic action, false Verified, retraction loss, Private→Shared leak, unbounded cycle, memory budget bypass, retained leak, recovery storm 검사
- v0.5 Bot-only/Provider/Side Effect/Live Control/Migration regression 검사

각 Review에서 발견한 문제는 수정 후 해당 Review 범위를 다시 실행한다.

## 11. Release Gate

기존 gate에 추가:
- Process replay/fault suite
- Epistemic/retraction/security suite
- Common Authorization/Grant suite
- Collaboration terminal/utility evidence
- fake memory budget/pressure deterministic suite
- real-process RSS/allocator/host integration where available
- leak/retention soak
- staged recovery memory storm

## 12. 검증 기준

- 신규 10개 Acceptance가 Test/Risk/OQ/Metric에 매핑됨.
- 2회 독립 Review evidence가 `manifest.md`에 존재함.
- 기존 v0.5/v0.4 Acceptance 의미 축소 0.
- Provider Framework/Side Effect/Live Control/Scheduler ownership regression 0.
