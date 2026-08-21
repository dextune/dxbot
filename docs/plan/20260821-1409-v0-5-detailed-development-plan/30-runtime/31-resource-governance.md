---
title: "자원·비용·Admission 관리"
document_id: "DXB-RUN-031"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-024", "DXB-RUN-030", "DXB-ARC-017"]
---

# 자원·비용·Admission 관리

## 1. 목적

Project/Channel 수와 participant 수가 증가해도 CPU/RSS/token/storage/queue/Provider 자원이 선형 폭증하지 않도록 bounded admission과 Policy SSOT를 유지한다.

## 2. 자원 차원

기존 Core/CPU/RSS/model/tool/storage/Artifact/MemoryIndex/Provider/Plugin/UI/Control 자원에 추가:
- Project/Channel hot metadata
- Channel inbound events
- per-message participant activation/fan-out
- concurrent Channel responses
- Channel background Task/delegation
- Channel/Thread history retrieval bytes
- scoped Memory retrieval candidate count/bytes
- Shared Memory promotion work
- Channel Presence projection/subscriber

## 3. Policy SSOT

다음의 hard/soft limit/default는 Resource/Policy owner에서만 정의한다.
- per-message max Bot activation
- per-Channel response concurrency
- per-Project/Channel hot cache budget
- routing/fan-out queue capacity
- history/Memory retrieval budget
- delegation depth/fan-out
- promotion concurrency

문서에 임의 숫자를 복제하지 않는다.

## 4. 계층 예산

논리적으로 다음 ceiling을 조합할 수 있다.

`Deployment → Workspace/Tenant → Project? → Channel? → Bot → Thread/Goal/Task → Execution/Core → Provider Call`.

하위 scope가 상위 hard ceiling을 완화하지 못한다. Project/Channel 없는 Bot-only path는 기존 계층을 그대로 사용한다.

## 5. Channel Backpressure

```text
Inbound Message
→ bounded admission
→ bounded recipient resolution
→ bounded selected participant set
→ per-Bot/Scheduler admission
→ bounded response aggregation
```

`Channel member count = activated Execution count`로 구현하지 않는다. overload 시 explicit reject/defer/coalesce/selected-subset semantic을 policy가 결정한다.

## 6. Memory / History Governance

Project/Channel/Thread history나 Shared Memory가 커져도 전체를 hot heap에 materialize하지 않는다.

pressure 대응:
1. Derived cache/presence/summary trim
2. prefetch/routing fan-out 감소
3. Working Context spill/evict
4. background retrieval/index/compaction/promotion throttle
5. 신규 Execution/Provider admission 축소

Canonical Shared Memory/Conversation History를 RSS pressure만으로 삭제하지 않는다.

## 7. Stable Prefix Sharing

Bot/Project/Channel의 immutable policy/identity/context prefix는 digest/revision 기준 shared reference/cache를 사용할 수 있다. Core별 deep copy는 피한다.

## 8. Fairness

- one Channel participant storm이 다른 Channel/Bot을 고갈시키지 않음
- Manager delegation storm도 Bot Network/Scheduler budget을 통과
- control safety reserve는 유지하되 무한 우선순위가 아님
- Provider quota wait가 global Scheduler를 독점하지 않음

## 9. 검증 기준

- production unbounded Channel/Work/Control/retrieval/subscriber queue 0.
- 100 participant Channel에서 일반 메시지가 policy cap 이상의 Bot을 자동 activate하지 않음.
- Shared Memory 1M-equivalent growth에서 hot RSS가 canonical size와 선형 동기 증가하지 않음.
- overload 이후 queue/permit/Lease/background task leak 0.
