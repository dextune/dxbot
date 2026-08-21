---
title: "자원·비용·Admission 관리"
document_id: "DXB-RUN-031"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-024", "DXB-RUN-030", "DXB-ARC-017"]
---

# 자원·비용·Admission 관리

## 1. 목적

다수 Bot/Thread/Core/Provider/Plugin과 Live Control이 CPU·메모리·토큰·Tool·Storage를 경쟁할 때 bounded admission과 하나의 Policy SSOT를 유지한다.

## 2. 자원 차원

Core concurrency, CPU/blocking workers, Runtime RSS, Working/Thread context cache/channel bytes, model request/token/cost, Tool/process/network/storage/Artifact, Memory index, Bot message fan-out, Provider/Plugin resource, UI subscriber, **Control Channel capacity/bytes/control processing reserve**를 독립 차원으로 관리한다.

## 3. Policy SSOT

공유 hard/soft limit/default 값은 Canonical Policy Owner에만 둔다. 문서의 “reserved control capacity”, “parallelism hint”는 semantic이지 고정 숫자가 아니다.

## 4. 계층 예산

`Deployment → Workspace/Tenant → Bot → Thread/Goal/Task → Execution/Core → Provider Host Call`.

Thread budget은 Bot hard ceiling을 초과할 수 없고 control request도 resource ceiling을 완화하지 못한다.

## 5. Provider Host Resource Wrapper

v0.3의 capability class → upper budget → provider quota → permit → bounded output → accounting → release 흐름을 유지한다. Provider가 permit/hard ceiling을 직접 mint/완화하지 않는다.

## 6. Control Safety Reserve

Normal Work와 Control을 동일 admission pool 하나에 완전히 의존시키지 않는다.

정책은 최소 다음을 구분할 수 있다.
- inspect/report/cancel/security control의 reserved admission class
- redirect/suspend/reprioritize의 bounded command rate
- per-Bot/per-principal control fairness
- control queue item+byte cap
- overflow reject/coalesce/supersede policy

Control reserve는 무한 우선순위가 아니다. flood 시 rate limit/authorization/coalescing으로 일반 work와 다른 Bot control을 보호한다.

## 7. Memory / Transcript Governance

Runtime pressure는 Canonical Memory/Conversation History 삭제 trigger가 아니다.

pressure 대응:
1. Derived cache/history view trim
2. prefetch/fan-out 감소
3. Working Context spill
4. background retrieval/index/compaction throttle
5. new Execution admission 감소

장기 Thread transcript와 lineage는 cold/archive/pagination/compaction 정책으로 관리하고 전체 hot-memory materialization을 금지한다.

## 8. Reprioritize / Parallelism Hint

user control은 requested priority/parallelism/resource adjustment를 만들 수 있으나 Scheduler가 feasibility/fairness/hard limit을 평가한다. “Core 하나 더”를 direct Lease allocation으로 해석하지 않는다.

## 9. Fairness / Overload

- per-Bot starvation protection
- interactive/background/control class 분리
- one Thread control storm이 다른 Thread/Bot을 고갈시키지 않음
- Provider quota wait가 global scheduler를 독점하지 않음
- Normal → Pressured → Degraded → Emergency 상태에서 control safety path를 우선 보전할 수 있음

## 10. 검증 기준

- production unbounded Work/Control/Provider/UI queue 0.
- Work Queue 포화에서 AT-CTRL-002 report/inspect 성공/명시 reject가 bounded latency class로 처리되고 무기한 대기하지 않음.
- control flood가 hard resource limit을 넘거나 일반 work를 영구 starvation시키지 않음.
- Runtime pressure가 Canonical Memory/Conversation item을 암묵 삭제하지 않음.
- quota/drain/cancel/control-induced yield 후 permit/activity/Lease leak 0.
