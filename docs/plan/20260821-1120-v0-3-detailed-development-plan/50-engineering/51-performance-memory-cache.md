---
title: "성능·메모리·캐시 계획"
document_id: "DXB-ENG-051"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-017", "DXB-RUN-031", "DXB-RUN-034", "DXB-ENG-050"]
---

# 성능·메모리·캐시 계획

## 1. 목적

Runtime CPU/RSS/allocation/cache 비용을 계측하고 Canonical Long-term Memory growth를 runtime hot memory와 분리하며, Provider Host/SDK가 안정된 correctness boundary를 유지하면서도 불필요한 copy/allocation/cache miss를 만들지 않는지 검증한다.

## 2. Engineering Target과 Policy 구분

이 문서의 수치는 benchmark target이며 Runtime hard/default policy의 SSOT가 아니다. 실제 limit/default는 `31-resource-governance.md`가 참조하는 Policy/Config Owner가 정의한다.

기존 idle RSS, Core overhead, local command, event append, scheduler admission, projection lag 목표를 baseline으로 사용하되 hardware/workload manifest와 함께 갱신한다.

## 3. Memory Ownership Map

| 영역 | Owner | 회수 |
|---|---|---|
| Bot registry/coordinator | Runtime | idle eviction/deactivate |
| Core Working Context | Core | terminal/drop/spill |
| Provider Host call metadata/grant | Provider Host | call terminal/drop |
| model/tool stream buffer | Host/Provider boundary owner | bounded/spill |
| Provider connection/process pool | Provider lifecycle | drain/stop |
| channels | producer/consumer owner | item+byte cap |
| Memory cache | Memory Runtime | byte eviction |
| Context cache | Brain/Adapter | generation+byte cap |
| Projection cache | Query | TTL/byte cap |
| Plugin host memory | Plugin Host | hard budget/disable |
| Artifact temp | Artifact Store | quota/expiry |
| telemetry spool | Observability | byte/disk cap |
| Canonical Memory | Memory Store | retention/archive/forget only |

Provider Call Context는 Runtime 전체 object graph를 참조해 old generation을 장기 retain하지 않는다.

## 4. Provider Host / SDK Overhead Benchmark

Provider Host를 제거해서 성능을 얻는 방식은 허용하지 않는다. 대신 다음 overhead를 분리 측정한다.

- Host admission/selection/security pipeline latency
- call context construction allocations/bytes
- Contract DTO → Provider DTO mapping copy
- streaming event fan-out/copy
- telemetry/audit envelope allocations
- output validation/accounting cost
- cancellation/deadline wiring cost
- registry generation pin/reference retention
- SDK helper wrapper overhead

동일 fake/reference Provider를 `direct SPI microbench`와 `Host path bench`에서 비교해 Common boundary overhead를 분리하되 production path는 Host를 유지한다.

## 5. Long-term Memory Growth Benchmark

item/revision growth rate, canonical bytes, index amplification, archival/compaction throughput, unused-age, tombstone lag, cold restore/recall, backup/recovery cost를 별도 workload로 측정한다. Runtime RSS 테스트와 Canonical storage growth를 분리한다.

## 6. Cache 원칙

각 cache는 canonical source, key/version, max bytes+entries, TTL/eviction, invalidation, stale 허용, permission/sensitivity, hit/miss/retained metric, rebuild path를 가진다. cache pressure는 canonical delete를 호출하지 않는다. key에는 Identity/Policy/Capability/Provider generation을 필요에 따라 포함한다.

Provider/SDK가 capability result cache를 임의 소유해 Common Policy revision을 우회하지 않는다. Provider-local protocol cache가 필요하면 source/invalidation/security scope를 명시한다.

## 7. Provider / Plugin Cost

- sidecar/process RSS/CPU
- Provider connection/process pool retained memory
- per-provider serialization copy
- Provider Host activity/ref/permit retention
- Plugin host RSS/CPU/channel/file descriptor
- registry generation retention
- hot reload old-generation drain time/memory

Provider/Plugin 간 동일 payload를 불필요하게 복사하지 않고 immutable shared buffer/Artifact/streaming을 사용한다.

## 8. Scheduler / Admission Benchmark

동일 priority tie-break overhead, queue latency per class, oldest waiting, fairness under one-Bot flood, Provider Host admission/Provider quota contention, Draining transition 중 queue behavior를 측정한다.

## 9. Workload

1/10/100/1000 Bot, Core 1/4/16/64, 1k/100k/1M Memory metadata, Task DAG 폭/깊이, multi-provider, lifecycle start/drain/restart/remove, Plugin enable/disable, large Artifact, retry storm, timeout/cancel storm, Waiting recovery, Routine burst, 24h/7d soak를 포함한다.

## 10. Regression Gate

- micro: ID/serialization/state/queue/selector/Host pipeline/SDK mapping
- component: event append/context/memory recall/continuation/ledger/lifecycle
- e2e: submit→schedule→Provider Host→Reference/Fake Provider→commit
- lifecycle: provider start/drain/restart/remove, Plugin disable
- soak: RSS/handle/queue/activity/permit/generation growth

waiver는 owner/expiry/issue를 가진다.

## 11. 검증 기준

- steady soak에서 RSS가 지속 선형 증가하지 않는다.
- 모든 cache/channel/Provider/Plugin host buffer에 byte cap/metric이 있다.
- memory pressure가 Canonical Memory item을 암묵 삭제하지 않는다.
- Provider 교체/Plugin disable 후 old generation retained memory/activity가 drain된다.
- Provider Host/SDK overhead가 baseline 대비 계측되고 회귀 gate를 가진다.
- 대규모 조회/Provider output이 전체 payload를 불필요하게 heap materialize하지 않는다.
