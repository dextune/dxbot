---
title: "미결정 사항과 권장 기본값"
document_id: "DXB-DEL-063"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-GOV-003", "DXB-ARC-017", "DXB-DOM-022", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-037", "DXB-RUN-038", "DXB-DEL-060"]
---

# 미결정 사항과 권장 기본값

## 1. 목적

v0.6 Canonical semantic은 고정하되 lifecycle enum, physical representation, numeric threshold, progress detector, memory estimation처럼 증거가 필요한 세부를 ADR/benchmark 전에 임의 확정하지 않는다.

## 2. 기존 Open Question 유지

v0.5의 OQ-001~050, OQ-101~124 및 P2 질문은 유효하다. v0.6이 명시적으로 닫지 않은 기존 결정 Gate를 축소하지 않는다.

## 3. v0.6 Open Questions

| ID | 질문 | 권장 기본 방향 | Gate |
|---|---|---|---|
| **OQ-051** | Durable Process 정확한 lifecycle/state representation | semantic 먼저, physical enum/schema는 ADR | V6-M0/M2 |
| **OQ-052** | Activity retry/fallback classification | semantic retry와 provider-local retry 분리 유지 | V6-M0/M2 |
| **OQ-053** | Epistemic Kind exact enum | 최소 class만 고정, ontology 과확장 금지 | V6-M1 |
| **OQ-054** | Assertion State exact transition | retract/supersede/stale/quarantine 의미 우선 | V6-M1 |
| **OQ-055** | retraction dependency traversal | bounded explicit relation, Graph DB 강제 금지 | V6-M1 |
| **OQ-056** | Information Label taxonomy / declassification | 최소 label + explicit publication policy | V6-M1/M3 |
| **OQ-057** | Durable ActionGrant consume model | action digest + remaining budget/use, token-only 금지 | V6-M3 |
| **OQ-058** | Collaboration progress/stall 판정 | deterministic runtime signals 우선, LLM self-report 단독 금지 | V6-M4 |
| **OQ-059** | Single→Multi collaboration admission rule | deterministic policy first | V6-M4 |
| **OQ-060** | utility benchmark default 승격 기준 | workload별 quality/cost/latency evidence | V6-M5 |
| **OQ-061** | Deployment Runtime Memory envelope의 source | host/container limit discovery 가능 시 반영 + explicit configured ceiling; 무제한 default 금지 | Cross-Cutting/M0 |
| **OQ-062** | per-Execution memory reservation granularity/estimation | coarse class + bytes hybrid, SDK/allocator overhead safety margin, Waiting 시 transient release; exact prediction 요구 금지 | Cross-Cutting/M2 |
| **OQ-063** | pressure state threshold/hysteresis | RSS 하나가 아니라 accounted bytes + resident/OS pressure 조합 benchmark | Cross-Cutting/M2 |
| **OQ-064** | large payload spill threshold/temporary storage | Policy SSOT + workload benchmark + disk quota/cleanup/recovery, Canonical owner와 분리 | M2/M5 |
| **OQ-065** | allocator stats/allocator 선택 | optional Infrastructure adapter; 특정 allocator를 Normative 필수로 고정하지 않음 | M5 |
| **OQ-066** | cgroup/PSI/native host pressure integration | 플랫폼 adapter로 격리, Core Runtime semantic 동일 | M5 |
| **OQ-067** | leak/retained RSS release gate | owner bytes/object quiescence + repeated workload trend + RSS 결합 | M5 |

## 4. v0.6에서 이미 고정된 항목

Open Question이 아니다.
- Durable Process는 Brain/Agent/Task engine이 아님
- Process는 cross-aggregate progress ref만 소유
- replay 중 completed non-deterministic Activity 재실행 금지
- global exactly-once execution 약속 금지
- Memory Epistemic/Assertion/Retraction 의미 필요
- legacy Memory auto-Verified 금지
- access authorization ≠ information-flow authorization
- Common Authorization Decision Owner 하나
- ActionGrant는 bounded artifact이며 기존 permission 대체 아님
- Collaboration Run은 bounded/terminal
- Single-Bot-first default
- Core count ≠ Runtime Memory ceiling
- reserve-before-admit + safety headroom
- OOM catch를 정상 recovery 전략으로 사용하지 않음
- staged/lazy recovery

## 5. 결정 증거

각 OQ는 use/non-use case, invariants, security/privacy, resource/RSS/allocation/cache, concurrency model, fault/recovery, migration/rollback, API compatibility, Provider independence, deterministic fixture, benchmark/prototype, owner, ADR link를 사용한다.

## 6. 검증 기준

- OQ-051~067이 해당 Gate 전에 ADR/실험/benchmark로 닫힌다.
- 질문이 남아 있다는 이유로 unsafe default, all-member wake, unbounded memory, catch-OOM recovery를 구현하지 않는다.
- exact enum/threshold/schema/allocator를 증거 없이 Normative 문서에 고정하지 않는다.
