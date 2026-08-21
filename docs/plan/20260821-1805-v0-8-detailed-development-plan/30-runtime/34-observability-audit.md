---
title: "관측성·Trace·Audit"
document_id: "DXB-RUN-034"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-RUN-032", "DXB-RUN-033"]
---

# 관측성·Trace·Audit

## 1. 목적

Runtime Instance부터 Command/Receipt, selector, Task/Process, Provider, Result, cursor/partial output까지 causal chain을 설명하되 secret/raw content와 high-cardinality metric 폭증을 방지한다.

## 2. Correlation Chain

```text
InstanceId / HostGeneration
→ ClientRequestId / CommandId / OperationId
→ PrincipalRef / Action / ResolvedResourceRef
→ ProcessId? / TaskId? / ExecutionId?
→ CoreLeaseId? / ProviderGeneration?
→ SideEffect/Evidence/Artifact refs
→ ReceiptRevision / OutcomeRef
→ QuerySnapshotId? / Cursor? / TerminalStatus?
```

trace/audit는 reference와 class를 우선하고 raw Message/Memory/secret payload를 기본 기록하지 않는다.

## 3. Metrics

bounded labels로 다음을 측정한다.
- Runtime instance lifecycle/readiness/host generation changes
- concurrent start winner/stale artifact recovery
- command accepted/pending/committed/rejected/recovery-required
- idempotent replay and digest conflict
- selector resolved/ambiguous/not-found class
- page items/bytes/snapshot age/cursor errors
- subscription active/buffered items+bytes/gap/resync/terminal reason
- partial machine stream and last-safe-cursor class
- endpoint auth/permission failures
- terminal sanitization counters by sequence class
- export success/partial/cleanup/no-follow rejection
- Runtime memory pressure/admission/reservation

ResourceId, raw user text, file path, token, full cursor를 metric label로 사용하지 않는다.

## 4. Source 상태 구분

status/doctor/CLI는 다음을 분리한다.
- host service/process state
- Runtime Instance/Control readiness
- Canonical Domain state/revision
- Provider availability/health/generation
- projection/index watermark/staleness
- derived presence/cache
- resource pressure/admission

## 5. Audit

security-sensitive operation은 principal, action, resolved target ID, revision, decision/ActionGrant ref, request digest, receipt/outcome, time을 기록한다. interactive confirmation은 audit authority가 아니다.

## 6. Diagnostic Safety

- secret canary와 sensitive content redaction
- terminal control neutralization
- output/file path 최소화 또는 digest/class화
- pressure 상태에서 allocation-heavy dump 금지
- trace export는 P1/P2이며 P0 doctor는 bounded summary

## 7. 검증 기준

- uncertain operation을 CommandId/OperationId로 추적 가능.
- ambiguous selector 후보와 final target을 secret 없이 설명 가능.
- Runtime/Provider/Projection/Pressure 상태 혼용 0.
- high-cardinality/secret metric label 0.
- partial/gap/timeout 원인을 terminal metadata와 trace로 설명 가능.
