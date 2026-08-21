---
title: "성능·메모리·캐시 계획"
document_id: "DXB-ENG-051"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-RUN-031", "DXB-RUN-034", "DXB-ENG-050"]
---

# 성능·메모리·캐시 계획

## 1. 목적

Runtime CPU/RSS/allocation/cache 비용을 계측하고 Canonical Long-term Memory growth를 runtime hot memory와 분리해 장기 상주 안정성을 검증한다.

## 2. Engineering Target과 Policy 구분

이 문서의 수치는 benchmark target이며 Runtime hard/default policy의 SSOT가 아니다. 실제 limit/default는 `31-resource-governance.md`가 참조하는 Policy/Config Owner가 정의한다.

기존 v0.1의 idle RSS, Core overhead, local command, event append, scheduler admission, projection lag 등 초기 benchmark target을 baseline으로 사용하되 하드웨어/workload manifest와 함께 갱신한다.

## 3. Memory Ownership Map

| 영역 | Owner | 회수 |
|---|---|---|
| Bot registry/coordinator | Runtime | idle eviction/deactivate |
| Core Working Context | Core | terminal/drop/spill |
| model/tool stream buffer | Provider Adapter | bounded/spill |
| channels | producer/consumer owner | item+byte cap |
| Memory cache | Memory Runtime | byte eviction |
| Context cache | Brain/Adapter | generation+byte cap |
| Projection cache | Query | TTL/byte cap |
| Plugin host memory | Plugin Host | hard budget/disable |
| Artifact temp | Artifact Store | quota/expiry |
| telemetry spool | Observability | byte/disk cap |
| Canonical Memory | Memory Store | retention/archive/forget policy only |

## 4. Long-term Memory Growth Benchmark

별도 workload로 다음을 측정한다.
- item/revision growth rate
- canonical bytes per Bot/project
- metadata index amplification
- archival/compaction throughput
- unused-age distribution
- tombstone propagation lag
- cold restore/recall latency
- backup/recovery cost

Runtime RSS 테스트와 Canonical storage growth 테스트를 분리한다.

## 5. Cache 원칙

cache마다 canonical source, key/version, maximum bytes+entries, TTL/eviction, invalidation event, stale 허용, permission/sensitivity, hit/miss/retained metric, rebuild path를 가진다.

cache pressure는 canonical delete를 호출하지 않는다. cache key에는 Identity/Policy/Capability/Provider generation이 필요에 따라 포함된다.

## 6. Provider / Plugin Cost

- sidecar/process RSS/CPU
- Provider connection/process pool retained memory
- per-provider serialization copy
- Plugin host RSS/CPU/channel/file descriptor
- registry generation retention
- hot reload old-generation drain time/memory

Provider/Plugin 간 동일 payload를 불필요하게 복사하지 않고 Artifact/shared immutable buffer를 사용한다.

## 7. Scheduler Metrics Benchmark

동일 priority tie-break의 deterministic overhead, queue latency per class, oldest waiting age, fairness under one-Bot flood, provider quota contention을 측정한다.

## 8. Workload

1/10/100/1000 Bot, Core 1/4/16/64, 1k/100k/1M Memory metadata, Task DAG 폭/깊이, multi-provider, Plugin enable/disable, Provider drain/remove, large Artifact, retry storm, Waiting recovery, Routine burst, 24h/7d soak를 포함한다.

## 9. Regression Gate

- micro: ID/serialization/state/queue/selector
- component: event append/context/memory recall/continuation/ledger
- e2e: submit→schedule→fake provider→commit
- lifecycle: provider drain/plugin disable
- soak: RSS/handle/queue growth

waiver는 owner/expiry/issue를 가진다.

## 10. 검증 기준

- steady soak에서 RSS가 지속 선형 증가하지 않는다.
- 모든 cache/channel/plugin host에 byte cap/metric이 있다.
- memory pressure가 Canonical Memory item을 암묵 삭제하지 않는다.
- Provider 교체/Plugin disable 후 old generation retained memory가 drain된다.
- 대규모 조회가 전체 payload를 heap에 materialize하지 않는다.
