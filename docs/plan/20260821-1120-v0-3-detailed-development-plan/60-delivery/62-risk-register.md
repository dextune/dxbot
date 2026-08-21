---
title: "위험 등록부"
document_id: "DXB-DEL-062"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DEL-060", "DXB-ARC-017", "DXB-RUN-032", "DXB-ENG-051"]
---

# 위험 등록부

## 1. 목적

제품 의미, 모듈성, Common Framework/SPI, 보안, 복구, 성능, 외부 의존 위험을 선행 신호·완화·Acceptance와 연결한다.

## 2. 기존 핵심 위험 유지

| ID | 위험 | 영향 | 핵심 완화 |
|---|---|---|---|
| R-001 | Session/Bot 의미 재혼합 | Critical | type/AT-BOT-001/dependency gate |
| R-002 | Core가 복제 Agent가 됨 | Critical | Lease/AT-CORE-002 |
| R-003 | DeepSeek 종속 | High | optional Provider/pin/conformance/removal test |
| R-004 | Memory 무한 성장 | Critical | growth metrics/retention/archive/forget |
| R-005 | Core 병렬 상태 충돌 | High | snapshot/revision/fencing |
| R-006 | unbounded queue OOM | Critical | CI byte-cap/admission |
| R-007 | delegation cycle/비용 폭발 | High | depth/fan-out/budget |
| R-008 | confused deputy | Critical | target reauth/delegation token |
| R-009 | prompt injection→민감 Tool | Critical | trust label/approval/guard |
| R-010 | secret prompt/log/env 유출 | Critical | SecretRef/scrub/redaction |
| R-011 | Sandbox 불완전 | Critical | capability enforcement/fail-closed |
| R-012 | Event/Projection 이중 원본 | High | Canonical owner/rebuild |
| R-013 | Event sourcing 과복잡 | High | Hybrid/current state/artifact |
| R-014 | Embedded DB writer 병목 | High | narrow transaction/scale trigger |
| R-015 | model nondeterminism CI flaky | Medium | Fake/Reference core gate |
| R-016 | Rust 과추상/crate 폭발 | High | seam/split criteria |
| R-017 | clone/retained RSS 증가 | High | ownership/allocation profile |
| R-018 | shutdown orphan resource | Critical | structured quiescence |
| R-019 | Migration으로 Bot 손상 | Critical | fixture/backup/replay |
| R-020 | Web Domain logic 중복 | High | shared Control API |
| R-021 | observability 민감정보 | Critical | redaction/retention |
| R-022 | Control Plane 장애 전파 | High | runtime independence |
| R-023 | 조기 분산화 | High | M7 entry gate |
| R-024 | 비용 accounting 오류 | High | reservation/reconciliation |
| R-025 | stale cache 권한 | Critical | revisioned key/security test |
| R-026 | Artifact orphan | Medium | two-phase/sweeper/digest |
| R-027 | Provider error 불일치 | High | typed normalize/conformance |
| R-028 | forget 미전파 | Critical | tombstone/delete ledger |
| R-029 | Task side effect 중복 | Critical | write-ahead/idempotency/reconcile |
| R-030 | scope 과대 | High | milestone gate |

## 3. 기존 v0.2 추가 위험

| ID | 위험 | 영향 | 선행 신호 | 핵심 완화 |
|---|---|---|---|---|
| R-031 | concrete Provider가 Domain/Application에 침투 | Critical | provider type/import 증가 | dependency gate, AT-MOD/AT-SPI-005 |
| R-032 | Plugin과 internal module/Provider 혼동 | High | internal trait ABI 공개 | ARC-016, SDK/wire/Host boundary |
| R-033 | Feature 제거 후 stale data/config/schema | High | removed ID/config 잔존 | Removal Contract, AT-SPI-010 |
| R-034 | Routine Canonical Owner 부재/중복 trigger | High | UI/scheduler별 schedule state | Routine owner/occurrence dedup/AT-ROUTINE-001 |
| R-035 | Waiting Continuation 유실 | Critical | Waiting row만 존재 | atomic Waiting+Continuation/AT-TASK-003 |
| R-036 | Side Effect Intent write-ahead 미보장 | Critical | mutate 후 ledger 없음 | Host guard/AT-SFX-001 |
| R-037 | Repository naming/layout drift | Medium | mixed case/path workaround | ENG-054/AT-REPO-001 |
| R-038 | Limit/default 중복으로 환경별 drift | High | 동일 숫자 여러 문서/코드 | Policy SSOT/AT-POL-001 |
| R-039 | Plugin data uninstall로 Core data 손상 | Critical | plugin cleanup가 shared table 접근 | data namespace/AT-PLUGIN-001 |

## 4. v0.3 SPI 신규 위험

