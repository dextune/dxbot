---
title: "Goal·Task·Execution·Supervisor 모델"
document_id: "DXB-DOM-023"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-015", "DXB-ARC-017", "DXB-DOM-020", "DXB-DOM-021", "DXB-DOM-027"]
---

# Goal·Task·Execution·Supervisor 모델

## 1. 목적

v0.4 Goal/Task/immutable Execution/Continuation/Side Effect semantics를 유지하고 Project/Channel provenance와 authoritative SupervisorRef를 추가한다.

## 2. Task 구성 확장

Task는 기존 필드에 필요 시 다음 provenance/reference를 가진다.
- ProjectId?
- ChannelId?
- primary ThreadId
- source Conversation Parent
- SupervisorRef?
- delegation/message refs
- authority/policy revision refs

ProjectId/ChannelId가 존재해도 Task owner는 Task Aggregate다.

## 3. SupervisorRef

P0 기본:
- 하나의 Task에 authoritative SupervisorRef는 최대 1명
- Channel Role/Manager label과 별도
- Supervisor 변경은 explicit authorized revision으로만 가능
- Supervisor는 Brain/Core/Memory 직접 mutation 권한이 아님
- report/redirect/suspend 등은 기존 Control Directive semantic 사용

## 4. Delegation

Manager가 Researcher에게 위임할 때:
1. parent Task decomposition
2. durable Bot Network Message/delegation intent
3. target Bot authorization
4. target Task idempotent create
5. Parent Waiting/Continuation 필요 시 same logical commit
6. result/Artifact return
7. supervisor review/control

Channel Message text만으로 target Task가 생기지 않는다.

## 5. Immutable Execution

Execution 시작 시 기존 v0.4 snapshot에 다음을 추가 pin할 수 있다.
- Project/Channel membership/authority generation relevant to Context
- selected scoped Memory revisions
- SupervisorRef/Task revision

running Execution의 snapshot을 membership/role change 때문에 mutation하지 않는다.

## 6. Channel archive / revoke

Channel archive는 Task를 암묵 cancel하지 않는다. revoked participant의 신규 control/read/write는 차단하고 running Execution 처리/high-risk effect policy는 Security/ADR가 결정한다.

## 7. Race

기존 redirect/cancel/complete/suspend/resume/side-effect race에 추가:
- supervisor change vs redirect
- membership revoke vs Task control
- Channel archive vs running Task
- result completion vs supervisor redirect

expected revision/generation + Execution fencing을 사용한다.

## 8. 검증 기준

- AT-COLLAB-001 delegation/result/report/redirect가 durable Task/Control semantic을 사용.
- Manager가 Researcher Brain/Core/Memory를 직접 mutate하지 않음.
- stale SupervisorRef/Authority revision으로 control commit 불가.
- v0.4 Task fixture의 state/revision/Side Effect semantics 유지.
