---
title: "Dynamic Core Scheduler"
document_id: "DXB-DOM-024"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-023", "DXB-ARC-010"]
---

# Dynamic Core Scheduler

## 1. 목적

Core를 Bot 생성 시 고정하는 worker 수가 아니라 Runtime이 필요·정책·자원에 따라 발급하고 회수하는 논리적 실행 Lease로 정의한다.

## 2. Core Lease

```text
CoreLeaseId
BotId / TaskId / ExecutionId
SchedulerGeneration
ResourceGrantRef
AcquiredAt / Deadline
State
```

Core는 Identity·Memory·Goal을 소유하지 않는다. Lease가 종료되어도 Bot/Task/Execution의 durable identity와 result는 유지된다.

## 3. Admission과 Scheduling

순서:

```text
Task ready
→ authorization/capability validation
→ Runtime memory/cost/concurrency admission
→ fair scheduling
→ Core Lease + Provider activity acquisition
→ Execution run
→ release/join/accounting
```

전체·scope·Bot별 ceiling, priority, fairness, starvation prevention, deadline, recovery headroom을 함께 고려한다. 단순 `max core count`만으로 메모리 안전을 보장하지 않는다.

## 4. 상태와 불변조건

```text
Requested → Granted → Active → Releasing → Released
Requested → Deferred | Rejected
Active → Preempting → Released
```

- 같은 Execution의 active authoritative Lease는 하나다.
- stale SchedulerGeneration/Lease가 write하지 못한다.
- Waiting/Suspended Task는 계산/메모리 reservation을 정책에 맞게 release한다.
- Provider activity와 Lease cleanup을 함께 추적한다.
- fire-and-forget worker와 unbounded work queue를 금지한다.

## 5. Dynamic 확장

Bot은 workload를 병렬izable unit으로 제안할 수 있으나 실제 Lease 수는 Scheduler가 결정한다. Project/Channel fan-out도 global admission을 우회하지 않는다. Core 개수는 persistent Bot configuration의 필수 값이 아니다.

## 6. Recovery와 Shutdown

Runtime restart 시 transient Lease를 복원하지 않는다. durable Task/Execution state를 reconcile해 새 generation의 Lease를 발급한다. shutdown은 신규 admission 차단 → active cancellation/drain → provider activity release → child join → resource accounting zero/known residue 순으로 완료한다.

## 7. 검증 기준

- Bot 생성에 고정 Core pool이 필요하지 않다.
- stale Lease/generation late write 0.
- waiting/suspended/shutdown 뒤 permit·reservation·activity leak 0.
- load에서 global Runtime memory ceiling을 Core count가 우회하지 않는다.
- 여러 Bot 간 starvation을 deterministic scheduler fixture로 검증한다.
