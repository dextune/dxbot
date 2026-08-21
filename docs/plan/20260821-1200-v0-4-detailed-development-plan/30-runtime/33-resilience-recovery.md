---
title: "오류·복구·회복성"
document_id: "DXB-RUN-033"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-015", "DXB-ARC-017", "DXB-DOM-027", "DXB-RUN-030"]
---

# 오류·복구·회복성

## 1. 목적

Provider/Tool/Storage/Plugin 장애와 crash/partial outcome에서도 Bot/Main Conversation/Thread/Memory/Task 연속성을 보존하고 pending Directive/Suspension을 중복 적용 없이 복구한다.

## 2. Startup Recovery 순서

1. process ownership/config/schema validation
2. storage integrity/migration marker
3. Bot + Main Conversation 1:1 invariant
4. Thread identities/lineage/message watermarks
5. Thread/Bot Memory scope/provenance links
6. inbox/outbox/idempotency
7. Side Effect Prepared/Unknown reconciliation queue
8. Core Lease/Execution reconcile
9. Waiting + Suspended Task Continuation/checkpoint restore
10. pending Control Directive restore/dedup
11. Routine occurrence reconcile
12. Artifact/projection/index reconcile
13. Provider config/contract compatibility + lifecycle rebuild
14. Plugin lifecycle rebuild
15. Bot lazy activation/readiness

Provider Session/process state를 Canonical state보다 먼저 신뢰하지 않는다.

## 3. Provider / Session Failure

v0.3 stable error normalization/lifecycle recovery를 유지한다. Provider Session resume failure는:
- Thread/Conversation corruption이 아님
- Canonical Context Plan에서 new session/call 시도 가능
- persisted resume token incompatibility를 explicit diagnostic으로 표시
- existing Execution resume 불가면 policy에 따라 recovery-required/new Execution; binding silent rewrite 금지

## 4. Thread Recovery

- Interface Session이 0개여도 Main Conversation/Thread restore
- Thread lineage source missing/corrupt는 quarantine/recovery-required, 임의 root 재작성 금지
- archived Thread의 running Task 상태는 archive policy와 reconcile
- Message history projection gap은 durable source에서 rebuild
- Thread-local Memory scope가 missing Thread를 가리키면 doctor에서 orphan/corruption 표시

## 5. Redirect Recovery

Crash windows:
1. Directive/Task revision commit 전 → no redirect
2. commit 후 Supervisor signal 전 → startup이 pending Directive 전달
3. old Execution yield 후 new Execution 생성 전 → idempotent scheduler admission
4. new Execution 생성 후 ack 전 → DirectiveId/Task revision으로 duplicate 생성 차단

old Execution snapshot을 새 revision으로 rewrite하지 않는다.

## 6. Suspend / Resume Recovery

- suspend committed, checkpoint incomplete/corrupt → RecoveryRequired, 임의 resume 금지
- suspended + valid checkpoint → inactive admission 상태 복원
- resume committed 후 crash → resume guard로 새 Execution 한 번만 생성/재사용
- already consumed continuation을 다시 consume하지 않음

## 7. Side Effect / Waiting / Routine

v0.3 semantics 유지:
- Unknown → reconcile 우선
- Waiting + Continuation atomic meaning
- Routine occurrence dedup/missed policy

Control event가 들어왔다고 Unknown Side Effect를 자동 retry하지 않는다.

## 8. Doctor

추가 진단:
- Bot without Main Conversation / duplicate Main Conversation
- orphan Thread/lineage edge
- Thread Memory scope orphan
- pending Directive stuck/unacked
- suspended Task missing/corrupt checkpoint
- redirect spec revision exists but new Execution absent
- stale Provider Session token
- control queue/runtime vs durable Directive divergence

기존 Waiting without Continuation, Side Effect Unknown, Provider lifecycle/activity leak, Plugin migration 진단도 유지한다.

## 9. 검증 기준

- AT-CONV-001 restart restore.
- AT-SESSION-002 Provider Session loss independence.
- AT-CTRL-003 redirect crash windows에서 duplicate/new revision corruption 0.
- AT-CTRL-004 suspend/resume crash 후 exactly-one semantic resume.
- Side Effect Unknown duplicate mutate 0.
- Provider/Plugin removal이 Conversation/Thread restore를 막지 않음.
