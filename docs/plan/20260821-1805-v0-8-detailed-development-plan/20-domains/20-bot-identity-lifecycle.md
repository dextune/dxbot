---
title: "Bot Identity와 Lifecycle"
document_id: "DXB-DOM-020"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-ARC-014", "DXB-ARC-015"]
---

# Bot Identity와 Lifecycle

## 1. 목적

Bot을 Interface Session이나 Provider process가 아니라 장기 보존되는 제품 엔티티로 정의한다. Bot lifecycle은 Project/Channel 참여, CLI 접속, Core 활성 수와 독립적으로 유지된다.

## 2. Aggregate와 Canonical State

Bot Aggregate가 소유하는 최소 상태는 다음과 같다.

```text
BotId
IdentityRevision
LifecycleState
DisplayName / scoped aliases
BrainPolicyRef
BotMemoryScopeRef
MainConversationRef
Permission / Resource / Routine references
CreatedAt / UpdatedAt / ArchivedAt?
```

Canonical state가 아닌 것:
- CLI profile·last-used target
- Control Connection·socket·process ID
- Provider Session·model invocation ID
- Project/Channel Role 문자열
- active Core 목록과 ephemeral Working Context
- cache/index/presence

## 3. Lifecycle

```text
Provisioning → Inactive → Activating → Active
                         ↘ Degraded
Active/Degraded → Quiescing → Inactive
Inactive/Active/Degraded → Archived → Restoring → Inactive
Archived → Deleting → Deleted
```

P0 CLI는 hard delete를 제공하지 않는다. `archive/restore`가 사용자 가시 lifecycle이며 delete는 P1/P2에서 retention·reference·export·purge 계약을 닫은 뒤 도입한다.

### 불변조건

- Deleted가 아닌 BotId는 재발급하거나 다른 Bot에 재사용하지 않는다.
- Main Conversation은 Bot 생성 성공과 같은 durable transaction 또는 명시적 recoverable provisioning process에서 연결한다.
- CLI disconnect, Runtime restart, Provider 교체, Channel leave는 Bot lifecycle 전이가 아니다.
- activation은 capability/resource/security 검증 실패 시 `Degraded` 또는 `Inactive`로 명시적으로 귀결되고 부분 활성 상태를 숨기지 않는다.

## 4. 이름과 Selector

BotId가 최종 식별자다. name/alias는 명시된 Runtime Instance와 principal-visible scope 안에서 exact match가 하나일 때만 `ResolvedResourceRef`로 변환된다. 이름 변경이 BotId나 Memory/Main Conversation identity를 바꾸지 않는다. selector 의미는 `DXB-IFC-040`이 소유한다.

## 5. Project/Channel 참여

Bot은 0..N Project/Channel Membership을 가질 수 있다. Role/Authority는 Membership owner가 소유하며 Bot Persona나 global permission으로 복사하지 않는다.

```text
Bot Identity + Project Context + Channel Role/Authority + Task Context
→ 해당 범위의 현재 행동
```

Membership revoke는 신규 scope access를 차단하되 Bot 자체를 deactivate/archive하지 않는다.

## 6. Recovery

Runtime startup에서 BotId, IdentityRevision, lifecycle, MainConversationRef, MemoryScopeRef를 canonical storage에서 복구한다. endpoint, HostGeneration, Provider Session은 재생성할 수 있으나 Bot identity는 유지한다. provisioning이 중단된 경우 receipt/process state를 통해 complete 또는 rollback하고 중복 Main Conversation을 만들지 않는다.

## 7. Resource와 보안

Bot별 Resource policy는 global Runtime ceiling 아래의 quota/weight다. Bot이 권한·예산을 자체 생성하지 않는다. name, metadata, diagnostics는 secret/raw sensitive Memory를 포함하지 않는다.

## 8. 검증 기준

- Runtime restart·endpoint 재생성 뒤 동일 BotId/IdentityRevision을 조회한다.
- 동일 이름 Bot fixture에서 scope 없는 mutation은 `ambiguous-target`이다.
- Channel Role 변경이 Bot IdentityRevision을 변경하지 않는다.
- Bot archive가 linked Task/Process를 암묵 cancel하지 않으며 정책에 따른 explicit quiescence/reconcile을 요구한다.
- CLI/Provider Session loss가 Bot identity 손실을 만들지 않는다.
