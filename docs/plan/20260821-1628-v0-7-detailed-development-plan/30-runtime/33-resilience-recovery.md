---
title: "오류·복구·회복성"
document_id: "DXB-RUN-033"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-015", "DXB-ARC-017", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-030"]
---

# 오류·복구·회복성

## 1. 목적

v0.6의 staged startup, Durable Process, ActionGrant, Memory revalidation, Runtime Memory/OOM recovery를 유지하고, v0.7 Runtime Host/Control Endpoint/CLI reconnect 및 **command committed but response lost** 복구 의미를 추가한다.

## 2. Staged Runtime Recovery 비회귀

v0.6 startup 순서와 원칙을 유지한다.
- schema/migration/config 검증
- safety/control/recovery headroom 초기화
- Domain identity/revision 복원
- Durable Process/Grant/Memory metadata 복원
- inbox/outbox/idempotency/Side Effect/Task/Execution reconcile
- transient reservation 복원 금지
- heavy history/index/projection lazy/bounded rebuild
- Provider Session optional recreation
- readiness 단계적 노출

## 3. Runtime Host / Endpoint Recovery

Runtime Host와 Control Endpoint lifecycle은 Domain lifecycle과 분리한다.

- endpoint socket/pipe recreation이 Bot/Thread/Task/Process identity를 변경하지 않는다.
- Runtime restart 후 CLI는 version negotiation을 다시 수행한다.
- endpoint 연결이 끊겨도 already committed Command를 rollback했다고 가정하지 않는다.
- connection-local subscription buffer는 Canonical recovery source가 아니다.

## 4. Command Committed / Response Lost

```text
Command accepted
→ Canonical commit/receipt
→ connection lost before response
→ CLI retry with same idempotency identity
→ existing outcome returned/reconciled
```

duplicate mutation을 만들지 않는다. non-idempotent external effect가 Unknown이면 Side Effect Ledger/Reconciliation을 우선한다.

## 5. Subscription Resume / Gap

- reconnect 시 last acknowledged/observed cursor를 제시할 수 있다.
- retention window 안이면 resume한다.
- gap/expired cursor면 명시적 resync/query를 수행한다.
- gap을 숨긴 채 stream을 current state로 표시하지 않는다.
- reauth/revision check 없이 stale stream을 이어가지 않는다.

## 6. CLI Crash / Broken Pipe

CLI crash/SIGINT/broken pipe는 Runtime fault가 아니다.

- Task/Process/Bot를 암묵 cancel/deactivate하지 않는다.
- local subscription/client buffer를 정리한다.
- server subscriber resource는 disconnect/drop으로 bounded cleanup한다.
- durable command/result는 Runtime에서 유지된다.

## 7. Doctor 진단 확장

기존 v0.6 진단에 추가:
- Runtime Host/Control Endpoint readiness/version
- protocol/schema compatibility mismatch
- orphan/stuck subscriber/connection resource
- command receipt/outcome reconciliation
- event cursor gap/resync requirement
- Runtime restart 후 CLI-visible identity mismatch 여부
- secret를 포함하지 않는 credential/profile validation

read-only diagnosis와 repair mutation을 분리한다. repair는 explicit command/authorization을 요구한다.

## 8. 검증 기준

- Runtime/endpoint restart 후 same Bot/Conversation/Thread/Task/Memory/Process identity 조회 가능.
- committed-response-lost window duplicate effect 0.
- stale cursor gap silent loss 0.
- CLI crash/disconnect가 Runtime lifecycle을 변경하지 않음.
- server/client subscriber cleanup leak 0.
- v0.6 AT-PROC/SEC/MEM/RMEM recovery regression 0.
