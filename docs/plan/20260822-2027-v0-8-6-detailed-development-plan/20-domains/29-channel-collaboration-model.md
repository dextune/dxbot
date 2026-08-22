---
title: "Channel Collaboration과 Membership 모델"
document_id: "DXB-DOM-029"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-DOM-025", "DXB-DOM-028", "DXB-DOM-027"]
---
# Channel Collaboration과 Membership 모델

Channel은 Project 안의 message, membership, Role/Authority, collaboration, Shared Memory boundary이며 Brain이나 Scheduler queue가 아니다.

## P0 Channel state

P0 public lifecycle은 `Active`만 노출한다. Channel archive/restore/delete는 P0 CLI와 state transition에서 제외한다. Project archive는 current policy로 Channel 신규 mutation을 차단하지만 Channel canonical lifecycle을 암묵 변경하지 않는다.

## Channel Membership canonical row

```text
key = (ChannelId, MemberBotId)
MembershipGeneration
RoleRef
AuthorityBindingRef
State = Active | Revoked
```

Channel aggregate가 uniqueness와 generation을 소유한다. set/remove는 Channel revision과 membership CAS를 검증한다. AuthorityBinding은 Security owner가 current Project/Channel policy에서 발급하며 client Role 문자열이 mutation 권한을 만들지 않는다.

## Message와 collaboration

Channel send는 Conversation owner의 `SendMessage`를 사용한다. message append가 Task delegation이나 Memory promotion을 암묵 생성하지 않는다.

collaboration은 participant/fan-out/task/bytes/deadline/budget을 bounded하게 적용한다. target Bot Scheduler/Memory를 Channel이 직접 mutate하지 않는다. revoke 후 신규 delegation/result publication을 거부하고 stale cursor/cache도 current authorization을 통과해야 한다.
