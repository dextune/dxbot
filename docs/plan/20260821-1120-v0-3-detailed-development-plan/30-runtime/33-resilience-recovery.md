---
title: "오류·복구·회복성"
document_id: "DXB-RUN-033"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-015", "DXB-ARC-017", "DXB-RUN-030"]
---

# 오류·복구·회복성

## 1. 목적

Provider/Tool/Storage/Plugin 장애와 crash/partial outcome에서도 Bot 지속성을 보존하고 Provider lifecycle/retry/reconcile 의미를 Common Framework에서 일관되게 복구한다.

## 2. Startup Recovery 순서

1. process ownership/config/schema 검증
2. storage integrity/migration marker
3. inbox/outbox/idempotency
4. Side Effect Ledger Unknown/Prepared reconcile queue
5. Core Lease/Execution reconcile
6. Waiting Task Continuation 복원
7. Routine occurrence reconcile
8. Artifact/projection/index reconcile
9. Provider config/contract compatibility validation
10. Provider Lifecycle `Declared → Validated → Starting → Ready` 재구축
11. Plugin lifecycle/Provider candidate 재구축
12. Bot lazy activation
13. readiness

외부 Provider session/process state를 persisted Canonical State보다 먼저 신뢰하지 않는다.

## 3. Provider Failure Normalization

Provider-specific error는 Provider/SDK mapping을 거쳐 stable taxonomy로 변환되고 Host가 deadline/cancel/resource/side-effect context와 함께 retry disposition을 계산한다.

- Provider는 semantic retry loop를 소유하지 않는다.
- retry 필요 시 상위 Runtime이 새 Execution/Attempt를 만든다.
- non-idempotent/Unknown Side Effect는 자동 retry 금지.
- Host/platform failure와 Provider failure를 같은 코드로 숨기지 않는다.

## 4. Provider Lifecycle Recovery

- validation/compatibility 실패 → `Incompatible`
- security/protocol violation → `Quarantined`
- health degradation → Common policy가 `Degraded` 판단
- process crash → `Failed` 또는 bounded restart policy
- restart는 Lifecycle Manager가 수행하고 Provider 자체 daemon이 Registry state를 임의 복구하지 않음
- Draining 중 crash → in-flight outcome/Side Effect를 reconcile한 뒤 detach/failed 처리

Provider self-restart가 발생하더라도 old generation을 다시 Ready로 승격하지 않으며 새로운 validated generation으로 등록한다.

## 5. Waiting Task Recovery

Waiting Commit과 Continuation은 같은 논리적 Commit이다. 복구 시 completed dependency dedup, deadline/cancel 재평가, resume guard를 사용한다. Continuation missing/corrupt이면 임의 재실행하지 않고 운영 가능한 RecoveryRequired/Quarantine 상태로 노출한다.

## 6. Side Effect Recovery

- Prepared + 미실행이 확실 → 정책상 실행 가능
- Confirmed → 재실행 금지
- Failed → retry disposition 평가
- 판정 불가 → Unknown
- Unknown → status lookup/idempotency lookup/compensation/manual review
- 결과 → Reconciled + evidence

Provider Host는 ledger/guard 상태가 허용하지 않는 semantic call을 재수행하지 않는다.

## 7. Routine Recovery

persisted occurrence/next trigger를 기준으로 missed/overlap policy와 dedup을 적용한다. Routine이 직접 Provider session을 resume하지 않는다.

## 8. Checkpoint

Task/Execution revision, pending work, Artifact refs, provider resume token+contract/provider version, Side Effect refs, Memory/Policy revisions, checksum을 저장할 수 있다. Working heap dump는 checkpoint가 아니다.

## 9. Doctor

- Waiting without Continuation
- Side Effect Unknown age
- Routine duplicate/next occurrence
- removed/incompatible Provider stale config/reference
- Provider lifecycle stuck in Starting/Draining
- Provider activity/ref leak
- Conformance/contract version mismatch
- Plugin migration marker/orphan dependency

## 10. 검증 기준

- Provider crash/restart가 Registry generation과 in-flight pinning을 위반하지 않는다.
- `Unknown` external outcome이 duplicate mutate를 만들지 않는다.
- Draining crash 후 activity/permit이 누수되지 않는다.
- removed/incompatible Provider가 silent fallback하지 않는다.
- Waiting/Routine recovery 기존 Acceptance를 유지한다.
