---
title: "자원·비용·Runtime Memory Admission 관리"
document_id: "DXB-RUN-031"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-024", "DXB-RUN-030", "DXB-ARC-017"]
---

# 자원·비용·Runtime Memory Admission 관리

## 1. 목적

v0.6의 process-wide Runtime Memory envelope, hierarchical reservation, pressure degradation, large-payload boundedness를 그대로 유지하고, v0.7 Application Contract/Control/CLI도 동일 Resource Governance를 우회하지 않도록 한다.

Runtime Memory의 Canonical Resource Owner는 계속 `DXB-RUN-031`이다. CLI/Control은 enforcement consumer이며 별도 memory policy owner가 아니다.

## 2. v0.6 Resource 의미 유지

다음 계약은 unchanged다.
- process-wide hierarchical byte envelope
- safety/control/recovery headroom
- reserve-before-admit
- Core count ≠ memory ceiling
- Waiting/Suspended transient reservation release
- cumulative + in-flight Provider/Tool/Artifact stream cap
- decoded/decompressed expansion bound
- Normal/Constrained/Critical/Emergency pressure semantics
- OOM prevention/fail-stop, not catch-OOM normal recovery
- RAII/terminal cleanup/leak gate
- staged/lazy recovery

## 3. Control / CLI Resource Dimension

v0.7에서 다음을 accounting 대상에 포함한다.
- request decode/validation bytes
- Query page/window materialization bytes
- public DTO serialization bytes
- subscription server/client buffer bytes
- CLI JSON/JSONL serialization buffer
- Artifact export/read buffer
- reconnect/resync burst
- diagnostics/trace output staging

human-readable formatting과 machine output도 unbounded aggregation의 예외가 아니다.

## 4. Pagination / `--all`

CLI `--all`은 편의 flag일 수 있으나 하나의 `Vec`/JSON array로 전체 dataset을 materialize한다는 뜻이 아니다.

```text
paged query
→ bounded page DTO
→ incremental render/write
→ page release
→ next cursor
```

history/memory/task/member/trace/export는 가능한 범위에서 paging/streaming을 사용한다. machine streaming에는 JSONL 또는 동등 incremental format을 사용할 수 있다.

## 5. Subscription / Slow Consumer

- Runtime internal queue와 public subscription buffer를 분리한다.
- server/client buffer 모두 item+byte cap을 가진다.
- stdout pipe가 느리면 backpressure 또는 bounded disconnect/gap policy를 사용한다.
- client가 느리다는 이유로 Runtime control/recovery headroom을 소비하지 않는다.
- reconnect storm은 bounded retry/backoff/admission을 가진다.

## 6. Resource Error Semantic

Application Contract는 Resource Governance decision을 재해석하지 않고 다음 stable class로 전달할 수 있다.
- resource exhausted
- admission rejected/deferred
- payload too large/decoded expansion rejected
- stream cumulative cap reached
- pressure degraded/critical
- budget exhausted

정확한 wire code는 `DXB-IFC-040`/ADR이 소유한다.

## 7. 검증 기준

- CLI RSS가 total dataset size와 선형 동기 증가하지 않는다.
- slow CLI consumer 때문에 Runtime subscriber/internal queue가 unbounded 증가하지 않는다.
- `--all`/history/export/watch가 global Runtime Memory envelope을 우회하지 않는다.
- Critical/Emergency에서 CLI diagnostic이 safety headroom을 고갈시키지 않는다.
- AT-RMEM-001~003 v0.6 의미 regression 0.
