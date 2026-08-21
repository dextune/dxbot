---
title: "미결정 사항과 권장 기본값"
document_id: "DXB-DEL-063"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-GOV-003", "DXB-DEL-060"]
---

# 미결정 사항과 권장 기본값

## 1. 목적

비가역 결정을 암묵 구현하지 않도록 질문, 권장 기본값, 결정 증거, ADR Gate를 관리한다.

## 2. P0 결정

| ID | 질문 | 권장 기본값 | Gate |
|---|---|---|---|
| OQ-001 | DeepSeek 실제 채택 범위 | Harness Contract+Fake 우선, DeepSeek optional Provider | M0 |
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
| OQ-012 | actual model release gate | canary, core gate Fake | M2 |
| OQ-013 | Side Effect exact state enum | Prepared/Confirmed/Failed/Unknown/Reconciled semantics 고정, 세부 enum ADR | M2 schema |
| OQ-014 | Routine missed/overlap policy set | bounded explicit enum, arbitrary script 금지 | M1/M2 |
| OQ-015 | Provider selector precedence | explicit Task override→Bot policy→deployment default within security ceiling | M0/M2 |

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
| OQ-111 | Plugin host isolation | process 또는 WASM 우선 검토, in-process native는 별도 승인 | Plugin P1 |
| OQ-112 | Plugin package format/signing | manifest+digest P1, signature P2 | Plugin P1/P2 |
| OQ-113 | Long-term Memory thresholds | benchmark/config 기반, 문서 상수 금지 | M2/P1 |

## 4. P2 결정

- Web frontend 기술: API 독립 선택
- external DB: 계측된 병목 후
- remote worker protocol: CoreExecutor wire
- multi-tenancy: explicit tenant scope
- Bot ownership/sharding: BotId single-writer lease
- public Plugin SDK language/transport: stable wire, Rust ABI 직접 공개 금지
- cross-Bot shared Memory: explicit namespace+ACL
- HA/RPO/RTO: deployment tier별
- third-party Plugin registry/marketplace: signing/isolation/rollback maturity 후

## 5. 이미 v0.2에서 기준으로 고정한 항목

다음은 더 이상 Open Question으로 취급하지 않는다.

- 기능 5개 분류
- Feature Removal Contract 필요성
- DeepSeek 비필수 Provider 원칙
- Plugin/Provider 분리
- Rust `.rs` snake_case 예외 + non-Rust kebab-case
- Waiting Continuation 원자 저장
- Side Effect write-ahead 의미
- shared limit/default SSOT 원칙

구현 세부가 바뀌어도 위 의미는 ADR 없이 제거할 수 없다.

## 6. 결정 증거

use/non-use case, invariant, security/privacy, benchmark, memory, operational complexity, migration/rollback, alternatives, prototype/conformance, owner, ADR link를 사용한다.

## 7. 검증 기준

- P0 질문은 해당 milestone exit 전에 ADR/실험 결과로 닫힌다.
- 질문 때문에 개발을 막지 않도록 reversible safe default가 있다.
- 비가역 schema/protocol/Plugin host 결정은 migration/rollback 증거를 가진다.
