---
title: "공통 계약과 Capability/Provider 확장 모델"
document_id: "DXB-ARC-012"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-ARC-011"]
---

# 공통 계약과 Capability/Provider 확장 모델

## 1. 목적과 책임

Model, Tool, Sandbox, Memory Index, Transport, Executor 등 교체 가능한 기능의 **Capability 의미와 안정 I/O 계약**을 정의한다. Provider Host/SDK/Lifecycle/Scaffold의 구현 Framework는 `17-extension-framework.md`가 Canonical Owner다.

## 2. Capability Seam

| 역할 | 책임 |
|---|---|
| Definition | 기능 의미와 invariant |
| Contract Package | request/response/event/error/config/cancel/deadline/resource/idempotency/side-effect/version |
| Provider | 실제 구현과 외부 형식 변환 |
| Consumer | Runtime/Application 사용 지점 |
| Selector | 복수 Provider 중 명시 선택 |
| Policy | 호출 허용·예산·승인·제약 |
| Provider Host | 공통 pre/post enforcement와 binding pin |
| Telemetry | 공통 metric/span/audit 의미 |
| Conformance | 모든 Provider가 같은 의미를 지키는지 실행 검증 |

실제 교체 지점이 아니면 내부 함수/모듈로 유지한다.

## 3. 초기 Capability Catalog

- `HarnessRunner`
- `ModelGateway`
- `ToolCatalog` / `ToolExecutor`
- `SkillResolver`
- `SandboxExecutor`
- `ContextAssembler`
- `ExecutionLoop`
- `ArtifactStore`
- `MemoryIndex`
- `BotTransport`
- `CoreExecutor`
- `ApprovalProvider`
- `SecretResolver`
- `TelemetrySink`

Domain-specific Store는 application Port이며 임의 범용 Provider/Plugin API로 노출하지 않는다.

## 4. Capability Contract Package

각 Capability는 최소 다음을 명시한다.

```text
Capability
├─ Request
├─ Response
├─ Streaming Event
├─ Stable Error / Outcome
├─ Config Contract
├─ Capability Metadata / Feature Negotiation
├─ Cancellation Contract
├─ Deadline Contract
├─ Resource Contract
├─ Idempotency Contract
├─ Side Effect Classification
├─ Version / Compatibility Contract
└─ Conformance Suite
```

### Request/Response
입력 필드의 required/optional 의미, size/bound, trust label, output partial/streaming semantics를 정의한다.

### Error/Outcome
validation/auth/resource/transient/permanent/cancel/timeout/partial/unknown-side-effect를 합치지 않는다. Provider-specific error code는 mapping 내부에 남긴다.

### Cancellation/Deadline
취소 요청 관측 시점, child termination 보장 범위, deadline 초과 후 late outcome 처리 의미를 정의한다.

### Resource
필요 resource class, usage reporting, output cap, concurrency/rate 특성을 정의하며 실제 Provider-specific admission은 Common Host/Resource Governance가 소유한다.

### Idempotency/Side Effect
read-only/idempotent/non-idempotent/reconciliation-required 분류와 required key/guard를 정의한다.

### Version
Capability semantic version/compatibility range와 optional feature negotiation을 정의한다.

## 5. 공통 Invocation Metadata

- correlation/causation/request ID
- Bot/Task/Execution/CoreLease ID 중 필요한 식별자
- Capability ID/version과 semantic selection slot/binding identifier
- effective deadline/cancellation
- idempotency/side-effect classification
- `ProviderSelectionRef`: selected Provider ID/version/config digest/registry generation
- policy/resource snapshot reference
- trust/sensitivity
- schema version

이 metadata는 Provider Host가 생성/검증하며 Provider가 authority를 임의 수정하지 않는다.

## 6. Registry와 Selection Binding

1. Provider manifest/config/conformance metadata 검증
2. Capability/Contract/SDK compatibility 검증
3. lifecycle validate/start
4. `Ready` 상태를 immutable Registry generation에 publish
5. Host/Selector가 신규 Execution Capability binding에 대해 Provider를 명시 선택
6. binding에 Provider ID/version/config digest/Registry generation을 `ProviderSelectionRef`로 pin
7. 동일 Execution에서 동일 binding 재사용 시 pinned selection을 검증·사용
8. unload 요청 시 `Draining`으로 전환하고 신규 binding selection에서 제외
9. Common activity/ref count 기반 quiescence
10. stop/resource dispose/Registry detach

Execution은 여러 Capability를 사용할 수 있으므로 binding도 여러 개일 수 있다. binding은 eager 또는 first-use lazy 생성이 가능하지만, **한 번 확정된 동일 binding은 Execution 동안 Provider를 바꾸지 않는다.** 같은 semantic binding의 fallback은 새 Execution/Attempt에서 수행한다.

`마지막 등록 승리`, hash iteration order, 암묵 fallback을 금지한다.

## 7. Provider Optional Feature

Provider별 고유 기능이 Capability의 본질을 바꾸지 않는다면 Capability Metadata/Negotiation으로 표현한다. 고유 기능 때문에 Consumer가 concrete Provider type을 검사하거나 downcast하게 만들지 않는다.

필요 기능이 실제로 다른 semantic contract라면 새 Capability 또는 Contract version으로 Tier A 검토한다.

## 8. Provider Removal Contract

Deprecated → 신규 binding selection 차단 → Draining → Detach → stale config/reference/binding compatibility → provider-owned derived data cleanup → dependency/code removal → Provider-free build/acceptance를 따른다.

## 9. Plugin과의 관계

Plugin은 Provider를 등록할 수 있지만 Plugin package lifecycle과 Capability semantic contract는 분리된다. Plugin이 제공한 Provider도 동일 Provider Host/Conformance를 사용한다. 내부 Rust trait를 Plugin public ABI로 노출하지 않는다.

## 10. Hook/Event 기준

- 영속 제품 사실 → Domain Event
- 실행 중 관찰/변환 → typed Hook
- notification → broadcast Event
- 승인/정책 → explicit Decision Port
- 상태 owner를 숨기는 global hook 금지
- security hook failure는 fail-closed

## 11. 검증 기준

- 모든 Provider가 Capability별 동일 Conformance Suite를 통과한다.
- Capability Contract Package 필수 semantic 누락이 CI/docs gate에서 검출된다.
- Provider 교체가 Domain/API semantic을 바꾸지 않는다.
- 한 Execution에서 여러 Capability binding을 가질 수 있고 동일 binding은 immutable하다.
- Provider A/B 동시 등록에서 selector 결과가 deterministic하다.
- Provider optional feature가 concrete type branching을 만들지 않는다.
