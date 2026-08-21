---
title: "보안·권한·승인·Sandbox"
document_id: "DXB-RUN-032"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-012", "DXB-ARC-016", "DXB-ARC-017", "DXB-DOM-025", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-031", "DXB-RUN-036"]
---

# 보안·권한·승인·Sandbox

## 1. 목적

Project/Channel/Shared Scope를 최소 권한·generation fencing·final authorization으로 보호하고 Role, natural-language intent, stale cache/index가 실제 Runtime Authority로 승격되지 않게 한다.

## 2. 기본 원칙

- default deny
- Project grant가 모든 Channel grant를 의미하지 않음
- Channel grant가 Project permission ceiling을 초과하지 못함
- Role label/Prompt/Model/Provider/Tool/Plugin output이 Authority를 생성하지 못함
- 다른 Bot의 Global Memory 직접 접근 금지
- Shared Memory publication은 별도 write authority 필요
- stale membership/authority generation은 신규 mutation/control 권한 상실
- sandbox unavailable → unsandboxed fallback 금지
- secret 원문을 Memory/Conversation/Event/diagnostic에 저장하지 않음

## 3. Resource 모델 추가

- Project / Project Membership
- Project Memory / Project Artifact/Resource
- Channel / Channel Membership
- Channel History / Thread
- Channel Memory
- Channel Role / Channel Authority
- Channel Task / Supervisor Control
- Memory Promotion/Publication

## 4. Authorization Path

```text
principal
→ Project ceiling
→ Channel membership/authority
→ target resource/Task/Supervisor scope
→ expected revision/generation
→ approval if required
→ Canonical Command
```

Read path도 candidate/index 결과를 final Canonical authorization으로 다시 필터링한다.

## 5. Membership Revocation

revoke commit 이후:
- 신규 Project/Channel Memory recall/write 차단
- 신규 Channel History/Artifact read 차단
- 신규 routing eligibility 차단
- 신규 delegate/redirect/suspend/resume/cancel/reprioritize 차단

이미 만들어진 immutable Execution Context를 retroactive rewrite하지 않는다. 고위험 Side Effect는 policy에 따라 external effect 직전 current authorization을 재검증한다.

## 6. Manager / Supervisor

Manager Role만으로 다른 Bot Task를 제어할 수 없다. Authority와 Task SupervisorRef 또는 별도 approved control grant를 모두 충족해야 한다.

Manager가 Researcher Brain/Core/Memory를 직접 접근·조작하는 API는 제공하지 않는다.

## 7. Shared Memory

Memory Proposal은 creator/ScopeRef/provenance를 기록하고 target Scope write authority를 확인한다. Project/Channel read grant와 promotion/publication grant를 동일하게 취급하지 않는다.

## 8. Prompt Injection

Channel Message, retrieved web/document content, Provider output가 `Manager`, `admin`, `cancel`, `redirect` 문자열을 포함해도 permission object가 되지 않는다.

## 9. Audit

actor/principal, ProjectId/ChannelId, membership/authority generation, Role, target ScopeRef/Thread/Task/Execution, SupervisorRef, DirectiveId, expected/result revision, approval/reason/outcome을 필요한 범위에서 감사한다.

## 10. 검증 기준

- AT-SEC-001 기존 least-privilege gate 유지.
- AT-SEC-002 revoke 직후 신규 scope read/write/control 모두 denied.
- stale semantic index/cache가 revoked Memory를 노출하지 않음.
- Role 변경만으로 effective authority가 증가하지 않음.
- Manager가 supervisor scope 밖 Task를 redirect하지 못함.
