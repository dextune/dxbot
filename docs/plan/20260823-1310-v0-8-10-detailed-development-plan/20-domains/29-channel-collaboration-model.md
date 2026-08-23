---
title: "Channel Collaboration과 Membership 모델"
document_id: "DXB-DOM-029"
version: "0.8.10"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-DOM-025", "DXB-DOM-028", "DXB-DOM-027"]
---
# Channel Collaboration과 Membership 모델

Channel은 Project 안의 message/membership/Shared Memory collaboration boundary다. P0 public lifecycle은 Active만 노출한다.

Membership canonical key는 `(ChannelId, MemberBotId)`이며 generation CAS를 사용한다. set/remove는 Channel revision과 membership generation을 검증한다. Membership row, AuthorityBinding create/revoke, AuditIntent는 하나의 application Unit of Work에서 atomic commit한다.

Channel create는 initial-member batch 입력을 받지 않는다. 멤버 구성은 기존 `channel member set` operation을 재사용해 책임과 실패 의미를 단일화한다.

Channel send/history의 user-facing target은 `ChannelSelector`다. Application은 ChannelId와 결박된 Channel ConversationId/revision을 materialize하고 Conversation owner의 SendMessage/ListMessages를 호출한다. CLI가 내부 ConversationId를 사용자에게 요구하지 않으며 collaboration fan-out은 bounded하다.
