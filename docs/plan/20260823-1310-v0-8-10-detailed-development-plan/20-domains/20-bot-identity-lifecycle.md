---
title: "Bot Identity와 P0 Lifecycle"
document_id: "DXB-DOM-020"
version: "0.8.10"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-BASE-000", "DXB-ARC-014", "DXB-ARC-015"]
---
# Bot Identity와 P0 Lifecycle

Bot은 CLI/profile/provider process와 독립된 persistent entity다.

```text
Provisioning → Inactive → Activating → Active
                         ↘ Degraded
Active/Degraded → Quiescing → Inactive
Inactive/Active/Degraded → Archived → Restoring → Inactive
```

CreateBot은 Provider readiness와 무관하게 BotId와 Main Conversation을 하나의 canonical mutation으로 durable하게 생성한다. policy ref 생략 시 Instance default generations를 resolve한다. configured production Provider가 없더라도 identity creation 자체를 reject하지 않으며 Bot은 Inactive로 생성된다.

CreateBot의 committed payload는 사용자가 추가 조회 없이 다음 행동을 선택할 수 있도록 최소 `BotRef`, `BotRevision`, `LifecycleState`, `MainConversationRef`를 반환한다. human renderer는 secret이나 policy 내부를 노출하지 않고 Bot selector와 `conversation send`의 다음 행동을 제시한다.

Provider availability는 Activate/Task admission에서 평가한다. Reference Provider는 explicit test policy에서만 선택할 수 있고 production fallback이 아니다. provider-unavailable은 Bot 생성 실패로 소급하지 않으며 `runtime doctor --section provider` action을 제공한다.

BotId는 재사용하지 않고 P0 hard delete state를 두지 않는다.
