---
title: "초기 아키텍처 결정 기준선"
document_id: "DXB-GOV-003"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-GOV-002"]
---

# 초기 아키텍처 결정 기준선

## 1. 목적

구현팀이 핵심 구조를 암묵적으로 재결정하지 않도록 기본 결정을 명시한다. 정식 검증 후 개별 ADR로 승격한다.

## 2. 결정 목록

| ADR | 기본 결정 | 재검토 트리거 |
|---|---|---|
| ADR-0001 | Bot 중심 Aggregate와 Session 비소유성 | Session이 Canonical State를 소유해야 하는 증거 |
| ADR-0002 | Domain Journal + Current State Hybrid | 복구/쓰기 비용이 예산 초과 |
| ADR-0003 | Bot별 단일 쓰기 조정자 | 계측된 병목 |
| ADR-0004 | Core는 Lease 기반 일시 실행 | 장기 독립 정체성 요구 |
| ADR-0005 | Harness는 Port/Adapter로 격리 | 단일 표준 고정의 실질 증거 |
| ADR-0006 | 초기 단일 노드/프로세스 | 원격 실행이 P0 요구가 됨 |
| ADR-0007 | Embedded Store 우선 | multi-writer/HA 요구 |
| ADR-0008 | durable Inbox/Outbox, at-least-once | exactly-once 필요 증명 |
| ADR-0009 | immutable Snapshot + Command write | snapshot 비용 초과 |
| ADR-0010 | 모든 queue/cache bounded | 비협상 |
| ADR-0011 | 모든 UI는 동일 Control API 사용 | 비협상 |
| ADR-0012 | 큰 데이터는 Artifact reference | 비협상 |
| ADR-0013 | 실행은 Policy/Config snapshot pin | 진행 중 실시간 정책 변경 필요 |
| ADR-0014 | 외부 오류와 Domain 오류 분리 | 비협상 |
| ADR-0015 | 최적화는 benchmark/profile 기반 | 비협상 |
| ADR-0016 | 모든 기능을 5개 Architecture 범주로 분류 | 새 범주 필요가 증명됨 |
| ADR-0017 | 선택 기능에 Feature Removal Contract 의무화 | 비협상 |
| ADR-0018 | repo path kebab-case, Rust `*.rs` snake_case 예외 | toolchain 관례 변화 |
| ADR-0019 | Side Effect는 write-ahead Intent 후 외부 실행 | 외부 시스템이 더 강한 원자 계약 제공 |
| ADR-0020 | Routine은 Bot-owned Task generator | Routine이 독립 Domain entity가 되어야 하는 증거 |
| ADR-0021 | Plugin 공개 경계는 stable Wire/SDK, 내부 Rust trait ABI 금지 | 안정 ABI를 별도 보증하는 런타임 채택 |
| ADR-0022 | 복수 Provider는 명시 Selector, 한 Execution에서 selection 고정 | execution 중 migration 요구 |
| ADR-0023 | 공용 limit/default는 Policy SSOT에서 소유 | 비협상 |
| ADR-0024 | production Provider 호출은 Provider Host를 반드시 경유 | Host가 semantic overhead를 만든다는 검증된 증거와 대체 enforcement 설계 |
| ADR-0025 | Provider Lifecycle/Drain/Activity 판정은 Common 소유 | 외부 orchestration 표준이 동등한 불변조건을 증명 |
| ADR-0026 | Capability는 완전한 Contract Package와 Conformance를 소유 | 비협상 |
| ADR-0027 | Provider는 Thin Implementation이며 최소 Context만 수신 | capability가 실제 Domain owner가 되어야 하는 증거 |
| ADR-0028 | Provider dependency firewall을 CI에서 강제 | 비협상 |
| ADR-0029 | 주요 Capability는 Executable Conformance + Reference Provider를 보유 | 비협상 |
| ADR-0030 | Provider SDK는 internal Runtime API가 아닌 허용된 stable surface만 공개 | 비협상 |
| ADR-0031 | Common Contract 변경은 Additive/Clarification/Deprecation/Breaking으로 분류 | 비협상 |
| ADR-0032 | Common/Contract/Host 변경은 Tier A, 기존 Provider 구현은 Tier B로 분리 | 프로젝트 개발 전략 변경 |
| ADR-0033 | Provider 생성은 표준 skeleton을 따르고 P1부터 Scaffold 자동화를 목표 | 수동 방식이 동등한 일관성을 CI로 증명 |

## 3. ADR 문서 필수 항목

- Context / Decision / Alternatives
- Classification과 Owner
- Invariants
- Compatibility / Migration / Removal
- Security / Resource / Failure
- Provider Host/SDK/Conformance 영향 여부
- Affected Providers / Plugin impact
- Rollout / Rollback
- Verification Evidence
- Supersedes / Superseded by

## 4. Provider Host 결정 상세

`Consumer → Provider Host → Provider SPI → Provider`가 production 기본 경로다. Host는 schema validation, selection/generation pin, permission, resource admission, deadline/cancellation, side-effect guard, telemetry/audit, error normalization, output validation, accounting을 공통으로 집행한다.

Host는 Domain rule을 재구현하지 않고 해당 Canonical Owner의 Port를 사용한다. Provider가 Host의 기능을 자체 구현하는 것은 Host 우회 허용 사유가 아니다.

## 5. Thin Provider 결정 상세

Provider가 소유하는 것은 provider-specific config, external SDK/API adapter, capability-specific logic, DTO/error mapping이다. `RuntimeContext`, raw Store/Scheduler/Registry, ambient secret 권한은 Provider SPI 표면에 포함하지 않는다.

Provider-specific 필요가 Common surface 확대를 요구하면 Provider에서 임시 우회하지 않고 Tier A Contract 변경으로 검토한다.

## 6. Contract Stability 결정 상세

Breaking Semantic Change에는 ADR, migration/compatibility, affected Provider 목록, Conformance/Reference/SDK/Acceptance/Risk 업데이트가 모두 필요하다. Extension 편의를 이유로 Common Contract를 좁은 구현 요구에 맞춰 변형하지 않는다.

## 7. Naming 결정 상세

사용자 정의 경로는 lowercase kebab-case를 사용한다. Rust module 파일은 `snake_case.rs`를 허용한다. 도구 고정 이름은 CI allowlist로 관리한다.

## 8. Side Effect 결정 상세

외부 비멱등 동작은 `Intent/Key/Action Digest`를 Commit한 뒤 수행한다. 외부 성공 여부가 불명확하면 `Unknown`으로 복구하고 자동 Retry보다 reconciliation을 우선한다. Provider Host는 필요한 guard가 준비되지 않은 Side Effect invocation을 통과시키지 않는다.

## 9. 검증 기준

- 각 P0 ADR 후보에 Architecture/Runtime/Conformance Test가 연결된다.
- Provider Host bypass와 forbidden dependency를 Architecture Test가 차단한다.
- Breaking Capability change가 Provider inventory/Conformance/SDK/Reference 갱신 없이 merge되지 않는다.
- 대안·rollback·검증 증거가 없는 비가역 결정은 Accepted로 승격하지 않는다.
