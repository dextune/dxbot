---
title: "DXBOT Control Plane과 Collaboration Surface"
document_id: "DXB-DOM-026"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-ARC-016", "DXB-ARC-017", "DXB-DOM-020", "DXB-DOM-025", "DXB-DOM-027"]
---

# DXBOT Control Plane과 Collaboration Surface

## 1. 목적

Bot/Main Conversation/Thread/Task/Core/Memory/Provider/Plugin/Project/Channel/Membership/Role/Authority를 하나의 **Headless Application Contract**로 운영하되 Domain/Storage/Scheduler/Provider Host를 우회하지 않는다. v0.7의 공식 Reference Consumer는 CLI다.

## 2. Application Use-Case Surface

기존 v0.6 Control Plane 기능을 다음 public use-case로 노출할 수 있다.

### Project
- create/list/get/archive/restore
- membership query/change
- policy/resource/artifact refs
- Project Memory search/proposal/promotion
- Channel list

### Channel
- create/list/get/archive/restore
- membership join/leave/change
- Role/Authority binding 관리
- message/history/thread
- Channel Memory
- participant/task/status
- collaboration control

Project membership과 Channel membership을 하나의 ACL mutation으로 암묵 병합하지 않는다.

## 3. Collaboration Control

`delegate/inspect/report/redirect/suspend/resume/cancel/reprioritize`는 authenticated Principal, target, expected revision/generation, SupervisorRef/Authority scope를 검증한 뒤 기존 Canonical Command/Directive를 사용한다.

자연어 지시는 intent 후보일 뿐 Canonical Task/Directive commit과 동일하지 않다. CLI가 문자열을 해석해 Runtime state를 직접 변경하지 않는다.

## 4. Role / Authority / Authorization

Application response와 CLI output은 Role, effective Authority, Authorization Decision을 구분한다.

- Role label만 보고 destructive/control action 가능 여부를 CLI가 판정하지 않는다.
- current authorization은 `DXB-RUN-032`이 소유한다.
- approval-required action은 existing Approval/ActionGrant semantic을 사용한다.
- stale membership/authority generation을 client가 자동 overwrite하지 않는다.

## 5. Query Freshness

Project/Channel member/status/history/Memory/presence query는 다음을 구분한다.
- Canonical revision/generation
- Projection/source watermark
- `observed_at`
- stale/degraded

Presence는 Canonical membership source가 아니다.

## 6. Headless Boundary

Control Plane은 Interface-neutral한 Application use-case를 제공한다. v0.7에서는 CLI가 이를 소비하며 다음을 금지한다.

- CLI→Coordinator/Core/DB handle 직접 조작
- CLI→Provider direct invocation
- CLI→Memory store direct mutation
- CLI-side permission/memory/scheduler policy duplicate
- CLI local state를 Project/Channel truth로 사용

## 7. Process Independence

Control connection/CLI process 종료가 Channel membership, Task/Directive/Durable Process/Bot lifecycle을 변경하지 않는다. explicit Command만 상태를 변경한다.

## 8. 검증 기준

- Headless Application Contract와 CLI가 동일 Project/Channel Canonical use-case를 사용한다.
- Role/Authority/Authorization이 별도 response/command semantic을 가진다.
- CLI 종료/disconnect가 Channel membership/Task 실행에 영향 0.
- direct Domain/Storage/Scheduler/Provider handle 노출 0.
- TUI/Web 전용 state/use-case가 active v0.7 Domain 계약에 없음.
