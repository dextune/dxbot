---
title: "오류·복구·회복성"
document_id: "DXB-RUN-033"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-015", "DXB-ARC-017", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-030"]
---

# 오류·복구·회복성

## 1. 목적

v0.5 crash/restart/Provider Session loss/index staleness recovery를 유지하면서 Durable Process, ActionGrant, Memory retraction/revalidation, Runtime Memory pressure/OOM-kill의 복구 의미를 추가한다.

## 2. Staged Startup Recovery

1. process/config/schema/migration marker 검증
2. 최소 control/recovery headroom 확보 및 Resource Governance 초기화
3. Bot/Project/Channel/Conversation/Thread identity/revision metadata 복원
4. Durable Process identity/definition version/progress/terminal/reconciliation state 복원
5. Membership/Role/Authority/ActionGrant current revision 복원
6. Memory Scope/epistemic/temporal/dependency/retraction metadata 검증
7. inbox/outbox/idempotency/Side Effect reconcile
8. Task/Execution/Continuation/Directive reconcile
9. Process step↔Activity outcome divergence reconcile
10. Waiting/Suspended transient reservation은 복원하지 않고 재-admission 대기
11. heavy history/Memory/index/projection은 lazy/on-demand 또는 bounded background rebuild
12. Provider config/lifecycle rebuild; Provider Session은 optional optimization으로 재생성
13. Presence/summary/index projection을 bounded하게 rebuild
14. readiness를 단계적으로 노출

모든 Bot/Project/Channel/Memory/index를 startup에서 동시에 hot materialize하지 않는다.

## 3. Durable Process Recovery

- committed Activity outcome이 있으면 replay는 이를 재사용하고 Provider/Tool을 재호출하지 않는다.
- pending command가 이미 child Aggregate에 적용되었는지 stable causation/idempotency key로 reconcile한다.
- Process step과 child Task/Memory state가 diverge하면 child canonical result를 reference로 재평가하고 state copy로 덮지 않는다.
- unknown/unresolvable 상태는 RecoveryRequired로 격리할 수 있다.
- terminal Process의 late result는 audit/reconcile하되 implicit resume 금지.

## 4. ActionGrant Recovery

- grant remaining use/budget/revoke/exhaust state는 durable source에서 복원한다.
- retry/restart가 consumption을 초기화하지 않는다.
- stale approval/policy revision으로 high-risk action을 재개하지 않는다.

## 5. Memory Retraction / Revalidation Recovery

- partial retraction propagation은 source state + dependency relation에서 revalidation queue를 재구성한다.
- dependent를 blind delete/Verified 복원하지 않는다.
- quarantine/revalidation backlog는 bounded background budget을 통과한다.
- missing/corrupt provenance는 recovery-required/quarantine이지 blind duplicate가 아니다.

## 6. Runtime Memory Recovery

active MemoryReservation/permit은 crash 후 durable truth가 아니다.
- startup에서 permit을 복원하지 않는다.
- Canonical Task/Execution/Process/Continuation 상태를 보고 새 admission을 수행한다.
- projection/index rebuild concurrency도 global Runtime Memory budget을 통과한다.
- Provider reconnect가 모든 Bot Context를 선행 생성하지 않는다.
- recovery 중 safety/control/reconciliation headroom을 일반 work에 양보하지 않는다.

## 7. OOM / Host Kill

OOM을 정상 제어 이벤트로 기다리지 않지만 host/container hard kill은 가능한 failure mode다.
- Canonical commit/Side Effect write-ahead 경계를 유지해 restart 가능해야 한다.
- Emergency pressure에서 정상 회복이 불가능하면 durable state를 보존한 fail-stop/restart를 허용한다.
- allocation-heavy crash dump/log가 recovery를 악화시키지 않게 한다.

## 8. Doctor 추가 진단

- stuck/recovery-required Process와 missing outcome/command ref
- Process terminal 뒤 late unauthorized activation
- exhausted/revoked ActionGrant의 pending action
- Memory retract propagation divergence
- quarantine/provenance chain break
- active/waiting Task 대비 orphan Runtime reservation telemetry
- repeated recovery rebuild peak memory
- task/subscriber/stream/permit count baseline 비회귀

기존 Project/Channel/Thread/Directive/Side Effect/Provider/Plugin 진단도 유지한다.

## 9. 검증 기준

- AT-PROC-001 crash window 전부에서 duplicate child/Activity/promotion/Side Effect 0.
- AT-SEC-004 restart 후 grant over-consumption 0.
- AT-MEM-008 partial revalidation state 복원 가능.
- AT-RMEM-002 외부 OOM-kill 뒤 Canonical recovery 가능.
- AT-RMEM-003 recovery/reconnect 반복 후 retained leak 0.
- Provider Session loss가 identity/Process/Memory truth를 변경하지 않음.
