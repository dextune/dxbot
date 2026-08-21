---
title: "위험 등록부"
document_id: "DXB-DEL-062"
version: "0.1.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DEL-060", "DXB-RUN-032", "DXB-ENG-051"]
---


# 위험 등록부

## 1. 목적

DXBOT의 제품 의미, 기술, 보안, 운영, 성능, 외부 의존 위험을 사전에 명시하고 탐지 신호·완화·비상 대응을 소유한다.

## 2. 책임 범위

- risk ID/owner
- likelihood/impact
- leading indicator
- 예방/완화
- contingency
- review gate

실제 담당자 이름은 팀 구성 후 채운다.

## 3. 평가 기준

- 가능성: Low / Medium / High
- 영향: Low / Medium / High / Critical
- 우선도: 가능성과 영향, 탐지 난이도를 함께 판단
- 상태: Open / Mitigating / Accepted / Closed

## 4. 핵심 위험

| ID | 위험 | 가능성 | 영향 | 선행 신호 | 핵심 완화 |
|---|---|---|---|---|---|
| R-001 | Session과 Bot 의미 재혼합 | M | Critical | UI/DB가 session owner로 설계 | 타입/AT-BOT-001/의존성 gate |
| R-002 | Core가 사실상 복제 Agent가 됨 | H | Critical | Core별 Persona/Memory store | Lease 계약/AT-CORE-002 |
| R-003 | DeepSeek preview 변경 종속 | H | High | adapter diff가 domain에 전파 | pin/adapter/conformance/canary |
| R-004 | Memory 무한 성장 | H | Critical | bytes/retention/compaction 부재 | quota/tiering/compaction/forget |
| R-005 | Core 병렬화로 상태 충돌 | H | High | shared mutable scratchpad | snapshot/proposal/revision/fencing |
| R-006 | 무제한 queue로 OOM | M | Critical | unbounded channel/lag 증가 | CI 금지/byte cap/admission |
| R-007 | Bot 간 delegation cycle/비용 폭발 | M | High | hop/fan-out 증가 | depth/visited/budget/idempotency |
| R-008 | 권한 confused deputy | M | Critical | source grant 자동 승계 | target reauth/delegation token |
| R-009 | Prompt injection→민감 Tool | H | Critical | untrusted content와 권한 결합 | data labels/approval/deterministic guard |
| R-010 | Secret가 prompt/log/env로 유출 | M | Critical | raw config/env 전달 | SecretRef/scrub/redaction canary |
| R-011 | Sandbox가 filesystem만 통제 | M | Critical | network/process ambient access | capability별 enforcement/fail-closed |
| R-012 | Event/Projection 이중 원본 | M | High | UI가 projection 직접 수정 | canonical owner/rebuild tests |
| R-013 | Event sourcing 과복잡성 | M | High | 모든 상세를 event로 저장 | hybrid/current state/artifact 분리 |
| R-014 | Embedded DB writer 병목 | M | High | lock p99/queue 증가 | bot serialization/batch/scale trigger |
| R-015 | 비결정 모델로 CI flaky | H | Medium | actual model core gate | deterministic Fake/contract assertions |
| R-016 | Rust 과추상화/crate 폭발 | M | High | trait/repository/helper 증가 | split criteria/architecture review |
| R-017 | 과도한 clone/Arc로 RSS 증가 | H | High | retained bytes 증가 | allocation profile/owner map |
| R-018 | 종료 시 orphan process/task | M | Critical | child count/permit leak | structured supervision/quiescence tests |
| R-019 | Migration으로 장기 Bot 손상 | M | Critical | old fixture 미지원 | version ledger/backup/replay/canary |
| R-020 | Web에 Domain 로직 중복 | M | High | UI reducer가 state transition 결정 | shared API/server authority |
| R-021 | 관측 데이터가 민감 정보 저장 | M | Critical | raw prompt/tool logs | classification/redaction/retention |
| R-022 | Control Plane 단일 장애점 | M | High | API down이 Core 중단 | runtime independence/durable operations |
| R-023 | 분산화를 너무 일찍 도입 | H | High | microservice/transport 선행 | M7 entry gate/단일 노드 완성 |
| R-024 | 비용 accounting 부정확 | M | High | estimate/invoice 차이 | reservation/usage reconciliation |
| R-025 | Cache invalidation으로 오래된 권한 사용 | M | Critical | policy key 누락 | revisioned keys/security test |
| R-026 | Artifact dangling/orphan | M | Medium | DB/content mismatch | two-phase finalize/sweeper/digest |
| R-027 | Provider 오류 표현 불일치 | H | High | throw/stream/exit 혼재 | Adapter normalization/conformance |
| R-028 | 사용자 forget가 index/backup에 미반영 | M | Critical | tombstone 후 recall | query filter/delete ledger/retention |
| R-029 | Task side effect 중복 | M | Critical | retry after unknown result | idempotency/side-effect ledger/reconcile |
| R-030 | 프로젝트 범위 과대 | H | High | P2 UI/분산이 P0 침범 | milestone gate/vertical slice |

