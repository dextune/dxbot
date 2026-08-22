---
title: "Bot Identity와 P0 Lifecycle"
document_id: "DXB-DOM-020"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-BASE-000", "DXB-ARC-014", "DXB-ARC-015"]
---
# Bot Identity와 P0 Lifecycle

Bot은 CLI/profile/provider process가 아니라 장기 보존되는 제품 엔티티다.

## P0 state

```text
Provisioning → Inactive → Activating → Active
                         ↘ Degraded
Active/Degraded → Quiescing → Inactive
Inactive/Active/Degraded → Archived → Restoring → Inactive
```

`Deleting`과 `Deleted`는 P0 state enum/transition에 포함하지 않는다. hard delete는 retention, reference, export, purge, rollback 계약을 가진 별도 P1/P2 ADR 이후 도입한다.

## Bootstrap defaults

`CreateBot` payload가 policy refs를 생략하면 Application은 current Instance default Brain/Permission/Resource/Provider policy generations를 resolve한다. resolved refs를 receipt/result에 기록하며 CLI가 default를 발명하지 않는다. Provider가 준비되지 않으면 partial Active로 숨기지 않고 Inactive/Degraded 또는 typed rejection으로 귀결한다.

## Invariants

- BotId는 재사용하지 않는다.
- Main Conversation은 create와 같은 durable boundary 또는 recoverable provisioning process로 정확히 하나 생성한다.
- CLI disconnect, Runtime restart, Provider 교체, Channel leave는 lifecycle transition이 아니다.
- Project/Channel Role은 Bot identity state에 복사하지 않는다.
