---
title: "수용 기준과 원칙 추적성"
document_id: "DXB-DEL-061"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-ARC-017", "DXB-DOM-027", "DXB-RUN-036", "DXB-DEL-060", "DXB-ENG-052"]
---

# 수용 기준과 원칙 추적성

## 1. 목적

기존 Persistent Bot/Provider Framework/Recovery Acceptance를 보존하면서 v0.4의 Main Conversation/Thread/Memory Scope/Bounded Context/Live Control/Provider Session Independence를 자동화 가능한 Acceptance로 연결한다.

## 2. v0.4 신규 요구

| Requirement | 의미 | Canonical Owner | Acceptance |
|---|---|---|---|
| FR-CONV-001 | Bot당 Persistent Main Conversation | DOM-020/027 | AT-CONV-001 |
| FR-THREAD-001 | Concurrent Thread isolation/lineage | DOM-027 | AT-THREAD-001 |
| NFR-CTX-002 | transcript-independent bounded Context | DOM-021 | AT-CTX-002 |
| FR-MEM-004 | Thread→Bot Memory promotion policy | DOM-022 | AT-MEM-004 |
| FR-CTRL-002 | out-of-band control starvation 방지 | RUN-036/RUN-031 | AT-CTRL-002 |
| FR-CTRL-003 | immutable live redirect | DOM-023/RUN-036 | AT-CTRL-003 |
| FR-CTRL-004 | suspend/resume crash recovery | DOM-023/RUN-033/036 | AT-CTRL-004 |
| FR-CTRL-005 | parallel intervention isolation | DOM-027/RUN-030/036 | AT-CTRL-005 |
| FR-SESSION-002 | Provider Session loss independence | ARC-013/DOM-027/RUN-033 | AT-SESSION-002 |

## 3. AT-CONV-001 — Persistent Main Conversation

Given 하나의 Persistent Bot과 Main Conversation/여러 Thread가 존재하고,
When 모든 Interface Session을 종료하고 Runtime을 restart한 뒤 새 Web/CLI/TUI Session으로 reconnect하면,
Then:
1. 동일 BotId가 복원된다.
2. 동일 Main Conversation identity가 하나만 존재한다.
3. 기존 Thread identity/revision/archive/lineage가 복원된다.
4. Session 종료 때문에 새 Conversation/Thread가 생성되지 않는다.
5. Conversation history cursor/source가 복원된다.
6. Bot-global/Thread-local Memory scope가 유지된다.
7. running/Waiting/Suspended Task recovery는 해당 owner semantic을 따른다.

실패 조건: browser/connection/session 수명에 따라 Conversation identity가 바뀜.

## 4. AT-THREAD-001 — Concurrent Thread Isolation

Given 같은 Bot의 Thread A/B/C가 서로 다른 local Memory/Task를 가지고,
When 세 Thread에서 Execution을 동시에 수행하면,
Then:
- A local Memory/Conversation context가 B/C Context Plan에 authorization/reference 없이 나타나지 않는다.
- B Task revision/control이 A/C Task revision을 변경하지 않는다.
- C Working Context/Core scratch가 A/B에 노출되지 않는다.
- Bot-global Identity/Policy/Memory 같은 의도된 shared owner만 공통으로 보인다.
- branch 후 source/child revision은 독립적이다.

## 5. AT-CTX-002 — Bounded Thread Context

Thread history를 정책 budget보다 훨씬 크게 증가시킨다.

Then Context Plan은:
- current input/Identity/Task 필수 섹션 유지
- local/global Memory와 recent/retrieved history를 deterministic selection
- token+byte budget 미초과
- 전체 transcript heap/prompt materialization 없음
- 동일 snapshot/input에서 동일 digest/order
- Provider Session history 유무로 semantic이 달라지지 않음

## 6. AT-MEM-004 — Thread Memory Promotion

Given Thread-local Memory candidate가 있고,
When promotion approval/policy가 없으면,
Then Bot-global Memory revision은 생성되지 않는다.

When valid PromotionProposal이 provenance/confidence/conflict/retention/security policy를 통과하면,
Then source Thread Memory revision을 보존한 새 global revision/reference가 생성된다.

실패 조건: Thread message 저장 또는 Working result만으로 global Memory 자동 commit.

## 7. AT-CTRL-002 — Out-of-Band Report / Control Starvation

Given Normal Work Queue가 bounded capacity까지 포화되고 long-running fake Provider call이 존재할 때,
When inspect/report/cancel 같은 control을 제출하면,
Then:
- control이 일반 Work Queue 뒤에서 무기한 대기하지 않는다.
- 별도 bounded Control Channel/admission evidence가 있다.
- overload면 explicit reject/coalesce semantics를 반환한다.
- control flood가 다른 Bot control이나 normal work를 영구 starvation시키지 않는다.
- item+byte/resource limit을 초과하지 않는다.

## 8. AT-CTRL-003 — Live Redirect

Scenario:
`Task Spec R / Execution N Running → Redirect commit R+1 → old Execution yield → Execution N+1/R+1`.

검증:
1. Directive + Task Specification R+1이 durable commit된다.
2. Execution N의 Task revision/Context Plan/ProviderSelectionRef는 mutation 0.
3. safe-point/cancellation capability에 따라 N이 yield/terminal된다.
4. checkpoint/result fragment/Side Effect state가 보존된다.
5. N+1은 R+1 snapshot으로 시작한다.
6. redirect commit/ack/yield/schedule 각 crash window에서 duplicate N+1 또는 lost redirect가 없다.
7. late N result가 R+1 Task를 성공으로 덮지 못한다.

