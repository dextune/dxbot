---
title: "Project Scope와 Membership 모델"
document_id: "DXB-DOM-028"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-DOM-020", "DXB-ARC-015"]
---
# Project Scope와 Membership 모델

Project는 durable collaboration/resource scope이며 Brain/Task/Memory assertion owner가 아니다.

Membership canonical key는 `(ProjectId, MemberBotId)`이고 generation CAS를 사용한다. set/remove는 expected Project revision과 expected membership generation/absent를 검증한다.

Application Unit of Work는 Membership row 변경, Security owner의 AuthorityBinding create/revoke, required AuditIntent를 atomic commit한다. RoleRef는 Authority가 아니다.

Project create는 name uniqueness/idempotency로 경쟁을 해결하며 global Instance revision CAS를 요구하지 않는다. owner Bot membership/binding은 create와 같은 durable Unit of Work에서 생성한다.
