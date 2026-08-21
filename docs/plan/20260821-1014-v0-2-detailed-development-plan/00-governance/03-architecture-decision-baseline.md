---
title: "초기 아키텍처 결정 기준선"
document_id: "DXB-GOV-003"
version: "0.2.0"
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

## 3. ADR 문서 필수 항목

- Context / Decision / Alternatives
- Classification과 Owner
- Invariants
- Compatibility / Migration / Removal
- Security / Resource / Failure
- Rollout / Rollback
- Verification Evidence
- Supersedes / Superseded by

## 4. Naming 결정 상세

사용자 정의 경로는 lowercase kebab-case를 사용한다. Rust module 파일은 toolchain의 자연스러운 module resolution, IDE, formatter, lint와의 마찰을 줄이기 위해 `snake_case.rs`를 허용한다. `Cargo.toml`, `Cargo.lock`, `.github` 등 도구가 요구하는 고정 이름은 CI allowlist로 관리한다. kebab-case `.rs`를 강제하기 위한 `#[path]` 우회는 기본 금지한다.

## 5. Side Effect 결정 상세

외부 비멱등 동작은 `Intent/Key/Action Digest`를 Commit한 뒤 수행한다. 외부 성공 여부가 불명확하면 `Unknown`으로 복구하고 자동 Retry보다 reconciliation을 우선한다. 외부 서비스가 idempotency key와 status lookup을 제공하면 ledger가 이를 사용한다.

## 6. 검증 기준

- 각 P0 ADR 후보에 Architecture/Runtime Test가 연결된다.
- 결정 변경 시 Migration 또는 Removal 영향이 명시된다.
- 대안·rollback·검증 증거가 없는 비가역 결정은 Accepted로 승격하지 않는다.
