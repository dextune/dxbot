---
title: "성능·메모리·캐시 계획"
document_id: "DXB-ENG-051"
version: "0.1.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-RUN-031", "DXB-RUN-034", "DXB-ENG-050"]
---


# 성능·메모리·캐시 계획

## 1. 목적

모델 지연에 가려지기 쉬운 Runtime 자체의 CPU, 메모리, 할당, 캐시, 저장 지연을 예산화하고 장시간 상주 안정성을 검증한다.

## 2. 책임 범위

- 성능 예산과 workload
- memory ownership/retained bytes
- cache 설계
- benchmark/profile
- admission 연동
- regression gate

수치는 P0 스파이크에서 측정해 조정하는 **초기 engineering target**이며 제품 SLA가 아니다.

## 3. 핵심 예산 후보

| 항목 | 초기 목표 |
|---|---:|
| idle daemon RSS | 100 MiB 이하(Provider sidecar 제외) |
| 비활성 Bot persistent hot metadata | Bot당 평균 16 KiB 이하 |
| 활성 idle Bot coordinator | Bot당 평균 128 KiB 이하 |
| Core base overhead | Core당 평균 512 KiB 이하(모델 context/artifact 제외) |
| local Command 처리 p95 | 50 ms 이하(외부 호출 제외) |
| event append p95 | 15 ms 이하(개발 기준 장치) |
| scheduler admission p99 | 10 ms 이하 |
| Core local start p95 | 25 ms 이하 |
| projection lag 정상 p95 | 1 s 이하 |
| 100k Memory metadata recall p95 | 150 ms 이하(embedding 외부 지연 제외) |
| graceful shutdown | owned resource leak 0 |
| unbounded allocation/channel | 0 |

하드웨어와 dataset을 benchmark manifest에 고정하고 절대 수치보다 회귀율도 함께 본다.

## 4. Workload Model

- 1/10/100/1000 Bot, 활성 비율 변화
- Core concurrency 1/4/16/64
- 짧은 Tool burst와 긴 model stream
- 1k/100k/1M Memory metadata
- Task DAG 폭/깊이
- Bot message fan-out
- large Artifact upload/stream
- projection/index backlog
- provider timeout/retry storm
- 24h/7d soak
- shutdown under load

Synthetic workload와 실제 trace replay를 모두 사용한다.

## 5. Memory Ownership Map

| 메모리 영역 | Owner | 상한/회수 |
|---|---|---|
| Bot registry | Runtime Host | idle eviction |
| Bot coordinator state | Bot Supervisor | deactivate/drop |
| Core working set | Core | hard byte budget/terminal drop |
| model stream buffer | Harness Adapter | bounded/coalesce/spill |
| channels | producer/consumer pair | capacity + item bytes |
| Memory cache | Memory Runtime | byte LRU/revision invalidation |
| context prefix cache | Brain/Adapter owner | generation/byte cap |
| projection cache | Query Service | TTL/byte cap |
| Artifact temp | Artifact Store | quota/expiry |
| telemetry spool | Observability | disk/memory cap |

Owner가 없는 allocation은 leak으로 간주한다.

## 6. 캐시 원칙

캐시를 추가하려면:
- canonical source
- key와 version
- maximum bytes/entries
- TTL/eviction
- invalidation event
- stale 허용
- permission/sensitivity key
- hit/miss/eviction/retained bytes metric
- rebuild/fallback
- stampede protection
- negative cache 정책
을 문서화한다.

동일 데이터의 Bot/Core/UI별 중복 캐시를 금지한다. Core는 공통 immutable snapshot을 참조한다.

## 7. Context Cache

모델 context는 다음으로 분리한다.
- stable: system policy, Identity revision, capability schema
- semi-stable: Goal/Memory summaries
- dynamic: current Task/messages/tool results

stable prefix digest를 Provider별 serialized form과 연결할 수 있다. invalidation key 누락으로 오래된 권한/Identity가 재사용되지 않게 policy/capability revision을 포함한다.

