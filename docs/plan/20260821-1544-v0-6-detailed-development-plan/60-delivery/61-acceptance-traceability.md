---
title: "수용 기준과 원칙 추적성"
document_id: "DXB-DEL-061"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-ARC-017", "DXB-DOM-022", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-036", "DXB-RUN-037", "DXB-RUN-038", "DXB-DEL-060", "DXB-ENG-052"]
---

# 수용 기준과 원칙 추적성

## 1. 목적

v0.5까지의 모든 Acceptance를 strict-superset regression 기준으로 유지하면서 v0.6 Durable Process, Epistemic/Information-Flow Memory, ActionGrant replay safety, Collaboration termination, Runtime Memory safety를 자동화 가능한 Acceptance로 추가한다.

## 2. 기존 Acceptance 유지

v0.5의 AT-PROJECT-001, AT-CHANNEL-001~003, AT-MEM-005/006, AT-CTX-003, AT-SEC-002, AT-COLLAB-001, AT-MIG-001과 v0.4/v0.3의 CONV/THREAD/CTX/MEM/CTRL/SESSION/BOT/BRAIN/CORE/TASK/SFX/ROUTINE/NET/HAR/IFC/STO/SEC/REC/OBS/MOD/PLUGIN/REPO/POL/SPI Acceptance의 상세 pass/fail 조건은 삭제·축소하지 않는다.

## 3. v0.6 신규 요구 추적

| Requirement | Canonical Owner | Acceptance |
|---|---|---|
| FR-PROC-001 Durable cross-aggregate replay | RUN-038 | AT-PROC-001 |
| FR-MEM-007 Epistemic state separation | DOM-022 | AT-MEM-007 |
| FR-MEM-008 Retraction/revalidation propagation | DOM-022 | AT-MEM-008 |
| FR-SEC-003 Information flow/declassification | RUN-032/DOM-022 | AT-SEC-003 |
| FR-SEC-004 Durable ActionGrant replay safety | RUN-032 | AT-SEC-004 |
| NFR-COLLAB-002 Causal run termination | RUN-037/RUN-038/RUN-031 | AT-COLLAB-002 |
| NFR-COLLAB-003 Cost-adjusted collaboration utility | RUN-037/ENG-051 | AT-COLLAB-003 |
| NFR-RMEM-001 Process-wide memory admission | RUN-031 | AT-RMEM-001 |
| NFR-RMEM-002 Pressure/OOM prevention | RUN-031/ENG-051 | AT-RMEM-002 |
| NFR-RMEM-003 Runtime retention/leak freedom | RUN-031/ENG-051 | AT-RMEM-003 |

## 4. AT-PROC-001 — Durable Process Crash / Replay

Given multi-aggregate process가 진행 중이고,
When child create / Waiting / result / review / promotion / target Memory commit / process-step / ack 지점에서 crash/restart를 수행하면,
Then:
- Process identity/definition version/progress가 복원된다.
- 이미 완료된 LLM/Tool/Provider Activity를 replay 과정에서 재실행하지 않는다.
- duplicate child/delegation/promotion/Side Effect가 없다.
- pending step만 idempotently resume/reconcile한다.
- Process가 child Aggregate state를 직접 덮어쓰지 않는다.
- terminal 이후 late outcome이 implicit resume하지 않는다.

## 5. AT-MEM-007 — Epistemic State Separation

- Unverified factual Claim이 Verified Claim과 동일 authority로 사용되지 않는다.
- Decision/Policy/Preference/Procedure와 factual Claim의 conflict/verification policy가 분리된다.
- Model/Tool output만으로 Verified가 생성되지 않는다.
- legacy v0.5 Memory가 migration만으로 Verified가 되지 않는다.

## 6. AT-MEM-008 — Retraction / Revalidation Propagation

`A → B → C` evidence/promotion chain에서 A를 retract하면:
- B/C를 blind delete하지 않는다.
- dependency를 찾을 수 있다.
- B/C가 Stale/RevalidationRequired 또는 동등 상태가 된다.
- 새 Context가 B/C를 Verified knowledge처럼 무조건 사용하지 않는다.
- revalidation 결과에 따라 유지/수정/Superseded/Retracted를 결정할 수 있다.

## 7. AT-SEC-003 — Information Flow / Declassification

Given Bot Private Memory read와 Channel write 권한이 각각 존재할 때,
When private content를 Shared Scope에 publish하려 하면,
Then target information-flow policy/declassification/approval 없이 commit되지 않는다.

valid approval이 있으면 current policy/revision을 pin해 commit할 수 있고, stale/revoked approval은 실패한다.

## 8. AT-SEC-004 — Durable ActionGrant / Semantic Replay

