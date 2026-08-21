---
title: "DXBOT 상세 개발 기획 문서 집합"
document_id: "DXB-INDEX"
version: "0.6.0"
status: "Reviewed Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000"]
---

# DXBOT 상세 개발 기획 문서 집합 v0.6

이 디렉터리는 `docs/plan/20260821-1409-v0-5-detailed-development-plan`을 strict superset으로 고도화한 Tier A 문서 패키지다. v0.5의 **Persistent Bot + Single Brain Semantics + Dynamic Core Lease + Main Conversation + Thread + Project + Channel + Scope-Aware Memory + Live Control + Provider Independence**를 유지하면서, **Durable Process + Epistemic/Secure Memory + Authorization/Information Flow + Bounded Collaboration + Runtime Memory Safety**를 추가한다.

> Identity와 판단은 Bot에, Task state는 Task에, Execution snapshot은 Execution에, Core Lease는 Scheduler에, 지식 의미는 Memory subsystem에, cross-aggregate 진행은 Durable Process에, Resource Memory budget은 Resource Governance에, Authorization/Information Flow는 Common Security owner에 속한다. Project/Channel/Process는 새 Brain이 아니다.

## 0. 버전 상속 규칙

v0.6은 v0.5 상세 개발기획 전체를 normative baseline으로 참조 상속한다. 동일 `document_id`의 v0.6 문서는 명시적으로 바꾼 의미만 supersede하며 나머지 v0.5/v0.4 계약은 계속 유효하다. 문장 생략은 제거·완화·deprecation을 뜻하지 않는다.

## 1. v0.6 핵심 경계

```text
Probabilistic Output
→ Proposal / Evidence
→ Authorization + Information Flow
→ Epistemic Validation
→ Durable Process / Canonical Command
→ Runtime Memory Admission / Reservation
→ Bounded Execution
→ Observable Outcome
→ Correction / Retraction / Recovery
```

## 2. 비협상 호환성

- Bot-only path 유지
- Project/Channel은 Brain이 아님
- `Thread ≠ Task ≠ Execution ≠ Core Lease`
- `Interface Session ≠ Provider Session ≠ Thread`
- History ≠ Memory
- Shared Memory ≠ Bot Memory replica
- immutable Execution
- Scheduler-owned Core Lease
- Provider Host/SPI/Tier A-B
- Side Effect/Live Control/recovery semantics

## 3. v0.6 신규 Canonical 문서

- `30-runtime/38-durable-process-orchestration.md` — cross-aggregate Process identity/progress/replay/reconciliation owner

이 문서만 v0.6에서 새 Canonical Owner다. Memory/Authorization/Resource/Channel은 기존 Canonical 문서를 강화한다.

## 4. Canonical Owner

| 의미 | Owner |
|---|---|
| Bot Identity/Lifecycle | `DXB-DOM-020` |
| Brain/Context Plan | `DXB-DOM-021` |
| Memory Scope + Epistemic Lifecycle | `DXB-DOM-022` |
| Task/Execution/Supervisor | `DXB-DOM-023` |
| Core Lease/Scheduler | `DXB-DOM-024` |
| Durable Bot Network | `DXB-DOM-025` |
| Project | `DXB-DOM-028` |
| Channel | `DXB-DOM-029` |
| Concurrency/Fencing | `DXB-RUN-030` |
| Runtime Resource/Memory Envelope | `DXB-RUN-031` |
| Authorization + Information Flow | `DXB-RUN-032` |
| Recovery | `DXB-RUN-033` |
| Observability | `DXB-RUN-034` |
| Live Control | `DXB-RUN-036` |
| Channel Routing/Run Termination | `DXB-RUN-037` |
| Durable Cross-Aggregate Process | `DXB-RUN-038` |
| Runtime Memory Benchmark | `DXB-ENG-051` |
| Testing | `DXB-ENG-052` |
| Acceptance | `DXB-DEL-061` |

## 5. 문서 구성

- Governance: 6
- Architecture: 8
- Domains: 10
- Runtime: 9
- Interfaces: 4
- Engineering: 5
- Delivery: 5
- Normative/plan documents: **47**
- `readme.md` + `manifest.md`: 2
- Total Markdown: **49**

## 6. 신규 Acceptance

- AT-PROC-001 Durable Process Crash/Replay
- AT-MEM-007 Epistemic State Separation
- AT-MEM-008 Retraction/Revalidation Propagation
- AT-SEC-003 Information Flow/Declassification
- AT-SEC-004 Durable ActionGrant/Semantic Replay
- AT-COLLAB-002 Collaboration Cycle Termination
- AT-COLLAB-003 Cost-Adjusted Collaboration Utility (P1)
- AT-RMEM-001 Process-Wide Memory Admission
- AT-RMEM-002 Memory Pressure/OOM Prevention
- AT-RMEM-003 Runtime Memory Retention/Leak Freedom

기존 v0.5/v0.4/v0.3 Acceptance는 삭제·축소하지 않는다.

## 7. 구현 순서

Runtime Memory Safety는 모든 V6 Gate의 cross-cutting 전제다.

`V6-M0 Durable Correctness → V6-M1 Epistemic/Secure Memory → V6-M2 Durable Process → V6-M3 Authorization Replay Safety → V6-M4 Collaboration Termination → V6-M5 Utility/Operational Gate`.

기존 v0.5 M0~M7의 순서는 유지한다.

## 8. 완료 정의

1. v0.5 Canonical Owner와 Bot-only path가 유지된다.
2. `DXB-RUN-038` 하나만 신규 cross-aggregate Process owner다.
3. replay 중 completed LLM/Tool/Provider Activity를 재실행하지 않는다.
4. Memory가 epistemic/retraction/dependency 의미를 가진다.
5. Private→Shared information flow가 별도 policy를 통과한다.
6. Authorization Decision Owner가 하나다.
7. ActionGrant가 semantic replay를 bounded하게 제한한다.
8. Collaboration Run이 total budget과 terminal reason을 가진다.
9. Single-Bot-first default와 utility evidence가 있다.
10. process-wide Runtime Memory가 hierarchical budget/headroom으로 bounded된다.
11. reserve-before-admit/pressure degradation/large-payload cap/leak gate/staged recovery가 연결된다.
12. Structural/Consistency + Cross-Layer Executability **2회 독립 재검수**를 통과한다.

검수 이력과 inventory evidence는 `manifest.md`가 소유한다.
