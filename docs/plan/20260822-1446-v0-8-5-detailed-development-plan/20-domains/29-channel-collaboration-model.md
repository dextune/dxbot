---
title: "Channel Collaboration 모델"
document_id: "DXB-DOM-029"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-025", "DXB-DOM-028", "DXB-DOM-027"]
---

# Channel Collaboration 모델

## 1. 목적

Channel을 Project 안의 메시지·멤버십·Role/Authority·협업·Shared Memory boundary로 정의하고, Bot Brain이나 Scheduler queue로 사용하지 않는다.

## 2. Channel Aggregate

```text
ChannelId / Revision
ProjectId
Name / aliases
Lifecycle
ConversationRef
Membership / Role / Authority binding refs
ChannelMemoryScopeRef
Collaboration/resource policy refs
```

Presence, active Core, process status, unread count는 derived state다.

## 3. Membership·Role·Authority

- Membership: 참여 관계와 generation
- Role: 사람이 이해하는 직무/표현
- Authority: 허용 가능한 action/resource scope의 Canonical binding
- Authorization Decision: 현재 principal/action/target/context에 대한 Runtime 결정

Role 문자열만으로 mutation을 허용하지 않는다. join/leave/member change는 current Project/Channel policy와 revision을 검증한다.

## 4. Message와 Thread

Channel send는 `DXB-DOM-026`의 `SendMessage`를 사용한다. Conversation/Thread identity는 `DXB-DOM-027`이 소유한다. message append가 Task delegation이나 Memory promotion을 암묵 생성하지 않는다.

## 5. Collaboration

협업 run은 participant/fan-out/depth/task/bytes/deadline/budget과 terminal reason을 가진다. target Bot의 Scheduler/Memory를 Channel이 직접 mutate하지 않는다. cross-aggregate coordination이 필요할 때만 Durable Process를 사용한다.

## 6. Channel Memory

Channel Shared Memory는 proposal→validation→promotion을 거친다. member read 권한과 publish/declassify 권한을 구분한다. leave/revoke 후 stale cursor/cache가 access를 유지하지 못한다.

## 7. Lifecycle

archive는 신규 join/send/task admission을 차단하고 active collaboration/Task를 policy에 따라 drain/reconcile한다. CLI disconnect나 participant leave는 Channel archive가 아니다.

## 8. 검증 기준

- Channel이 Brain/Core Scheduler/Task owner가 되지 않는다.
- Role/Authority/Authorization 혼용 0.
- duplicate channel send가 duplicate Message를 만들지 않는다.
- collaboration fan-out/queue/buffer가 item+byte bound를 가진다.
- revoke 후 stale cursor/access denied.
- Channel archive가 linked Task를 silent terminal 처리하지 않는다.
