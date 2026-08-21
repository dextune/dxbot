---
title: "Persistent Conversation과 Thread 모델"
document_id: "DXB-DOM-027"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-GOV-002"]
---

# Persistent Conversation과 Thread 모델

## 1. 목적

v0.4 Bot Main Conversation + Thread Graph를 보존하면서 Thread가 Bot 전용 parent에만 묶이지 않도록 Conversation Parent를 일반화한다.

## 2. Conversation Parent

```text
ConversationParentRef
├─ BotMainConversation(BotId/ConversationId)
└─ ChannelConversation(ProjectId/ChannelId/ConversationId)
```

정확한 physical enum/FK는 ADR 대상이다.

## 3. Bot Main Conversation 보존

`Persistent Bot → Main Conversation → Threads`는 계속 P0 Canonical path다. Project/Channel 사용 여부와 무관하다. Main Conversation은 Thread 0/Channel/UI room이 아니다.

## 4. Channel Conversation

Channel은 협업 메시지 surface와 Thread parent relation을 가질 수 있다. Channel Conversation이 Bot Brain을 소유하지 않는다. 참여 Bot의 발화/응답은 각 Bot identity로 기록된다.

## 5. Thread Identity

Thread는 자체 ThreadId/revision/lineage를 가진다. v0.5 Memory Scope owner 식별을 위해 `BotId + ThreadId` pair만으로 identity를 정의하지 않는다.

Thread는:
- Conversation Parent
- Message sequence/history refs
- Memory Scope relation
- Task/Artifact refs
- lineage/archive/restore metadata

를 가진다.

## 6. History / Memory

Conversation History는 Memory가 아니다. Bot Main Conversation/Channel Conversation의 Message가 자동으로 Bot/Project/Channel/Thread Memory를 만들지 않는다.

## 7. Thread / Task

- Thread 0..N Task relation 유지
- Channel Thread도 Task owner가 아니다
- Channel archive가 Thread-linked Task를 암묵 cancel하지 않는다
- branch는 source revision을 pin하고 새 Thread identity를 만든다
- Channel branch participant inheritance는 OQ/ADR 대상

## 8. Access / Revocation

Channel Conversation/Thread history read는 current Project/Channel authorization을 확인한다. revoked participant의 stale history cursor/cache는 final auth를 통과하지 못한다.

Bot Main Conversation의 권한 모델은 기존 v0.4와 동일하다.

## 9. Recovery

restart 시 ParentRef, ThreadId/revision/lineage, message watermark, scoped Memory relation, Task refs를 복구한다. Provider Session을 Parent/Thread identity로 사용하지 않는다.

## 10. Migration

기존 v0.4 ThreadId를 그대로 보존하고 ParentRef를 기존 Bot Main Conversation으로 재구성한다. 새 ID를 발급하지 않는다.

## 11. 금지 패턴

- 모든 Thread에 Project parent 강제
- Channel을 Thread 0로 표현
- Provider session/channel room ID를 ThreadId로 사용
- Thread Scope를 계속 `BotId+ThreadId`만으로 저장
- Channel message를 participant Bot Global Memory로 자동 commit

## 12. 검증 기준

- AT-CONV-001/THREAD-001 기존 의미 유지.
- Channel Thread가 특정 Manager Bot owner로 잘못 귀속되지 않음.
- migration 후 기존 Thread identity/lineage unchanged.
- Project/Channel revoke가 신규 Channel history access를 차단.
