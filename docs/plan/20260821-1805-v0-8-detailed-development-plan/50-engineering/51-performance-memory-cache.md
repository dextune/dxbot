---
title: "성능·Runtime Memory·캐시 계획"
document_id: "DXB-ENG-051"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ENG-050", "DXB-RUN-031", "DXB-RUN-034"]
---

# 성능·Runtime Memory·캐시 계획

## 1. 목적

v0.8 Application Contract/Runtime Instance/CLI가 correctness 경계를 유지하면서 allocation, retained memory, cache locality, stream/backpressure, startup recovery 비용을 측정 가능한 Gate로 만든다.

## 2. 측정 분리

### Runtime Host
- RSS/peak RSS, accounted/reserved bytes
- canonical state vs query page/snapshot vs subscriber vs receipt/index/cache bytes
- allocations/op, bytes/op
- request decode/DTO map/response serialization
- active snapshots/cursors/subscribers/receipts
- startup recovery/readiness latency
- lock/fencing/endpoint recovery overhead

### CLI
- RSS/peak RSS
- page/chunk/live buffer bytes
- serialization/sanitization temporary allocations
- local journal retained bytes
- stdout/file writer throughput/backpressure
- spawned task/file descriptor/subscriber count

Runtime과 CLI 수치를 합쳐 leak source를 숨기지 않는다.

## 3. 필수 Workload

1. 1M-equivalent Message/Task/Memory rows snapshot scan
2. concurrent insert/update/delete 중 cursor continuation
3. large Artifact result stdout/file export
4. long-running Task/Process watch soak
5. fast vs slow stdout pipe
6. repeated broken pipe/SIGINT/reconnect/gap/resync
7. 1M-equivalent terminal receipt/key retention simulation
8. CLI local journal append/compaction/crash recovery
9. same key retry/response loss/reconcile
10. 100-way concurrent Runtime start and stale endpoint recovery
11. Runtime restart + endpoint recreation + subscription resync burst
12. terminal sanitizer with control-sequence-heavy/unicode input
13. partial export/disk full/temp cleanup
14. decoded/decompressed expansion rejection

## 4. 핵심 불변조건

- CLI retained RSS가 total dataset size와 선형 증가하지 않는다.
- Runtime page materialization은 page/byte ceiling과 동기화된다.
- snapshot retention이 stale consumer 때문에 무한 증가하지 않는다.
- slow subscriber가 Runtime internal queue/control headroom을 고갈시키지 않는다.
- reconnect loop 후 subscriber/task/fd/bytes가 baseline 또는 정책상 bounded state로 회귀한다.
- receipt metadata growth는 configured retention/capacity model과 일치하고 full payload를 retain하지 않는다.
- terminal sanitizer output expansion과 CPU가 input class별 cap을 가진다.
- concurrent start에서 storage writer/endpoint 1개다.

## 5. Cache 계획

Cache마다 다음을 측정한다.
- source revision/watermark/generation
- key cardinality와 encoded bytes
- hit/miss/eviction/pin
- invalidation latency
- retained bytes/RSS 영향
- rebuild cost

selector result, Authorization Decision, cursor visibility를 장기 authority cache로 사용하지 않는다. query/contract cache가 Domain revision을 숨기지 않는다.

## 6. Benchmark Gate 정의

정확한 절대 threshold는 reference hardware와 representative dataset benchmark ADR에서 freeze한다. freeze 전에도 다음 상대 regression을 사용한다.

- 동일 fixture 대비 allocation/op와 bytes/op 기록
- schema/Control adapter 도입 overhead 분리
- `--all` peak RSS가 fixed page/chunk budget의 작은 상수배
- no-output fast sink 대비 sanitizer/safe writer overhead 기록
- soak 종료 후 retained resource delta 0 또는 명시된 cache warm baseline

근거 없이 micro-optimization, lock-free structure, custom allocator를 P0 필수로 하지 않는다.

## 7. Fault/Pressure Benchmark

Constrained/Critical pressure에서 page size/fan-out/export가 제한되고 safety/control/recovery headroom을 보존하는지 측정한다. OOM을 유발해 recoverability를 증명하는 방식이 아니라 preflight/admission/fail-stop을 검증한다.

## 8. 검증 기준

- AT-RMEM 계열과 AT-CLI-004/010, AT-APP-006/007, AT-HOST-001 workload 연결.
- full dataset-proportional retained growth 0.
- subscriber/snapshot/local-journal/export temp leak 0.
- endpoint recovery/concurrent start 성능 결과 기록.
- correctness/security 경계를 제거한 최적화 0.
