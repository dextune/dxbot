---
title: "관측성·Trace·Audit"
document_id: "DXB-RUN-034"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-017", "DXB-RUN-032", "DXB-RUN-033", "DXB-RUN-036"]
---

# 관측성·Trace·Audit

## 1. 목적

v0.6의 Process/Cycle/Activity/Grant/Evidence/Runtime Memory correlation을 유지하고, v0.7 Application Command/Control/CLI version 및 reconnect 정보를 같은 causal chain에서 설명할 수 있게 한다.

## 2. Correlation Chain

```text
ClientRequestId / CommandId?
→ Principal / Control Endpoint
→ ProcessId? / CycleId?
→ Project/Channel/Bot/Thread
→ Task/Delegation
→ Activity/Execution
→ ActionGrant?
→ Core Lease
→ Provider/Tool/Side Effect
→ Result/Evidence
→ Memory Proposal/Assertion/Relation
```

필요한 revision/generation/correlation만 trace/audit에 두고 raw content/high-cardinality ID를 metric label로 남발하지 않는다.

## 3. v0.7 Control Metrics

bounded dimension으로 다음을 측정한다.
- command accepted/committed/rejected/conflict/resource-exhausted class
- idempotent retry/duplicate suppression
- response-loss reconciliation count
- protocol/schema compatibility failure
- active control connection/subscriber count
- event cursor gap/resync count
- subscription buffered items/bytes
- slow consumer disconnect/backpressure
- CLI-visible query page/response bytes class

## 4. Runtime / Provider / Projection 상태 구분

CLI status/doctor가 다음을 혼합하지 않도록 source를 구분한다.
- Canonical Runtime state
- Provider availability/health
- Projection/index freshness
- Presence derived state
- Runtime Memory pressure/resource admission

모든 derived observation은 가능한 범위에서 `observed_at`, source revision/watermark, stale/degraded를 제공한다.

## 5. Version / Client Context

진단에 필요한 범위에서 다음을 기록할 수 있다.
- runtime version
- protocol version
- schema version
- client version
- negotiated feature/capability set

Data schema version을 protocol version으로 표시하지 않는다.

## 6. Secret / Diagnostic Safety

- argv/env/config secret 원문 기록 금지
- credential/token/action grant secret material 출력 금지
- raw sensitive Memory/Message를 doctor/trace에 기본 dump하지 않음
- `--verbose`가 redaction을 무효화하지 않음
- Emergency pressure에서 allocation-heavy diagnostic 생성 회피

## 7. 검증 기준

- CLI command→Task/Process/Provider/Result causal trace가 연결된다.
- duplicate/retry/reconnect/gap 원인을 설명 가능하다.
- Runtime/Provider/Projection/Pressure 상태가 구분된다.
- secret/raw sensitive content leakage 0.
- v0.6 Process/Epistemic/Security/Memory metrics regression 0.