## 9. AT-CTRL-004 — Suspend / Resume Recovery

Scenario:
`suspend commit → safe checkpoint → Suspended → crash → restart → resume duplicated delivery`.

Then:
- suspended semantic state와 checkpoint/Continuation이 복원된다.
- Provider Host activity/Core Lease가 누수되지 않는다.
- resume guard/idempotency로 새 Execution이 정확히 하나 생성된다.
- missing/corrupt checkpoint는 explicit RecoveryRequired이며 임의 재실행하지 않는다.
- Unknown Side Effect는 resume 때문에 재실행되지 않는다.

## 10. AT-CTRL-005 — Parallel Intervention Isolation

Given Thread A/B/C가 동시에 실행 중일 때,
When A에는 report, B에는 redirect, C에는 suspend를 거의 동시에 수행하면,
Then:
- A는 report/freshness만 생성하고 Task spec을 변경하지 않는다.
- B old Execution만 yield하고 B new revision/Execution이 생성된다.
- C만 suspended state로 전환한다.
- A/B/C local Memory/Continuation/Directive가 교차 오염되지 않는다.
- B/C control이 A의 Core Lease/Provider binding을 취소하지 않는다.

## 11. AT-SESSION-002 — Provider Session Loss Independence

Given Provider Session/resume token을 사용하는 Harness execution history가 있고,
When Provider Session을 제거/만료/incompatible로 만든 뒤 Runtime을 restart하면,
Then:
- BotId/Main Conversation/ThreadId/Memory/Task canonical state가 유지된다.
- Provider Session ID가 Thread identity로 재사용되지 않는다.
- Canonical Context Plan으로 new Provider call/session을 생성할 수 있다.
- provider resume 불가가 필요한 경우 stable recovery/unavailable outcome으로 보인다.
- same Execution binding을 다른 Provider로 silent rewrite하지 않는다.

## 12. 기존 v0.3 SPI Acceptance 유지

다음 Acceptance는 v0.4에서도 변경 없이 필수다.

| ID | 핵심 의미 |
|---|---|
| AT-SPI-001 | Capability Contract completeness |
| AT-SPI-002 | Common-owned Provider Lifecycle |
| AT-SPI-003 | Mandatory Provider Host enforcement |
| AT-SPI-004 | Thin Provider / Minimum Surface |
| AT-SPI-005 | Forbidden Provider dependency |
| AT-SPI-006 | deterministic Provider Scaffold |
| AT-SPI-007 | Executable Conformance incl. Host path |
| AT-SPI-008 | Contract compatibility discipline |
| AT-SPI-009 | Reference Provider + negative fixture |
| AT-SPI-010 | Framework Provider removal |

v0.3의 상세 pass/fail 조건(Ready/Draining/activity, Host stage, Context/dependency firewall, Conformance scenarios, removal cleanup)은 축소하지 않는다.

## 13. 기존 제품 Acceptance 유지

- AT-BOT-001 Session independence
- AT-BOT-003 Persistent restore
- AT-BRAIN-001 Single Brain
- AT-CORE-002 Core non-identity
- AT-CORE-005 Dynamic limit
- AT-CTX-001 shared Memory / isolated Working Context
- AT-MEM-002 Working Memory promotion
- AT-MEM-003 Canonical Memory pressure independence
- AT-TASK-002 retry/cancel
- AT-TASK-003 Waiting Continuation recovery
- AT-SFX-001 Side Effect write-ahead/reconciliation
- AT-ROUTINE-001 Routine restart
- AT-MOD-001 Provider replacement
- AT-MOD-002 Provider removal
- AT-MOD-003 Multiple Provider explicit selection
- AT-PLUGIN-001 Plugin lifecycle/data/permission
- AT-REPO-001 Repository naming/layout
- AT-POL-001 Policy SSOT
- AT-NET-001 durable delegation
- AT-CTRL-001 Control Plane independence
- AT-HAR-001 Harness conformance
- AT-IFC-001/002 headless/interface separation
- AT-STO-001 atomic commit
- AT-SEC-001 least privilege
- AT-REC-001 recovery
- AT-OBS-001 end-to-end traceability

## 14. Cross-Layer Traceability

```text
FR-CONV-001
→ DOM-027 Main Conversation
→ ARC-014/015 Event+Storage
→ RUN-033 Recovery
→ IFC-040
→ ENG-052
→ AT-CONV-001
→ R-040/R-041

FR-CTRL-003/004
→ DOM-023 Task Revision/Continuation
→ RUN-036 Control/Preemption
→ RUN-030 Race
→ ARC-014/015 Durable Directive
→ RUN-033 Recovery
→ IFC-040
→ ENG-052
→ AT-CTRL-003/004
→ R-042/R-044/R-047/R-048
```

## 15. Evidence

기존 test ID/commit/build/version/seed/workload/result/trace/artifact/dependency/platform/waiver에 ConversationId/ThreadId/DirectiveId/TaskSpecRevision/control queue metrics/report freshness/provider session-loss fixture를 추가한다.

## 16. 검증 기준

- 신규 9개 Acceptance 모두 Canonical Owner/Test/Risk에 연결됨.
- v0.3 Acceptance ID 및 상세 의미가 회귀하지 않음.
- skip/waiver는 success가 아니며 owner/expiry/risk를 가짐.
