---
title: "Dynamic Core Scheduler"
document_id: "DXB-DOM-024"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-023", "DXB-DOM-021", "DXB-ARC-012", "DXB-ARC-017"]
---

# Dynamic Core Scheduler

## 1. 목적

Scheduler-owned Core Lease/fairness/admission을 유지하면서 `DXB-RUN-031`의 Runtime Memory capacity를 별도 admission 차원으로 소비한다. Core 수와 process memory를 동일 ceiling으로 취급하지 않는다.

## 2. 비협상 원칙

- Bot/Project/Channel/Process schema에 fixed Core count를 Identity로 저장하지 않는다.
- Coordinator/Process/Manager가 Core Lease를 mint/revoke하지 않는다.
- Scheduler가 Runtime Memory budget의 Canonical Owner가 되지 않는다.
- memory-heavy WorkItem은 필요한 reservation/admission 결과 없이 먼저 실행하지 않는다.
- stale/expired Lease late result는 Canonical commit 권한이 없다.

## 3. Admission Composition

논리적 입력:
- WorkItem priority/class/fairness key
- Bot/Task/Project?/Channel?/Process? provenance
- CPU/Core/Provider quota
- ResourceHint / memory class/estimated bytes
- current MemoryPressureState
- deadline/cancellation

```text
WorkItem
→ Resource Governance memory admission/reservation
→ Scheduler fairness/Core capacity
→ Core Lease
→ Context/Provider bounded execution
```

구현 순서는 deadlock을 막기 위해 composite admission 또는 고정 acquisition order를 사용한다. unrelated resource를 기다리며 memory permit을 장기 점유하지 않는다.

## 4. Waiting / Suspension

외부 event/Provider quota/long wait 때문에 실행이 멈춘 상태가 large transient reservation과 Core Lease를 불필요하게 유지하지 않게 한다. durable checkpoint/reference를 남기고 재개 시 새 admission을 받는다.

## 5. Pressure Integration

- Constrained: speculative/background parallelism을 줄일 수 있음
- Critical: memory-heavy 신규 Lease admission을 축소/거부
- Emergency: 일반 신규 work admission 중단, control/recovery path headroom 보존

정확한 threshold/policy는 `DXB-RUN-031`이 소유하며 Scheduler가 재정의하지 않는다.

## 6. Race / Cleanup

- reservation acquire vs cancel/timeout
- Lease issue vs pressure transition
- Lease expiry vs reservation release
- Provider late chunk vs terminal release

permit/Lease release는 idempotent/RAII 또는 동등 cleanup contract를 사용하고 double release/retention leak을 검증한다.

## 7. 검증 기준

- AT-RMEM-001에서 local Core ceiling이 남아 있어도 global memory capacity 부족 시 work가 defer/reject됨.
- Core count hard ceiling만으로 memory safety를 주장하지 않음.
- control/recovery admission이 일반 Channel burst에 starvation되지 않음.
- v0.5 deterministic fairness/lease fencing tests 유지.
