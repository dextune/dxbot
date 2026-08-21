---
title: "Multi-Bot Network와 협업"
document_id: "DXB-DOM-025"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P1"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-020", "DXB-DOM-022", "DXB-DOM-023", "DXB-ARC-014"]
---

# Multi-Bot Network와 협업

## 1. 목적

v0.5 durable delegation/idempotent result/Waiting Continuation을 유지하고 Process/Cycle/Grant causation을 추가해 crash/retry에서 duplicate delegation과 semantic replay를 방지한다.

## 2. 불변조건

- Bot A/B는 Brain/Working Context/Bot-global Canonical Memory를 직접 공유하지 않는다.
- 위임은 durable Message + target Task로 수행한다.
- delivery는 at-least-once일 수 있으며 Message/Task/Result가 idempotent하다.
- source ambient authority/ActionGrant budget은 target에 자동 증폭되지 않는다.
- delegation depth/fan-out뿐 아니라 Collaboration Run 전체 activation/round/cost/deadline이 bounded다.
- Shared Memory는 Project/Channel Scope를 통해서만 공유한다.

## 3. Envelope 확장

필요 시:
- ProcessId / ProcessDefinitionVersion?
- CycleId
- correlation / causation
- parent/child delegation refs
- Activity/Execution outcome ref
- ActionGrantRef? / attenuation metadata
- ProjectId / ChannelId / ThreadId / SupervisorRef
- membership/authority/policy revision refs
- deadline/resource budget refs

이 metadata가 permission을 자동 부여하지 않는다.

## 4. Duplicate / Replay

```text
Process step
→ stable delegation semantic key
→ durable Bot Network Message
→ target Task idempotent create
→ result outcome ref
→ Process observes result
```

replay가 같은 logical step을 다시 평가해도 새 Message/Task를 무한 생성하지 않는다. late/duplicate result는 same correlation/causation으로 reconcile하고 terminal Process/Cycle을 암묵 재개하지 않는다.

## 5. Authorization / Delegation Attenuation

Manager의 authority나 ActionGrant를 Researcher에게 전달할 때 target operation/scope/use budget이 source보다 넓어지지 않는다. Role 문자열/Prompt를 delegation grant로 사용하지 않는다.

## 6. Collaboration Utility

Channel에 여러 Bot이 있다는 이유만으로 모두 wake하지 않는다. Single-Bot-first admission 후 실제 필요가 있을 때 bounded participant subset을 선택한다. Bot Network 자체는 품질 판단 owner가 아니며 routing/termination은 `DXB-RUN-037`이 소유한다.

## 7. 검증 기준

- 기존 AT-NET-001/AT-COLLAB-001 유지.
- AT-PROC-001 replay에서 duplicate child/delegation 0.
- AT-SEC-004 retry/delegation이 grant budget을 증폭하지 않음.
- AT-COLLAB-002 causal loop가 bounded terminal reason으로 종료됨.
- Shared Memory 사용 중에도 Bot Global Memory 직접 공유 0.
