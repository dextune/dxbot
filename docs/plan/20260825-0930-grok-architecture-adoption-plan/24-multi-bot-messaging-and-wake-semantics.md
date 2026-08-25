---
title: "Multi-Bot Messaging과 Fresh Wake Semantics 보완 계획"
document_id: "DXB-ADP-024"
version: "0.1.0"
status: "Reference Snapshot"
normative: false
priority: "P1"
last_updated: "2026-08-25"
depends_on: ["DXB-ADP-010"]
target_owners: ["application", "dxbot-core", "runtime-host", "runtime-security", "application-contract"]
package_path: "docs/plan/20260825-0930-grok-architecture-adoption-plan"
source_baseline:
  dxbot_commit: "e43739614631c95752482c2ad2ec53cb7ebd251f"
  grok_reconstructed_commit: "a9f633e09d49a85829b8236331b9e21f7e612634"
---
# Multi-Bot Messaging과 Fresh Wake Semantics 보완 계획

## 1. 판단

**분류: Close.** DXBOT은 Delegation을 recipient Bot의 durable Task intent로 정의한다. Grok의 `SendToAgent` 패턴에서 차용할 부분은 별도의 일반 Message delivery 의미다.

핵심은 다음이다.

- 보내기는 durable commit 후 즉시 acknowledgement를 반환
- recipient의 답변을 같은 turn에서 기다리지 않음
- 수신은 recipient를 이후의 fresh Execution으로 깨움
- polling이나 synchronous call chain을 만들지 않음
- group/fan-out은 명시적 bound와 authorization을 가짐

## 2. Message와 Delegation 분리

| 의미 | Message | Delegation |
|---|---|---|
| 목적 | 정보·질문·알림 전달 | recipient가 수행할 durable Task 생성 |
| commit effect | Message append + delivery intent | recipient Task intent |
| sender wait | 금지 | Task watch는 별도 query/subscription |
| recipient activation | wake policy가 fresh Execution을 요청할 수 있음 | Task admission이 Execution을 요청 |
| 결과 | optional reply Message | Task Result/Evidence |
| idempotency | duplicate Message effect 금지 | duplicate Task effect 금지 |

Message를 Task로 과장하지 않고, Task를 chat message로 축소하지 않는다.

## 3. Delivery state

```text
MessageCommitted
→ DeliveryPending
→ Delivered
→ WakeEligible
→ WakeRequested
→ WakeAdmitted | WakeDeferred | WakeRejected

DeliveryPending → DeliveryFailedRetryable | DeliveryFailedTerminal
```

Message commit 성공과 recipient Execution 성공을 하나의 receipt 상태로 합치지 않는다.

## 4. Sender contract

Send operation의 terminal result는 최소 다음을 구분한다.

- message committed
- delivery intent created
- immediate recipient wake가 요청되었는지
- operation may continue 여부

recipient reply 또는 task result를 기다리지 않는다. `--wait`가 있더라도 Message commit/delivery 상태까지만 의미를 제한하고 reply wait 기능을 암묵 추가하지 않는다.

## 5. Recipient wake

Wake는 새로운 Execution이다.

```text
committed inbound message
→ recipient lifecycle/authorization check
→ wake dedupe/coalescing
→ admission
→ immutable Context Plan with message ref
→ fresh Execution
```

- sender Execution의 model/tool state를 공유하지 않는다.
- recipient의 Context Plan은 message 원문을 무조건 전체 materialize하지 않는다.
- 같은 burst의 메시지는 policy에 따라 bounded coalescing할 수 있다.
- wake deferred가 Message delivery 실패를 의미하지 않는다.
- recipient suspended/deleted/provider-unready 상태를 각각 구분한다.

## 6. Reply와 ping-pong 방지

- reply는 새 Message operation이다.
- 자동 acknowledgement reply를 생성하지 않는다.
- 동일 두 Bot 간 자동 reply depth 또는 causal chain을 bound한다.
- wake payload에 causal message ID와 hop count를 포함한다.
- 정책상 의미 없는 ack-only 응답을 억제할 수 있으나 LLM 출력 규칙에 correctness를 의존하지 않는다.

## 7. Group/Channel fan-out

Channel send는 하나의 canonical Message append다. member별 canonical 메시지를 복제하지 않는다.

```text
Channel Message
→ bounded member delivery projection/intents
→ per-recipient authorization/lifecycle
→ wake policy
```

- participant/item/byte/deadline budget을 가진다.
- membership revision을 commit 시 pin한다.
- membership 변화가 이미 committed Message를 rewrite하지 않는다.
- N명 fan-out을 N개의 synchronous provider call로 실행하지 않는다.
- overflow는 partial/continuation 또는 deferred delivery로 명시한다.

## 8. Privacy와 Information Flow

- sender Bot의 private conversation 원문을 다른 Bot에 자동 전달하지 않는다.
- Message content는 explicit user/Bot action과 ActionGrant에 결박한다.
- cross-scope 전달은 label/declassification policy를 적용한다.
- wake Context에는 수신 scope에서 허용된 content만 포함한다.
- diagnostic/audit에는 raw message 대신 digest와 safe metadata를 사용한다.

## 9. Idempotency와 recovery

- MessageId 또는 command idempotency key가 canonical duplicate guard다.
- response loss 후 lookup으로 동일 Message를 확인한다.
- delivery worker crash 후 pending intent를 재개한다.
- delivered marker와 wake request가 atomic하지 않다면 각각 idempotent effect를 가진다.
- stale membership/provider state에서 무한 retry하지 않는다.

## 10. 구현 작업

1. 기존 Conversation Message와 Delegation owner를 inventory한다.
2. Message commit, delivery, wake를 별도 state/effect로 문서화한다.
3. single recipient vertical slice를 구현한다.
4. duplicate/response-loss/crash-between-delivery-and-wake fault test를 추가한다.
5. recipient fresh Execution의 Context Plan pinning을 검증한다.
6. Channel fan-out은 기존 membership owner와 bounded batch를 재사용한다.
7. public DTO 변경은 M3/M4 acceptance 전까지 freeze하지 않는다.

## 11. Acceptance

- sender operation은 recipient reply를 기다리지 않는다.
- duplicate retry가 duplicate Message 또는 duplicate wake Execution을 만들지 않는다.
- recipient wake는 fresh Execution이며 sender local state를 공유하지 않는다.
- suspended recipient가 Message commit을 소급 취소하지 않는다.
- Channel fan-out이 participant/byte/deadline bound를 넘지 않는다.
- privacy label과 membership revision이 전달 시 검증된다.
- crash/restart 후 pending delivery가 정확히 한 effect로 수렴한다.
