---
title: "위험 등록부"
document_id: "DXB-DEL-062"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DEL-060", "DXB-RUN-032", "DXB-ENG-051"]
---

# 위험 등록부

## 1. 목적

제품 의미, 모듈성, 보안, 복구, 성능, 외부 의존 위험을 선행 신호·완화·Acceptance와 연결한다.

## 2. 기존 핵심 위험 유지

| ID | 위험 | 영향 | 핵심 완화 |
|---|---|---|---|
| R-001 | Session/Bot 의미 재혼합 | Critical | type/AT-BOT-001/dependency gate |
| R-002 | Core가 복제 Agent가 됨 | Critical | Lease/AT-CORE-002 |
| R-003 | DeepSeek preview 종속 | High | optional Provider/pin/conformance/removal test |
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
| R-015 | model nondeterminism CI flaky | Medium | Fake core gate |
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
| R-027 | Provider error 불일치 | High | Adapter normalize/conformance |
| R-028 | forget 미전파 | Critical | tombstone/delete ledger |
| R-029 | Task side effect 중복 | Critical | write-ahead/idempotency/reconcile |
| R-030 | scope 과대 | High | milestone gate |

## 3. v0.2 신규 위험

| ID | 위험 | 영향 | 선행 신호 | 핵심 완화 |
|---|---|---|---|---|
| R-031 | concrete Provider가 Domain/Application에 침투 | Critical | provider type/import 증가 | dependency gate, AT-MOD-001/002 |
| R-032 | Plugin과 internal module/Provider 혼동 | High | internal trait ABI 공개 | ARC-016, SDK/wire boundary |
| R-033 | Feature 제거 후 stale data/config/schema | High | removed ID/config 잔존 | Removal Contract, AT-MOD-002 |
| R-034 | Routine Canonical Owner 부재/중복 trigger | High | UI/scheduler별 schedule state | Routine owner/occurrence dedup/AT-ROUTINE-001 |
| R-035 | Waiting Continuation 유실 | Critical | Waiting row만 존재 | atomic Waiting+Continuation/AT-TASK-003 |
| R-036 | Side Effect Intent write-ahead 미보장 | Critical | mutate 후 ledger 없음 | AT-SFX-001/failpoint |
| R-037 | Repository naming/layout drift | Medium | mixed case/path workaround | ENG-054/AT-REPO-001 |
| R-038 | Limit/default 중복으로 환경별 drift | High | 동일 숫자 여러 문서/코드 | Policy SSOT/AT-POL-001 |
| R-039 | Plugin data uninstall로 Core data 손상 | Critical | plugin cleanup가 shared table 접근 | data namespace/AT-PLUGIN-001 |

## 4. 상위 위험 대응

### R-031 Provider 침투
탐지: `cargo metadata`, import/API scan. 대응: stable Capability contract, Provider-free build. Gate: AT-MOD-001/002.

### R-035 Continuation 유실
탐지: doctor가 Waiting without Continuation 검사. 대응: 같은 Unit of Work, checksum/revision, failpoint. Gate: AT-TASK-003.

### R-036 Side Effect 중복
탐지: external call 전에 ledger entry 없는 trace. 대응: write-ahead orchestration, Unknown state, provider status lookup. Gate: AT-SFX-001.

### R-039 Plugin data 손상
탐지: Plugin migration/uninstall이 Core schema/table을 요구. 대응: plugin-owned namespace, explicit Domain Command, backup/approval. Gate: AT-PLUGIN-001.

## 5. 검증 기준

- Critical risk마다 자동 test 또는 operational control이 있다.
- owner 없는 Open Critical risk는 release blocker다.
- Risk ID가 milestone/Acceptance와 연결된다.
