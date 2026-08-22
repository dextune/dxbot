---
title: "Channel Collaboration Orchestration"
document_id: "DXB-RUN-037"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-RUN-030", "DXB-RUN-031", "DXB-DOM-029"]
---

# Channel Collaboration Orchestration

## 1. 목적

Project/Channel 협업을 bounded Task/Delegation/Message/Process 흐름으로 실행하면서 Membership/Authority/Memory/Task state를 orchestration layer에 복제하지 않는다.

## 2. 입력과 소유권

Orchestrator는 current Project/Channel revision, participant Membership/Authority generation, collaboration policy, Task/Delegation refs를 읽고 owner command를 호출한다. 자체 canonical ACL, Task table, Memory copy를 만들지 않는다.

## 3. Collaboration Run

필요한 경우 다음 coordination metadata를 소유한다.

```text
CollaborationRunId
Project/Channel refs
Participant refs + pinned generations
Goal/Task/Process refs
fan-out/depth/bytes/deadline/budget
state / terminal reason
```

각 participant Bot의 Identity/Memory/Scheduler state는 reference만 가진다.

## 4. Bounded fan-out와 Backpressure

- global/scope/run별 participant/task/message/event item+byte cap
- admission before delegation
- per-recipient delivery idempotency
- slow/unavailable participant의 bounded retry/defer/reject
- cycle/depth detection
- deadline/budget terminal reason

Channel queue를 unbounded work queue로 사용하지 않는다.

## 5. Authorization와 정보 흐름

각 delegation/message/result/promotion 단계에서 current authorization를 재검증한다. pinned generation은 audit/correlation에 사용하지만 revoke를 무시하는 권한 token이 아니다. Private result의 Channel Shared publication은 Declassification을 요구한다.

## 6. Failure와 Recovery

participant failure, duplicate result, Runtime restart, membership revoke, process crash를 Task/Process/receipt로 reconcile한다. CLI subscriber loss는 collaboration failure가 아니다. terminal decision은 하나며 child state는 owner에서 조회한다.

## 7. 검증 기준

- orchestration layer의 duplicate Membership/Task/Memory canonical state 0.
- fan-out/queue/buffer가 bounded.
- revoke 후 신규 delegation/result publication denied.
- duplicate delivery/result semantic effect 0.
- CLI disconnect가 Collaboration Run terminal reason을 만들지 않음.
