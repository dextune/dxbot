---
title: "관측성·Trace·Audit"
document_id: "DXB-RUN-034"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-017", "DXB-RUN-032", "DXB-RUN-033"]
---

# 관측성·Trace·Audit

## 1. 목적

Bot→Task→Execution→Core→Provider Host→Provider→Tool→Memory/Recovery 경로를 공통 envelope로 추적하고 Provider별 telemetry drift를 방지한다.

## 2. 공통 Correlation

trace/span, command, correlation/causation, Bot/Goal/Task/Execution/CoreLease, Routine/Occurrence, SideEffectEntry, Capability, Provider ID/version/generation, contract/conformance version, policy/config generation을 가능한 범위에서 기록한다.

Metric label에는 unbounded user content/ID를 직접 넣지 않는다.

## 3. Provider Host Automatic Telemetry

Provider 개발자가 직접 구현하지 않아도 Host가 최소 다음을 자동 기록한다.

- capability/provider selection + reason class
- lifecycle state/generation
- admission wait/resource grant
- effective deadline/cancel source
- call start/end/outcome class
- normalized error/retry disposition
- output bytes/stream events/partial result
- reported/settled usage and cost
- side-effect guard/reconciliation state reference
- drain/quiescence duration

Provider는 provider-specific diagnostic attribute/Artifact를 bounded facade로 추가할 수 있으나 global telemetry pipeline을 대체하지 않는다.

## 4. Scheduler / Resource Metrics

active/queued Core, queue latency, oldest waiting, starvation, provider admission/rate wait, permit/activity utilization/leak reconciliation을 기록한다.

## 5. Memory Metrics

Canonical item/revision/bytes와 Derived cache/index bytes/hit/lag를 분리한다. cache eviction을 canonical deletion으로 집계하지 않는다.

## 6. Routine / Waiting / Side Effect

Routine trigger/missed/dedup, Waiting age/continuation validation/resume, Side Effect Prepared/Confirmed/Failed/Unknown/Reconciled 및 reconciliation latency를 기록한다.

## 7. Provider / Plugin Lifecycle

Provider:
- validation/start/ready/degraded/drain/stop/fail/quarantine/incompatible
- conformance version/evidence
- removed/deprecated reference hit
- crash/restart count

Plugin:
- install/enable/disable/upgrade/uninstall
- package digest/version/permission
- 제공 Provider registration/drain
- resource/protocol/data migration outcome

## 8. Audit 대상

Bot lifecycle, permission/policy, approval, secret, sensitive Memory, high-risk Tool, delegation, config/migration/backup, Provider/Plugin lifecycle, Side Effect reconciliation, destructive Routine changes를 감사한다.

## 9. Retention

Domain/Audit/Trace/Trajectory/Profile을 구분한다. duplicate payload 저장을 피하고 Artifact digest/reference를 사용한다. provider diagnostic buffer/spool도 bounded다.

## 10. 검증 기준

- CommandId에서 Provider Host selection/admission/call/normalized outcome까지 추적할 수 있다.
- 동등 Provider가 최소 공통 metric/audit 필드를 동일하게 생성한다.
- Provider가 telemetry facade를 사용하지 않아도 Host core span은 존재한다.
- secret canary가 logs/traces/audit/provider diagnostic에 나타나지 않는다.
