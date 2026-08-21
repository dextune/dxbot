---
title: "Project Scope와 Membership 모델"
document_id: "DXB-DOM-028"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-020", "DXB-ARC-015"]
---

# Project Scope와 Membership 모델

## 1. 목적

Project를 여러 Bot·Channel·Artifact·Memory를 묶는 durable 협업/정책/자원 Scope로 정의하되 Brain, global IAM tenant, transient workspace로 오인하지 않는다.

## 2. Project Aggregate

```text
ProjectId / Revision
Name / scoped aliases
Lifecycle
Membership policy refs
Resource/Retention/Information-flow policy refs
ProjectMemoryScopeRef
Artifact/Channel refs
CreatedAt / ArchivedAt?
```

Project는 Bot Identity, Channel membership, Task state, Memory assertion을 복제하지 않고 reference와 policy를 소유한다.

## 3. Lifecycle

```text
Active → Archiving → Archived → Restoring → Active
```

archive는 신규 mutation/admission을 차단하고 linked Channel/Task/Process에 명시된 policy를 적용한다. 모든 child를 자동 delete/cancel하지 않는다. hard delete는 P0 CLI 비범위다.

## 4. Membership

Project Membership과 Channel Membership을 하나의 ACL row/command로 병합하지 않는다. Project membership은 project-level visibility/participation의 base context이며 Channel Role/Authority는 별도 current generation을 가진다.

Role label, Authority, Authorization Decision을 구분한다. membership mutation은 expected revision과 approval/authorization을 요구한다.

## 5. Memory·Artifact·Resource

Project Memory는 Shared scope이며 Bot private Memory의 복사본이 아니다. promotion은 provenance/declassification을 요구한다. Project resource limit은 global Runtime ceiling을 완화하지 못한다. Artifact는 owner/reference/retention를 가진다.

## 6. Selector와 Query

ProjectId가 최종 식별자다. exact name/alias는 principal-visible Runtime Instance scope에서 하나일 때만 resolve된다. list/show는 canonical revision과 derived channel/member count watermark를 구분한다.

## 7. 검증 기준

- Project가 Brain/Task/Memory assertion owner로 변질되지 않음.
- Project archive가 child Task를 silent cancel/delete하지 않음.
- Project/Channel membership mutation이 하나의 implicit ACL write로 합쳐지지 않음.
- Private→Project Memory promotion이 Information Flow를 우회하지 않음.
- 동일 이름 Project의 ambiguous mutation 거부.