## 5. 상위 위험 상세 대응

### R-003 — Harness 종속
- 예방: DXBOT Harness Port, upstream type leakage test
- 탐지: adapter 변경 LOC와 domain compile diff
- contingency: Fake/native minimal provider로 운영
- trigger: protocol incompatibility 또는 breaking upgrade
- gate: compatibility matrix/canary 없이는 기본 provider 승격 금지

### R-004 — Memory 성장
- 예방: byte quota, retention, Working Memory 분리
- 탐지: Bot별 bytes/revision/unused age
- contingency: compaction/cold tier/admission restriction
- 금지: 임의 자동 삭제로 문제 은폐

### R-009 — Prompt Injection
- 예방: untrusted labels, 권한 분리, high-risk approval
- 탐지: injection regression suite, denied action signals
- contingency: Bot/provider quarantine, grant revoke, trace preserve
- gate: sensitive Tool security suite

### R-014 — Storage 병목
- 예방: narrow transaction, current state, background projection
- 탐지: DB busy, append p99, queue fill
- contingency: write batching, projection 분리, external store ADR
- trigger: 정의된 workload에서 예산 지속 초과

### R-016 — 과추상화
- 예방: 실제 seam 기준, 초기 crate 최소화
- 탐지: trait impl 1개, pass-through layers, compile time
- contingency: crate/module 병합, dead abstraction 제거
- gate: 새 crate/trait의 변형 축 설명

## 6. 위험 데이터 흐름

관측 지표/incident/test failure → Risk indicator 업데이트 → owner review → mitigation issue/ADR → 검증 → residual risk 승인/종료.

보안 incident는 일반 backlog보다 우선하고 증거 보존과 노출 범위를 통제한다.

## 7. 예외상황

- 수치 없는 "낮은 위험" 평가는 Accepted 근거가 되지 않는다.
- 외부 프로젝트의 인기/라이선스가 production readiness를 보증하지 않는다.
- Risk를 feature flag로 숨겨도 기본 profile에서 노출 가능하면 Open 상태다.
- mitigation이 새 위험을 만들면 별도 ID로 등록한다.
- owner가 없는 Critical risk는 release blocker다.

## 8. 확장성

분산/multi-tenant 단계에서 split-brain, tenant escape, data residency, fleet upgrade, network partition 위험을 추가한다. Risk schema는 Control Plane에 Projection될 수 있다.

## 9. 구현 우선순위

- **P0:** R-001~019, R-023~030에 owner/metric/test 연결
- **P1:** 운영 SLO와 incident playbook
- **P2:** distributed/multi-tenant risk
- **P3:** quantitative risk forecasting

## 10. 검증 기준

- Critical risk마다 자동 테스트 또는 운영 control이 있다.
- risk indicator가 metrics/audit/CI 중 하나에 연결된다.
- release candidate에 Open Critical ownerless risk가 없다.
- Harness, Memory, Security, Migration risk가 각 milestone gate에 반영된다.
