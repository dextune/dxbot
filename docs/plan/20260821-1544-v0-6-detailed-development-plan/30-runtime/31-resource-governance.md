---
title: "자원·비용·Runtime Memory Admission 관리"
document_id: "DXB-RUN-031"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-024", "DXB-RUN-030", "DXB-ARC-017"]
---

# 자원·비용·Runtime Memory Admission 관리

## 1. 목적

v0.5의 CPU/Core/token/storage/queue/cache/Provider boundedness 위에 **process-wide Runtime Memory envelope + hierarchical reservation + pressure degradation**을 추가한다. 각 component가 local cap을 지켜도 다수 Bot/Core가 동시에 Context/Provider buffer/serialization/Artifact를 잡아 process OOM에 도달하는 경로를 차단한다.

Runtime Memory는 Knowledge Memory와 다른 실행 자원이다. Canonical Resource Owner는 본 문서 `DXB-RUN-031`이며 Scheduler/Context Runtime/Provider Host/Channel Orchestration은 enforcement point다.

## 2. 자원 차원

기존 Core/CPU/token/cost/storage/Artifact/index/Provider/Plugin/UI/Control 자원에 다음 memory 차원을 명시한다.
- main process Runtime Memory envelope
- owned child Provider process resident memory where observable
- accelerator/device/VRAM where applicable
- Context/retrieval candidate bytes
- Provider input/output/in-flight stream bytes
- Tool/Artifact staging bytes
- serialization/response aggregation bytes
- cache/index/prefix retained bytes
- temporary spill/storage quota
- reservation/accounting overhead

host RAM과 child-process/VRAM은 별도 dimension으로 계측하며 하나를 다른 하나 뒤에 숨기지 않는다.

## 3. Hierarchical Runtime Memory Budget

논리적 ceiling:

```text
Deployment / Process Memory Envelope
├─ Safety / Control / Recovery Headroom
└─ General Runtime Memory Budget
   ├─ Shared Runtime / Cache / Index Budget
   ├─ Project? / Channel? Work Budget
   ├─ Bot / Thread / Task Budget
   └─ Execution / Core Work Budget
      ├─ Context / Retrieval
      ├─ Provider Input / Output Buffer
      ├─ Tool / Artifact Staging
      └─ Serialization / Response Aggregation
```

원칙:
- 하위 budget 합은 상위 hard ceiling을 우회하지 못한다.
- 일반 work가 safety/control/recovery headroom을 소비하지 못한다.
- token 수/Core 수/queue item 수만으로 byte budget을 대체하지 않는다.
- exact byte/ratio/threshold는 Policy SSOT + benchmark가 결정한다.
- host/container hard limit을 알 수 있으면 operational ceiling은 그보다 낮게 둔다.
- host hard limit을 알 수 없는 배포도 configured envelope 없이 무제한 admission을 기본값으로 두지 않는다.

## 4. Reservation-Aware Admission

**Core Lease count는 Memory bound가 아니다.**

```text
Work Candidate
→ estimate bounded memory class/bytes
→ reserve parent Runtime Memory capacity
→ Scheduler / Context / Provider admission
→ actual/incremental usage accounting
→ ownership transfer / adjustment
→ terminal/cancel/drop release
```

규칙:
- memory-heavy work는 reservation 없이 먼저 시작한 뒤 사후 제한하지 않는다.
- 정확한 예측이 어려운 stream/tool work는 incremental byte charging을 사용한다.
- SDK/TLS/compression/allocator overhead는 safety headroom과 observed resident pressure로 accounting drift를 흡수한다.
- reservation estimate를 실제 heap byte와 동일하다고 가정하지 않는다.
- queue→Context→Provider→response에서 동일 payload를 별도 예산으로 중복 reserve/clone하지 않고 ownership transfer/shared immutable ref를 우선한다.
- reservation은 runtime-only permit이며 Canonical Task/Core state가 아니다.
- Waiting/Suspended/외부 이벤트 대기는 large transient reservation을 release하고 durable reference/checkpoint만 남긴다.
- nested resource reservation은 composite admission 또는 고정 acquisition order를 사용한다.
- cancellation/timeout/error/panic/drop/shutdown에서 reservation을 회수한다.
- reservation leak은 release blocker다.

## 5. Collaboration / Process Budget

per-message activation cap 외 다음 total budget을 지원한다.
- per-process/cycle Bot activation
- total delegation depth/fan-out
- total round/hop
- token/cost
- deadline/wall budget
- concurrent Activity
- revalidation/dependency traversal work

정확한 숫자는 Policy SSOT가 소유한다. Channel membership 수가 budget을 자동 늘리지 않는다.

## 6. Multiplicative Buffer / Large Payload 방지

금지 패턴:

```text
History Vec
→ Context clone
→ JSON String clone
→ Provider request clone
→ retry buffer clone
→ response aggregation clone
```

