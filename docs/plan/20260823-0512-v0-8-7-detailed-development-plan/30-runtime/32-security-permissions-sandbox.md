---
title: "보안·Principal·Approval·Authority·Local CLI Threat Model"
document_id: "DXB-RUN-032"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-RUN-031", "DXB-DOM-023", "DXB-DOM-025", "DXB-DOM-029", "DXB-ARC-012"]
---
# 보안·Principal·Approval·Authority·Local CLI Threat Model

P0 LocalPrincipal은 `InstanceId + authenticated OS UID`에서 server-side로 파생한다. 같은 UID process 간 isolation은 주장하지 않는다.

AuthorityBinding은 Security owner가 발급/revoke하며 Membership은 ref만 가진다.

Approval은 pending OperationId와 action/target/policy generation에 결박된다.

```text
Approval: Requested → Approved | Denied | Expired | Revoked
ActionGrant: Active → Consumed | Expired | Revoked
```

Approval decision은 application-operation owner가 original pending operation을 재평가하도록 durable wakeup을 생성한다. CLI는 grant/resume token을 전달하지 않는다.

`--yes`/Confirmation은 local UX state이며 wire payload, RequestDigest, Authority에 포함되지 않는다.

Private/Sensitive publication/export는 declassification+audit를 요구하고 endpoint/terminal/export는 v0.8.6 fail-closed 규칙을 유지한다.
