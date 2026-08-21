---
title: "Dynamic Core Scheduler"
document_id: "DXB-DOM-024"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-023", "DXB-DOM-021", "DXB-ARC-012", "DXB-ARC-017"]
---

# Dynamic Core Scheduler

## 1. 목적

Bot 생성 시 Core 수를 고정하지 않고 resource/fairness/parallel benefit에 따라 Core Lease를 동적으로 발급한다. **Scheduler는 Core/Execution scheduling admission과 fairness를 소유하고, concrete Provider selection과 Provider-specific resource admission은 Provider Host가 소유한다.**

## 2. Core 불변조건

Core는 Bot/Task/Execution에 속한 일시 Lease이며 독립 Persona, Goal, Long-term Memory를 갖지 않는다. terminal/expired Lease의 late result는 Canonical State를 Commit할 권한이 없다.

## 3. Admission 책임 분리

### Scheduler가 소유

```text
Deployment/Global execution capacity
→ Per-Bot fairness/cap
→ Task class/priority/deadline
→ coarse Execution/Core resource reservation
→ Core Lease
```

### Provider Host가 소유

```text
Capability compatibility/binding
→ Provider selection/generation pin
→ Provider-specific concurrency/rate/cost quota
→ permission/approval
→ call deadline/cancellation
→ Provider activity/permit
→ output/usage accounting
```

Scheduler가 Provider별 permit을 직접 획득하거나 Provider별 semantic queue를 Canonical scheduling owner로 만들지 않는다. 필요 시 Host가 제공하는 opaque admission/selection reference만 Execution에 연결한다.

본 문서는 resource limit 값의 Owner가 아니며 실제 상한/default는 `DXB-RUN-031` Policy SSOT를 참조한다. 이는 의미 참조이며 문서 선행 dependency는 아니다.

## 4. Execution 준비와 Provider Binding 관계

1. Task가 required Capability, priority, budget, deadline, policy snapshot을 제공한다.
2. Scheduler가 provider-independent Core/Execution admission을 수행한다.
3. Runtime Consumer가 필요한 Capability binding을 Host에 요청한다.
4. Host가 existing binding을 확인하거나 신규 binding에 대해 Registry/Selector/Compatibility/Security 정책으로 Provider를 선택한다.
5. 생성된 `ProviderSelectionRef`는 Execution snapshot에 pin된다.
6. 실제 call마다 Host가 pinned Provider generation의 activity/permit을 획득한다.
7. Provider quota 부족, Draining, unavailable이면 Host가 stable outcome/admission result를 반환한다.
8. 상위 Runtime/Task policy가 wait/reject/new Attempt 여부를 결정한다.

binding은 Execution 준비 시 eager 또는 first-use lazy 생성이 가능하다. Scheduler는 binding의 concrete Provider를 선택·변경하지 않으며 silent fallback/semantic retry를 시작하지 않는다.

## 5. Queue / Fairness

- 모든 Scheduler queue bounded by item+bytes
- class/priority/deadline/aging을 Common Policy가 계산
- per-Bot fair share/cap
- low-priority starvation protection
- overflow는 reject/replace/coalesce 의미를 명시
- Provider-specific rate/concurrency wait는 Provider Host/Resource Governance에서 별도 관측
- Scheduler가 Host 내부 Provider admission queue를 복제하지 않음

### Deterministic Tie-Break

동일 effective priority는 stable enqueue sequence + WorkItemId 등 명시 key로 결정한다. hash/container iteration order에 의존하지 않는다.

## 6. Provider Lifecycle과 Core Lease

Core Lease lifecycle과 Provider lifecycle은 서로 다른 owner를 가진다.

- Provider가 `Draining`으로 전환되어도 이미 실행 중 Core Lease 자체를 Provider가 임의 취소하지 않는다.
- 신규 Capability binding/call은 Host lifecycle/activity rule을 따른다.
- pin된 Provider가 더 이상 호출 불가하면 Execution은 stable failure/unavailable outcome을 받고 상위 policy가 후속 Attempt를 결정한다.
- Provider stop/late callback은 generation/fencing으로 stale commit을 차단한다.
- Core Lease 종료가 Provider lifecycle state를 직접 변경하지 않는다.

## 7. 관측 지표

Scheduler:
- queue latency by class/priority
- oldest waiting/starvation
- per-Bot fair share utilization
- Core Lease utilization/reject/coalesce

Provider Host/Resource Governance:
- Provider binding/admission wait
- Provider quota/rate-limit wait
- Provider activity/permit utilization/leak

두 metric family를 합쳐 Scheduler가 Provider quota owner인 것처럼 표현하지 않는다.

## 8. 결과 Merge

Core output은 immutable Result Fragment로 반환한다. disjoint keyed result는 deterministic union, ordered result는 stable sort key, 동일 entity write는 expected revision conflict, natural-language synthesis는 별도 merge step을 사용한다.

## 9. Restart

persisted Requested/Lease/Execution state를 reconcile한다. Waiting Continuation owner는 Task다. persisted Capability binding은 Provider Host/Recovery가 contract/config compatibility를 재검증하고, Scheduler가 Registry/Lifecycle을 재구축하지 않는다. expired Lease의 late result는 fencing으로 거부한다.

## 10. 검증 기준

- Bot 생성 API에 core count가 없다.
- 동일 input/priority에서 deterministic scheduling order가 재현된다.
- flood test에서 다른 Bot이 무기한 굶지 않는다.
- Scheduler가 concrete Provider type/Registry mutator/Provider permit API에 의존하지 않는다.
- Provider-specific permit/activity는 Host만 획득/반환한다.
- 동일 Execution 내 여러 binding이 독립 Provider를 가질 수 있고 같은 binding은 immutable하다.
- Draining/new-call, crash/cancel/timeout 후 Core Lease + Provider activity/permit이 각각 owner에서 누수 없이 정리된다.
