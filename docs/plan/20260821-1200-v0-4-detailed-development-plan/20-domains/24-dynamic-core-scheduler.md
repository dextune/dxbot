---
title: "Dynamic Core Scheduler"
document_id: "DXB-DOM-024"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-023", "DXB-DOM-021", "DXB-ARC-012", "DXB-ARC-017"]
---

# Dynamic Core Scheduler

## 1. 목적

Bot 생성 시 Core 수를 고정하지 않고 resource/fairness/parallel benefit에 따라 Core Lease를 동적으로 발급한다. Scheduler는 Core/Execution scheduling admission과 fairness를 소유하고, Live Control은 priority/parallelism/rebalance **요청**만 제공한다.

## 2. 불변조건

- Core는 독립 Persona/Agent가 아니라 Execution의 일시 Lease다.
- Bot/Task schema에 fixed Core count를 Canonical identity로 저장하지 않는다.
- terminal/expired Lease late result는 Canonical commit 권한이 없다.
- Provider selection/Provider permit은 Provider Host 소유다.
- Control Plane/Main Conversation이 Core Lease를 직접 mint/revoke하지 않는다.

## 3. Admission 책임

Scheduler:
`Deployment capacity → per-Bot fairness/cap → Task priority/deadline/class → coarse Execution reservation → Core Lease`.

Provider Host:
`Capability compatibility → Provider selection/pin → provider-specific quota/permission/deadline → activity/permit`.

Resource hard/default 값은 `DXB-RUN-031` Policy SSOT가 소유한다.

## 4. Thread / Task Scheduling

Scheduler의 WorkItem은 BotId + TaskId + primary ThreadId/reference를 가져 observability/isolation에 사용한다. Thread는 scheduling owner가 아니다.

- Thread A/B/C Task는 독립 admission 가능
- 같은 Bot 내부에서도 per-Thread control storm/large task가 fairness를 왜곡하지 않도록 Task class/priority 정책을 사용
- Thread archive/suspend 상태는 Task admissibility projection을 통해 반영

## 5. Live Reprioritize / Rebalance

`DXB-RUN-036`의 Control Directive가 Task priority, parallelism hint, deadline class, resource adjustment request를 commit하면 Scheduler가 새 revision을 관찰해 재평가한다.

허용:
- queued WorkItem reorder
- 신규 Core Lease admission 확대/축소(정책 범위)
- cooperative yield/rebalance request
- low-priority work preemption candidate 선정

금지:
- user가 요청한 숫자만큼 Core Lease를 강제 생성
- hard resource ceiling 우회
- running Execution snapshot mutation
- 특정 worker thread/process handle 직접 조작

## 6. Control-induced Yield

Scheduler가 Control/Safety/Rebalance 이유로 running Execution의 Lease 회수를 원할 때 `Execution Supervisor → safe point/cooperative yield` 경로를 사용한다. Side Effect/Provider Host cleanup을 건너뛰는 hard kill을 정상 경로로 사용하지 않는다.

Provider가 cancellation을 지원하지 않으면 Lease/Execution은 `awaiting-safe-point` 상태로 관측될 수 있으며 Hard Real-Time을 약속하지 않는다.

## 7. Queue / Fairness

- Scheduler Work Queue item+byte bounded
- stable priority + enqueue sequence tie-break
- per-Bot starvation protection
- interactive/background class
- control channel은 별도이며 Scheduler Work Queue와 합치지 않음
- Provider quota wait queue를 Scheduler가 중복 소유하지 않음

## 8. Lifecycle / Restart

persisted Task/Execution/Core Lease state를 reconcile한다. Waiting/Suspended Continuation owner는 Task, pending Directive owner는 Control/Task domain이다. Scheduler는 recovery 결과를 입력으로 admission을 다시 결정하며 Provider Registry를 복구하지 않는다.

## 9. 관측

- queue latency/oldest/starvation by class
- per-Bot utilization
- Thread/Task active/queued distribution
- Core Lease utilization/reject/yield/rebalance
- control-induced yield reason/latency
- provider admission wait는 별도 Host metric

## 10. 검증 기준

- Bot 생성 API에 core count가 없다.
- deterministic tie-break가 재현된다.
- one-Bot/thread flood에서 다른 Bot이 무기한 굶지 않는다.
- reprioritize가 queued scheduling에 반영되되 running Execution snapshot을 mutation하지 않는다.
- “Core 추가” 요청이 Resource Policy를 넘어 Lease를 만들지 않는다.
- control-induced yield 후 Lease/Provider activity가 owner에서 누수 없이 정리된다.
