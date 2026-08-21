---
title: "관측성·Trace·Audit"
document_id: "DXB-RUN-034"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-RUN-032", "DXB-RUN-033"]
---

# 관측성·Trace·Audit

## 1. 목적

Bot→Task→Execution→Core→Provider→Tool→Memory/Routine/Recovery의 경로와 Provider/Plugin lifecycle을 상관관계로 추적한다.

## 2. 공통 Correlation

trace/span, command, correlation/causation, Bot/Goal/Task/Execution/CoreLease, Routine/Occurrence, Message, SideEffectEntry, Provider/Plugin ID+version, policy/config generation을 가능한 범위에서 기록한다.

Metric label에는 unbounded user content/ID를 직접 넣지 않는다.

## 3. Scheduler Metrics

- active/queued Core by class
- queue latency by priority/class
- oldest waiting age by class
- starvation indicator/count
- per-Bot fair-share utilization
- provider queue/rate-limit wait
- permit utilization/leak reconciliation
- deterministic ordering violation counter(test/debug)

## 4. Memory Metrics

Canonical과 Derived를 분리한다.

Canonical:
- item/revision count
- canonical bytes
- unused age distribution
- retention/archive/compaction candidate bytes
- tombstone/delete backlog

Derived/Runtime:
- cache bytes/hit/eviction
- index bytes/lag/rebuild
- recall latency/candidate count

cache eviction을 canonical deletion metric으로 집계하지 않는다.

## 5. Routine / Waiting / Side Effect

Routine:
- enabled count
- trigger latency
- missed/coalesced/skipped occurrences
- duplicate prevented count
- generated Task outcome

Waiting:
- waiting age
- pending child count
- continuation validation failure
- resume count/duplicate suppression

Side Effect:
- Prepared/Confirmed/Failed/Unknown/Reconciled counts
- Unknown age
- reconciliation latency/outcome
- duplicate prevented

## 6. Provider / Plugin

Provider:
- selection count/reason class
- availability/health
- conformance version
- deprecated/removed reference hit
- drain duration

Plugin:
- install/enable/disable/upgrade/rollback/uninstall events
- package digest/version
- permission decisions
- resource usage/overrun
- crash/protocol violation
- data migration/purge outcome

## 7. Audit 대상

Bot lifecycle, permission/policy, approval, secret, sensitive Memory, high-risk Tool, delegation, config/migration/backup, Provider lifecycle, Plugin lifecycle, Side Effect reconciliation, destructive Routine changes를 감사한다.

## 8. Retention

Domain/Audit/Trace/Trajectory/Profile을 구분한다. duplicate payload 저장을 피하고 Artifact digest/reference를 사용한다. telemetry local buffer/spool도 bounded다.

## 9. 검증 기준

- CommandId에서 Provider selection과 Side Effect outcome까지 추적할 수 있다.
- starvation/oldest waiting metric이 부하 test에서 변화를 보인다.
- Canonical Memory bytes와 cache bytes가 별도 series다.
- Plugin uninstall이 actor/digest/data policy와 함께 audit된다.
- secret canary가 logs/traces/audit payload에 나타나지 않는다.
