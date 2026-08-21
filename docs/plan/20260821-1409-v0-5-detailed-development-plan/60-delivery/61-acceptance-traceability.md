---
title: "수용 기준과 원칙 추적성"
document_id: "DXB-DEL-061"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-ARC-017", "DXB-DOM-022", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-036", "DXB-RUN-037", "DXB-DEL-060", "DXB-ENG-052"]
---

# 수용 기준과 원칙 추적성

## 1. 목적

v0.4까지의 모든 Acceptance를 비회귀 기준으로 유지하면서 Project/Channel/Scope-Aware Memory/Persistent Multi-Bot Collaboration을 자동화 가능한 P0 Acceptance로 추가한다.

## 2. v0.5 신규 요구

| Requirement | Canonical Owner | Acceptance |
|---|---|---|
| FR-PROJECT-001 Project persistence/isolation | DOM-028 | AT-PROJECT-001 |
| FR-CHANNEL-001 Persistent membership/role/authority | DOM-029 | AT-CHANNEL-001 |
| NFR-CHANNEL-002 Bounded participant routing | RUN-037/RUN-031 | AT-CHANNEL-002 |
| FR-CHANNEL-003 Role/Authority separation | DOM-029/RUN-032 | AT-CHANNEL-003 |
| FR-MEM-005 Generic scope isolation | DOM-022 | AT-MEM-005 |
| FR-MEM-006 Promotion provenance | DOM-022 | AT-MEM-006 |
| NFR-CTX-003 Scope-aware bounded Context | DOM-021 | AT-CTX-003 |
| FR-SEC-002 Membership revocation | RUN-032 | AT-SEC-002 |
| FR-COLLAB-001 Channel manager collaboration | DOM-023/025/RUN-037 | AT-COLLAB-001 |
| FR-MIG-001 v0.4 compatibility | ENG-053 | AT-MIG-001 |

## 3. AT-PROJECT-001 — Project Persistence / Isolation

Given Project A/B가 별도 Membership/Memory/Artifact/Channel을 가지고,
When Runtime restart와 Interface reconnect를 수행하면,
Then:
1. 동일 ProjectId/revision이 복원된다.
2. Membership current generation이 복원된다.
3. Project Memory Scope relation이 보존된다.
4. A principal이 권한 없이 B Memory/Artifact/Channel을 읽지 못한다.
5. Project archive가 member Bot Identity/lifecycle을 변경하지 않는다.
6. Bot-only fixture에는 Project 생성이 강제되지 않는다.

## 4. AT-CHANNEL-001 — Persistent Channel Membership

Given Channel C에 Manager/Researcher/Reviewer Bot이 참여하고 Role/Authority binding이 존재할 때,
When Runtime restart와 client reconnect를 수행하면,
Then ChannelId, ProjectId, membership, Role binding, Authority binding의 identity/revision/generation이 동일하게 복원된다. Presence는 재생성 가능하며 Canonical revision을 변경하지 않는다.

## 5. AT-CHANNEL-002 — Bounded Participation Routing

Given Channel member 수를 1/10/100으로 증가시키고,
When 일반 message, explicit mention, role mention, debate request를 제출하면,
Then:
- selected participant 수는 Policy cap을 넘지 않는다.
- 일반 message가 모든 member Execution을 자동 생성하지 않는다.
- inbound/fan-out/response queue는 item+byte cap을 유지한다.
- overload는 explicit reject/defer/coalesce/partial-selection semantic을 가진다.
- unrelated Channel/Bot이 무기한 starvation되지 않는다.

## 6. AT-CHANNEL-003 — Role / Authority Separation

Given Bot Role=`Manager`이지만 delegate/redirect Authority가 없을 때,
When prompt/message/UI에서 Manager action을 요청하면,
Then Canonical delegate/Directive commit은 거부된다.

When valid Authority binding이 별도 부여되면 해당 granted operation만 가능하다. Role 변경만으로 Bot Global Identity revision을 생성하지 않는다.

## 7. AT-MEM-005 — Scope Isolation

Given Bot/Project/Channel/Thread 각각에 서로 다른 Memory가 있고 stale semantic index가 unauthorized candidate를 반환할 수 있을 때,
When recall/Context Plan을 생성하면,
Then:
- Authorized Scope Set 밖 Canonical Memory는 final authorization에서 제거된다.
- Bot A/B Global Memory는 직접 공유되지 않는다.
- Shared Scope Memory만 유효한 grant로 읽힌다.
- candidate score/index가 permission authority가 아니다.

## 8. AT-MEM-006 — Promotion Provenance

Scenario:
`Thread Memory A → Channel Memory B → Project Memory C`.

