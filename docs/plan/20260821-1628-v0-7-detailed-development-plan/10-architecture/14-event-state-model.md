---
title: "Command·Event·State·Projection 모델"
document_id: "DXB-ARC-014"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-GOV-002", "DXB-ARC-010"]
---

# Command·Event·State·Projection 모델

## 1. 목적

v0.6의 `Command → Aggregate Decide → Atomic Journal/State/Outbox Commit → Projection`과 Durable Process/Memory/ActionGrant 의미를 유지하고, v0.7 Control Client의 duplicate/reconnect를 Canonical state/event와 안전하게 연결한다.

## 2. Canonical State 비회귀

Bot/Project/Channel/Conversation/Thread/Task/Execution/Memory/Directive/Side Effect/Durable Process/ActionGrant의 기존 owner는 그대로다.

다음은 Canonical Domain State가 아니다.
- CLI Process/Session
- Control Connection/socket
- event subscriber local buffer
- terminal rendering state
- current spinner/progress line
- Runtime MemoryReservation/pressure probe
- Presence/cache/index

## 3. Public Command Metadata

`DXB-IFC-040`의 mutation Command는 필요한 범위에서 다음 stable 의미를 Canonical command path까지 전달한다.

```text
CommandId
PrincipalRef
Target ResourceRef / Action
Expected Revision / Generation
IdempotencyKey
Correlation / Causation
Deadline
Optional Approval / ActionGrantRef
Payload
```

CommandId/IdempotencyKey는 CLI process-local sequence만으로 만들지 않으며 response-loss retry에서 동일 logical mutation을 식별할 수 있어야 한다.

## 4. Commit / Lost Response

```text
client command
→ validation/authorization
→ aggregate decide
→ atomic canonical commit/outbox/receipt
→ response
```

commit 후 response가 손실되어도 동일 idempotency key retry가 새 canonical effect를 만들지 않는다. client가 성공 여부를 모르는 경우 기존 receipt/outcome 또는 reconciliation query로 확인한다.

## 5. Event / Subscription 경계

Domain Event와 public Subscription Event는 같은 타입일 필요가 없다.

```text
Canonical Event/Projection
→ authorization/filter/mapping
→ public subscription event
→ bounded transport buffer
→ CLI watch/follow
```

public stream은 stable event identity, cursor/watermark, source revision/observed_at, gap/resync 의미를 가질 수 있다. internal Work Queue/Control Channel을 그대로 외부에 expose하지 않는다.

## 6. CLI Disconnect 의미

- CLI disconnect/SIGINT/broken pipe는 Domain Event가 아니다.
- local subscriber 종료가 TaskCancelled/ProcessCancelled를 생성하지 않는다.
- 명시적 cancel/suspend/stop Command만 Canonical state를 바꾼다.
- reconnect는 cursor가 유효하면 resume하고 gap이면 explicit resync한다.

## 7. Projection

v0.6 process/memory/grant/pressure projection에 다음 read 의미를 추가한다.
- command receipt/outcome safe view
- control/runtime capability/version view
- event cursor/watermark state

Projection은 authorization, process progress, Task state, Memory truth의 source가 아니다.

## 8. 검증 기준

- duplicate Control delivery가 duplicate semantic effect를 만들지 않는다.
- response-loss retry가 기존 committed outcome을 회수한다.
- CLI disconnect가 Canonical lifecycle event를 암묵 생성하지 않는다.
- gap/resync 없이 stale stream을 current state로 오판하지 않는다.
- public event mapping이 internal queue/state owner를 노출하지 않는다.
