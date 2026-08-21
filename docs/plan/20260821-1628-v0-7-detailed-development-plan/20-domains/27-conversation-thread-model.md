---
title: "Persistent Conversation과 Thread 모델"
document_id: "DXB-DOM-027"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-GOV-002"]
---

# Persistent Conversation과 Thread 모델

## 1. 목적

v0.6의 Persistent Bot Main Conversation + Conversation Parent + Thread Graph 의미를 그대로 보존하고, CLI/control connection이 Conversation/Thread identity로 오인되지 않도록 v0.7 Interface 경계를 추가한다.

## 2. Conversation Parent

```text
ConversationParentRef
├─ BotMainConversation(BotId/ConversationId)
└─ ChannelConversation(ProjectId/ChannelId/ConversationId)
```

exact physical enum/FK는 ADR 대상이다.

## 3. Bot Main Conversation

`Persistent Bot → Main Conversation → Threads`는 계속 P0 Canonical path다. Main Conversation은 Thread 0, Channel, CLI Session, Control Connection이 아니다.

CLI invocation이 종료되거나 새 shell에서 재접속해도 동일 Bot의 Main Conversation identity는 유지된다.

## 4. Channel Conversation / Thread

Channel은 협업 Message surface와 Thread parent relation을 가질 수 있으나 Brain을 소유하지 않는다. Thread는 자체 ThreadId/revision/lineage를 유지하고 Conversation Parent, Message refs, Memory Scope relation, Task/Artifact refs를 가진다.

## 5. History / Memory

Conversation History는 Memory가 아니다. `dxb conversation history`/`thread history`/`channel history`가 Message를 읽는다고 Bot/Project/Channel/Thread Memory가 자동 생성되지 않는다.

## 6. CLI Send / History / Branch

모든 mutation/read는 Application Contract를 통과한다.

- `send`: parent/thread revision과 authorization을 보존
- `history`: cursor pagination과 bounded window 사용
- `branch`: source revision을 pin하고 새 Thread identity 생성
- CLI transport/session/connection ID를 ThreadId로 사용하지 않음
- CLI local history cache를 Canonical transcript로 사용하지 않음

## 7. Thread / Task

- Thread 0..N Task relation 유지
- Channel Thread도 Task owner가 아님
- Channel archive/CLI exit가 Thread-linked Task를 암묵 cancel하지 않음
- Task cancel은 explicit Canonical Command로만 수행

## 8. Access / Revocation

Channel Conversation/Thread history read는 current Project/Channel authorization을 확인한다. revoked participant의 stale cursor/cache는 final auth를 통과하지 못한다.

## 9. Recovery / Reconnect

restart/reconnect 시 ParentRef, ThreadId/revision/lineage, message watermark, scoped Memory relation, Task refs를 Canonical source에서 복구한다.

```text
Provider Session loss
CLI Process restart
Control Endpoint recreation
```

모두 Conversation/Thread identity 변경 사유가 아니다.

## 10. Migration

v0.6 ID/revision/ParentRef를 그대로 보존한다. v0.7 Interface 도입 때문에 새 ConversationId/ThreadId를 발급하지 않는다.

## 11. 검증 기준

- AT-CONV/THREAD 기존 의미 유지.
- 새 CLI/control session으로 reconnect해도 동일 Conversation/Thread identity 조회 가능.
- terminal/control connection ID를 ThreadId로 사용하지 않음.
- CLI exit가 Task/Conversation/Thread lifecycle mutation을 만들지 않음.
- history pagination이 full transcript materialization을 요구하지 않음.
