---
title: "Project Scope 모델"
document_id: "DXB-DOM-028"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-GOV-002"]
---

# Project Scope 모델

## 1. 목적

Project를 Bot과 독립된 장기 Collaboration/Knowledge/Resource Boundary로 정의하고 Project Membership, Policy/Resource relation, Shared Memory relation, Channel ownership의 Canonical 의미를 소유한다.

## 2. 정의

```text
Project ≠ Bot
Project ≠ Brain
Project ≠ Channel
Project ≠ Thread
Project ≠ Session
```

Project는 LLM을 호출하거나 판단하지 않는다.

## 3. Canonical 구성

논리적으로:
- ProjectId / Revision
- Identity/Metadata
- Lifecycle state
- Membership references
- Policy references
- Project Shared Memory Scope relation
- Resource/Artifact references
- Channel references
- Goal/Task references where applicable
- Retention/Deletion policy refs
- provenance/audit metadata

## 4. Membership

Project Membership은 Project-level 접근 ceiling을 제공한다. Channel Membership과 동일하지 않다.

Membership은 최소 principal/participant ref, state, revision/generation, policy/grant refs를 가진다. 정확한 principal model과 enum은 ADR 대상이다.

## 5. Channel Ownership

P0에서 Channel은 Project-scoped resource다.

```text
Project
└─ Channel
```

Project에 참여한다고 모든 Channel을 자동 읽지 못한다. Channel grant는 Project ceiling을 초과하지 못한다.

## 6. Project Memory

Project Shared Memory는 독립 `Project(ProjectId)` Scope다. Bot Memory 또는 Channel Memory의 상위 namespace가 아니다.

Project Memory write/read는 Memory subsystem과 Project authorization을 모두 통과한다. Project owner가 Memory store를 직접 mutate하지 않는다.

## 7. Lifecycle

P0 의미:
- create: durable identity 생성
- active: membership/resource/channel 사용 가능
- archive: 신규 일반 mutation 제한, history/resources 삭제 아님
- restore: same identity/revision lineage 유지
- delete/purge: explicit retention/export/internalized-memory policy 필요

정확한 enum은 ADR 대상이다.

## 8. Resource Boundary

Project는 Artifact/resource/policy reference를 모으는 boundary지만 모든 하위 리소스의 권한을 자동 부여하지 않는다. 각 resource owner와 permission check를 유지한다.

## 9. Recovery

restart 후 Project identity/revision, memberships, channels refs, policy/resource refs, Memory Scope relation을 Canonical store에서 복원한다.

## 10. Non-Goals P0

- Project 자체 Brain/Agent
- 모든 Bot conversation의 Project 강제 귀속
- Project Memory의 automatic Bot internalization
- cross-Project implicit sharing
- exact quota/retention 숫자 고정

## 11. 검증 기준

- AT-PROJECT-001 restart 후 identity/membership/shared memory relation 복원.
- Project A/B Memory/Artifact/Channel 권한 누출 0.
- Project archive가 member Bot 자체 lifecycle을 변경하지 않음.
- Bot-only path에 Project row 생성 강제 0.
