---
title: "테스트·검증 전략"
document_id: "DXB-ENG-052"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-013", "DXB-ARC-016", "DXB-ARC-017", "DXB-DOM-027", "DXB-RUN-036", "DXB-ENG-050"]
---

# 테스트·검증 전략

## 1. 목적

상태기계·동시성·복구·Provider Framework에 더해 Persistent Conversation/Thread/Live Control의 isolation, boundedness, immutable redirect, crash recovery를 deterministic evidence로 검증한다.

## 2. 계층

Unit, Property/Model, Concurrency Model, Component, Contract/Conformance, Host Integration, E2E, Fault/Recovery, Performance, Soak, Security, Architecture/Docs를 유지한다. Core correctness는 deterministic Reference/Fake Provider를 사용한다.

## 3. Deterministic Testkit 추가

기존 virtual clock/ID/RNG/scripted Provider/store/event recorder/scheduler/failpoint/invariant checker에 추가:
- persistent Conversation/Thread fixture
- scripted message/history retriever
- Thread-local/global Memory fixture
- Control Channel probe with bounded capacity
- Execution Supervisor safe-point barrier
- Directive delivery/ack loss injector
- Provider Session loss/recreate fixture
- scripted Harness: cancellation/yield/steering supported/unsupported
- branch/archive concurrent command barriers

## 4. Domain / Property Test

- Bot당 Main Conversation exactly one
- Interface Session connect/disconnect가 Conversation revision 불필요 변경 0
- Thread identity/lineage revision invariants
- Thread ≠ Task cardinality (0..N Task)
- Thread A/B Memory scope non-leak
- Memory promotion source revision/provenance
- Task spec revision immutable history
- terminal Task cannot be revived by stale redirect/resume

## 5. Context Tests

AT-CTX-002:
1. deterministic Thread에 history를 단계적으로 증가
2. current input/Identity/Task/local+global Memory/recent+retrieved history 구성
3. Context Plan token+byte cap 검사
4. stable ordering/digest 반복 재현
5. Provider Session history 유무로 Canonical Context meaning이 변하지 않는지 검사

## 6. Live Control Concurrency Suite

barrier/fake clock을 사용해 다음 race를 실제 interleaving으로 검증한다.
- redirect vs complete
- redirect vs cancel
- suspend vs complete
- suspend vs Side Effect external outcome
- resume vs duplicate resume
- reprioritize vs Scheduler admission
- Thread archive vs running Task
- simultaneous Supervisor commands
- Directive commit vs delivery crash
- old Execution yield vs new Execution schedule crash

sleep 길이로 winner를 맞추지 않는다.

## 7. Control Starvation / Fairness

AT-CTRL-002:
- bounded Work Queue를 포화
- long/blocked fake Provider calls 유지
- 별도 control capacity에서 inspect/report/cancel 제출
- control이 일반 backlog 뒤에서 무기한 대기하지 않음을 검증
- 동시에 control flood를 발생시켜 normal work와 다른 Bot control starvation도 차단
- queue item+byte cap/leak 확인

## 8. Redirect / Suspend Acceptance

AT-CTRL-003:
`Execution N/R → Redirect commit R+1 → signal → safe yield → N immutable history → Execution N+1/R+1`.

검사:
- old Context Plan/ProviderSelectionRef mutation 0
- old result late commit fencing
- crash window별 duplicate N+1 0
- Side Effect Confirmed/Unknown semantics 유지

AT-CTRL-004:
`suspend commit → checkpoint → crash → restart suspended → resume duplicate delivery → new Execution exactly one`.

## 9. Cross-Thread Isolation

AT-THREAD-001 / AT-CTRL-005:
- Thread A/B/C 동시 Task/Execution
- A report, B redirect, C suspend
- each Thread local Memory/context/Directive/task revision 검사
- B/C operation이 A provider binding/Core/working context를 변형하지 않음
- Bot-global security policy 같은 shared owner effect는 명시적으로 구분

## 10. Provider Session Independence

AT-SESSION-002:
- Provider Session을 가진 실행 후 durable Conversation/Thread/Memory commit
- Provider Session/token 제거 또는 incompatible 처리
- Runtime restart
- same Bot/Conversation/Thread 복원
- Canonical Context Plan에서 new Provider call/session 생성
- ThreadId/Memory scope 변화 0

## 11. Existing Provider Conformance / Architecture Tests

v0.3 AT-SPI-001~010, Host-path equivalence, Reference/negative Provider, dependency firewall, Provider removal/minimal build를 모두 유지한다.

Harness Conformance에 supported/unsupported cancellation/yield/steering/progress feature cases를 추가하되 native feature 지원이 DXBOT redirect semantics를 바꾸지 않는지 검증한다.

## 12. 문서 관계성 3회 Review

v0.4 Tier A 패키지는 서로 목적이 다른 세 Review evidence를 남긴다.

### Review 1 — Structural / Canonical Ownership
file/path/ID/depends_on, Canonical Owner, Session/Thread/Conversation 용어, duplicate state/policy, manifest/doc count를 검사한다.

### Review 2 — Contract / Traceability
다음 chain을 끝까지 추적한다.

`Conversation → Thread → Memory/Context → Task → Live Control → Scheduler/Execution → Persistence/Recovery → API → Acceptance/Risk`.

Provider Session independence, Conversation History≠Memory, Execution immutable redirect, Control Channel separation도 별도 trace한다.

### Review 3 — Cross-Layer Executability
시나리오:
`Thread A/B/C 병렬 → A report → B redirect → C suspend → B old Execution yield → B new revision/Execution → crash → restart → A/B/C exact recovery → Main Conversation에서 전체 상태 조회`.

상태 중복, cross-thread contamination, duplicate Side Effect, old Execution mutation, control starvation이 없어야 한다.

각 Review 발견 사항을 수정한 뒤 같은 범위를 재확인한다.

## 13. Release Gate

기존 P0 format/lint/build/naming/dependency/unit/property/storage/recovery/Host/Conformance/security/migration/Cross-Layer gate에 다음을 추가한다.
- Conversation/Thread state/property suite
- Context boundedness
- Control starvation/fairness
- redirect/suspend fault matrix
- Provider Session loss independence
- Thread/Directive migration fixture

## 14. 검증 기준

- AT-CONV-001, AT-THREAD-001, AT-CTX-002, AT-MEM-004, AT-CTRL-002~005, AT-SESSION-002가 자동 suite에 매핑됨.
- 신규 concurrency test가 sleep race에 의존하지 않음.
- 3 Review 별도 evidence가 release artifact/manifest에 존재함.
