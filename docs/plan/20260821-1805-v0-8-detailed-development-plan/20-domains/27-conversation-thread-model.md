---
title: "Persistent Conversation과 Thread 모델"
document_id: "DXB-DOM-027"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-020", "DXB-ARC-014"]
---

# Persistent Conversation과 Thread 모델

## 1. 목적

Bot Main Conversation과 Channel Conversation을 durable parent로, Thread를 명시적 branch/working lineage로 정의하고 CLI/control session과 분리한다.

## 2. Parent 모델

```text
ConversationParentRef
├─ BotMainConversation(BotId, ConversationId)
└─ ChannelConversation(ProjectId, ChannelId, ConversationId)
```

Bot당 Main Conversation은 하나다. Main Conversation은 Thread 0, CLI session, Provider session이 아니다.

## 3. Thread

```text
ThreadId / Revision
ConversationParentRef
ParentThreadRef? / BranchPointMessageRef?
Message watermark
Task/Artifact/Memory scope refs
Lifecycle
```

branch는 source thread/message revision을 pin하고 새 ThreadId를 생성한다. 기존 transcript를 mutable clone으로 복사하지 않고 lineage/reference를 사용한다.

## 4. Message와 History

Message append는 `SendMessage` operation을 사용하고 parent/thread expected revision과 authorization을 검증한다. History는 snapshot pagination이며 full transcript materialization을 요구하지 않는다. history read가 Memory promotion을 만들지 않는다.

## 5. Selector

Conversation/Thread selector는 parent scope와 결박된다. 동일 thread name/alias가 여러 parent에 있을 때 scope 없는 mutation은 ambiguous error다. implicit last-used thread는 UX default로 표시할 수 있어도 mutation authority가 아니다.

## 6. Task 관계

Thread는 0..N Task reference를 가질 수 있으나 Task owner가 아니다. Thread archive, Channel archive, CLI exit가 linked Task를 암묵 cancel하지 않는다. 별도 policy/explicit command가 필요하다.

## 7. Recovery와 Access

Runtime/endpoint/provider session restart 후 ConversationId, ThreadId, lineage, message watermark를 canonical storage에서 복구한다. revoked Channel principal의 stale cursor/cache는 current authorization을 통과하지 못한다.

## 8. 검증 기준

- reconnect 후 동일 Main Conversation/Thread identity.
- transport/session/process ID를 ConversationId/ThreadId로 사용 0.
- branch source revision conflict가 explicit error.
- large history가 snapshot page와 stable tie-breaker로 조회됨.
- CLI exit가 Conversation/Thread/Task lifecycle mutation을 만들지 않음.
