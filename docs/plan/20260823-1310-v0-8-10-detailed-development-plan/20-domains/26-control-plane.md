---
title: "Typed Control Plane Use-Case Surface"
document_id: "DXB-DOM-026"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-DOM-020", "DXB-DOM-023", "DXB-DOM-025", "DXB-RUN-032", "DXB-ARC-010"]
---
# Typed Control Plane Use-Case Surface

모든 external use-case는 typed Application operation 또는 Host Action 하나에 귀결된다. generic JSON mutation을 금지한다.

## Approval continuation

고위험 operation이 Approval을 요구하면 Application은 original typed operation intent, CommandId/IdempotencyKey/RequestDigest, resolved target/policy refs와 `ApprovalRef`를 durable하게 결박하고 Receipt를 `Accepted`로 유지한다.

`approval approve/deny`는 Approval aggregate만 끝내고 방치하지 않는다. decision commit 후 application-operation owner가 원 operation을 current revision/policy로 재평가한다.

- approve + still-valid -> 원 operation `Committed` 또는 새 Approval/typed conflict
- deny/expire/revoke -> 원 operation `Rejected`

CLI가 ActionGrant나 resume token을 직접 만들 필요가 없다.

Membership set/remove는 Project/Channel row와 AuthorityBinding generation/revoke를 같은 Unit of Work에서 처리한다.
