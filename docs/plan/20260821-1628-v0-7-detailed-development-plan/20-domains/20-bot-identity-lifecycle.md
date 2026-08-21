---
title: "Bot Identity와 Lifecycle"
document_id: "DXB-DOM-020"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-GOV-002", "DXB-ARC-014", "DXB-ARC-015"]
---

# Bot Identity와 Lifecycle

## 1. 목적

v0.4의 Persistent Bot/Main Conversation/Provider-Session independence를 유지하고 Project/Channel 참여가 Bot Global Identity를 변경하지 않도록 한다.

## 2. Bot Aggregate

Canonical:
- BotId / Identity revision / lifecycle
- Brain policy reference
- Bot-global Memory scope reference
- Main Conversation relation
- Goal/Permission/Resource/Routine refs
- aggregate revision/provenance

비Canonical:
- Project/Channel Role 문자열을 Identity Persona로 덮어쓴 값
- active Core 목록
- Channel Presence Runtime
- Interface/Provider Session
- cache/index/live Plugin object

## 3. Project/Channel Participation

Bot은 0..N Project/Channel에 참여할 수 있다. 참여 관계는 Project/Channel Membership이 소유한다.

```text
Bot Identity
+ Project Context
+ Channel Role
+ Channel Authority
+ Thread/Task Context
= 해당 Channel에서의 현재 행동
```

Channel Role 변경은 Bot Identity revision을 자동 생성하지 않는다. 동일 Bot이 서로 다른 Channel에서 다른 Role을 가질 수 있다.

## 4. Lifecycle

v0.4 Provisioning/Inactive/Activating/Active/Degraded/Quiescing/Archived/Deleting/Deleted 의미를 유지한다.

Bot archive/delete는 Project/Channel membership에 명시적 lifecycle policy를 적용한다. Session 종료나 Channel leave를 Bot delete로 취급하지 않는다.

## 5. Activation / Recovery

1. Bot/Identity/Main Conversation 복원
2. Thread Graph/Bot Memory 복원
3. Project memberships 조회
4. Channel memberships/Role/Authority current generation 연결
5. pending Task/Delegation/Directive reconcile
6. Provider capability/lifecycle 재검증
7. Active/Degraded 판정

Membership 자체의 Canonical 복구는 Project/Channel Domain이 소유한다.

## 6. 권한 변화

Identity/Persona 변경과 Membership/Authority revoke를 구분한다. revoke는 신규 scope access/control에 즉시 적용할 수 있으나 running Execution snapshot을 Identity mutation처럼 rewrite하지 않는다.

## 7. 검증 기준

- Channel Role 변경이 Bot Identity revision을 임의 변경하지 않는다.
- Bot이 여러 Channel에 참여해도 하나의 logical Brain semantics를 유지한다.
- Provider Session/Channel Presence 손실이 Bot identity를 손상시키지 않는다.
- Bot-only v0.4 lifecycle fixture가 그대로 통과한다.
