---
title: "Persistent Conversation과 Thread 모델"
document_id: "DXB-DOM-027"
version: "0.8.10"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-DOM-020", "DXB-ARC-014"]
---
# Persistent Conversation과 Thread 모델

Bot Main Conversation과 Channel Conversation은 durable parent이며 Thread는 명시적 lineage/context branch다. CLI/control/provider session과 분리한다.

```text
ConversationParentRef
├─ BotMainConversation(BotId, ConversationId)
└─ ChannelConversation(ProjectId, ChannelId, ConversationId)
```

Bot당 Main Conversation은 정확히 하나다. user-facing `ConversationSelector`는 canonical ConversationId 또는 Bot selector를 허용하며, Bot selector는 해당 Bot의 Main Conversation 하나로만 materialize된다. Channel command는 Channel selector에서 Channel Conversation을 materialize한다. 이 lookup은 authorization과 ambiguity 검사를 우회하지 않는다.

## P0 Thread 상태

P0 Thread는 생성 후 persistent open lineage다. archive/delete/close lifecycle은 P0 state machine과 CLI에 넣지 않는다. branch는 source Thread/Message revision을 pin하고 새 ThreadId를 생성하며 transcript를 mutable clone하지 않는다.

Message append는 parent/thread expected revision과 authorization을 검증한다. CLI가 selector로 preflight한 revision도 Runtime에서 다시 검증한다. History는 snapshot pagination이고 Memory promotion을 만들지 않는다.

Thread selector는 parent scope에 결박된다. canonical ThreadId가 parent를 이미 결박하므로 user-facing command에 redundant ConversationId를 강제하지 않는다. implicit last-used thread는 UX hint일 뿐 mutation authority가 아니다. Thread나 CLI exit가 linked Task를 암묵 cancel하지 않는다.
