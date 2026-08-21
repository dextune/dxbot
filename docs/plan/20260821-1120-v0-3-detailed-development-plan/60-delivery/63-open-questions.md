---
title: "미결정 사항과 권장 기본값"
document_id: "DXB-DEL-063"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-GOV-003", "DXB-ARC-017", "DXB-DEL-060"]
---

# 미결정 사항과 권장 기본값

## 1. 목적

v0.3 Common Framework 방향은 고정하되 비가역 구현 세부를 암묵 결정하지 않도록 질문, 권장 기본값, 결정 증거, ADR Gate를 관리한다.

## 2. P0 결정

| ID | 질문 | 권장 기본값 | Gate |
|---|---|---|---|
| OQ-001 | DeepSeek 실제 채택 범위 | Harness Contract+Reference/Fake 우선, DeepSeek optional Provider | M0 |
| OQ-002 | embedded DB | transactional embedded SQL 계열, domain-specific store | M0 spike |
| OQ-003 | Event payload | versioned structured payload + normalized hot fields | M0 |
| OQ-004 | ID 형식 | sortable 128-bit newtype 후보 | schema freeze |
| OQ-005 | Bot activation | lazy activation + idle eviction | M1 soak |
| OQ-006 | trajectory 보존 | Artifact + Domain summary/ref | M2 privacy |
| OQ-007 | Rust async stack | 하나의 표준 runtime/tracing stack | workspace freeze |
| OQ-008 | local control transport | Unix socket/named pipe abstraction | M1 |
| OQ-009 | CLI/daemon packaging | logical binaries, shared Control contract | M1 |
| OQ-010 | Memory delete/backup | immediate tombstone + retention ledger | M2 |
| OQ-011 | Sandbox OS | strongest supported capability + unsupported fail-closed | M2 |
| OQ-012 | actual model release gate | canary, core gate Reference/Fake | M2 |
| OQ-013 | Side Effect exact state enum | Prepared/Confirmed/Failed/Unknown/Reconciled semantics 고정, 세부 enum ADR | M2 schema |
| OQ-014 | Routine missed/overlap policy set | bounded explicit enum, arbitrary script 금지 | M1/M2 |
| OQ-015 | Provider selector precedence | explicit Task override→Bot policy→deployment default within security ceiling | M0/M2 |
| OQ-016 | Provider Host를 독립 crate로 즉시 분리할지 | 초기 명확한 module boundary 허용, independent test/reuse/dependency pressure 시 `dxb-provider-host` 분리 | M0 workspace ADR |
| OQ-017 | Provider SDK 최초 stable surface 범위 | contract re-export + config/error/lifecycle/telemetry/test helper 최소 surface | Phase 2 API review |
| OQ-018 | Capability Contract version 단위 | Capability별 semantic version + feature negotiation, Runtime global version과 분리 | Phase 1 freeze |
| OQ-019 | Provider-local transport retry 허용 범위 | semantic-invisible + idempotent/read-only + deadline 내 + Contract 명시 조건 | Phase 1 error ADR |
| OQ-020 | Lifecycle restart policy | Common-owned bounded policy, Provider self-restart로 old generation 재활성화 금지 | M0/M2 fault test |
| OQ-021 | ProviderCallContext 구체 타입 분할 | Capability별 typed context 우선, generic service lookup 금지 | Phase 1 API review |

## 3. P1 결정

| ID | 질문 | 권장 기본값 | Gate |
|---|---|---|---|
| OQ-101 | 자동 Task 분해 | Brain proposal + deterministic validation | M3 |
| OQ-102 | fairness algorithm | per-Bot round-robin + priority aging + stable tie-break | M3 load |
| OQ-103 | vector index | optional derived Provider | benchmark |
| OQ-104 | Bot capability discovery | projection catalog, authorization 아님 | M4 |
| OQ-105 | Bot transport | local durable DB first | M4 |
| OQ-106 | TUI framework | Control client only | M5 |
| OQ-107 | remote API | HTTP+SSE 후보, worker RPC 분리 | M5 |
| OQ-108 | policy DSL | typed config, arbitrary script 금지 | M2/M5 |
| OQ-109 | audit physical store | logical separate, embedded 가능 | M5 |
| OQ-110 | Artifact addressing | digest-based | M2 |
| OQ-111 | Plugin host isolation | process 또는 WASM 우선 검토, in-process native 별도 승인 | Plugin P1 |
| OQ-112 | Plugin package format/signing | manifest+digest P1, signature P2 | Plugin P1/P2 |
| OQ-113 | Long-term Memory thresholds | benchmark/config 기반, 문서 상수 금지 | M2/P1 |
| OQ-114 | Scaffold 구현 방식 | repository-owned deterministic template + 작은 Rust CLI, 과도한 template framework 금지 | Phase 3 spike |
| OQ-115 | Scaffold version pin 전략 | generated package에 Contract/SDK compatibility metadata 기록 | Phase 3 |
| OQ-116 | Remote Provider enforcement 위치 | authoritative selection/security/resource/deadline은 local Common Host, remote에는 scoped versioned envelope | P2 remote ADR |
| OQ-117 | Reference Provider 배치 | Capability owner/testkit 인접, production default와 분리 | Phase 2 workspace review |

## 4. P2 결정

- Web frontend 기술: API 독립 선택
- external DB: 계측된 병목 후
- remote worker protocol: CoreExecutor wire
- multi-tenancy: explicit tenant scope
- Bot ownership/sharding: BotId single-writer lease
- public Plugin SDK language/transport: stable wire, Rust ABI 직접 공개 금지
- cross-Bot shared Memory: explicit namespace+ACL
- HA/RPO/RTO: deployment tier별
- third-party Plugin/Provider registry/certification: signing/isolation/rollback maturity 후
- remote Provider credential delegation/attestation

## 5. v0.3에서 이미 고정된 항목

다음은 Open Question이 아니다.

- 기능 5개 Architecture Classification
- Feature Removal Contract 필요성
- DeepSeek 비필수 Provider 원칙
- Plugin/Provider 분리
- Rust `.rs` snake_case 예외 + non-Rust kebab-case
- production Provider call의 Provider Host 경유
- Provider lifecycle/activity/quiescence의 Common ownership
- Complete Capability Contract Package 필요성
- Thin Provider / Minimum Provider Surface
- Provider dependency firewall
- Provider SDK가 internal Runtime API를 공개하지 않는 원칙
- Executable Conformance + Host-path 검증
- 주요 Capability Reference Provider
- Common Contract change classification
- Tier A Common / Tier B Extension 분리와 Tier A escalation
- Waiting Continuation 원자 저장
- Side Effect write-ahead 의미
- shared limit/default SSOT

구현 세부가 바뀌어도 위 의미는 ADR/Compatibility 영향 없이 제거할 수 없다.

## 6. 결정 증거

use/non-use case, invariant, security/privacy, benchmark, memory, operational complexity, migration/rollback, alternatives, affected Provider inventory, Conformance/Reference impact, prototype, owner, ADR link를 사용한다.

## 7. 검증 기준

- P0 질문은 해당 Phase/Milestone exit 전에 ADR/실험 결과로 닫힌다.
- 질문 때문에 개발을 막지 않도록 reversible safe default가 있다.
- Tier B Provider 구현이 Open Question을 임의 결정하지 않는다.
- 비가역 Contract/SDK/Schema/remote enforcement 결정은 migration/rollback/compatibility 증거를 가진다.
