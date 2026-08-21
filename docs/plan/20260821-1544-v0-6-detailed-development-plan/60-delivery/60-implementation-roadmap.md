---
title: "구현 로드맵과 단계별 Gate"
document_id: "DXB-DEL-060"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-ARC-010", "DXB-ARC-016", "DXB-ARC-017", "DXB-DOM-022", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-037", "DXB-RUN-038", "DXB-ENG-052"]
---

# 구현 로드맵과 단계별 Gate

## 1. 목적

v0.5 M0~M7의 Common Provider Framework/Persistent Bot/Project/Channel/Scope-Aware Memory/Live Control 구현 순서를 뒤집지 않고, v0.6 correctness boundary를 **선행/교차 Gate**로 삽입한다. 기간보다 dependency와 exit evidence를 우선한다.

## 2. 유지되는 Foundation

재설계하지 않는다.
- Common Capability Contract/Provider Host/Lifecycle/SDK/Conformance/Tier A-B
- Persistent Bot/Main Conversation/Thread
- Project/Channel persistent collaboration
- Conversation History ≠ Memory
- immutable Execution + durable Live Control
- Waiting/Suspension/Side Effect recovery
- Dynamic Core Scheduler ownership
- Provider Session independence
- Bot-only path

## 3. Cross-Cutting — Runtime Memory Safety Gate

모든 V6 milestone에 공통 적용한다.

먼저 확정:
1. Deployment/process Runtime Memory envelope + safety/control/recovery headroom
2. hierarchical byte accounting/reservation owner
3. Scheduler/Core admission과 memory budget 연계
4. pressure state/degradation order/hysteresis
5. large payload/stream/spill boundary
6. owner graph/RAII cleanup/leak gate
7. OOM은 사후 catch recovery가 아니라 prevention/fail-stop 대상이라는 의미

V6-M2 이전:
- reservation acquire/release primitive 또는 동등 Resource admission
- Context/Provider/queue 주요 byte owner accounting
- AT-RMEM-001 기본 fixture

V6-M4 이전:
- Channel fan-out/response aggregation global memory admission 연계
- Critical/Emergency 신규 collaboration 억제
- AT-RMEM-002

V6-M5:
- soak/reconnect/recovery storm AT-RMEM-003
- RSS/accounted/retained/allocator 지표 분리 evidence

## 4. V6-M0 — Durable Correctness Boundary

확정:
- Durable Process semantic/owner
- Workflow/Activity replay boundary
- no global exactly-once guarantee
- Authorization Decision Owner
- ActionGrant semantic
- Epistemic/Assertion 상위 의미
- Information-flow invariant

Exit:
- 신규 Canonical Owner 중복 0
- replay 중 Provider/Tool call 재실행 의미 0
- authorization policy drift 경로 0
- v0.5 inheritance/dependency DAG 검증

## 5. V6-M1 — Epistemic / Secure Memory

구현/문서화:
- Epistemic Kind / Assertion State
- temporal metadata
- evidence/dependency relation
- retraction/revalidation/quarantine
- information label
- declassification-aware promotion
- legacy v0.5 Memory migration

Exit:
- AT-MEM-007/008
- AT-SEC-003
- existing AT-MEM-005/006 regression 0

## 6. V6-M2 — Durable Process Runtime

구현/문서화:
- Process state/definition version
- Activity outcome linkage
- stable command causation/idempotency
- recovery/reconciliation
- Task/Bot Network/Directive linkage
- process resource budget

Exit:
- AT-PROC-001
- replay-time LLM/Tool call 0
- existing Waiting/Side Effect/Live Control regression 0

## 7. V6-M3 — Authorization Replay Safety

구현/문서화:
- Common Authorization Decision path
- SecurityDomainRef minimal semantic
- durable ActionGrant
- high-risk effect preflight
- delegation attenuation
- retry/replan/restart consumption

Exit:
- AT-SEC-004
- existing membership revoke/Role-Authority Acceptance regression 0

## 8. V6-M4 — Collaboration Termination

구현/문서화:
- Collaboration Cycle
- total activation/delegation/round-hop/cost/deadline budget
- progress/stall
- terminal reason
- Single-Bot-first admission
- Reviewer evidence validation
- global Runtime Memory admission for fan-out/aggregation

Exit:
- AT-COLLAB-002
- unbounded causal loop 0
- AT-RMEM-002

## 9. V6-M5 — Utility / Operational Gate

구현/측정:
- Memory quality benchmark
- Single vs Multi collaboration benchmark
- Process/retraction/grant/cycle observability
- Runtime Memory pressure/leak/recovery soak

Exit:
- AT-COLLAB-003 또는 동등 P1 evidence
- AT-RMEM-003
- collaboration default policy에 quality/cost/latency 근거 존재

## 10. 기존 M0~M7 관계

v0.5의 M0 Contract/Scope → M1 Project/Channel → M2 Scope-Aware Memory/Context → M3 Channel Runtime → M4 Collaboration → M5 API/CLI/TUI → M6 Web → M7 Distributed/HA 방향은 유지한다.

v0.6 Gate는 관련 구현 전에 선행한다.
- v0.5 M2 전/중: V6-M1
- v0.5 M3/M4 전: V6-M2/M3/M4 및 Runtime Memory cross-cutting
- v0.5 M5~M6 전: v0.6 API/observability/utility evidence
- M7에서도 Process/Authorization/Memory/Resource semantic은 transport에 독립

## 11. Scope Control / Non-Goals

P0에서 선행 구현하지 않는다.
- 범용 Workflow DSL
- Process/Project/Channel Brain
- Graph DB 필수화
- 학습 기반 collaboration router
- 분산 consensus/Raft
- 모든 allocation custom fallible wrapper
- 특정 allocator 강제
- exact numeric budget/threshold를 Normative 문서에 하드코딩
- global exactly-once execution 약속

## 12. 검증 기준

- 각 V6-M0~M5에 Acceptance/Risk/OQ가 연결된다.
- Runtime Memory Gate가 각 milestone의 Scheduler/Context/Provider/Channel 경로에 연결된다.
- v0.5 Bot-only/Provider/Live Control/Side Effect path를 막지 않는다.
- UI workaround로 Process/Authorization/Memory/Resource owner를 복제하지 않는다.
