---
title: "성능·Runtime Memory·캐시 계획"
document_id: "DXB-ENG-051"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-017", "DXB-DOM-022", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-031", "DXB-RUN-034", "DXB-RUN-037", "DXB-RUN-038", "DXB-ENG-050"]
---

# 성능·Runtime Memory·캐시 계획

## 1. 목적

v0.5의 queue/cache/context boundedness를 유지하면서 process-wide Runtime Memory safety를 실제 workload로 측정한다. 구조적 correctness와 성능 최적화를 분리하고, `RSS`, accounted bytes, retained bytes, allocator resident를 동일 지표로 취급하지 않는다.

## 2. Runtime Memory Ownership / Accounting Map

| 영역 | Owner | 제한/회수 |
|---|---|---|
| Runtime Memory envelope/headroom | `DXB-RUN-031` | configured ceiling/pressure policy |
| Execution MemoryReservation | Resource Governance | reserve-before-admit + terminal/drop release |
| Context/retrieval | Brain/Context Runtime | token+byte cap / shared ref / terminal release |
| Provider input/output stream | Provider Host | cumulative+in-flight bytes/backpressure |
| Tool/Artifact staging | Tool/Artifact Runtime | bounded stream/spill/ref |
| Serialization/response aggregation | API/Orchestration | byte cap/backpressure |
| Project/Channel/Memory caches | owner Query/Runtime | entries+bytes cap/eviction/rebuild |
| Prefix cache | Context Runtime | digest/revision/pin lifetime |
| Canonical Conversation/Memory | durable store | retention/archive/forget only |

## 3. 측정 차원

최소:
- logically accounted/reserved in-flight bytes
- owner별 live/retained object bytes/count
- cache retained bytes/pin count
- allocator active/reserved/resident stats if available
- process RSS / peak RSS
- owned child-process RSS where observable
- device/VRAM where applicable
- mmap/file-backed/spill bytes
- queue/subscriber/task/permit/Lease count

allocator가 free를 받아도 RSS가 즉시 내려가지 않을 수 있으므로 `RSS 증가 = leak`으로 판정하지 않는다.

## 4. Reservation Benchmark

필수 측정:
- per-Execution reserved vs actual/accounted bytes estimation error
- heterogeneous Context size × concurrent Bot/Core peak RSS
- reserve/release throughput/lock contention
- admission reject/defer latency
- Waiting/Suspended transient reservation release
- nested resource acquisition under cancellation/timeouts
- permit leak negative fixture

reservation hot path는 하나의 global mutable map scan/장시간 mutex를 병목으로 만들지 않는다.

## 5. Copy Amplification / Large Payload Benchmark

측정:
- queue→Context→serialization→Provider→retry→response 단계별 payload copies
- encoded vs decoded/decompressed expansion ratio
- cumulative/in-flight stream bytes
- slow consumer under large Provider output
- Artifact spool/reference peak heap
- temporary storage quota/cleanup

구조 위반:
- full history clone→JSON clone→Provider retry clone chain
- unbounded response aggregation
- `Content-Length`만 믿고 decoded allocation
- spill을 무제한 disk buffer로 사용

## 6. Leak / Retention Soak

반복 lifecycle:
- Bot/Channel/Thread create/use/archive
- Channel join/leave/reconnect
- Provider start/stop/restart/session recreation
- subscription register/unregister
- cancel/timeout/retry storm
- cache pin/eviction 경쟁
- Process create/wait/terminal/recovery
- staged recovery/index rebuild

quiescence 후:
- task/subscriber/stream/pin/Reservation/permit/Lease object count가 baseline 또는 bounded cache state로 회귀
- owner retained bytes가 반복 횟수에 비례해 증가하지 않음
- strong `Arc` cycle/orphan task가 large Context/Artifact를 영구 retain하지 않음
- RSS는 allocator 특성을 감안하되 repeated workload에서 unbounded trend가 없어야 함

## 7. Pressure-State Benchmark

Normal→Constrained→Critical→Emergency를 deterministic fake budget/probe로 먼저 검증하고 real-process integration에서 resident behavior를 측정한다.

측정:
- transition/hysteresis stability
- cache trim latency/bytes reclaimed
- background throttle
- new work rejection/defer
- control/recovery latency
- throughput/latency degradation
- hard limit까지 safety margin
- recovery after pressure relief

Linux/cgroup 환경에서는 제한된 integration fixture를 사용할 수 있으나 특정 allocator/container를 Canonical 요구로 고정하지 않는다.

## 8. Recovery Memory Storm Benchmark

- startup active entity count × index/history size
- lazy/on-demand materialization peak RSS
- rebuild concurrency/bytes
- Provider reconnect Context creation count
- safety headroom 유지
- crash/OOM-kill restart recovery time/peak

모든 Bot/Project/Channel/Memory/index를 eager load하는 방식은 구조 위반이다.

## 9. Collaboration / Utility Cost

Single-Bot vs Multi-Bot 비교:
- task success/acceptance coverage
- verified evidence coverage
- duplicate work
- reviewer correction
- activation/round/hop
- latency/token/cost
- Runtime overhead vs Provider time
- peak/reserved Runtime Memory

품질 이득 없이 비용/latency/memory만 증가하면 Multi-Bot default 승격 근거가 아니다.

## 10. Regression Gate

- AT-RMEM-001: global/hierarchical memory admission 우회 0
- AT-RMEM-002: hard OOM 이전 deterministic pressure degradation
- AT-RMEM-003: lifecycle 반복 retained leak 0
- local queue/cache/context cap 유지
- cache/index 제거 후 canonical rebuild 가능
- selected Context token+byte cap 유지
- v0.5 Project/Channel scale workload regression 0
