---
title: "오류·복구·회복성"
document_id: "DXB-RUN-033"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-RUN-030", "DXB-ARC-015"]
---

# 오류·복구·회복성

## 1. 목적

Runtime/process/endpoint/client가 중단되어도 Canonical Identity, committed mutation, Task/Process/Memory semantic을 durable state에서 복구하고 connection-local 추측이나 silent retry에 의존하지 않는다.

## 2. Staged Startup

```text
ProcessStarted
→ config/path/permission 검증
→ instance lock + HostGeneration fencing
→ storage schema/migration 검증
→ canonical identity/journal 복구
→ inbox/outbox/receipt/side-effect/process reconcile
→ resource/recovery headroom 초기화
→ Provider Host/reference provider 준비
→ RuntimeReady
→ Control Endpoint bind/peer policy
→ ControlReady
```

heavy history/index/projection은 source watermark를 가진 lazy/bounded rebuild로 분리한다.

## 3. Stale Endpoint / Lock Recovery

endpoint/descriptor/lock가 남아 있으면:
1. owner/permission/path no-follow 검증
2. descriptor InstanceId/HostGeneration 비교
3. service manager/process liveness 확인
4. lock acquisition/fencing 성공
5. stale artifact만 제거하고 새 endpoint를 exclusive create

PID 값만으로 stale 판정을 하지 않는다. 다른 data root의 endpoint를 재사용하지 않는다.

## 4. Operation Recovery

Command는 commit과 같은 durability boundary에서 receipt/outcome reference를 기록한다. response loss/CLI crash 후 `operation show/reconcile`로 상태를 조회한다.

- nonterminal receipt는 expiry로 사라지지 않는다.
- terminal receipt/key binding은 최소 30일 보존한다.
- retention 이후에는 `operation-expired`를 반환하고 same key blind replay를 자동 수행하지 않는다.
- Side Effect Unknown은 ledger/reconciliation을 사용한다.

## 5. Cursor / Subscription Recovery

retention 안의 valid cursor는 resume한다. expired/foreign/authorization-changed/version-incompatible cursor는 explicit gap/resync다. connection-local buffer는 replay source가 아니다. partial JSONL은 terminal record에 last safe cursor와 reconciliation hint를 남긴다.

## 6. Export Recovery

cancel, disk full, broken pipe, serialization failure에서 destination를 성공 파일로 노출하지 않는다. temp cleanup을 시도하고 실패 시 restricted path와 cleanup instruction을 diagnostic으로 남긴다. Runtime receipt와 export file success를 동일 transaction으로 가장하지 않는다.

## 7. CLI/Endpoint Failure 의미

CLI crash/SIGINT/broken pipe, endpoint recreation, Provider Session loss는 Bot/Conversation/Thread/Task/Process identity 또는 terminal transition이 아니다. explicit owner command만 lifecycle을 변경한다.

## 8. Doctor와 Reconcile

read-only doctor:
- host/service/instance/endpoint/readiness/version
- storage/recovery/reconcile backlog
- receipt nonterminal/expired statistics
- cursor/projection gap
- subscriber/resource leak indicators
- secret-free profile/config validation

repair mutation은 별도 typed command, authorization, expected revision, Operation Receipt를 요구한다.

## 9. 검증 기준

- crash window별 canonical identity/receipt/process/side-effect 복구.
- stale endpoint/lock가 active instance를 제거하지 않음.
- commit-after-response-loss duplicate effect 0.
- receipt expiry가 blind duplicate mutation을 만들지 않음.
- partial export가 complete file로 노출되지 않음.
- endpoint/CLI restart 뒤 same Domain IDs 조회.
