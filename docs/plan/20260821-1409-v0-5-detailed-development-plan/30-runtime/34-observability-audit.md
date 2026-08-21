---
title: "관측성·Trace·Audit"
document_id: "DXB-RUN-034"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-017", "DXB-RUN-032", "DXB-RUN-033", "DXB-RUN-036"]
---

# 관측성·Trace·Audit

## 1. 목적

`Project/Channel → Bot → Thread → Task/Delegation → Directive → Execution → Core Lease → Provider Host → Scoped Memory/Recovery`를 하나의 correlation chain으로 추적하되 unbounded ID/content를 metric label에 사용하지 않는다.

## 2. Correlation

trace/event/audit에는 필요한 범위에서 다음 reference를 가진다.
- CommandId / Correlation / Causation
- BotId / ConversationId / ThreadId / MessageId
- ProjectId / ProjectRevision
- ChannelId / ChannelRevision
- MembershipRevision/Generation
- RoleBindingRevision / AuthorityGeneration
- Goal/Task/TaskSpecRevision / SupervisorRef
- DirectiveId / ExecutionId / CoreLeaseId
- MemoryId/Revision/ScopeRef/Promotion relation
- Capability/Provider ID/version/generation
- Policy/Config generation

고카디널리티 ID는 trace/audit field로 유지하고 metric label은 bounded dimension을 사용한다.

## 3. Project / Channel Metrics

- Project/Channel active/archived count
- membership join/leave/revoke/conflict rate
- Role/Authority change rate
- inbound message/event rate
- recipient resolution count
- selected participant count / fan-out reject/defer
- response concurrency/queue wait
- Channel Presence projection lag
- Thread/message/history bytes and retrieval latency

## 4. Scoped Memory Metrics

- items/bytes/revisions by scope class
- candidate→proposal→commit rate
- promotion accepted/rejected/conflict/dedup
- semantic candidate vs final authorization filtered count
- stale index candidate count
- recall candidate/selected count and bytes
- cache hit/miss/retained bytes/invalidation cause

raw Memory/Conversation content는 metric label이 아니다.

## 5. Collaboration Metrics

- Manager→Researcher/Reviewer delegation depth/fan-out
- result latency/duplicate/reconcile
- Supervisor report/redirect/suspend latency
- revoked participant routing/control rejection
- stale authority generation conflict

기존 v0.4 Control queue/Directive/safe-point/Provider/Scheduler/Side Effect metrics를 유지한다.

## 6. Audit

감사 대상:
- Project/Channel create/archive/delete/restore
- membership/Role/Authority mutation
- Shared Memory read/write/promotion/forget/export
- cross-scope Artifact/history access
- delegation/Supervisor changes
- redirect/suspend/resume/cancel/reprioritize
- Provider/Plugin lifecycle/high-risk Tool/Side Effect

## 7. Freshness

Presence, summary, index, managed-task view는 `observed_at`, source revision/watermark, stale/degraded를 표현한다. Derived view를 Canonical authority처럼 표시하지 않는다.

## 8. 검증 기준

- Channel Message에서 target Bot→Task→Execution→MemoryProposal까지 trace 가능.
- revoke 사건 이후 stale cache/index/routing rejection을 trace 가능.
- fan-out/queue/cache byte pressure 원인을 scope별로 추적 가능.
- secret canary/raw sensitive content가 metrics/log/trace/provider diagnostic에 나타나지 않음.
