---
title: "Harness Provider 통합"
document_id: "DXB-ARC-013"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-012", "DXB-ARC-014", "DXB-ARC-017"]
---

# Harness Provider 통합

## 1. 목적

Harness를 교체 가능한 Reference Capability 사례로 유지하면서 Provider Session과 DXBOT Thread를 명확히 분리하고, cancellation/yield/steering capability를 Live Control과 안전하게 연결한다.

## 2. 기본 구조

`Task/Core Runtime → Provider Host → HarnessRunner SPI → DeepSeek/Native/Reference Provider`를 유지한다. DeepSeek는 optional Provider다.

## 3. DXBOT Common 소유 의미

Bot Identity/Lifecycle, Main Conversation/Thread, Brain/Context Plan, Canonical Memory, Goal/Task/Execution/Core Lease, Routine/Continuation/Suspension, Control Directive, Permission/Resource, Side Effect, Provider selection/lifecycle/retry disposition/telemetry는 Harness Provider가 소유하지 않는다.

## 4. Harness Contract Package

ExecutionRequest / HarnessEvent / ExecutionOutcome와 config/cancel/deadline/resource/idempotency/side-effect/checkpoint/version/conformance 의미를 유지한다.

v0.4 feature negotiation 후보:
- cooperative cancellation 지원 여부
- safe yield/checkpoint 지원 여부
- provider-native steering 지원 여부
- resumable Provider Session/token 지원 여부
- progress/heartbeat/report observation 지원 여부

지원 여부는 Capability metadata이지 Core Domain state가 아니다.

## 5. Provider Session 규칙

Provider Session은 다음에만 사용할 수 있다.
- connection/context reuse
- provider-native conversation optimization
- compatible resume/checkpoint optimization

금지:
- ThreadId로 사용
- Main Conversation identity로 사용
- Canonical transcript/Memory의 유일 저장소로 사용
- Provider Session 없이는 Task/Thread 복구 불가능한 설계
- hidden Provider Session history가 DXBOT Context Plan보다 높은 authority를 갖는 설계

Provider Session token을 저장해야 하면 Contract version/compatibility와 checkpoint/Artifact reference로 취급한다.

## 6. Live Control 연결

- redirect/suspend 전에 DXBOT Directive/Task revision이 Canonical commit된다.
- Host가 cancellation/yield/steering feature를 negotiated surface로 호출한다.
- native steering이 있어도 old Execution snapshot은 mutation하지 않는다.
- Provider가 steering을 미지원하면 safe boundary까지 기다리거나 cancellation outcome을 명시한다.
- Hard Real-Time preemption을 약속하지 않는다.
- provider call 중 Side Effect safety는 `DXB-DOM-023`/`DXB-RUN-036`을 따른다.

## 7. DeepSeek Adapter

DeepSeek 채택 시 exact version/digest/protocol fingerprint pin, raw trajectory Artifact, diagnostics 분리, process tree/cancellation/quiescence hook, scoped secret, unknown event quarantine를 유지한다.

DeepSeek Session은 Provider-private optimization이며 손실되어도 Bot/Conversation/Thread/Memory/Task를 복원할 수 있어야 한다.

## 8. Reference Harness

deterministic Reference Provider는 session loss, cancellation unsupported/supported, yield safe-point, progress heartbeat, malformed/partial stream을 scripted하게 재현해 Live Control test를 외부 모델 비결정성과 분리한다.

## 9. Removal / Replacement

Harness Provider 제거가 Conversation/Thread/Memory schema 변경을 요구하지 않는다. persisted Provider Session token이 남으면 stale/unsupported diagnostic 또는 migration policy를 적용하고 Thread를 silent recreate하지 않는다.

## 10. 검증 기준

- DeepSeek crate/session 없이 Core/Conversation/Thread acceptance 통과
- Reference/DeepSeek/Native가 동일 Conformance 사용
- Provider Session loss 후 Canonical Context Plan으로 새 call 가능
- redirect가 native steering 지원 여부와 무관하게 Task revision/Execution immutable invariant 유지
- upstream session/type가 Domain public identity에 등장하지 않음
