---
title: "관측성·Trace·Audit"
document_id: "DXB-RUN-034"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-017", "DXB-RUN-032", "DXB-RUN-033", "DXB-RUN-036"]
---

# 관측성·Trace·Audit

## 1. 목적

v0.5 correlation chain에 Process/Cycle/Activity/Grant/Evidence/Runtime Memory를 추가해 cross-aggregate 흐름, epistemic correction, semantic replay rejection, memory pressure를 한 trace에서 설명할 수 있게 한다. raw content/high-cardinality ID를 metric label로 사용하지 않는다.

## 2. Correlation Chain

```text
ProcessId?
→ CycleId?
→ Project/Channel/Bot/Thread
→ Task/Delegation
→ Activity/Execution
→ ActionGrant?
→ Core Lease
→ Provider/Tool/Side Effect
→ Result/Evidence
→ Memory Proposal/Assertion/Relation
→ Retraction/Revalidation?
```

trace/audit field에는 필요한 범위에서 IDs/revisions/generations를 저장하고 metric label은 bounded dimension만 사용한다.

## 3. Process / Collaboration Metrics

- process active/age/stuck/recovery-required/terminal reason
- process step latency / duplicate outcome suppression
- cycle rounds/hops/activation/delegation/cost/token/deadline
- NoProgress/LoopDetected/BudgetExhausted rate
- single→multi admission rate
- single-vs-multi quality/cost/latency delta
- duplicate research/result ratio
- reviewer correction/evidence coverage

## 4. Epistemic / Security Metrics

- Proposed/Verified/Disputed/Retracted/Stale/Quarantined count/rate by bounded class
- retraction→revalidation backlog/latency
- evidence relation traversal work
- poisoned/quarantined proposal rate
- Private→Shared publication denied/approval-required/declassified count
- ActionGrant issued/consumed/exhausted/revoked/replay-rejected
- stale policy/grant generation rejection

raw Memory content, source secret, private text는 metric label이 아니다.

## 5. Runtime Memory Metrics

최소 분리:
- process RSS / peak RSS
- main process + owned child-process resident memory where observable
- logically accounted in-flight bytes
- reserved/granted/rejected/released reservation bytes/count
- owner별 live/retained object bytes/count
- cache retained bytes/pins
- Waiting/Suspended retained reservation bytes
- Context/Provider/Tool/Artifact/serialization cumulative/in-flight bytes
- pressure state transition/time-in-state
- large payload spill/stream abort bytes
- recovery rebuild concurrency/bytes
- allocator active/reserved/resident stats if available
- subscriber/task/permit/Lease object count

`RSS 증가 = leak`로 단정하지 않는다. quiescence owner bytes/count + repeated workload retained trend + RSS/allocator trend를 함께 본다.

## 6. Audit

추가 감사 대상:
- Process create/advance/terminal/recovery-required
- Activity outcome linkage/reconciliation
- Memory assertion state/retract/revalidate/quarantine
- declassification/approval/publication
- ActionGrant issue/consume/revoke/exhaust
- cycle terminal reason/budget exhaustion
- Emergency admission stop/work shedding/fail-stop decision

## 7. Freshness / Diagnostic Safety

Presence/index/summary/process view/pressure probe는 `observed_at`, source revision/watermark, stale/degraded를 표현한다. Emergency에서 allocation-heavy full dump나 large payload logging을 피한다.

## 8. 검증 기준

- 하나의 Process에서 child Task→Activity→Grant→Evidence→Memory 관계를 추적 가능.
- duplicate replay/grant reject/retraction propagation 원인을 설명 가능.
- global memory pressure 원인을 owner/reservation/buffer 차원에서 구분 가능.
- secret/raw sensitive content가 metrics/log/trace/provider diagnostic에 나타나지 않음.
- leak gate가 RSS 단일 지표에 의존하지 않음.