## 8. 데이터 배치와 캐시 효율

- hot metadata와 cold payload 분리
- contiguous iteration 우선
- pointer-heavy graph 전체 로드 금지
- hot struct field size/align 조사
- frequently accessed counters와 rarely accessed strings 분리
- false sharing 가능 atomic/counter padding은 profile 후
- ID/reference로 large object 중복 방지
- batch DB reads와 prepared statements
- N+1 query 탐지
- Task/Memory list는 column projection
- serialization buffer reuse는 owner-local로 제한

## 9. Allocation 규칙

- benchmark에서 allocations/op, bytes/op 측정
- repeated rendering/serialization의 temporary String 제거
- `Vec` capacity는 percentile 기반
- excessive Arc clone도 비용 계측
- small object pool은 fragmentation/retention 비교
- long-lived arena가 short-lived object를 붙잡지 않게 함
- stream chunk size를 너무 작게 하지 않고 latency와 균형
- compressed Artifact는 CPU/메모리 window 제한
- `clone()` count를 단순 금지 metric이 아닌 hot path profile과 결합

## 10. Storage/Index 성능

- command write와 background index workload 분리
- write batch는 latency와 crash window 고려
- event payload size histogram
- snapshot threshold
- projection checkpoint batch
- DB busy/lock contention
- WAL/checkpoint stall
- query plan regression
- vector index resident bytes
- compaction의 foreground 영향
- backup/restore throughput

## 11. Profiling

- CPU flamegraph
- heap allocation/retained graph
- async task/lock/channel wait
- I/O latency
- DB query plan
- sidecar process RSS/CPU
- cache hit/eviction
- allocator fragmentation
- file descriptor/process/thread count
- network buffering
- model/provider 외부 시간 분리

Profile은 production-like build와 representative workload에서 수행한다.

## 12. Regression Gate

- microbenchmark: ID/serialization/state transition/queue
- component: event append, context assembly, memory recall
- end-to-end: submit→schedule→fake harness→commit
- soak: leak/growth
- comparative baseline commit
- 허용 회귀율과 absolute ceiling
- noisy benchmark quarantine가 아니라 재현 환경 개선
- regression waiver에 owner/expiry/issue

## 13. 예외상황

- cache hit은 높지만 RSS 증가: retained bytes 기준으로 제거
- low allocation이나 CPU 증가: 전체 budget 균형
- throughput 증가와 tail latency 악화: p95/p99 우선
- sidecar memory 폭증: provider별 hard limit/restart circuit
- memory pressure: cache trim→fan-out 감소→admission stop
- index rebuild가 foreground를 방해: resource class 분리
- tracing 때문에 overhead: sampling/batch, audit는 별도
- benchmark hardware 차이: normalized comparison/manifest

## 14. 확장성

다중 노드에서는 node별 resource profile과 cluster aggregate를 분리한다. data locality와 Artifact transfer bytes를 scheduler 비용에 포함한다. 캐시는 node-local이고 canonical state를 공유 cache에 의존하지 않는다.

## 15. 구현 우선순위

- **P0:** benchmark harness, allocation/RSS/latency metrics, channel/cache budgets
- **P1:** soak, context cache, Memory index profile, regression CI
- **P2:** cluster workload, remote transfer/locality optimization
- **P3:** adaptive cache/scheduler tuning

## 16. 검증 기준

- 24시간 soak에서 idle/steady workload RSS가 지속 선형 증가하지 않는다.
- 모든 cache/channel에 byte cap과 metric이 있다.
- Core concurrency 증가가 global memory hard limit을 넘지 않는다.
- P0 benchmark가 기준 commit 대비 허용 회귀율을 통과한다.
- context stable prefix cache가 권한/Identity 변경 후 무효화된다.
- Memory/Task 대규모 조회가 전체 payload를 heap에 materialize하지 않는다.
