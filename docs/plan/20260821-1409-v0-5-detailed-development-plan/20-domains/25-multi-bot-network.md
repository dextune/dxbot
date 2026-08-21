---
title: "Multi-Bot Network와 협업"
document_id: "DXB-DOM-025"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P1"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-020", "DXB-DOM-022", "DXB-DOM-023", "DXB-ARC-014"]
---

# Multi-Bot Network와 협업

## 1. 목적

v0.4의 durable delegation/idempotent result/Waiting Continuation을 유지하고 Channel collaboration에서도 Bot 간 직접 Brain/Canonical Memory 공유 없이 협업하도록 한다.

## 2. 불변조건

- Bot A와 Bot B는 서로의 Brain/Working Context/Bot-global Canonical Memory를 직접 공유하지 않는다.
- 위임은 durable Message + target Task로 수행한다.
- source ambient authority는 target에 자동 상속되지 않는다.
- delivery는 at-least-once일 수 있으므로 Message/Task/Result가 idempotent하다.
- delegation depth/fan-out/deadline/budget은 bounded다.
- Shared Memory가 필요하면 Project/Channel Scope를 독립적으로 읽고 쓴다.

## 3. Channel과 Bot Network 관계

Channel은 transport나 Task engine을 대체하지 않는다.

```text
Channel Manager intent
→ target participant resolution
→ durable delegation via Bot Network
→ target Bot Task
→ result message/artifact
→ Manager/Supervisor review
```

Channel Message text가 곧 Bot Network delivery나 Task create commit이 아니다.

## 4. Shared Scope Memory

허용:

```text
Channel Memory
  ↑      ↑
 read   read
  │      │
Bot A  Bot B
```

금지:
`Bot A Global Memory ↔ Bot B Global Memory` 직접 merge/share.

Shared Scope publication에는 별도 write authority와 MemoryProposal을 사용한다.

## 5. Delegation Envelope 확장

기존 MessageId/source/target/correlation/deadline/capability/payload에 필요 시:
- ProjectId / ChannelId / ThreadId provenance
- SupervisorRef
- membership/authority generation reference
- publication/result destination hint

를 포함할 수 있다. 이 메타데이터가 permission을 자동 부여하지 않는다.

## 6. Revocation

Target Bot의 Channel membership이 revoke되면 신규 Channel-scoped input/Memory read/control은 거부한다. 이미 durable하게 생성된 Task의 처리 정책은 Task/Security owner가 결정하며 membership row 삭제만으로 Task history를 삭제하지 않는다.

## 7. 검증 기준

- AT-NET-001 기존 duplicate/loss/restart semantics 유지.
- AT-COLLAB-001 Manager→Researcher→Reviewer workflow가 durable delegation으로 수행.
- Shared Memory 사용 중에도 Bot global Memory 직접 공유 0.
- revoked participant가 새 Channel-scoped delegation을 승인받지 못함.
