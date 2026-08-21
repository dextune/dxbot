---
title: "Harness Provider 통합"
document_id: "DXB-ARC-013"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-012", "DXB-ARC-014"]
---

# Harness Provider 통합

## 1. 목적

DXBOT 실행 기반을 안정된 Harness Contract 뒤에 격리하여 DeepSeek, Native, 향후 Provider를 교체·복수 등록할 수 있게 한다. DeepSeek의 유용한 실행 철학은 참고하되 DXBOT의 제품 정체성이나 Canonical State를 위임하지 않는다.

## 2. 기본 구조

```text
dxb-harness
  └─ HarnessRunner / Harness Contract

providers/
  ├─ dxb-harness-deepseek
  ├─ dxb-harness-native
  └─ dxb-harness-<future>

dxb-testkit
  └─ deterministic fake harness
```

DeepSeek는 **initial/default candidate Provider**일 수 있으나 필수 dependency가 아니다.

## 3. DXBOT이 소유하는 의미

- Bot Identity/Lifecycle
- Brain policy/context semantics
- Canonical Memory
- Goal/Task/Execution/Core Lease
- Routine과 Task Continuation
- Permission의 제품 의미
- Bot Network
- Domain Journal/Control Plane
- resource/admission
- Side Effect Ledger의 제품 상태

Harness Provider는 이를 소유하지 않는다.

## 4. Harness가 제공하는 Capability

- model request/stream
- tool execution loop
- skill/context injection
- sandbox adapter
- step/turn 운전
- checkpoint/resume token 후보
- trajectory/usage

Harness Session ID는 BotId가 아니며 Harness fork는 Bot clone/Core creation이 아니다.

## 5. 논리 계약

입력 `ExecutionRequest`:
- Bot/Task/Execution/Core identifiers
- immutable Brain/Identity/Memory/Policy references
- selected capability/provider config
- deadline/cancellation/resource grant
- prior compatible checkpoint

stream `HarnessEvent`:
- admitted/model delta/tool call/tool result
- approval/checkpoint/usage
- terminal provider signal

최종 `ExecutionOutcome`:
- completed/failed/cancelled/timed-out/aborted 의미
- partial result 여부
- result/trace Artifact references
- usage/cost
- retry disposition
- Memory Proposals

외부 Provider DTO/Event는 Domain Event가 아니다.

## 6. Provider Selection

- Bot profile default 가능
- Task별 override 가능
- 여러 Harness Provider 동시 등록 가능
- selector가 compatibility/security/budget/availability를 평가
- 하나의 Execution에서는 Provider ID/version/config를 고정
- Provider 변경/fallback은 다음 Execution attempt부터

암묵적 global singleton Harness를 금지한다.

## 7. DeepSeek Adapter

DeepSeek를 채택할 경우:
- sidecar 또는 명시된 integration boundary로 격리
- exact version/digest/protocol fingerprint pin
- raw trajectory는 Artifact로 보존
- stdout/protocol과 diagnostics 분리
- process tree/cancellation/quiescence 관리
- secret는 handle/filtered environment로 제공
- upstream unknown event는 추측 변환하지 않고 quarantine

DeepSeek Session은 resume optimization일 뿐 Canonical Bot/Task/Memory state가 아니다.

## 8. Removal / Replacement

DeepSeek Provider 제거 시:

- `dxb-domain`, `dxb-application`, `dxb-runtime` public schema 변경 금지
- DeepSeek config는 deprecated→migration/error 처리
- in-flight DeepSeek Execution drain 또는 명시 cancel/reconcile
- 새 Execution은 selector가 다른 compatible Provider를 선택하거나 unsupported 반환
- Bot Identity/Memory/Waiting Task는 그대로 복원
- conformance suite를 대체 Provider에 실행

## 9. 검증 기준

- DeepSeek crate 없이 core workspace가 build된다.
- Fake/DeepSeek/Native가 동일 conformance scenario를 따른다.
- Bot A=Provider A, Bot B=Provider B, Task C=Provider C 선택이 가능하다.
- 진행 중 Execution의 Provider가 hot reload로 바뀌지 않는다.
- upstream type/session ID가 Domain public API에 등장하지 않는다.
