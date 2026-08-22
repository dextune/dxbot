---
title: "Multi-Bot Network와 Delegation"
document_id: "DXB-DOM-025"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-DOM-020", "DXB-DOM-023", "DXB-ARC-010"]
---
# Multi-Bot Network와 Delegation

독립 Persistent Bot은 typed Message/Request/Delegation/Result/Evidence로 상호작용한다. Bot A가 Bot B를 호출해도 B의 Identity/Memory가 A의 child state가 되지 않는다.

## Delegation

Delegation은 recipient Bot이 수락할 durable Task intent다. sender가 recipient Scheduler/Core/Memory Store를 직접 조작하지 않는다. recipient lifecycle, authorization, capability, admission을 검증해 accepted/rejected/deferred receipt를 반환한다.

## Sender binding

- **Bot-executed delegation**: sender Bot은 current Execution과 ActionGrant에서 server-side로 파생한다. client payload가 지정하지 않는다.
- **Operator-issued delegation**: CLI의 `requested_sender_bot_id`는 요청일 뿐이다. Runtime이 current principal의 act-as authority를 검증하고 `ResolvedSenderContext`를 receipt/audit에 기록한다.

requested sender와 resolved sender가 다르거나 권한이 없으면 reject한다. BotId 문자열만으로 sender authority가 생기지 않는다.

## Bounded collaboration

fan-out, depth, participants, concurrent Tasks, message/event bytes, deadline, retry, terminal reason을 policy로 제한하고 process-wide resource accounting을 적용한다.

Bot-to-Bot result는 recipient Memory로 자동 복사하지 않는다. Evidence/Proposal로 전달하고 scope owner가 검증한다. duplicate delegation은 duplicate Task를 만들지 않는다.
