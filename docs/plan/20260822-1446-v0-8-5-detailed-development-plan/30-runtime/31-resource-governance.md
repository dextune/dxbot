---
title: "자원·비용·Runtime Memory Admission 관리"
document_id: "DXB-RUN-031"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-RUN-030", "DXB-DOM-024"]
---

# 자원·비용·Runtime Memory Admission 관리

## 1. 목적

장기 실행 Runtime의 process-wide memory/cost/concurrency를 하나의 Common owner가 bounded하게 관리하고 Application Contract, cursor, receipt, CLI stream/export도 예외 없이 accounting한다.

## 2. Runtime Memory Envelope

```text
Process ceiling
├─ safety/control/recovery headroom
├─ canonical state working set
├─ execution/context/provider activity
├─ query snapshot/page materialization
├─ subscription buffers
├─ receipt/idempotency metadata
├─ projection/index/cache
└─ recovery/export temporary allocations
```

Core count는 memory ceiling이 아니다. work admit 전에 필요한 reservation을 획득하고, 완료/cancel/failure/drop에서 정확히 release한다.

## 3. Accounted dimensions

- request encoded/decoded/decompressed bytes
- DTO mapping/serialization temporary bytes
- page items/bytes and snapshot metadata
- cursor/token retained bytes
- Operation Receipt/key index bytes와 retention
- CLI local journal 최대 entries/bytes
- server/client subscriber items/bytes
- resync/reconnect burst
- artifact/export chunks와 temporary disk bytes
- provider/tool cumulative + in-flight stream bytes
- cache/index retained/pinned bytes

item cap만 두고 large item을 허용하지 않는다.

## 4. Pressure State

```text
Normal → Constrained → Critical → Emergency
```

- Constrained: optional cache/large page/fan-out 축소, 신규 low-priority admission 지연
- Critical: recovery/control headroom을 보호하고 optional rebuild/export 제한
- Emergency: 신규 semantic work 차단, controlled shutdown/fail-stop 준비

OOM을 정상 retryable response로 약속하지 않는다. allocation 전에 preflight와 admission을 수행한다.

## 5. Query와 `--all`

```text
bounded page fetch
→ incremental encode/write
→ page/chunk release
→ next cursor
```

전체 dataset을 `Vec`/JSON array/String으로 모으지 않는다. snapshot lifetime 자체도 duration/bytes 정책을 갖고 오래 정체된 consumer는 expired cursor/resync로 종료한다.

## 6. Receipt와 Local Journal

- receipt payload는 full request/result 복제가 아니라 digest와 canonical outcome reference를 우선한다.
- nonterminal receipt는 GC하지 않되 retained bytes를 metric/alert하고 recovery policy를 적용한다.
- terminal receipt/key binding은 최소 retention 동안 보존하고 compact 가능한 metadata format을 사용한다.
- CLI local journal은 bounded entries/bytes이며 secret/full payload/authority state를 저장하지 않는다.

## 7. Subscription과 Export

server/client buffer 모두 item+byte cap과 slow-consumer policy를 가진다. file writer는 bounded chunk를 사용하고 temporary disk quota를 확인한다. partial export가 메모리나 disk를 무기한 점유하지 않는다.

## 8. Cache

모든 cache는 canonical source, key+version, invalidation, max entries+bytes, eviction/TTL, pin policy, metrics를 정의한다. snapshot/cursor/selector/authorization 결과를 무기한 authority cache로 사용하지 않는다.

## 9. 검증 기준

- 1M-equivalent page scan에서 CLI/Runtime retained RSS가 dataset 크기와 선형 증가하지 않음.
- slow consumer에서 subscriber bytes가 cap 안에 유지됨.
- receipt/local journal retention soak에서 bounded metadata growth 또는 명시된 capacity policy.
- decoded expansion bomb가 allocation 전 거부됨.
- Critical/Emergency에서도 control/recovery headroom 유지.
- cancel/crash/reconnect/export failure 뒤 reservation/temp/subscriber leak 0.
