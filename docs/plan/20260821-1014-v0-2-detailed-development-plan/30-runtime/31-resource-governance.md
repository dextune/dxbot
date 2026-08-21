---
title: "자원·비용·Admission 관리"
document_id: "DXB-RUN-031"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-024", "DXB-RUN-030"]
---

# 자원·비용·Admission 관리

## 1. 목적

다수 Bot/Core/Provider/Plugin이 CPU·메모리·토큰·Tool·Storage를 경쟁할 때 bounded admission과 단일 Policy SSOT로 안정성과 공정성을 유지한다.

## 2. 자원 차원

Core concurrency, CPU/blocking workers, Runtime RSS, Working Memory/cache/channel bytes, model request/token/cost, Tool/process, network, storage/Artifact, Memory index, Bot message fan-out, Plugin resource, UI subscribers를 독립 차원으로 관리한다.

한 `max_concurrency`가 모든 자원을 대표하지 않는다.

## 3. Policy / Limit SSOT

공유 상한·기본값은 하나의 Canonical Policy Owner에만 정의한다.

```text
Policy Catalog / Config Owner
  ├─ policy-id / config-key
  ├─ value + unit + scope
  ├─ hard/soft/default semantics
  ├─ version/generation
  └─ provenance
       ↓
Domain/Runtime docs reference only
```

다른 문서가 같은 숫자를 복사해 Normative limit으로 재정의하지 않는다. 성능 문서의 benchmark target은 runtime policy default와 구분한다.

## 4. 계층 예산

`Deployment → Tenant/Workspace → Bot → Goal/Task → Execution/Core → Provider/Tool call`

하위 grant는 상위 잔여 예산을 초과할 수 없다. Provider/Plugin manifest의 요구 budget도 deployment hard ceiling을 완화할 수 없다.

## 5. Memory Governance

Runtime memory:
- RSS soft/hard watermark
- Bot hot state
- Core Working Context
- channel bytes
- cache class budget
- Artifact spill

Canonical Long-term Memory:
- canonical bytes/revisions
- retention/archive/compaction policy
- cleanup/forget operation

**Runtime memory pressure는 Canonical Memory 삭제 trigger가 아니다.** pressure 시 cache trim → prefetch/fan-out 감소 → Working Context spill → background throttle → admission 감소 순으로 대응한다.

## 6. Provider/Plugin Resource

- Provider concurrency/rate/cost quota
- sidecar/process RSS/CPU/file descriptor
- Plugin CPU/memory/process/network/channel budget
- health/circuit breaker
- per-provider queue
- Plugin overrun은 해당 Plugin/Provider scope에서 throttle/quarantine

한 Provider 장애가 unrelated Capability queue를 막지 않도록 자원 class를 분리한다.

## 7. Fairness

- per-Bot starvation protection
- priority aging
- Routine/background와 interactive class 구분
- recovery/control safety reserve
- queue latency와 oldest waiting age 관측
- fairness regression test

## 8. Overload

`Normal → Pressured → Degraded → Emergency` 상태를 사용하며 hysteresis를 둔다.

- Pressured: fan-out/cache/background 감소
- Degraded: low-priority 신규 Task 제한
- Emergency: 신규 execution 중지, durable checkpoint/reconciliation 우선

## 9. 예외

- usage 누락: reservation 기준 보수 정산 후 reconcile
- permit leak: Lease terminal/reconciler 회수
- provider quota 감소: 신규 admission 즉시 반영
- disk full: write-heavy 차단, control/recovery reserve
- Plugin runaway: Plugin scope 격리/disable
- policy key 삭제: startup/reload validation error, silent default fallback 금지

## 10. 검증 기준

- 동일 limit/default가 둘 이상의 Normative owner에서 직접 정의되지 않는다.
- hard limit이 부하 테스트에서 초과되지 않는다.
- memory pressure가 Canonical Memory를 자동 삭제하지 않는다.
- Provider/Plugin resource overrun이 Runtime 전체 OOM으로 전파되지 않는다.
- fairness/oldest waiting/starvation 지표가 Scheduler test와 연결된다.
