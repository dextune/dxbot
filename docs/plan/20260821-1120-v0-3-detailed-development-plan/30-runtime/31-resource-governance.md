---
title: "자원·비용·Admission 관리"
document_id: "DXB-RUN-031"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-024", "DXB-RUN-030", "DXB-ARC-017"]
---

# 자원·비용·Admission 관리

## 1. 목적

다수 Bot/Core/Provider/Plugin이 CPU·메모리·토큰·Tool·Storage를 경쟁할 때 bounded admission과 단일 Policy SSOT를 유지하며, Provider가 자체 admission/rate/retry 경로로 Common 예산을 우회하지 못하게 한다.

## 2. 자원 차원

Core concurrency, CPU/blocking workers, Runtime RSS, Working Memory/cache/channel bytes, model request/token/cost, Tool/process, network, storage/Artifact, Memory index, Bot message fan-out, Provider/Plugin resource, UI subscriber를 독립 차원으로 관리한다.

## 3. Policy / Limit SSOT

공유 상한·기본값은 하나의 Canonical Policy Owner에만 정의한다. Provider config의 `recommended concurrency`나 외부 SDK default는 DXBOT hard/soft limit의 Canonical 값이 아니다.

## 4. 계층 예산

`Deployment → Tenant/Workspace → Bot → Goal/Task → Execution/Core → Provider Host Call → Provider/Tool`

하위 grant는 상위 잔여 예산을 초과할 수 없다.

## 5. Provider Host Resource Wrapper

Provider 호출 전 Host가 다음을 수행한다.

1. capability resource class 확인
2. Bot/Task/Execution 상위 budget 확인
3. Provider별 concurrency/rate/cost quota 확인
4. reservation/permit 획득
5. bounded output/stream budget 설정
6. call context에 grant/reference만 전달
7. Provider usage observation 수집
8. 실제 accounting/reconciliation
9. permit/activity 반환

Provider는 permit을 직접 mint하거나 hard ceiling을 완화할 수 없다. Provider가 반환한 usage는 관측 입력이며 Canonical accounting을 임의 수정하지 않는다.

## 6. Memory Governance

Runtime memory pressure는 Canonical Memory 삭제 trigger가 아니다. pressure 시 cache trim → prefetch/fan-out 감소 → Working Context spill → background throttle → admission 감소 순으로 대응한다.

Provider SDK/Host buffer도 item+byte cap을 가진다. Provider-specific response를 무제한 materialize하지 않고 streaming/Artifact spill을 사용한다.

## 7. Provider/Plugin Resource

- Provider concurrency/rate/cost quota
- sidecar/process RSS/CPU/file descriptor
- per-provider Host queue/admission
- Plugin CPU/memory/process/network/channel budget
- health/circuit state

한 Provider 장애가 unrelated Capability queue를 막지 않도록 resource class와 permit pool을 분리한다.

## 8. Fairness

per-Bot starvation protection, priority aging, interactive/background class, recovery/control safety reserve를 둔다. Provider별 rate limit 때문에 한 Bot이 global scheduler를 독점하지 않도록 queue wait를 Host/Scheduler에서 관측한다.

## 9. Overload

`Normal → Pressured → Degraded → Emergency` 상태를 사용한다. Provider overrun은 해당 Provider의 신규 Host admission 감소/차단, lifecycle Degraded/Quarantine 판단으로 연결할 수 있다.

## 10. 예외

- usage 누락: reservation 기준 보수 정산 후 reconcile
- permit/activity leak: terminal/reconciler 회수
- provider quota 감소: 신규 Host admission 즉시 반영
- Provider 자체 retry가 budget을 중복 소비할 가능성: contract 위반으로 차단
- Plugin runaway: Plugin/Provider scope 격리/disable
- policy key 삭제: startup/reload validation error

## 11. 검증 기준

- 같은 semantic limit/default가 둘 이상의 Normative owner에 없다.
- Provider가 Common permit 없이 external semantic call을 시작하지 못한다.
- hard limit이 부하 테스트에서 초과되지 않는다.
- Provider/Plugin resource overrun이 Runtime 전체 OOM으로 전파되지 않는다.
- quota/drain/timeout 시 permit/activity leak가 0이다.
