---
title: "위험 등록부"
document_id: "DXB-DEL-062"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DEL-060", "DXB-ARC-017", "DXB-DOM-027", "DXB-RUN-032", "DXB-RUN-036", "DXB-ENG-051"]
---

# 위험 등록부

## 1. 목적

기존 제품/Provider Framework 위험을 보존하고 Persistent Conversation/Thread/Live Control로 새로 생기는 semantic, concurrency, resource, recovery 위험을 선행 신호·완화·Acceptance에 연결한다.

## 2. 기존 R-001~R-039 유지

v0.3의 R-001~R-039는 모두 유효하다. 특히 다음과 직접 연결한다.

| ID | 기존 위험 | v0.4 연계 |
|---|---|---|
| R-001 | Session/Bot 의미 재혼합 | R-040 Session/Thread 재혼합으로 확장 |
| R-004 | Memory 무한 성장 | R-041 transcript/Thread growth와 별도 계측 |
| R-005 | Core 병렬 상태 충돌 | R-047/AT-CTRL-005 Supervisor command race 포함 |
| R-006 | unbounded queue OOM | R-043 Control Channel 포함 |
| R-012 | Event/Projection 이중 원본 | Directive/Thread Projection도 Derived 유지 |
| R-017 | clone/retained RSS 증가 | long transcript/context cache와 연결 |
| R-018 | shutdown orphan resource | suspended/control/provider activity cleanup |
| R-020 | Web Domain logic 중복 | Thread/control routing도 shared API만 사용 |
| R-022 | Control Plane 장애 전파 | Runtime Control semantics와 UI 분리 |
| R-025 | stale cache 권한 | Thread local Memory/history permission cache 포함 |
| R-029 | Task Side Effect 중복 | R-044 interrupt/Side Effect 충돌과 직접 연결 |
| R-031 | concrete Provider Domain 침투 | Provider Session/steering도 Domain owner 금지 |
| R-035 | Waiting Continuation 유실 | Suspension checkpoint에도 동일 crash-safety 적용 |
| R-038 | limit/default drift | control/context/history limits Policy SSOT |

나머지 R-002~003, R-007~011, R-013~016, R-019, R-021, R-023~024, R-026~028, R-030, R-032~034, R-036~037, R-039도 기존 완화/Acceptance를 그대로 유지한다.

## 3. v0.3 SPI Risk 유지

R-SPI-001~010(느슨한 Capability 계약, Provider Runtime 접근, retry/security/resource 중복, mega-interface, service locator, error drift, Conformance/Host 불일치, Scaffold drift, Common 계약 오염, Tier B semantic 수정)을 모두 유지한다.

Provider native session/steering 요구가 R-SPI-002/005/009/010을 재도입하지 않는지 특히 검사한다.

## 4. v0.4 신규 위험

| ID | 위험 | 영향 | 선행 신호 | 핵심 완화 / Gate |
|---|---|---|---|---|
| **R-040** | Thread와 Session 의미 재혼합 | Critical | UI tab/session/provider conversation ID가 ThreadId로 사용 | Glossary/Domain type/API gate, AT-CONV-001, AT-SESSION-002 |
| **R-041** | Main Conversation/Thread transcript 무한 hot-memory 성장 | Critical | history 전체 load/context 삽입, RSS가 transcript와 선형 증가 | pagination/cold archive/bounded Context/cache, AT-CTX-002, perf soak |
| **R-042** | Redirect가 running Execution snapshot을 mutation | Critical | prompt/config/provider binding in-place 수정 | Task Spec Revision + Directive + new Execution, AT-CTRL-003 |
| **R-043** | Control command가 Work Queue에서 starvation | Critical | report/cancel이 long work 뒤에서 대기 | separate bounded Control Channel + reserve/fairness, AT-CTRL-002 |
| **R-044** | Interrupt와 Side Effect 충돌로 외부 동작 중복 | Critical | suspend/cancel 후 same external call retry | Side Effect Ledger + safe point + Unknown reconcile, AT-SFX-001/AT-CTRL-004 |
| **R-045** | Thread-local Memory가 Bot-global Memory 오염 | High | failed hypothesis/temporary info가 global recall에 등장 | scoped Memory + PromotionProposal, AT-MEM-004 |
| **R-046** | Thread branch/lineage 무제한 성장 | High | branch storm/depth/node 증가, graph query 폭증 | bounded policy/metrics/archive/compaction, OQ/benchmark |
| **R-047** | 여러 Interface Session/Supervisor command 충돌 | Critical | redirect/cancel/suspend가 arrival order로 덮어씀 | expected revision/idempotency/precedence/audit, concurrency suite |
| **R-048** | 외부 Provider에 Hard Real-Time preemption을 잘못 가정 | High | “cancel accepted=provider stopped” UI, stuck call hidden | cooperative safe point/feature negotiation/freshness, AT-CTRL-003 |
| **R-049** | 직접 Core 조작으로 Scheduler ownership 붕괴 | Critical | control API에 core_count/set_worker/kill-worker mutation | priority/parallelism hint + Scheduler authority, architecture gate |

## 5. R-040 — Session / Thread 의미 재혼합

### 탐지
- `session_id`와 `thread_id` alias/type conversion
- browser tab opening 시 Thread auto-create
- Provider Session resume failure 시 new Thread 생성

### 완화
명시적 newtype/schema, `DXB-DOM-027`, migration negative fixture, ambiguous bare Session lint/architecture review.

## 6. R-041 / R-046 — 장기 성장

Conversation raw storage growth와 Runtime hot-memory growth, Thread lineage growth를 별도 metric으로 본다. Canonical history 삭제를 RSS pressure의 즉시 대응으로 사용하지 않는다. exact retention/archive/compaction limit은 Policy/OQ에서 결정한다.

## 7. R-042 / R-044 — Redirect / Side Effect

immutable Execution과 write-ahead Side Effect는 live responsiveness보다 우선한다. redirect/suspend가 active Provider call을 즉시 끊지 못하더라도 safe state가 확보되기 전 same semantic effect를 재실행하지 않는다.

## 8. R-043 / R-047 — Control Path

별도 Control Channel만으로 충분하지 않다. queue cap, per-Bot/principal fairness, expected revision, idempotency, security/safety precedence, coalescing/supersede semantics를 함께 검증한다.

## 9. R-045 — Memory Scope Pollution

Thread-local→global promotion rate, rejection/conflict를 관측하고 random/failed hypothesis fixture로 global recall pollution test를 유지한다.

## 10. R-048 / R-049 — Runtime Ownership

Provider capability는 cancellation/yield/steering을 “지원/미지원”으로 노출할 뿐 execution authority가 아니다. Core Lease는 Scheduler만 발급/회수하며 user control은 hint/policy revision으로 입력된다.

## 11. Phase Blocker

- M1 blocker: R-040, R-041의 structural foundation
- M2 blocker: R-042, R-044, R-045, R-048
- M3 blocker: R-043, R-047, R-049
- M5/M6 blocker: Session/Thread UI 재혼합, stale report 오표현, direct worker control

Critical risk에 owner 없는 상태로 해당 Milestone을 종료하지 않는다.

## 12. 검증 기준

- R-040~049 모두 Acceptance/Test/OQ 또는 operational control에 연결됨.
- R-001/R-005/R-029와 신규 risk cross-link 유지.
- Provider Framework R-SPI-001~010 완화가 v0.4에서 회귀하지 않음.
