---
title: "Multi-Bot Network와 협업"
document_id: "DXB-DOM-025"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P1"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-020", "DXB-DOM-023", "DXB-ARC-014"]
---

# Multi-Bot Network와 협업

## 1. 목적

독립 Identity와 Memory를 가진 Bot들이 요청·위임·결과를 durable하게 교환하고, Parent Task의 Waiting Continuation과 재시작 복구가 중복 없이 연결되게 한다.

## 2. 불변조건

- Bot은 서로의 Brain/Working Context/Canonical Memory를 직접 공유하지 않는다.
- 위임은 메시지와 Target Task를 통해 수행한다.
- source 권한은 target에 자동 상속되지 않는다.
- delivery는 at-least-once일 수 있으므로 Message/Task/Result가 idempotent해야 한다.
- delegation cycle/fan-out/deadline/budget을 제한한다.
- Parent가 delegated child를 기다리면 해당 child/message reference를 Task Continuation에 보존한다.

## 3. Message Envelope

MessageId, source/target BotId, type/version, correlation/causation, source Task/Execution, target Task hint/ref, delegation depth, deadline, capability requirement, permission token, payload/Artifact refs, digest, sequence/idempotency metadata를 가진다.

## 4. Delegation 흐름

1. Source Task가 delegation intent를 결정
2. durable Outbox에 Message 저장
3. Target inbox가 MessageId로 dedup
4. target authorization/capability validation
5. Target Task를 idempotent하게 생성
6. Source Parent가 기다려야 하면 `Waiting + Continuation(pending delegation)` Commit
7. Target Task 수행
8. Result Message/Artifact 발행
9. Source가 Result를 dedup하고 Continuation pending set에서 제거
10. wait condition 완료 시 Parent를 한 번 resume

Transport callback 자체가 Parent resume authority가 아니다.

## 5. Restart / Loss / Duplicate

- outbox sent 후 ack loss: MessageId로 재전송/중복 수신 허용
- target Task 생성 후 reply loss: correlation/query로 reconcile
- source crash: Waiting Continuation에서 pending delegation 복원
- target crash: Target Task state로 복구
- duplicate Result: result/message ID + continuation revision으로 1회 반영
- late Result after cancellation: history/audit로 보존하되 terminal Parent를 다시 열지 않음

## 6. Capability Discovery

Discovery는 authorized catalog/projection이며 권한 부여가 아니다. target이 특정 Provider를 사용하는지는 source Bot이 내부 구현으로 가정하지 않는다. 위임에는 필요한 Capability semantic을 표현한다.

## 7. 보안

Delegation token은 audience, scope, task, expiry에 묶는다. target은 자신의 policy로 input/Artifact/Memory reference를 재검증한다. secret 원문과 source ambient authority 전달을 금지한다.

## 8. Resource / Cycle

- max delegation depth
- per-source/target rate
- fan-out budget
- payload bytes
- deadline propagation
- visited correlation/path
- loop detection
- target unavailable retry/backoff

정확한 상한은 Resource/Policy SSOT를 참조한다.

## 9. 검증 기준

- Message duplicate/loss/restart에서 Target Task가 하나다.
- Parent crash 후 completed child는 재요청하지 않고 pending delegation만 기다린다.
- duplicate Result가 Parent를 두 번 resume하지 않는다.
- A→B→A cycle이 정책 제한을 우회하지 않는다.
- source permission이 target에 자동 상속되지 않는다.
