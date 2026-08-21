---
title: "테스트·검증 전략"
document_id: "DXB-ENG-052"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-013", "DXB-ARC-016", "DXB-ARC-017", "DXB-DOM-022", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-036", "DXB-RUN-037", "DXB-ENG-050"]
---

# 테스트·검증 전략

## 1. 목적

v0.4의 deterministic state/concurrency/recovery/Provider Framework 검증을 유지하고 Project/Channel/Scope-Aware Memory/Collaboration의 isolation, authorization, boundedness, migration을 자동화 가능한 evidence로 검증한다.

## 2. Test Layer

Unit, Property/Model, Concurrency Model, Component, Contract/Conformance, Host Integration, E2E, Fault/Recovery, Performance, Soak, Security, Architecture/Docs를 유지한다. Core correctness는 deterministic fake/reference Provider를 사용한다.

## 3. 신규 Deterministic Fixture

- Project/Project Membership fixture
- Channel/Membership/Role/Authority generation fixture
- Channel Conversation/Thread Parent fixture
- scoped Memory store/index with stale candidate injection
- Memory promotion/dedup/conflict fixture
- Channel Coordinator probe with bounded routing/fan-out
- membership revoke barrier during queue/Execution
- Manager/Researcher/Reviewer durable delegation fixture
- Presence projection rebuild fixture
- v0.4→v0.5 migration fixture preserving IDs/revisions

기존 virtual clock/ID/RNG/failpoint/Scheduler/Provider Session loss/Control safe-point fixture를 재사용한다.

## 4. Domain / Property Test

- Project/Channel identity persistence and isolation
- Project Membership ≠ Channel Membership
- Role ≠ Authority
- Bot Identity unaffected by Channel Role
- Memory ScopeRef exactly one owner scope
- Shared Memory commit does not mutate Bot Global Memory
- Thread Parent Bot/Channel relation without ThreadId replacement
- promotion source revision/provenance preservation
- SupervisorRef separate from Role

## 5. Scope-Aware Context Test

AT-CTX-003:
1. Project/Channel/Thread history와 Memory를 정책 budget보다 크게 생성
2. authorized Bot/Project/Channel/Thread candidate 구성
3. stale/unauthorized index candidate 주입
4. canonical fetch 후 final authorization 확인
5. deterministic selection/token+byte cap 확인
6. selected revision/digest pin 확인
7. Provider Session 유무로 semantic이 변하지 않음 확인

## 6. Revocation Concurrency Suite

barrier/fake clock으로 다음 interleaving을 실제 재현한다.
- membership revoke vs routing dispatch
- revoke vs Memory recall/write
- authority downgrade vs Manager redirect
- revoke vs Context creation
- revoke vs high-risk Side Effect preflight
- Channel archive vs running Task
- Supervisor change vs redirect
- promotion vs source update
- simultaneous promotion
- Manager redirect vs Research completion

sleep timing으로 winner를 맞추지 않는다.

## 7. Bounded Channel Routing

AT-CHANNEL-002:
- member 1/10/100 fixture
- 일반 message, explicit @bot, @role, manager-first, debate request 실행
- selected participant count가 policy cap을 넘지 않음
- inbound/fan-out/response queue item+byte cap 확인
- overloaded case가 explicit reject/defer/coalesce 의미를 가짐
- unrelated Channel/Bot starvation 없음

## 8. Collaboration E2E

```text
Project P
→ Channel C
→ Manager M + Researcher A/B + Reviewer R
→ User request
→ Coordinator selects M
→ M Core Lease
→ Scope-Aware Context
→ durable delegation A/B
→ A/B independent Core Leases
→ parallel research
→ Thread Memory Proposals
→ Manager concurrent status report on another Lease
→ Reviewer validation
→ approved finding promoted Thread→Channel→Project
→ final synthesis
```

검사:
- 각 Bot Identity/Brain/Global Memory 독립
- Channel Shared Memory만 authorized shared knowledge
- Manager가 다른 Bot Brain/Core/Memory 직접 mutate하지 않음
- Task/Supervisor/Directive durable
- Scheduler가 모든 Lease authority 유지

## 9. Failure Injection Scenario

위 E2E 중 동시에:
- Researcher A membership revoke
- Runtime crash/restart
- Provider Session loss
- semantic index stale candidate
- duplicate Manager redirect
- long Channel history
- promotion conflict
- v0.4 Bot-only Thread 실행

Then:
- revoke 이후 신규 A scope access/control denied
- Canonical state exact recovery
- Provider Session 없이 Context rebuild
- stale index candidate final auth에서 제거
- redirect idempotent/fenced
- Context bounded
- v0.4 Bot-only path unchanged

## 10. 신규 Acceptance Mapping

- AT-PROJECT-001
- AT-CHANNEL-001
- AT-CHANNEL-002
- AT-CHANNEL-003
- AT-MEM-005
- AT-MEM-006
- AT-CTX-003
- AT-SEC-002
- AT-COLLAB-001
- AT-MIG-001

기존 v0.4/v0.3 Acceptance는 모두 regression suite에 남긴다.

## 11. 문서 관계성 3회 Review

### Review 1 — Structural / Canonical Ownership
file/path/ID/depends_on, document inventory, owner duplication, Scope/Project/Channel/Thread terminology, dependency cycle, naming을 검사한다.

### Review 2 — v0.4 Compatibility / Contract Traceability
`Bot-only path → Conversation/Thread → Memory/Context → Task/Execution → Scheduler/Provider → Recovery/API/Acceptance`가 동일하게 유지되는지와 `Project/Channel → Scoped Memory/Collaboration` additive chain을 추적한다.

### Review 3 — Cross-Layer Executability / Security / Resource
위 E2E + revoke/crash/session-loss/stale-index/fan-out/promotion-conflict를 결합해 실행 가능성, final auth, boundedness, immutable Context/Execution, recovery를 검증한다.

각 Review에서 발견한 문제는 수정 후 동일 범위를 다시 검사한다.

## 12. Release Gate

기존 format/lint/build/naming/dependency/unit/property/storage/recovery/Host/Conformance/security/migration gate에 추가:
- Project/Channel state/property
- generic ScopeRef migration
- final canonical authorization
- membership/authority generation fencing
- bounded Channel routing
- scoped Context boundedness
- promotion provenance/dedup/conflict
- v0.4 compatibility fixture

## 13. 검증 기준

- 신규 10개 Acceptance가 자동 suite/Test/Risk에 매핑됨.
- 3 Review evidence가 `manifest.md`에 존재함.
- Provider Framework/Side Effect/Live Control 기존 acceptance 의미 축소 0.
