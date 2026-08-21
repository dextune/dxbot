---
title: "위험 등록부"
document_id: "DXB-DEL-062"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DEL-060", "DXB-ARC-017", "DXB-DOM-022", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-032", "DXB-RUN-037", "DXB-ENG-051"]
---

# 위험 등록부

## 1. 목적

v0.4의 R-001~049와 R-SPI-*를 유지하면서 Project/Channel/Generic Memory Scope/Collaboration의 신규 위험을 Acceptance/Test/OQ/Operational metric에 연결한다.

## 2. 기존 위험 유지

R-001~R-049와 R-SPI-001~010의 완화/Acceptance를 모두 유지한다. 특히 Session/Thread 혼합, Memory 성장, Core 상태 충돌, unbounded queue, stale cache 권한, Side Effect 중복, Provider Domain 침투, Waiting/Suspension recovery, limit drift, Thread growth, immutable redirect, Control starvation 위험은 v0.5에서도 직접 회귀 gate다.

## 3. v0.5 신규 Risk

| ID | 위험 | 영향 | 선행 신호 | 핵심 완화/Gate |
|---|---|---|---|---|
| **R-050** | Project/Channel을 Bot Identity/Brain과 혼합 | Critical | Project/Channel에 Brain/provider session/identity state 저장 | DOM-028/029 ownership, architecture test, AT-PROJECT-001 |
| **R-051** | Shared Memory를 participant Bot Global Memory에 복제 | Critical | Channel join 시 Bot Memory bulk copy | DOM-022 independent ScopeRef, AT-MEM-005/006 |
| **R-052** | Generic ScopeRef로 authorization boundary 약화 | Critical | scope ID만 있으면 recall 허용, final auth 없음 | RUN-032 + canonical final filter, AT-MEM-005/SEC-002 |
| **R-053** | Channel fan-out으로 Execution 폭증 | Critical | member 수와 Execution 수가 선형 동기 증가 | RUN-037/031 bounded routing, AT-CHANNEL-002 |
| **R-054** | Role label이 Runtime Authority로 사용됨 | Critical | `role == manager` permission check | DOM-029 Role/Authority split, AT-CHANNEL-003 |
| **R-055** | stale Membership/cache/index에서 정보/권한 누출 | Critical | revoke 후 cached Memory/control 성공 | generation fencing + final auth, AT-SEC-002 |
| **R-056** | Thread 일반화 중 v0.4 Main Conversation semantic 회귀 | Critical | 모든 Thread에 Project/Channel parent 강제, new ThreadId migration | DOM-027 additive ParentRef, AT-MIG-001 |
| **R-057** | Project/Channel cache로 RSS 선형 증가 | High | cache retained bytes가 total scope state와 동기 증가 | ENG-051 byte cap/shared prefix/soak |
| **R-058** | Manager가 다른 Bot Brain/Core/Memory 직접 조작 | Critical | worker handle/core assignment/mutable memory API | DOM-023/025 + RUN-037, AT-COLLAB-001 |
| **R-059** | Scope promotion conflict/duplicate knowledge explosion | High | simultaneous promotions create repeated near-duplicate records | revision pin/dedup/conflict, AT-MEM-006 |
| **R-060** | migration 중 Thread/Memory ownership provenance 손실 | Critical | new ThreadId/Memory rewrite/provider-session mapping | ENG-053, AT-MIG-001 |

## 4. R-050 / R-058 — Intelligence Ownership

Project/Channel/Coordinator는 intelligence owner가 아니다. 모든 reasoning은 Persistent Bot Brain에서 수행되고 execution authority는 Scheduler/Task/Control owner에 남긴다.

탐지:
- Channel struct에 provider/session/brain mutable state
- Coordinator 직접 model call
- Manager direct core allocation/worker kill

## 5. R-051 / R-059 — Shared Knowledge

Shared Scope는 Bot Memory union/copy가 아니다. promotion은 source revision을 pin하고 target Scope에 explicit relation을 만든다. join/leave/revoke가 Bot Global Memory bulk mutation을 유발하지 않는다.

## 6. R-052 / R-055 — Authorization

index/cache/routing snapshot은 candidate/optimization이다. `candidate → Canonical fetch → FINAL current authorization`을 필수화한다. membership/authority generation을 cache key와 mutation precondition에 포함한다.

## 7. R-053 / R-057 — Resource

member/Channel/Project 수와 activation/RSS가 선형 결합되지 않게 entries+bytes cap, activation cap, response concurrency, cold history, shared immutable prefix, admission/fairness를 사용한다.

## 8. R-056 / R-060 — Compatibility

Project/Channel은 additive다. v0.4 Bot-only Main Conversation/Thread/Memory ID와 provenance를 그대로 보존한다. migration이 새 ID를 발급하거나 Provider Session을 identity source로 사용하면 release blocker다.

## 9. Phase Blocker

- M0: R-050, R-052, R-054, R-056, R-060
- M1: R-050, R-055, R-056
- M2: R-051, R-052, R-055, R-059, R-060
- M3: R-053, R-057
- M4: R-058, R-051, R-054
- M5/M6: stale projection/Role UI가 authority로 오표현되는 위험 포함

## 10. 검증 기준

- R-050~060 모두 Acceptance/Test/OQ/Metric 중 하나 이상에 연결.
- Critical risk owner 없이 Milestone 종료 금지.
- 기존 R-001~049/R-SPI mitigation regression 0.
