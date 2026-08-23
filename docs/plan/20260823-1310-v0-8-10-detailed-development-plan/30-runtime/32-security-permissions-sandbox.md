---
title: "보안·Principal·Approval·Authority·Local CLI Threat Model"
document_id: "DXB-RUN-032"
version: "0.8.9"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-RUN-031", "DXB-DOM-023", "DXB-DOM-025", "DXB-DOM-029", "DXB-ARC-012"]
---
# 보안·Principal·Approval·Authority·Local CLI Threat Model

P0 LocalPrincipal은 `InstanceId + authenticated OS UID`에서 server-side로 파생한다. 같은 UID process 간 isolation은 주장하지 않는다. client profile, argv Principal, BotId는 Authority가 아니다.

IdempotencyKey는 principal scope와 issuance epoch를 검증할 수 있어야 한다. 다른 UID/Instance의 key 또는 Instance-global CommandId 충돌은 Receipt 존재를 노출하지 않는 PreAccept conflict이며 신규 operation을 생성하지 않는다.

AuthorityBinding은 Security owner가 발급/revoke하며 Membership은 ref만 가진다. Membership row, AuthorityBinding generation/revoke, required AuditIntent는 하나의 Application Unit of Work다.

Approval은 pending OperationId와 action, target, policy generation에 결박된다.

```text
Approval: Requested → Approved | Denied | Expired | Revoked
ActionGrant: Active → Consumed | Expired | Revoked
```

Approval decision은 application-operation owner가 original pending operation을 재평가하도록 durable wakeup을 생성한다. CLI는 grant/resume token을 전달하지 않는다.

`--yes`와 Confirmation은 local UX state이며 wire payload, RequestDigest, Authority에 포함되지 않는다. non-TTY에서 implicit confirmation은 금지한다.

## Fail-closed local boundary

- Control endpoint discovery는 owner, file type, symlink 여부, InstanceId, HostGeneration, protocol compatibility를 검증한다. mismatch는 다른 endpoint/host-stop으로 자동 fallback하지 않는다.
- terminal output은 untrusted control sequence를 escape하고 stdout에는 result/stream만, stderr에는 diagnostic만 쓴다.
- `--output`은 descriptor-relative no-follow, bounded temp write, fsync, atomic no-replace를 사용하며 기존 파일을 암묵적으로 덮어쓰지 않는다.
- Private/Sensitive publication/export는 explicit declassification decision과 durable audit를 요구한다.
- stale endpoint removal/replace는 Runtime host owner가 lock과 generation을 검증해 수행하며 일반 CLI client가 추측으로 삭제하지 않는다.
