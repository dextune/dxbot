---
title: "오류·복구·회복성"
document_id: "DXB-RUN-033"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-015", "DXB-ARC-017", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-030"]
---

# 오류·복구·회복성

## 1. 목적

crash/restart/Provider Session loss/index staleness에서도 Bot/Project/Channel/Conversation/Thread/Scoped Memory/Task의 Canonical identity를 보존하고, Membership/Authority/Directive를 중복 없이 복구한다.

## 2. Startup Recovery 순서

1. process/config/schema/migration marker 검증
2. Bot identity/Brain semantics/Main Conversation 복원
3. Project identity/revision/membership 복원
4. Channel identity/revision/membership/Role/Authority 복원
5. Conversation Parent/Thread identity/lineage/message watermark 복원
6. Bot/Project/Channel/Thread Memory revision/provenance/ScopeRef 검증
7. inbox/outbox/idempotency/Side Effect reconcile
8. Task/Execution/Core Lease reconcile
9. Waiting/Suspended Continuation/checkpoint 복원
10. pending Control Directive/Supervisor relation reconcile
11. scoped Memory index/projection watermark 확인
12. Channel Presence/Activity projection 재생성
13. Routine/Artifact/other projection reconcile
14. Provider config/contract/lifecycle rebuild
15. Provider Session은 optional optimization으로만 재검증
16. Bot/Project/Channel readiness 노출

## 3. Canonical vs Runtime Loss

다음 손실은 Canonical identity 손실이 아니다.
- Provider Session/token
- active Core handle
- Channel Presence Runtime
- semantic index/cache
- Interface Session/WebSocket
- transient routing queue

## 4. Membership / Authority Recovery

- durable revision/generation을 source of truth로 사용한다.
- stale runtime routing/presence에서 member를 복원하지 않는다.
- revoke event commit 후 crash해도 old grant를 활성 상태로 되살리지 않는다.
- Role/Authority binding projection은 Canonical state에서 rebuild한다.

## 5. Scoped Memory Recovery

- missing Scope owner를 임의 다른 Scope로 재배치하지 않는다.
- stale semantic index는 canonical fetch/authorization으로 필터링한다.
- promotion relation은 source MemoryId/revision/digest를 검증한다.
- source missing/corrupt는 recovery-required/quarantine이며 blind duplicate를 만들지 않는다.

## 6. Channel Task / Directive Recovery

v0.4 redirect/suspend crash windows를 그대로 유지한다. 추가로 SupervisorRef와 membership/authority generation을 재검증하고, stale Manager control을 startup에서 재적용하지 않는다.

Channel archive 상태가 Task terminal state를 추론하는 근거가 아니다.

## 7. Doctor 추가 진단

- Project/Channel orphan relation
- Channel without valid Project
- membership/Role/Authority generation divergence
- ScopeRef owner missing/orphan
- promotion source revision missing
- stale Channel Presence vs Canonical Membership
- archived Channel with unexpected new routing
- revoked participant with pending unauthorized control

v0.4의 Conversation/Thread/Directive/Suspension/Side Effect/Provider/Plugin 진단도 유지한다.

## 8. 검증 기준

- AT-PROJECT-001, AT-CHANNEL-001 restart restore 통과.
- Provider Session loss 후 동일 Project/Channel/Thread/Memory identity 유지.
- semantic index 전체 삭제 후 scoped Memory exact lookup/rebuild 가능.
- revoke commit 직후 crash/restart해도 old authority가 복구되지 않음.
- Channel Presence 재생성이 Membership revision을 변경하지 않음.
