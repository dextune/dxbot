---
title: "DXBOT Control Plane과 Collaboration Surface"
document_id: "DXB-DOM-026"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-ARC-016", "DXB-ARC-017", "DXB-DOM-020", "DXB-DOM-025", "DXB-DOM-027"]
---

# DXBOT Control Plane과 Collaboration Surface

## 1. 목적

Bot/Main Conversation/Thread/Task/Core/Memory/Provider/Plugin에 Project/Channel/Membership/Role/Authority/Shared Memory를 추가해 하나의 shared API로 운영하되 Domain/Storage/Scheduler를 우회하지 않는다.

## 2. Project 기능

- create/list/get/archive/restore
- membership query/change
- policy/resource/artifact refs
- Project Memory search/proposal/promotion
- Channel list

## 3. Channel 기능

- create/list/get/archive/restore
- membership join/leave/change
- Role/Authority binding 관리
- message/history/thread
- Channel Memory
- participant/task/status
- collaboration control

Project membership과 Channel membership을 한 ACL command로 암묵 병합하지 않는다.

## 4. Collaboration Control

`delegate/inspect/report/redirect/suspend/resume/cancel/reprioritize`는 authenticated principal, target, expected revision/generation, SupervisorRef/Authority scope를 검증한 뒤 기존 Canonical Command/Directive를 사용한다.

자연어:

```text
"Researcher A, persistence 다시 조사해"
```

는 intent 후보일 뿐 Task/Directive commit과 동일하지 않다.

## 5. Role / Authority UX

Control Plane은 Role과 effective Authority를 별도 필드/화면으로 노출한다. `Manager` label만 보고 destructive action을 활성화하지 않는다.

## 6. Query Freshness

Project/Channel member/status/history/Memory/presence query는 Canonical revision과 Derived observation timestamp/watermark를 구분한다. Presence는 Canonical membership source가 아니다.

## 7. Safety

- default deny
- bulk/destructive dry-run
- membership/authority expected generation
- shared Memory write 별도 permission
- cross-scope export/forget separate approval
- stale projection을 mutation authority로 사용 금지

## 8. 검증 기준

- CLI/TUI/Web이 동일 Project/Channel schema를 사용.
- Role과 Authority가 별도 response field/command semantic을 가짐.
- Control Plane 종료가 Channel membership/Task 실행 의미를 바꾸지 않음.
- UI가 Coordinator/Core/DB handle을 직접 조작하지 않음.
