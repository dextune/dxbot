---
title: "Provider Adapter Conformance와 Usage Accounting 계획"
document_id: "DXB-ADP-025"
version: "0.1.0"
status: "Reference Snapshot"
normative: false
priority: "P0"
last_updated: "2026-08-25"
depends_on: ["DXB-ADP-010", "DXB-ADP-020"]
target_owners: ["provider-host", "runtime-host", "runtime-security", "application"]
package_path: "docs/plan/20260825-0930-grok-architecture-adoption-plan"
source_baseline:
  dxbot_commit: "e43739614631c95752482c2ad2ec53cb7ebd251f"
  grok_reconstructed_commit: "a9f633e09d49a85829b8236331b9e21f7e612634"
---
# Provider Adapter Conformance와 Usage Accounting 계획

## 1. 판단

**분류: Close.** Grok은 Cursor, Claude Code, Codex, OpenRouter를 하나의 inference routing surface에 연결하여 실제 provider 이질성을 드러낸다. DXBOT은 이 provider 목록을 복제하지 않고, **서로 다른 전송·수명 특성을 가진 두 Reference Adapter**로 Provider SPI를 검증한다.

권장 조합:

1. HTTP streaming adapter
2. subprocess/CLI streaming adapter

실제 상용 provider 추가가 아니라 contract conformance를 위한 reference/canary 구현으로 시작한다.

## 2. 공통 Provider contract

```text
ProviderDescriptor
- provider_id
- adapter_kind
- capability_set
- protocol_version
- config_schema_version
- credential_requirements
- lifecycle_generation
- resource_profile

ProviderInvocation
- execution_id
- immutable context ref
- capability request
- deadline/cancellation
- action grants
- tool policy
- usage budget
```

Provider는 Bot/Task/Conversation/Receipt의 public schema owner가 아니다.

## 3. Conformance matrix

| 항목 | HTTP Adapter | Subprocess Adapter | 공통 판정 |
|---|---|---|---|
| startup/readiness | endpoint/auth probe | binary/version/auth probe | typed readiness |
| streaming | network chunks | stdout/protocol frames | ordered bounded events |
| cancellation | request abort | signal/kill escalation | deadline 내 종료 |
| timeout | transport timeout | process timeout | normalized error |
| credential | scoped token | credential broker/explicit lease | raw secret 미노출 |
| tool call | API/tool protocol | MCP/stdin protocol | action grant 재검증 |
| usage | provider usage fields | CLI result/estimated fields | normalized confidence |
| retry | status/error class | exit/error class | common policy owner |
| crash | connection reset | process exit | generation/cleanup |

## 4. Provider-specific branch 방화벽

허용 위치:

- adapter module 내부 DTO
- external error mapping
- credential acquisition adapter
- protocol frame parser
- model/capability discovery

금지 위치:

- Application use case의 `if provider == ...`
- CLI output schema 분기
- Domain state에 provider session 저장
- provider response를 receipt schema로 직접 노출
- coordinator가 credential refresh를 소유

## 5. Streaming contract

```text
Started
→ TextDelta | ToolRequest | EvidenceDelta | UsageDelta
→ Completed | Cancelled | Failed
```

- event ordering key와 sequence를 갖는다.
- chunk/item/byte buffer는 bounded다.
- malformed frame은 silent skip하지 않는다.
- terminal event는 정확히 하나다.
- cancellation 이후 새 event publish를 막는다.
- partial output은 canonical result가 아니며 checkpoint/evidence policy를 따른다.

## 6. Tool/MCP 경계

Grok의 routed MCP bridge는 practical reference지만, DXBOT에서는 다음을 강제한다.

- Provider가 host tool을 직접 실행하지 않는다.
- tool request는 typed capability/action request로 변환한다.
- runtime-security가 ActionGrant, scope, side-effect class를 재검증한다.
- execution depth, tool count, byte, deadline budget을 적용한다.
- tool result는 provider-specific response와 분리된 Evidence/Result mapping을 거친다.

## 7. Credential lifecycle

- credential 원본 경로를 provider adapter에 영구 노출하지 않는다.
- `CredentialRef → SecretGrant → materialize → use → revoke`를 사용한다.
- refresh ownership과 persistence owner를 명시한다.
- symlink, permission, audience, expiry를 검증한다.
- provider CLI의 기존 로그인 파일을 읽는 canary가 필요하면 read-only broker를 통해 제한한다.
- adapter가 사용자 credential 파일을 rewrite하는 방식은 기본 금지한다.

## 8. Usage accounting

```text
NormalizedUsage
- provider_id
- execution_id
- input_units
- output_units
- cache_read_units?
- cache_write_units?
- tool_units?
- cost_amount?
- currency?
- source: Reported | Derived | Estimated
- completeness
- observed_at
```

- activity record와 invoice를 구분한다.
- provider가 값을 주지 않으면 0으로 단정하지 않는다.
- retry/continuation/tool step의 중복 합산을 방지한다.
- metric label에는 prompt/model-user input을 넣지 않는다.
- usage는 scheduling/accounting input이 될 수 있으나 provider invoice truth가 아니다.

## 9. Error normalization

최소 class:

```text
ConfigurationInvalid
CredentialUnavailable
CredentialExpired
ProviderUnavailable
RateLimited
ProtocolInvalid
CapabilityUnsupported
InvocationRejected
InvocationTimedOut
InvocationCancelled
ProcessCrashed
ToolRequestRejected
ResourceExhausted
```

외부 provider raw error는 safe details와 digest로 제한한다.

## 10. 구현 작업

1. 현재 `provider-host` protocol/harness/deepseek 경계를 inventory한다.
2. conformance test suite를 adapter-independent fixture로 만든다.
3. HTTP Reference Adapter를 통과시킨다.
4. subprocess Reference Adapter를 통과시킨다.
5. cancellation, deadline, malformed stream, crash, credential expiry fault를 넣는다.
6. NormalizedUsage와 confidence를 private result로 연결한다.
7. Application에는 ProviderId/capability/result만 노출되는지 검증한다.

## 11. Acceptance

- 두 adapter가 같은 conformance suite를 통과한다.
- Application/CLI에 provider-specific branch가 추가되지 않는다.
- cancellation 뒤 process/socket/task가 남지 않는다.
- malformed frame이 성공으로 처리되지 않는다.
- tool request가 ActionGrant를 우회하지 않는다.
- credential raw value가 log/audit/receipt에 나타나지 않는다.
- usage의 reported/estimated/unknown이 구분된다.
- adapter 제거 build가 다른 adapter와 core test를 깨뜨리지 않는다.
