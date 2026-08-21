---
title: "Harness Provider 통합"
document_id: "DXB-ARC-013"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-012", "DXB-ARC-014", "DXB-ARC-017"]
---

# Harness Provider 통합

## 1. 목적

Harness를 v0.3 Extension Framework의 **Reference Capability 사례**로 정의한다. DeepSeek, Native, 향후 Harness를 교체·복수 등록할 수 있으며 모든 호출은 동일 Provider Host/Conformance 경로를 사용한다.

## 2. 기본 구조

```text
dxb-harness
  └─ HarnessRunner Contract Package

dxb-provider-host
  └─ common invocation/lifecycle enforcement

dxb-provider-sdk
  └─ allowed implementation helpers

providers/
  ├─ dxb-harness-deepseek
  ├─ dxb-harness-native
  └─ dxb-harness-<future>

dxb-testkit
  └─ deterministic reference/fake harness + conformance
```

DeepSeek는 initial/default candidate가 될 수 있으나 필수 dependency가 아니다.

## 3. DXBOT Common이 소유하는 의미

Bot Identity/Lifecycle, Brain, Canonical Memory, Goal/Task/Execution/Core Lease, Routine/Continuation, Permission, Bot Network, resource/admission, Side Effect Ledger, Provider selection/lifecycle, retry disposition, telemetry/audit는 Harness Provider가 소유하지 않는다.

## 4. Harness Contract Package

입력 `ExecutionRequest`, stream `HarnessEvent`, 최종 `ExecutionOutcome`에 더해 다음을 명시한다.

- config/capability metadata
- cancellation/deadline semantics
- resource/usage contract
- idempotency/side-effect classification
- checkpoint compatibility
- version/feature negotiation
- conformance suite

외부 Provider DTO/Event는 Domain Event가 아니다.

## 5. Provider Host 적용

Harness Consumer가 Provider를 직접 호출하지 않는다.

```text
Task/Core Runtime
→ Harness Provider Host
→ HarnessRunner SPI
→ DeepSeek/Native/Reference Provider
```

Host가 selection, permission, resource admission, deadline/cancellation, side-effect guard, telemetry/audit, output limit, error normalization을 적용한다.

Harness Provider가 자체 global retry loop, permission resolver, scheduler, Domain persistence를 구현하지 않는다. protocol 수준 reconnect/retry가 필요하면 Harness Contract가 허용하는 범위와 idempotency를 지켜야 한다.

## 6. Provider Selection

- Bot profile default 가능
- Task별 override 가능
- 여러 Harness Provider 동시 등록 가능
- selector가 compatibility/security/budget/availability를 평가
- 하나의 Execution에서는 Provider ID/version/config generation을 고정
- Provider 변경/fallback은 새 Execution attempt부터

암묵적 global singleton Harness를 금지한다.

## 7. DeepSeek Adapter

DeepSeek를 채택할 경우:
- sidecar 또는 명시 integration boundary로 격리
- exact version/digest/protocol fingerprint pin
- raw trajectory는 Artifact로 보존
- stdout/protocol과 diagnostics 분리
- process tree/cancellation/quiescence 관리 hook 제공
- secret는 Host가 승인한 handle/filtered environment로만 제공
- unknown upstream event는 추측 변환하지 않고 quarantine/unsupported mapping

DeepSeek Session은 resume optimization일 뿐 Canonical Bot/Task/Memory state가 아니다.

## 8. Reference Harness Provider

Common team은 외부 모델 비결정성과 무관한 deterministic Reference Harness Provider를 유지한다. 이는 Provider SDK 사용, lifecycle hook, streaming/error/cancel mapping, Host-path Conformance의 정답 예제다.

Reference Provider가 production 기본 구현일 필요는 없다.

## 9. Removal / Replacement

DeepSeek Provider 제거 시 Domain/Application public schema 변경 금지, stale config 명시 처리, in-flight drain/cancel/reconcile, 대체 Provider Conformance, Provider-free build/restore를 수행한다.

## 10. 검증 기준

- DeepSeek crate 없이 core workspace가 build된다.
- Reference/DeepSeek/Native가 동일 Harness Conformance를 따른다.
- Host를 우회해 Harness Provider를 호출하는 production path가 없다.
- 진행 중 Execution의 Provider가 hot reload로 바뀌지 않는다.
- upstream type/session ID가 Domain public API에 등장하지 않는다.