| ID | 위험 | 영향 | 선행 신호 | 핵심 완화 / Gate |
|---|---|---|---|---|
| R-SPI-001 | Capability 계약이 지나치게 느슨함 | Critical | Provider마다 null/error/cancel 의미 상이 | Complete Contract, AT-SPI-001/007 |
| R-SPI-002 | Provider가 Runtime 내부에 접근 | Critical | RuntimeContext/DB/Scheduler import | Minimum Surface + Firewall, AT-SPI-004/005 |
| R-SPI-003 | Provider별 retry/security/resource 중복 | Critical | 각 Provider에 retry/permission/admission 코드 | Host ownership, AT-SPI-003/004 |
| R-SPI-004 | Capability 계약이 mega-interface로 성장 | High | unrelated Provider가 불필요 method 구현 | 작은 seam/contract review, AT-SPI-001 |
| R-SPI-005 | Common Context가 service locator가 됨 | Critical | generic get/service map/ambient handles | typed context, AT-SPI-004 |
| R-SPI-006 | Provider별 error semantics drift | High | 문자열 parsing/retry class 차이 | stable taxonomy + Conformance, AT-SPI-007 |
| R-SPI-007 | Conformance와 실제 Runtime 경로 불일치 | Critical | direct SPI green, Host E2E fail | Host-path suite, AT-SPI-003/007 |
| R-SPI-008 | Scaffold가 오래되어 잘못된 구현 유도 | High | generated code가 deprecated API 사용 | template drift test, AT-SPI-006 |
| R-SPI-009 | Extension 편의를 위해 Common 계약 오염 | Critical | 한 Provider 요구로 public Contract 확장 | change classification/Tier A, AT-SPI-008 |
| R-SPI-010 | Tier B Agent가 Contract를 임의 수정 | Critical | Provider PR에 Common semantic diff | task template + classification + escalation, AT-SPI-008 |

## 5. 상위 기존 위험 대응

### R-031 Provider 침투
탐지: `cargo metadata`, import/API scan. 대응: Stable Capability + SDK/Firewall + Provider-free build. Gate: AT-SPI-004/005/010, AT-MOD-001/002.

### R-035 Continuation 유실
탐지: doctor의 Waiting without Continuation. 대응: same Unit of Work, checksum/revision/failpoint. Gate: AT-TASK-003.

### R-036 Side Effect 중복
탐지: external call 전에 ledger entry 없는 trace. 대응: write-ahead orchestration, Host guard, Unknown/Reconciliation. Gate: AT-SFX-001/AT-SPI-003.

### R-039 Plugin data 손상
탐지: Plugin uninstall이 Core schema/table을 요구. 대응: plugin-owned namespace, explicit Domain Command, backup/approval. Gate: AT-PLUGIN-001.

## 6. 상위 SPI 위험 대응

### R-SPI-001 — 느슨한 계약
Provider별 예외 문서나 서로 다른 cancel/terminal semantics를 탐지한다. Request~Conformance를 Capability owner가 하나의 Package로 정의한다. Gate: AT-SPI-001/007.

### R-SPI-002 / R-SPI-005 — 내부 접근과 Service Locator
Provider dependency/API scan으로 탐지한다. typed minimum context와 SDK allowlist를 사용하고 generic service lookup/raw DB/Runtime handle을 금지한다. Gate: AT-SPI-004/005.

### R-SPI-003 — Common 책임 중복
Provider별 retry scheduler, permission cache, resource queue, telemetry subsystem을 탐지한다. Mandatory Host로 중앙화하고 provider-local transport retry는 Contract 허용 범위로 제한한다. Gate: AT-SPI-003/004.

### R-SPI-007 — Conformance 경로 불일치
Direct Provider test만 존재하는지 탐지한다. suite 일부를 반드시 `Consumer→Host→Provider`로 실행하고 negative fixture를 둔다. Gate: AT-SPI-007/009.

### R-SPI-009 / R-SPI-010 — Contract 오염
Provider 구현 PR이 Capability/Host/SDK semantic을 함께 바꾸는지 탐지한다. change classification, affected Provider inventory, Tier A escalation을 강제한다. Gate: AT-SPI-008.

## 7. Risk와 개발 단계 연결

- Phase 1 blocker: R-SPI-001~007, R-SPI-009~010
- Phase 2 blocker: R-SPI-006~007 + Reference/Conformance false-positive
- Phase 3 blocker: R-SPI-008
- Phase 4: remote/plugin boundary가 R-SPI-002/003/005/007을 재도입하지 않는지 재검증

## 8. 검증 기준

- 모든 기존 Critical risk 완화가 유지된다.
- R-SPI-001~010에 자동 test/architecture gate 또는 명시 operational control이 있다.
- owner 없는 Open Critical risk는 Phase/release blocker다.
- Risk ID가 milestone/Acceptance와 연결된다.
