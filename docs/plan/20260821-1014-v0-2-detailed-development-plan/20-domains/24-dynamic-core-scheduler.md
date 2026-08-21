---
title: "Dynamic Core Scheduler"
document_id: "DXB-DOM-024"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-023", "DXB-DOM-021", "DXB-ARC-012"]
---

# Dynamic Core Scheduler

## 1. 목적

Bot 생성 시 Core 수를 고정하지 않고 resource/fairness/parallel benefit에 따라 Core Lease를 동적으로 발급하며 Provider selection과 deterministic queue semantics를 명확히 한다.

## 2. Core 불변조건

Core는 Bot/Task/Execution에 속한 일시 Lease이며 독립 Persona, Goal, Long-term Memory를 갖지 않는다. terminal/expired Lease의 late result는 Canonical State를 Commit할 권한이 없다.

## 3. Admission

Resource Governance가 제공하는 grant/limit snapshot을 입력으로 `Global → Per-Bot → Provider → Tool/Sandbox → Memory/Artifact Budget → Lease` admission을 수행한다. Permit 획득/반환 순서를 고정해 deadlock과 leak을 방지한다. Provider permit은 selector가 고른 Provider에 대해 획득한다.

본 문서는 자원 limit 값의 Owner가 아니며 실제 상한/default는 `DXB-RUN-031`의 Policy SSOT를 참조한다.

## 4. Provider Selection 관계

Scheduler가 Provider 구현을 직접 고르지 않는다.

1. Task/Execution 요구 capability 전달
2. Selector가 immutable Registry/Policy snapshot에서 Provider 결정
3. Scheduler가 해당 Provider의 quota/permit을 admission에 적용
4. Execution/Lease에 selection reference pin

Provider unavailable이면 명시적 queue/reject/alternate-attempt policy를 적용한다.

## 5. Queue / Fairness

- 모든 queue bounded by items + bytes
- class/priority/deadline/aging을 policy가 계산
- per-Bot fair share/cap
- low priority starvation protection
- provider별 rate/concurrency queue 분리 가능
- overflow 결과를 reject/replace/coalesce로 명시

### Deterministic Tie-Break
동일 effective priority의 작업은 안정된 enqueue sequence와 stable WorkItemId 같은 명시 key로 결정한다. container iteration order나 hash order에 의존하지 않는다. 정확한 자료구조는 구현 ADR에서 결정한다.

## 6. 관측 지표

- queue latency by class/priority
- oldest waiting age
- starvation indicator/count
- per-Bot fair share utilization
- provider queue latency/rate-limit wait
- permit utilization/leak reconciliation
- admission reject/coalesce reason

## 7. 결과 Merge

Core output은 immutable Result Fragment로 반환한다. disjoint keyed result는 deterministic union, ordered result는 stable sort key, 동일 entity write는 expected revision conflict, natural language synthesis는 별도 merge step을 사용한다.

## 8. Restart

persisted Requested/Lease state를 reconcile한다. Waiting Task의 Continuation owner는 Task이며 Scheduler가 별도 duplicate continuation을 만들지 않는다. expired Lease의 Provider callback은 fencing으로 거부한다.

## 9. 검증 기준

- Bot 생성 API에 core count가 없다.
- 동일 입력/priority에서 deterministic scheduling order가 재현된다.
- flood test에서 oldest waiting/starvation metric이 관측되고 다른 Bot이 무기한 굶지 않는다.
- Provider A/B의 quota가 독립 적용되고 selection은 Execution 동안 고정된다.
- crash/cancel/timeout 후 permit leak가 0이다.
