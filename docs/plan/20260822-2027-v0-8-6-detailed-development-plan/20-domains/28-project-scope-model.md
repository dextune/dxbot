---
title: "Project Scope와 Membership 모델"
document_id: "DXB-DOM-028"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-DOM-020", "DXB-ARC-015"]
---
# Project Scope와 Membership 모델

Project는 여러 Bot·Channel·Artifact·Memory를 묶는 durable 협업/정책/자원 Scope이며 Brain, IAM tenant, transient workspace가 아니다.

## Project Aggregate

```text
ProjectId / Revision / Name / aliases
Lifecycle: Active | Archiving | Archived | Restoring
Policy refs / ProjectMemoryScopeRef / Channel refs
```

archive는 신규 mutation/admission을 차단하되 child Task/Process를 직접 terminal 처리하지 않는다.

## Project Membership canonical row

```text
key = (ProjectId, MemberBotId)
MembershipGeneration
RoleRef
AuthorityBindingRef  // security owner가 발급
State = Active | Revoked
CreatedAt / UpdatedAt / RevokedAt?
```

- Project aggregate가 row와 uniqueness를 소유한다.
- set은 expected Project revision과 `expected_membership_generation | absent` CAS를 요구한다.
- remove는 current generation을 검증하고 revoke한다. Bot identity를 삭제하지 않는다.
- RoleRef는 Authority가 아니며 Security owner가 Role+scope policy에서 AuthorityBinding generation을 생성한다.
- Project create는 owner Bot membership과 owner binding을 같은 durable unit 또는 recoverable provisioning process로 정확히 하나 생성한다.
- concurrent set/remove winner는 generation CAS 하나다.

Project Memory는 Bot private Memory의 복사본이 아니며 promotion/declassification을 요구한다. Project resource limit은 global Runtime ceiling을 완화하지 못한다.