Given approval-required action에 1회 또는 bounded grant가 발급되고,
When retry/replan/delegation/restart/provider fallback/duplicate delivery를 발생시키면,
Then 허용된 semantic execution budget보다 많은 Canonical action/Side Effect가 발생하지 않는다.

Grant는 existing idempotency/Side Effect Ledger를 대체하지 않으며 revoke/exhaust 뒤 신규 covered action은 시작되지 않는다.

## 9. AT-COLLAB-002 — Collaboration Cycle Termination

A→B→C→A 반복 응답을 유도해도:
- total activation/round/hop/delegation/cost/deadline cap이 유지된다.
- Cycle이 explicit terminal reason으로 종료된다.
- terminal Cycle late result가 새 Cycle을 암묵 생성하지 않는다.
- per-message cap만 지키며 causal loop가 무한 지속되는 경로가 없다.

## 10. AT-COLLAB-003 — Cost-Adjusted Collaboration Utility / P1

대표 workload에서 Single-Bot baseline과 Multi-Bot mode를 반복 비교하고:
- task quality/acceptance coverage
- verified evidence coverage
- reviewer correction/error propagation
- duplicate work/stall/loop
- latency/token/cost/activation
- Runtime overhead/peak memory
를 측정한다.

Multi-Bot default 승격은 품질과 비용의 측정 근거를 가져야 한다.

## 11. AT-RMEM-001 — Process-Wide Memory Admission

Given 많은 Bot/Channel이 heterogeneous Context/Provider/Tool workload를 동시에 생성하고,
When local queue/cache/context cap은 정상 범위지만 합산 예상 memory가 Deployment Runtime Memory budget을 초과하면,
Then:
- Scheduler/Core/Provider activity가 global/hierarchical admission을 우회하지 않는다.
- reservation이 없는 memory-heavy work는 defer/reject/partial-selection된다.
- Waiting/Suspended work가 transient Context/Provider reservation을 무기한 보유하지 않는다.
- nested resource wait가 memory permit 교착/장기 점유를 만들지 않는다.
- safety/control/recovery headroom은 일반 work가 소비하지 못한다.
- control/recovery path가 memory saturation으로 starvation되지 않는다.

## 12. AT-RMEM-002 — Memory Pressure / OOM Prevention

Given Runtime Memory를 Normal→Constrained→Critical→Emergency 경계까지 증가시키고 large Provider stream/slow consumer를 동시에 발생시키면,
Then:
- cache trim/background throttle/new admission stop이 deterministic하게 적용된다.
- streaming cumulative/in-flight byte cap과 backpressure가 유지된다.
- allocator OOM을 정상 제어 이벤트로 기다리지 않는다.
- Canonical State 삭제 없이 일반 work를 감압한다.
- hard limit 이전 workload shedding을 시도하고 외부 OOM-kill 발생 시 Canonical recovery가 가능하다.

## 13. AT-RMEM-003 — Runtime Memory Retention / Leak Freedom

Given Bot/Channel/Thread/Provider/Process lifecycle을 create/use/cancel/archive/reconnect/restart로 반복하고 quiescence를 만들면,
Then:
- task/subscriber/stream/cache pin/MemoryReservation/permit/Lease count가 baseline 또는 bounded cache state로 회귀한다.
- owner-accounted retained bytes가 반복 횟수에 비례해 증가하지 않는다.
- strong `Arc` cycle/orphan task가 large Context/Artifact를 영구 보유하지 않는다.
- RSS는 allocator fragmentation을 감안해 별도 관찰하되 반복 workload에서 unbounded retained trend가 없다.

## 14. Cross-Layer Traceability

```text
FR-PROC-001
→ ARC-014/015 event/storage
→ RUN-038 process owner
→ DOM-023/025 typed command/outcome
→ RUN-033 recovery
→ ENG-052 fault fixture
→ AT-PROC-001
→ R-061/R-062

FR-SEC-003/004
→ DOM-022 labels/epistemic state
→ RUN-032 Authorization/Flow/Grant
→ RUN-030 race
→ IFC-040 API
→ ENG-052 security fixture
→ AT-SEC-003/004
→ R-063~067

NFR-RMEM-001~003
→ RUN-031 envelope/reservation/pressure
→ DOM-021/024 + RUN-037 enforcement
→ RUN-033 staged recovery
→ RUN-034 metrics
→ ENG-051 benchmark
→ ENG-052 fixture/soak
→ AT-RMEM-001~003
→ R-070~077
```

## 15. 검증 기준

- 신규 10개 Acceptance가 Owner/Test/Risk/OQ/Metric에 연결된다.
- 기존 Acceptance ID/semantic regression 0.
- skip/waiver는 success가 아니며 owner/expiry/risk를 가진다.
