---
title: "성능·Runtime Memory·캐시 계획"
document_id: "DXB-ENG-051"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-017", "DXB-DOM-022", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-031", "DXB-RUN-034", "DXB-RUN-037", "DXB-RUN-038", "DXB-ENG-050"]
---

# 성능·Runtime Memory·캐시 계획

## 1. 목적

v0.6 process-wide Runtime Memory safety benchmark를 유지하고, v0.7 CLI/Application Contract의 paging/serialization/subscription/reconnect workload를 추가한다. CLI RSS와 Runtime RSS를 구분해 측정한다.

## 2. v0.6 Benchmark 비회귀

Reservation, copy amplification, large payload, leak/retention soak, pressure state, recovery storm, collaboration utility benchmark의 기존 gate는 모두 유지한다.

## 3. v0.7 CLI / Control Workload

필수 benchmark/soak 후보:
- 1M-equivalent history/message record를 paging으로 scan
- large Memory search/history result
- large Artifact export
- event/task/process follow 장시간 soak
- disconnect/reconnect loop
- high-frequency task status watch
- JSON/JSONL serialization
- slow stdout pipe
- broken pipe 반복
- command response-loss retry/reconcile
- Runtime restart 후 reconnect/resync burst

## 4. 측정 차원

### CLI process
- RSS/peak RSS
- allocations/op, bytes/op
- active page/chunk bytes
- serialization temporary bytes
- local subscription buffer items/bytes
- spawned task/subscriber count

### Runtime Host
- control request decode/response serialization bytes
- per-subscriber buffered items/bytes
- total subscriber bytes
- query page materialization bytes
- resync/reconnect load
- global Runtime Memory pressure/accounted/reserved bytes

CLI와 Runtime 수치를 합쳐 leak 원인을 숨기지 않는다.

## 5. 핵심 성능 불변조건

- CLI RSS가 total dataset size와 선형 동기 증가하지 않는다.
- Runtime RSS/accounted bytes가 slow CLI consumer 때문에 무한 증가하지 않는다.
- `--all`은 paged incremental output으로 구현 가능해야 한다.
- JSONL streaming은 전체 record collection을 먼저 만들지 않는다.
- reconnect loop 후 subscriber/task/connection retained count가 baseline 또는 bounded state로 회귀한다.
- control/serialization overhead와 Provider execution time을 분리해 측정한다.

## 6. Large Output / Pipe Backpressure

다음 두 상황을 별도로 측정한다.

```text
Runtime → fast CLI → terminal/file
Runtime → slow CLI → blocked pipe consumer
```

slow path에서:
- bounded server/client buffers 유지
- backpressure/gap/disconnect policy 작동
- control/recovery headroom starvation 0
- canonical Task/Process를 slow output 때문에 실패로 표시하지 않음

## 7. Regression Gate

기존 AT-RMEM-001~003에 더해 AT-CLI-004를 검증한다.

- full result heap materialization 0
- total dataset-proportional retained growth 0
- slow consumer unbounded queue 0
- watch/reconnect lifecycle leak 0
- protocol/control overhead benchmark 기록
