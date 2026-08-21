---
title: "성능·메모리·캐시 계획"
document_id: "DXB-ENG-051"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-017", "DXB-DOM-027", "DXB-RUN-031", "DXB-RUN-034", "DXB-RUN-036", "DXB-ENG-050"]
---

# 성능·메모리·캐시 계획

## 1. 목적

Runtime CPU/RSS/allocation/cache 비용과 장기 Conversation/Thread/Memory growth를 분리 계측하고, Live Control의 responsiveness를 위해 immutable state/Side Effect safety/Provider Host boundary를 희생하지 않는다.

## 2. Engineering Target과 Policy 구분

benchmark target은 Runtime hard/default Policy의 SSOT가 아니다. limit/default는 Resource/Config Policy Owner가 소유한다.

## 3. Memory Ownership Map

| 영역 | Owner | 회수/제한 |
|---|---|---|
| Bot coordinator/registry | Runtime | idle eviction/deactivate |
| Main Conversation hot view | Conversation Runtime | cursor/window/byte cap |
| Thread metadata/lineage cache | Conversation Runtime/Query | byte cap/rebuild |
| Thread history retrieval buffer | Query/Context | pagination/stream/spill |
| Thread-local Memory cache | Memory Runtime | byte eviction, canonical store 별도 |
| Core Working Context | Core | terminal/drop/spill |
| Control Channel/Directive runtime state | Live Control | item+byte cap/terminal cleanup |
| Provider Host call metadata | Host | call terminal/drop |
| Provider connection/process pool | Provider Lifecycle | drain/stop |
| Projection/cache/index | owner별 | TTL/byte cap/rebuild |
| Canonical Conversation/Memory | durable store | retention/archive/forget only |

## 4. Conversation / Thread Growth Benchmark

별도 workload로 측정:
- total Conversation message count/bytes
- per-Thread message count/bytes
- Thread count/lineage depth/branch rate
- hot-view retained bytes vs canonical storage bytes
- history pagination/retrieval latency
- summary/compaction/archive throughput
- Thread-local Memory bytes/items/revisions
- Thread→Bot promotion rate/conflict/dedup cost
- Context Plan selected bytes/tokens vs total transcript bytes

전체 transcript를 Context 또는 UI heap에 materialize하는 구현은 benchmark 대상 이전에 구조 위반이다.

## 5. Thread-aware Context Benchmark

- recent history selection
- historical retrieval
- Thread/global Memory merge
- deterministic ordering/truncation
- shared Bot prefix reuse
- content digest/cache key construction
- Provider-neutral render

100/10k/1M message-equivalent history가 증가해도 selected Context token/byte budget은 Policy cap을 유지해야 한다.

## 6. Live Control Benchmark

측정:
- control command validation/commit latency
- control queue wait p50/p95/p99
- Work Queue saturation에서 inspect/report latency
- Directive commit→delivery→ack
- redirect commit→old Execution yield→new Execution start
- suspend request→checkpoint/quiesced
- resume→new Execution admission
- control flood에서 normal work throughput/fairness
- per-Bot control isolation

외부 Provider가 cancellation/safe-point를 지원하지 않는 시간은 `provider wait`와 Common Runtime overhead를 분리한다.

## 7. Provider Host / SDK Overhead

v0.3의 Host admission/selection/security/context construction/DTO mapping/stream/telemetry/cancel/deadline/accounting/generation pin benchmark를 유지한다. native steering/yield feature negotiation이 추가 allocation/copy를 만들면 별도 측정한다.

## 8. Cache 원칙

각 cache는 canonical source, key/version, max bytes+entries, TTL/eviction, invalidation, stale policy, permission/sensitivity, metrics, rebuild path를 가진다.

Thread cache key는 필요한 범위에서 Bot/Thread/Message watermark/Memory/Identity/Policy revision을 포함한다. Provider Session history cache가 Canonical Context cache를 대체하지 않는다.

## 9. Workload

기존 1/10/100/1000 Bot, Core 1/4/16/64, 1k/100k/1M Memory, multi-provider, lifecycle, Plugin, Artifact, retry/cancel/timeout storm, Waiting/Routine, soak에 다음을 추가한다.
- Bot당 Thread 1/10/100/10k
- Thread transcript short/long/cold archive
- A/B/C Thread concurrent execution/control
- report storm while Work Queue saturated
- redirect/suspend/resume races
- Provider Session loss/recreation
- lineage branch growth

## 10. Regression Gate

micro/component/e2e/lifecycle/soak gate를 유지하고 Conversation/Thread Context assembly, Control Channel, Directive recovery를 추가한다.

## 11. 검증 기준

- steady soak에서 RSS/control queue/Thread cache가 선형 누수하지 않음.
- AT-CTX-002 bounded Context 통과.
- AT-CTRL-002 control starvation test가 latency/fairness evidence를 남김.
- Provider Session loss가 canonical history를 heap 복제로 복원하지 않음.
- 모든 cache/channel/history view가 bounded.
