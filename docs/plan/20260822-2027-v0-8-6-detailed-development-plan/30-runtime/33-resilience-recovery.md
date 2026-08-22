---
title: "오류·복구·회복성"
document_id: "DXB-RUN-033"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-RUN-030", "DXB-ARC-015"]
---
# 오류·복구·회복성

## 1. Staged startup

```text
ProcessStarted
→ path/config/permission validation
→ instance lock + HostGeneration fencing
→ storage schema/migration validation
→ canonical journal/identity recovery
→ receipt/directive/side-effect/process/audit reconcile
→ resource/recovery headroom
→ Provider Host/reference provider readiness
→ RuntimeReady
→ Control Endpoint bind/auth
→ ControlReady
```

## 2. Operation recovery

response loss/CLI crash 후 `OperationId`, `CommandId`, authenticated principal의 IdempotencyKey로 조회한다. `ClientRequestId`는 recovery key가 아니다.

- acceptance horizon 안 same key/digest는 existing outcome을 반환한다.
- same key/different digest는 conflict다.
- horizon 밖 epoch-bearing key는 `idempotency-key-expired`로 거부한다.
- receipt retention이 끝났더라도 expired key를 새로운 mutation으로 해석하지 않는다.
- `RecoveryRequired`는 explicit reconcile과 expected receipt revision을 요구한다.

## 3. CLI local recovery

PreparedUnsent는 original typed payload와 exact digest를 다시 제공한 경우만 송신할 수 있다. SentUnknown은 lookup 후 처리한다. blind replay를 금지한다. Abandoned는 local-only이며 Runtime effect가 없음을 확인하거나 송신 전이어야 한다.

## 4. Cursor/subscription/export

expired/foreign/authorization-changed/incompatible cursor는 explicit gap/resync다. connection-local buffer는 replay source가 아니다. export failure는 complete destination을 노출하지 않는다.

## 5. RuntimeReady 이전 status/doctor

- Host source: service/process/descriptor/InstanceId/HostGeneration
- Bootstrap source: last durable startup stage와 safe error class
- Control source: ControlReady 이후 Runtime/Domain/Provider/Projection status

Control endpoint가 없을 때 host-lifecycle client는 bounded host/bootstrap summary만 반환하며 Domain repair를 수행하지 않는다.

CLI crash/SIGINT/broken pipe/endpoint recreation은 Bot/Task/Process terminal transition이 아니다.
