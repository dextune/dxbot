---
title: "공통 계약과 Capability/Provider 확장 모델"
document_id: "DXB-ARC-012"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-ARC-011"]
---

# 공통 계약과 Capability/Provider 확장 모델

## 1. 목적

Model, Tool, Sandbox, Memory Index, Transport, Executor 등 교체 가능한 기능을 일관된 계약으로 확장하되 Plugin과 내부 Capability를 혼동하지 않는다.

## 2. Capability Seam

각 Optional Capability는 다음 역할을 가진다.

| 역할 | 책임 |
|---|---|
| Definition | 기능 의미, input/output, error, cancellation, security/resource contract |
| Provider | 실제 구현과 외부 형식 변환 |
| Consumer | Runtime/Application 사용 지점 |
| Selector | 복수 Provider 중 명시 선택 |
| Policy | 호출 허용·예산·승인·제약 |
| Telemetry | 공통 metric/span/audit 표면 |
| Conformance | 모든 Provider가 같은 의미를 지키는지 검증 |

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

Domain-specific Store는 application Port이며 임의 범용 Plugin API로 노출하지 않는다.

## 4. 공통 호출 계약

외부 호출은 가능한 범위에서 다음을 가진다.

- correlation/causation/request ID
- Bot/Task/Execution/CoreLease ID
- deadline/cancellation
- idempotency 또는 non-idempotent 표시
- provider selection/config digest
- policy/resource snapshot
- trust/sensitivity
- outcome의 독립 필드
- trace/raw reference
- schema/capability version

공통 base class를 강제하지 않고 작은 값 타입을 조합한다.

## 5. Registry와 Selection

1. Provider manifest/config/conformance metadata 검증
2. dependency/capability version 검증
3. resource 초기화
4. immutable registry generation에 등록
5. selector가 신규 Execution에 대해 Provider를 명시 선택
6. Execution에 provider ID/version/config digest pin
7. unload 요청 시 신규 selection에서 제외
8. in-flight drain/quiescence
9. resource dispose와 registry 제거

복수 Provider가 존재할 때 `마지막 등록 승리`, hash iteration order, 암묵 fallback을 금지한다.

## 6. Selection Policy

우선순위는 config/policy 문서에서 Canonical하게 정의하며 일반적으로 다음 정보를 입력으로 사용한다.

- required capability/version
- explicit Task override
- Bot profile default
- security/trust constraint
- availability/health
- cost/resource class
- deployment policy

한 Execution 안에서 selection을 바꾸지 않는다. fallback은 새 attempt다.

## 7. Provider Removal Contract

제거는 다음 lifecycle을 따른다.

1. `Deprecated`: catalog/API에 표시
2. 신규 selection 차단
3. in-flight execution drain
4. registry detach
5. stale config와 stored reference compatibility 처리
6. provider-owned data migrate/export/purge
7. derived cache/index 제거
8. dependency/feature flag/code 제거
9. Provider-free build + acceptance

제거된 Provider ID를 silent fallback하지 않는다.

## 8. Plugin과의 관계

Plugin은 이 문서의 Provider registry 위에 Provider를 등록할 수 있지만 별도 lifecycle/security/package 계약을 가진다. Built-in Provider가 Plugin일 필요는 없다. Plugin SDK는 `16-plugin-system.md`를 따르며 내부 Rust trait를 public ABI로 노출하지 않는다.

## 9. Hook/Event 기준

- 영속 제품 사실 → Domain Event
- 실행 중 관찰/변환 → typed Hook
- notification → broadcast Event
- 승인/정책 → explicit Decision Port
- 상태 owner를 숨기는 global hook 금지
- Hook phase/priority를 명시하고 registration order에 의존하지 않음
- security hook failure는 fail-closed

## 10. 검증 기준

- 모든 Provider가 conformance suite를 통과한다.
- Provider 교체가 Domain test/CLI semantic을 바꾸지 않는다.
- Provider 제거 후 domain/runtime compile과 DB restore가 가능하다.
- Provider A/B 동시 등록에서 selector 결과가 deterministic하다.
- Plugin 없이도 Capability/Provider 구조가 완전하게 동작한다.
