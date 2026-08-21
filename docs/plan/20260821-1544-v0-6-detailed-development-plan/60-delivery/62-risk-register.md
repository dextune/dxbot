---
title: "위험 등록부"
document_id: "DXB-DEL-062"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DEL-060", "DXB-ARC-017", "DXB-DOM-022", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-032", "DXB-RUN-037", "DXB-RUN-038", "DXB-ENG-051"]
---

# 위험 등록부

## 1. 목적

v0.5의 R-001~060과 R-SPI-*를 유지하면서 Durable Process, epistemic/security Memory, semantic replay, collaboration termination, Runtime Memory/OOM 계열 위험을 Acceptance/Test/OQ/Metric에 연결한다.

## 2. 기존 위험 유지

R-001~060과 R-SPI-*의 완화/Acceptance는 모두 유효하다. 특히 Session/Thread 혼합, unbounded queue/cache, stale authorization, Side Effect duplicate, Provider Domain 침투, Project/Channel Brain화, Shared Memory 복제, Channel fan-out, Role/Authority 혼합, migration provenance 손실은 v0.6에서도 직접 회귀 gate다.

## 3. v0.6 신규 Risk

| ID | 위험 | 영향 | 핵심 완화/Gate |
|---|---|---|---|
| **R-061** | Cross-Aggregate progress owner 부재 | Critical | `DXB-RUN-038` Durable Process, AT-PROC-001 |
| **R-062** | replay 중 LLM/Tool/Side Effect 재실행 | Critical | replay/Activity boundary + outcome refs + Side Effect Ledger |
| **R-063** | unverified/poisoned Memory가 durable truth로 고착 | Critical | Epistemic state + untrusted proposal + quarantine, AT-MEM-007 |
| **R-064** | source 철회 후 promoted Memory가 계속 authoritative | Critical | dependency + revalidation propagation, AT-MEM-008 |
| **R-065** | Private→Shared 정보 유출 | Critical | information-flow/declassification, AT-SEC-003 |
| **R-066** | retry/delegation으로 승인 action semantic replay | Critical | ActionGrant/budget/consumption, AT-SEC-004 |
| **R-067** | authorization logic이 서비스별로 drift | Critical | Common Authorization Decision Owner |
| **R-068** | bounded message가 무한 causal collaboration을 형성 | Critical | Cycle total budget + terminal reason, AT-COLLAB-002 |
| **R-069** | Multi-Bot이 비용만 증가시키고 품질 이득 없음 | High | Single-Bot-first + AT-COLLAB-003 |
| **R-070** | revalidation/dependency 추적으로 Memory/RSS 폭증 | High | bounded traversal/background budget/cache |
| **R-071** | 각 component는 bounded지만 동시 합산 in-flight bytes가 process budget 초과 | Critical | global/hierarchical memory admission + AT-RMEM-001 |
| **R-072** | Core 수를 memory ceiling으로 오인해 heterogeneous Context/Provider payload 폭주 | Critical | per-Execution class/reservation + Scheduler enforcement |
| **R-073** | Context/serialization/Provider/retry/response buffer copy amplification | Critical | shared refs + bounded render/stream/spill + cumulative cap |
| **R-074** | allocator allocation failure를 잡아 정상 복구할 수 있다고 가정 | Critical | proactive pressure/admission + bounded/fallible large allocation |
| **R-075** | `Arc` cycle/orphan task/subscriber/cache pin이 payload를 장기 보유 | Critical | ownership direction + Weak/ID + cancellation/join/drop/leak soak |
| **R-076** | MemoryReservation/permit accounting drift 또는 release 누락 | Critical | RAII/terminal cleanup + granted/released invariant + fault test |
| **R-077** | restart/index rebuild/Provider reconnect가 동시에 hot-load되어 recovery OOM | High | staged/lazy recovery + rebuild memory admission |

## 4. Risk Cluster / Owner

### Durable Correctness — R-061/062
Owner: RUN-038 + RUN-033. Gate: V6-M0/M2, AT-PROC-001.

### Epistemic / Security — R-063~067
Owner: DOM-022 + RUN-032. Gate: V6-M1/M3, AT-MEM-007/008, AT-SEC-003/004.

### Collaboration — R-068/069
Owner: RUN-037 + RUN-031/ENG-051. Gate: V6-M4/M5.

### Runtime Memory — R-070~077
Owner: RUN-031(Resource), ENG-051(measurement), RUN-033(recovery), enforcement by DOM-021/024/RUN-037/Provider Host. Gate: Cross-Cutting + AT-RMEM-001~003.

## 5. 선행 신호

- Process step replay 시 Provider call count 증가
- same causal key duplicate child/Side Effect
- Verified without evidence/policy validation
- retract source 대비 dependent Verified 잔존
- Private source publication without declassification
- grant remaining use보다 effect count가 큼
- Cycle hop/activation이 root intent 하나에서 계속 증가
- selected participant 증가 대비 quality evidence 없음
- accounted bytes 대비 RSS drift/peak 급증
- reservation granted-released delta 누적
- quiescence 후 task/subscriber/pin count baseline 미회귀
- recovery startup peak RSS가 entity 수와 선형 hot-load

## 6. Phase Blocker

- V6-M0: R-061/062/067/071/074
- V6-M1: R-063/064/065/070
- V6-M2: R-061/062/071/072/076/077
- V6-M3: R-065/066/067
- V6-M4: R-068/071~074
- V6-M5: R-069/070/075~077

Critical risk owner/evidence 없이 관련 Gate를 종료하지 않는다.

## 7. 검증 기준

- R-061~077 모두 Acceptance/Test/OQ/Metric 중 하나 이상에 연결된다.
- 기존 R-001~060/R-SPI mitigation regression 0.
- Runtime Memory risk를 단순 성능 이슈로 낮추지 않는다.
