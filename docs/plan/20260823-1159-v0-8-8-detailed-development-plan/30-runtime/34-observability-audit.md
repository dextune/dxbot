---
title: "관측성·Trace·Durable Audit"
document_id: "DXB-RUN-034"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-RUN-032", "DXB-RUN-033"]
---
# 관측성·Trace·Durable Audit

## 1. Correlation

```text
InstanceId / HostGeneration
→ ClientRequestId / CommandId / OperationId
→ PrincipalRef / RequestDigest / ResolvedBindingDigest
→ Target/Task/Execution/CoreLease/ProviderGeneration refs
→ ReceiptRevision / OutcomeRef / Cursor / TerminalStatus
```

raw Message/Memory/secret payload를 기본 trace에 기록하지 않는다.

## 2. Durable Audit Owner

`runtime-audit`와 `audit-store`가 security-sensitive decision의 Canonical audit record를 소유한다.

```text
AuditSequence / RecordDigest
Principal / Action / ResolvedTarget
RequestDigest / ResolvedBindingDigest
Policy/Authority/Approval/ActionGrant refs
Decision / Receipt / Outcome refs
Time / InformationLabel / RedactionClass
```

필수 audit action은 mutation transaction에서 `AuditIntent`를 함께 commit하고 durable outbox로 append한다. audit admission/capacity를 확보할 수 없으면 해당 high-risk mutation을 commit 전에 거부한다. log best-effort로 대체하지 않는다.

## 3. Retention·integrity·pressure

retention/export/purge는 policy generation이 소유하며 숫자는 M6에서 freeze한다. record sequence와 digest checkpoint로 truncation/corruption을 검출하고 startup에서 reconcile한다. audit store pressure가 control/recovery headroom을 고갈시키지 않도록 별도 byte budget을 둔다.

## 4. Metrics와 status

metric label에 ResourceId, raw text/path/token/cursor를 넣지 않는다. Host/Runtime/Domain/Provider/Projection/ResourcePressure 상태를 분리해 보고한다.

## 5. Diagnostic safety

secret canary/redaction, terminal neutralization, bounded dump, path 최소화, partial/gap/timeout의 receipt/cursor hint를 제공한다.