기본 규칙:
- large immutable history/prefix/artifact는 shared reference/durable ref 사용
- Provider render에 불필요한 Canonical object graph 복제 금지
- response stream은 **cumulative bytes + in-flight bytes + downstream backpressure** 제한
- Provider/Tool이 backpressure를 지원하지 않으면 local accumulation을 무제한 허용하지 않고 cap에서 cancel/truncate-as-policy/reject/reconcile
- encoded/input size뿐 아니라 decoded/decompressed size, collection element count, nested expansion을 allocation 전에 제한
- `Content-Length`/size hint는 신뢰값이 아니라 preflight candidate이며 실제 누적 byte 재측정
- 큰 Tool/Artifact output은 bounded stream/spool 후 reference 전환 가능
- spill은 별도 disk quota/owner/cleanup/recovery를 가지며 무제한 임시 저장소가 아님

## 7. Memory Pressure State

최소 semantic:

```text
Normal → Constrained → Critical → Emergency
```

threshold/hysteresis/cooldown은 ADR/benchmark 대상이다. 빠른 admission은 accounted/reserved bytes를 우선하고 RSS/allocator/OS pressure는 accounting drift를 보정하는 관측 입력으로 사용한다.

### Constrained
- Derived cache/prefetch/Presence/summary 적극 trim
- background index/promotion/revalidation concurrency 감소
- speculative retrieval/fan-out 감소

### Critical
- 신규 memory-heavy Execution/Provider admission 축소/거부
- Context/Artifact spill 적극 사용
- Provider output accumulation 제한 강화
- low-priority/background work pause/defer
- restartable/low-priority work는 기존 Live Control/Continuation semantic으로 cooperative preempt/cancel 후보
- raw worker/task kill로 Canonical state 우회 금지
- safety/control/recovery headroom 보존

### Emergency
- 신규 일반 work admission 중단
- policy가 restartable work를 우선순위에 따라 shed할 수 있으나 Task/Side Effect/Continuation safety는 기존 owner가 보장
- cancel/reconcile/shutdown/recovery 최소 경로만 허용
- allocation-heavy diagnostic dump/log formatting 회피
- Canonical State를 메모리 확보 목적으로 삭제 금지
- 회복 불가 시 durable state를 보존한 fail-stop/restart 허용. process OOM까지 기다리는 것을 정상 전략으로 사용하지 않음

## 8. Rust Allocation Failure 의미

Rust `std` allocation failure를 catch해서 정상 복구한다는 설계를 사용하지 않는다.
- user-controlled/large collection capacity는 checked arithmetic + configured cap 선행
- 큰 `Vec`/`String`/map 성장에는 적합한 경우 stable fallible reserve 또는 bounded incremental build 사용
- 모든 작은 allocation을 custom fallible wrapper로 감싸지 않음
- allocation error hook, panic catch, allocator-specific behavior를 correctness boundary로 사용하지 않음

## 9. Leak / Retention Resource Rule

- `Arc` strong cycle/back-reference 구조는 architecture review 대상
- long-lived async task/future/closure가 필요 이상으로 large Context/Artifact/Project state를 capture하지 않음
- subscriber/listener/Provider stream/registry entry에 unregister/drop/generation cleanup 존재
- cache pin/refcount lifetime이 eviction을 영구 차단하지 않음
- Reservation/permit/Lease/activity counter는 RAII 또는 동등 cleanup
- reconnect/join-leave/Provider restart/Thread archive 반복 후 live object/retained bytes가 baseline 또는 bounded cache state로 회귀

## 10. Host / Container Safety Net

Linux/container의 cgroup memory pressure/hard limit, PSI 등은 optional Infrastructure Adapter로 사용할 수 있다.
- soft pressure signal은 proactive trim/load shedding trigger 가능
- hard limit/OOM kill은 최종 safety net이며 Runtime admission의 대체물이 아님
- owned child Provider process는 가능하면 동일 deployment/resource envelope 아래 둔다.
- native/desktop도 configured envelope + RSS/pressure telemetry를 사용한다.

## 11. Recovery Rebuild Budget

restart 직후 모든 Bot/Project/Channel/Thread/Memory/index를 eager hot-load하지 않는다.
- Canonical identity/state metadata 우선
- heavy history/Memory/index lazy/on-demand 또는 bounded background rebuild
- projection/index rebuild도 memory admission 통과
- Provider reconnect가 모든 Bot Context를 선행 생성하지 않음
- recovery headroom 유지

## 12. Fairness

- one Channel/Process burst가 unrelated Bot/Channel/Control을 고갈시키지 않음
- Provider quota wait가 memory permit을 과도하게 장기 점유하지 않음
- Emergency에서도 control/reconciliation이 starvation되지 않음

## 13. 검증 기준

- AT-RMEM-001 process-wide/hierarchical admission 우회 0.
- AT-RMEM-002 hard OOM 이전 pressure degradation/load shedding 동작.
- AT-RMEM-003 reservation/task/subscriber/cache-pin retained leak 0.
- local queue/cache/context가 cap 안이어도 global budget 부족 시 admission 제한 가능.
- Canonical Memory/History를 RSS pressure만으로 삭제하지 않음.
- v0.5 queue/cache/resource/fairness fixture regression 0.
