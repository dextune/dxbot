---
title: "Multi-Bot Network와 Delegation"
document_id: "DXB-DOM-025"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-020", "DXB-DOM-023", "DXB-ARC-010"]
---

# Multi-Bot Network와 Delegation

## 1. 목적

독립적인 Persistent Bot들이 typed message, request, delegation, result, collaboration으로 상호작용하되 일회성 sub-agent hierarchy나 공유 Brain으로 축약되지 않게 한다.

## 2. Bot 독립성

각 Bot은 고유 BotId, Brain policy, Memory scope, lifecycle, permission/resource profile을 유지한다. Bot A가 Bot B를 호출해도 B의 Identity/Memory가 A의 child state로 복제되지 않는다.

## 3. Interaction 종류

```text
Message
Request / Ask
Delegation
Result / Evidence
Notification
Collaboration proposal
```

각 interaction은 Sender/Recipient, Scope, Correlation/Causation, Authorization context, payload/artifact refs, deadline/resource bound, delivery/outcome를 가진 typed use-case다. generic free-form event를 mutation 권한으로 사용하지 않는다.

## 4. Delegation

Delegation은 target Bot이 수락할 durable Task intent다. sender가 target Scheduler/Core를 직접 조작하지 않는다. target authorization, admission, current lifecycle/capability를 검증하고 accepted/rejected/deferred receipt를 반환한다.

## 5. Collaboration bound

fan-out, depth, participants, concurrent tasks, message/event bytes, deadline, retry, terminal reason을 정책으로 제한한다. cycle detection과 max-depth만으로 충분하지 않으며 process-wide resource accounting을 적용한다.

terminal reason 예:
- completed
- cancelled
- deadline/budget exhausted
- participant unavailable/revoked
- conflict/recovery-required
- policy/resource rejected

## 6. Memory와 정보 흐름

Bot-to-Bot result가 recipient Memory로 자동 복사되지 않는다. Evidence/Proposal로 전달하고 recipient scope owner가 검증한다. Private/Sensitive 정보의 cross-Bot/Shared publication은 Information Flow와 Declassification을 통과한다.

## 7. Failure와 Recovery

duplicate delivery는 duplicate Task/Memory effect를 만들지 않는다. sender/recipient Runtime fault에서 correlation, receipt, Durable Process/Task state로 reconcile한다. recipient removal/archival은 explicit unavailable result를 만들고 silent fallback Bot을 선택하지 않는다.

## 8. 검증 기준

- Bot 호출이 새로운 anonymous sub-agent identity를 만들지 않는다.
- sender가 recipient Scheduler/Core/Memory Store에 직접 접근하지 않는다.
- duplicate delegation이 duplicate Task를 만들지 않는다.
- collaboration fan-out/bytes/task count가 bounded다.
- revoke/participant loss에서 explicit terminal/recovery state를 가진다.
