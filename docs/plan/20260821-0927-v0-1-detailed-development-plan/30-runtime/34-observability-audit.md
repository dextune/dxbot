---
title: "관측성·Trace·Audit"
document_id: "DXB-RUN-034"
version: "0.1.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-RUN-032", "DXB-RUN-033"]
---


# 관측성·Trace·Audit

## 1. 목적

Bot, Task, Execution, Core, Harness, Tool, Message의 전체 경로를 상관관계로 추적하고 성능·비용·보안·정합성 문제를 진단할 수 있게 한다.

## 2. 책임 범위

- structured logs, traces, metrics, audit
- correlation과 span taxonomy
- cardinality/retention/redaction
- health/SLO
- UI/CLI 조회
- replay/trajectory reference

Domain Journal과 Audit Log는 목적이 다르며 서로 대체하지 않는다.

## 3. 관측 데이터 구분

| 종류 | 목적 | 손실 허용 | 민감도 |
|---|---|---|---|
| Domain Event | 제품 사실/복구 | 불가 | 중~고 |
| Audit | 보안·운영 행위 | 민감 작업은 불가 | 고 |
| Trace | 요청 경로/지연 | sampling 가능 | 중 |
| Log | 진단 메시지 | 등급별 | 중 |
| Metric | 집계 상태/성능 | 일부 가능 | 낮음~중 |
| Harness Trajectory | 모델 입력/Tool 재현 | 정책 기반 | 고 |
| Profile | CPU/heap 분석 | sampling | 중 |

## 4. Correlation 체계

모든 관측 이벤트는 가능한 범위에서:
- trace_id/span_id
- command_id
- correlation_id/causation_id
- BotId
- GoalId/TaskId/ExecutionId/CoreLeaseId
- MessageId
- Harness run/trace reference
- provider/tool name
- policy/config/schema version
을 가진다.

원문 content와 ID를 label로 남발하지 않는다. Metric label은 bounded cardinality만 사용한다.

## 5. Span 구조

```text
control.command
  └─ application.handle
      ├─ domain.decide
      ├─ storage.commit
      └─ runtime.schedule
          └─ execution.run
              ├─ context.assemble
              ├─ model.request
              ├─ tool.execute
              ├─ memory.propose
              └─ result.validate
```

비동기 outbox/projector는 original correlation을 link로 유지한다. Bot 간 위임은 source trace와 target trace를 causal link로 연결한다.

## 6. 핵심 Metrics

### Runtime
- active/idle/degraded Bots
- active/queued Core by class
- scheduler queue latency
- permit utilization/leak reconciliation
- mailbox/channel fill ratio
- task/execution outcome rates
- shutdown drain time

### Storage/Memory
- event append latency/failure
- DB busy/lock time
- projection lag/watermark
- outbox lag/retry/poison
- Memory item/revision bytes
- recall latency/candidate counts
- index lag/rebuild time
- Artifact bytes/orphans

### Harness
- model latency to first token/total
- token/cost usage
- tool latency/outcome
- cancellation observation
- sidecar restarts/protocol violations
- context bytes/tokens/cache hit
- trace spill bytes

### Security
- denied/asked/approved actions
- sandbox violations
- secret redaction hits
- delegation rejects
- break-glass actions
- quarantined Bot/provider

## 7. 로그 규칙

- structured key-value
- stable event name와 error code
- 사용자 content 기본 비기록
- secret/credential/header redaction
- stderr/stdout raw 출력 bounded
- error chain은 source를 보존
- 동일 오류 반복은 rate limit/sampling하되 count metric 유지
- panic은 process-wide context가 아닌 해당 scope IDs와 backtrace
- 로그 메시지로 program logic을 파싱하지 않음

## 8. Audit

Audit 대상:
- Bot create/delete/archive/clone/import
- permission/policy 변경
- approval
- secret access
- sensitive Memory read/write/forget
- high-risk Tool
- Bot delegation
- config/migration/backup/restore
- break-glass
- audit export

Audit record는 actor, action, resource, result, reason, request digest, policy, timestamp, correlation, source address/device(해당 시), previous/new revision을 가진다. 모델이 작성한 설명과 실제 결정자를 구분한다.

## 9. Health와 SLO

Health:
- liveness: process/event loop alive
- readiness: storage write path, schema, essential providers usable
- degraded: 일부 capability unavailable
- component health: projector/indexer/harness/sandbox

초기 engineering SLO 후보:
- Domain commit 성공률
- scheduler queue p95/p99
- projection lag
- graceful shutdown success
- orphan resource count
- memory recall p95
- control command availability

모델/외부 Tool 지연은 Runtime overhead와 분리해 측정한다.

## 10. Retention과 비용

- high-cardinality trace는 sampling/tiering
- audit/domain은 정책 기반 장기
- model delta 원문은 Artifact 압축/만료
- metric raw series retention 제한
- duplicate payload 저장 금지
- content digest와 reference 사용
- 사용자 forget 요청 시 trace/log/index의 적용 범위를 명시
- observability 자체가 Runtime memory pressure를 만들지 않게 local buffers bounded

## 11. 예외상황

- telemetry backend down: bounded local spool/drop policy, Runtime 핵심 경로 차단 금지
- audit backend down: 민감 action fail-closed 또는 durable protected spool
- trace export slow: async batch + backpressure/drop
- clock skew: monotonic duration, wall clock separately
- sampling으로 오류 trace 누락: errors/security는 tail sampling 우선
- PII redaction 실패 의심: export 중지와 incident
- metric cardinality 폭증: label guard가 drop/aggregate
- event stream client lag: cursor resync

## 12. 확장성

단일 노드 local tracing에서 외부 collector로 교체한다. Open telemetry 표준과 유사한 개념을 사용할 수 있으나 Domain type에 vendor SDK를 노출하지 않는다. multi-tenant에서는 tenant별 access/filter와 cost allocation을 추가한다.

## 13. 구현 우선순위

- **P0:** tracing IDs, structured log, core metrics, audit store, local inspect CLI
- **P1:** distributed trace export, dashboards, SLO alerts, trajectory viewer data
- **P2:** multi-node correlation, long-term analytics
- **P3:** anomaly detection

## 14. 검증 기준

- 한 CLI Task를 Bot→Core→Harness→Tool→Memory까지 correlation으로 추적한다.
- metric label cardinality가 부하 증가에 따라 무제한 증가하지 않는다.
- secret canary가 logs/traces/audit content에 나타나지 않는다.
- telemetry backend 장애가 Task commit을 막지 않는다.
- 민감 action은 audit 손실 없이 fail-closed/spool 정책을 따른다.
- projection lag, queue saturation, permit leak가 dashboard/CLI에서 관측된다.
