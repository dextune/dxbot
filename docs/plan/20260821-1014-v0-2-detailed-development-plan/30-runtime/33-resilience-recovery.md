---
title: "오류·복구·회복성"
document_id: "DXB-RUN-033"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-015", "DXB-RUN-030"]
---

# 오류·복구·회복성

## 1. 목적

Provider/Tool/Storage/Plugin 장애와 crash/partial outcome에서도 Bot 지속성과 Waiting/Routine/Side Effect의 재개 의미를 보존한다.

## 2. Startup Recovery 순서

1. process ownership/config/schema 검증
2. storage integrity/migration marker
3. inbox/outbox/idempotency
4. Side Effect Ledger Unknown/Prepared reconcile queue
5. Core Lease/Execution reconcile
6. Waiting Task Continuation 검증 및 subscription 복원
7. Routine occurrence/next trigger reconcile
8. Artifact finalize/orphan cleanup
9. projection/index checkpoint
10. Provider/Plugin registry generation rebuild
11. Bot lazy activation
12. readiness

Provider 세션이나 Plugin process state를 persisted Canonical State보다 먼저 신뢰하지 않는다.

## 3. Waiting Task Recovery

Invariant:

> Waiting 상태가 Commit됐다면 재개에 필요한 Continuation도 같은 논리적 Commit에서 존재한다.

복구:
- Task/Continuation revision 검증
- completed dependency 결과 dedup 확인
- pending dependency만 다시 기다림
- deadline/cancellation 재평가
- 모든 wait condition 충족 시 resume guard로 한 번만 `ResumeTask` Commit
- duplicate/late child result 무시 + audit

Continuation missing/corrupt이면 임의 재실행하지 않고 Task를 `RecoveryRequired/Quarantine` 성격의 운영 상태로 노출한다. 정확한 external status mapping은 API 문서가 담당한다.

## 4. Side Effect Recovery

판정:
- `Prepared` + external call 미시작이 확실 → 정책상 execute 가능
- `Confirmed` → 재실행 금지
- `Failed` → retry disposition 평가
- 결과 판정 불가 → `Unknown`
- `Unknown` → provider status lookup, idempotency lookup, compensation, human review 중 하나로 reconcile
- reconciliation 결과 → `Reconciled` + evidence

외부 동작 직후 crash한 entry를 일반 transient failure로 간주하지 않는다.

## 5. Routine Recovery

- persisted next/last occurrence와 occurrence dedup load
- downtime 동안 missed occurrence 계산
- policy: skip/coalesce/run-latest/run-bounded-catchup 등 명시된 선택만 적용
- overlap policy와 active generated Task 확인
- occurrence claim + Task 생성 idempotency
- archive/inactive Bot이면 신규 trigger 금지

시간대/DST/calendar semantics는 schedule policy version으로 재현 가능해야 한다.

## 6. Provider/Plugin Recovery

Provider:
- compatibility/config validation 후 registry에 등록
- removed/deprecated provider reference는 silent fallback하지 않음
- in-flight attempt의 provider version이 unavailable하면 checkpoint compatibility를 평가하고 새 attempt 또는 manual failure로 전환

Plugin:
- install/enable lifecycle marker와 package digest 검증
- partial upgrade/migration marker reconcile
- crash-loop Plugin은 disabled/quarantine
- Plugin failure가 Core Runtime readiness를 막는지는 `required` capability 정책으로 결정

## 7. Checkpoint

Task/Execution revision, completed/pending work, Artifact refs, provider resume token+version, Side Effect refs, selected Memory/Policy revisions, checksum을 저장할 수 있다. Working heap dump를 checkpoint로 사용하지 않는다.

## 8. Error / Outcome

validation, auth, conflict, resource, transient/permanent provider, invariant, corruption, cancellation, internal defect, security incident를 stable code/retry disposition으로 구분한다. Timeout/cancel/signal/partial/cleanup 상태를 하나의 enum으로 소실하지 않는다.

## 9. Doctor

추가 검사:
- Waiting without Continuation
- stale/consumed Continuation
- Side Effect Unknown age
- Routine next occurrence/duplicate occurrence
- removed Provider stale config/reference
- Plugin lifecycle/data migration marker
- orphan dependency/config/catalog

## 10. 검증 기준

- AT-TASK-003에서 A/B 완료 후 crash가 C만 기다리고 Parent를 한 번만 resume한다.
- AT-SFX-001에서 external mutate 후 crash가 duplicate mutate를 만들지 않는다.
- Routine restart가 missed policy와 occurrence dedup을 따른다.
- removed Provider reference가 명시 diagnostic으로 나타난다.
- Plugin upgrade crash가 partial active generation을 남기지 않는다.
