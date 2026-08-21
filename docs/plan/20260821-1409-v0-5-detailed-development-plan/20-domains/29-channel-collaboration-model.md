---
title: "Channel Collaboration 모델"
document_id: "DXB-DOM-029"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-GOV-002", "DXB-DOM-028", "DXB-DOM-027"]
---

# Channel Collaboration 모델

## 1. 목적

Channel을 Project 안에서 여러 Persistent Bot과 사용자가 협업하는 영속 Multi-Bot Collaboration Space로 정의한다. Channel identity/lifecycle/membership/Role/Authority/Conversation relation의 Canonical Owner다.

## 2. 정의

```text
Channel ≠ Bot
Channel ≠ Brain
Channel ≠ Core
Channel ≠ Task
Channel ≠ Thread
```

Channel은 판단을 수행하지 않는다.

## 3. Canonical 구성

- ChannelId / ProjectId / Revision
- lifecycle/metadata
- Membership references
- Role Bindings
- Authority Bindings
- Participation Policy ref
- Channel Conversation relation
- Channel Shared Memory Scope relation
- Thread refs
- Task/Artifact refs where needed
- retention/audit metadata

CoreLeaseId/processing state는 Canonical Membership에 넣지 않는다.

## 4. Membership

Canonical participant는 Persistent Bot이다. User/principal participation representation은 Permission/API ADR에서 정교화할 수 있으나 Bot runtime 참여는 BotId를 기준으로 한다.

```text
ChannelMembership
├─ ChannelId
├─ BotId
├─ RoleBindings
├─ AuthorityBindings
├─ ParticipationPolicyRef
├─ MembershipRevision/Generation
└─ State
```

## 5. Role / Authority

Role: Manager, Researcher, Reviewer 등 협업 의미.
Authority: delegate, inspect, report, redirect, suspend, resume, cancel, reprioritize 등 Runtime 권한.

Role label과 Authority binding을 동일 field로 저장하지 않는다. Role 변경이 Bot Identity를 변경하지 않는다.

## 6. Channel Conversation / Thread

Channel은 persistent conversation surface를 가질 수 있고 Thread의 Conversation Parent가 된다. Thread의 lifecycle/lineage는 Conversation Domain이 소유한다.

## 7. Channel Memory

Channel Shared Memory는 독립 `Channel(ChannelId)` Scope다. participating Bot Memory에 자동 복제하지 않는다. write는 Membership/Authority + Memory policy를 검증한다.

## 8. Participation Semantics

Membership이 존재한다고 모든 메시지에 Bot이 실행되는 것은 아니다. 실제 speaker/recipient/admission은 Runtime Orchestration이 소유한다.

Domain은 다음만 보장한다.
- participant eligibility source
- role/authority current revision
- participation policy reference
- membership state/generation

## 9. Manager / Supervisor

Manager Role은 Task SupervisorRef와 별도다. Manager가 delegation을 시작할 수 있는지는 Authority가 결정하고, 생성된 Task의 authoritative SupervisorRef는 Task Domain이 소유한다.

## 10. Lifecycle

P0 의미:
- create/active/archive/restore
- archive가 running Task를 자동 cancel하지 않음
- archive 후 신규 normal message/task admission policy는 제한 가능
- restore는 same Channel identity 유지

정확한 enum/manager failover는 ADR/OQ다.

## 11. Revocation

Membership/Authority revoke 후:
- 신규 Channel history/Memory read denied
- 신규 Shared Memory write denied
- 신규 control denied
- 신규 Channel routing eligibility 제외

already-built immutable Context 처리와 high-risk side effect는 Security Runtime 규칙을 따른다.

## 12. Presence

`speaking/processing/active Execution/CoreLease/background task/last activity`는 Derived `ChannelPresenceRuntime`이며 Membership truth가 아니다.

## 13. 검증 기준

- AT-CHANNEL-001 restart/reconnect 후 identity/member/role/authority revision 보존.
- AT-CHANNEL-003 Role 문자열만으로 control authority 생성 불가.
- Channel member 수 증가가 자동 Execution 수 증가를 의미하지 않음.
- Channel Memory가 Bot Global Memory replica가 아님.
