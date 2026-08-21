---
title: "Live Control과 Safe-Point Preemption"
document_id: "DXB-RUN-036"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-RUN-030", "DXB-RUN-032", "DXB-DOM-023"]
---

# Live Control과 Safe-Point Preemption

## 1. 목적

실행 중 Task/Process에 report, redirect, suspend, resume, cancel을 적용하면서 immutable Execution, external Side Effect, safe point, receipt의 의미를 보존한다.

## 2. Typed Directive

```text
ControlDirectiveId
Action: Report | Redirect | Suspend | Resume | Cancel
Target ResourceRef
ExpectedRevision / ExecutionGeneration
Principal / Authority / ActionGrant?
Payload/ArtifactRef?
SubmittedAt
State / OutcomeRef
```

`process control <json>` 같은 범용 mutation은 금지한다. Process view에서 요청하더라도 실제 owner가 Task/Directive/Approval operation을 수행한다.

## 3. Safe Point

즉시 적용 가능한 control과 safe point가 필요한 control을 구분한다.

```text
Accepted
→ Pending/AwaitingSafePoint
→ Applied/Committed | Rejected | Superseded | RecoveryRequired
```

CLI의 local wait timeout은 directive를 rollback하지 않는다. Operation Receipt로 계속 상태를 조회한다.

## 4. Redirect

running Execution Context를 in-place 수정하지 않는다. redirect는 current attempt를 safe-point에서 종료/mark하고 새 TaskRevision/Execution snapshot을 생성하거나 정책상 reject한다.

## 5. Suspend/Resume

suspend는 cancellation request와 실제 quiesced/checkpoint 완료를 구분한다. transient Core/Provider/resource를 release하고 durable continuation/checkpoint만 보존한다. resume은 current authorization/config/capability를 재검증하고 새 generation을 사용한다.

## 6. Cancel

cancel requested 후 in-flight side effect가 unknown이면 terminal Cancelled로 성급히 표시하지 않고 RecoveryRequired/Reconciliation을 사용한다. completion/cancel race winner는 revision/fencing으로 하나다.

## 7. Report

report는 read-like observation을 생성하거나 durable report artifact를 요청하되 Task state를 임의 변경하지 않는다. huge trace/context dump는 P0 기본이 아니며 bounded summary를 우선한다.

## 8. 검증 기준

- safe-point operation이 accepted를 committed로 오표시하지 않음.
- wait timeout 뒤 동일 receipt로 상태 조회 가능.
- redirect가 running Execution snapshot을 mutate하지 않음.
- cancel/side-effect unknown이 blind retry나 false terminal을 만들지 않음.
- generic process-control mutation escape hatch 0.