Then:
- A/B/C는 독립 Canonical revision/reference다.
- 각 target record/relation이 immediate source MemoryId/revision/digest/provenance를 보존한다.
- source scope를 in-place mutation/delete하지 않는다.
- simultaneous duplicate promotion은 dedup/conflict policy로 지식 폭증을 막는다.
- Shared→Bot internalization이 있다면 별도 Proposal/Policy를 요구한다.

## 9. AT-CTX-003 — Scope-Aware Bounded Context

Project/Channel/Thread history와 Memory를 매우 크게 증가시킨다.

Then Context Plan은:
- current Bot/Project?/Channel?/Thread/Task scope를 명시
- final authorization/trust filter 적용
- selected immutable Memory revision pin
- deterministic order/digest
- token+byte budget 미초과
- 전체 history/Memory heap/prompt materialization 없음
- Provider Session 유무로 Canonical semantic이 변하지 않음

## 10. AT-SEC-002 — Membership Revocation

Given Researcher A가 Channel에서 실행/조회 가능하고 routing/cache/index가 warmed된 상태에서,
When A membership 또는 relevant Authority를 revoke하면,
Then revoke commit 이후 신규:
- Channel/Project Memory read/write
- Channel history/Artifact read
- Channel routing admission
- delegate/redirect/suspend/resume/cancel/reprioritize
가 denied된다.

stale cache/index/routing snapshot이 이를 우회하지 못한다. 이미 생성된 immutable Execution Context는 rewrite하지 않으며 high-risk effect는 policy가 current authorization을 재확인한다.

## 11. AT-COLLAB-001 — Channel Manager Collaboration

Scenario:
`User → Manager M → durable delegation Researcher A/B → parallel results → Reviewer R → Manager report/redirect → approved Memory publication → final synthesis`.

검증:
- 각 Bot은 독립 Identity/Brain/Global Memory를 유지한다.
- delegation/result는 durable Bot Network/Task semantic을 사용한다.
- Manager의 natural-language message만으로 Task/Directive commit하지 않는다.
- SupervisorRef와 Role/Authority가 분리된다.
- 각 Execution의 Core Lease는 Scheduler가 발급한다.
- Manager가 다른 Bot Brain/Core/Memory를 직접 mutate하지 않는다.
- Shared Memory publication은 MemoryProposal/authorization을 통과한다.

## 12. AT-MIG-001 — v0.4 Compatibility

Given representative v0.4 Bot/Main Conversation/Thread/Memory/Task/Directive fixture,
When v0.5 schema/data migration 후 restart하면,
Then:
- BotId/ConversationId/ThreadId/MemoryId/revision/provenance unchanged
- 기존 Bot Global Memory 의미 unchanged
- 기존 Thread Memory가 same Thread ScopeRef로 연결
- existing Main Conversation/Thread/Live Control/Provider Session independence acceptance 통과
- 임의 Project/Channel 생성 0
- Provider Session ID를 migration key로 사용하지 않음

## 13. 기존 v0.4/v0.3 Acceptance 유지

최소 다음을 포함해 기존 inventory의 상세 pass/fail 조건을 삭제·축소하지 않는다.
- AT-CONV-001 / AT-THREAD-001
- AT-CTX-001/002
- AT-MEM-002/003/004
- AT-CTRL-001~005
- AT-SESSION-002
- AT-BOT / BRAIN / CORE / TASK / SFX / ROUTINE / NET / HAR / IFC / STO / SEC / REC / OBS / MOD / PLUGIN / REPO / POL
- AT-SPI-001~010

## 14. Cross-Layer Traceability

```text
FR-CHANNEL-002
→ DOM-029 participant eligibility
→ RUN-037 routing/admission
→ RUN-031 resource cap
→ ENG-051 benchmark
→ ENG-052 deterministic fixture
→ AT-CHANNEL-002
→ R-053/R-057

FR-SEC-002
→ DOM-029 membership generation
→ RUN-032 current authorization
→ RUN-030 revoke races
→ DOM-021/022 final auth
→ ENG-052 fault fixture
→ AT-SEC-002
→ R-052/R-055

FR-MIG-001
→ DOM-027 ConversationParentRef
→ DOM-022 ScopeRef
→ ARC-015 storage
→ ENG-053 migration
→ AT-MIG-001
→ R-056/R-060
```

## 15. 검증 기준

- 신규 10개 Acceptance가 Owner/Test/Risk/OQ 또는 operational metric에 연결됨.
- 기존 Acceptance ID/semantic regression 0.
- skip/waiver는 success가 아니며 owner/expiry/risk를 가진다.
