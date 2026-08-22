---
title: "Harness Capability 계약과 실제 실행 canary"
document_id: "DXB-ARC-013"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-ARC-010", "DXB-ARC-012"]
---
# Harness Capability 계약과 실제 실행 canary

## 1. 경계와 Owner

```text
Application/Runtime
→ P0 Harness Capability Contract
→ Provider Host
→ Harness Provider Adapter
→ external Model / Tool / Sandbox
```

Capability semantic과 Host enforcement는 `provider-host`가 소유한다. Provider는 Domain, Memory, Permission, Scheduler state를 직접 mutate하지 않는다.

## 2. Request

```text
ExecuteRequest {
  execution_ref: ExecutionId + ExecutionGeneration,
  context_plan_ref: ContextPlanId + digest,
  capability_requirements,
  input_artifact_refs,
  provider_generation,
  resource_grant_ref,
  action_grant_refs,
  effective_deadline,
  cancellation_ref,
  output_budget
}
```

외부 Session/Agent/Subagent ID를 Bot/Thread/Core/Memory identity로 사용하지 않는다.

## 3. Stream과 Result

```text
ExecuteEvent = Started
             | OutputChunk(sequence, bytes)
             | ToolActivity(activity_ref, state)
             | Evidence(evidence_ref)
             | Progress(class, bounded_summary)
             | Terminal(Completed | Failed | Cancelled | Unknown)

ExecuteResult = ResultRef + EvidenceRefs + UsageAccounting + ProviderTraceRef
```

partial output은 terminal success가 아니다. output sink와 event buffer는 item+byte cap, slow-consumer, cancellation, cleanup을 가진다.

## 4. Stable error

`InvalidRequest | Incompatible | Unavailable | PermissionDenied | ResourceExhausted | Timeout | Cancelled | ExternalUnknown | ProtocolViolation`.

`ExternalUnknown`은 blind retry하지 않고 Side Effect/Execution reconciliation으로 전달한다. Provider 문자열 parsing으로 retry/security 결정을 내리지 않는다.

## 5. 이중 검증

- Reference Provider: fake clock/seed/result로 deterministic correctness 검증
- Real Adapter canary: 실제 외부 경로가 같은 Provider Host pipeline을 통과해 success/failure/cancel Task를 수행

Reference Provider는 production unavailable의 fallback이 아니다.

## 6. Release evidence

adapter exact version/commit/license, Host-path trace, bounded I/O, cancel/deadline propagation, success/failure/cancel canary, Adapter 제거 build를 기록한다.
