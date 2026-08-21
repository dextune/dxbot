---
title: "보안·권한·정보 흐름·승인·Sandbox"
document_id: "DXB-RUN-032"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-012", "DXB-ARC-016", "DXB-ARC-017", "DXB-DOM-025", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-031", "DXB-RUN-036"]
---

# 보안·권한·정보 흐름·승인·Sandbox

## 1. 목적

v0.5 least-privilege/generation fencing/final authorization을 유지하고 **Common Authorization Decision Owner + Information Flow/Declassification + Durable ActionGrant + Untrusted Proposal boundary**를 추가한다.

## 2. Common Authorization Decision Owner

Authorization policy semantic의 SSOT는 본 문서 `DXB-RUN-032` 하나다. Project/Channel/Memory/Task/Provider Host/Tool/API는 각자의 enforcement point(PEP)로 동작하되 policy 의미를 재구현하지 않는다.

논리적 입력:

```text
PrincipalRef
SecurityDomainRef
ResourceRef
Action
Relationship / Membership / Authority refs
PolicyRevision
Context / Risk class
```

논리적 결과:

```text
Allow / Deny / ApprovalRequired
DecisionRevision
Reason / matched policy refs
Optional GrantRef
```

정확한 타입/API는 ADR 대상이다.

## 3. Security Domain

P0 single-user/local Runtime에서도 principal의 최상위 boundary를 명시한다. 권장 기본 의미는 local owner/workspace owner다. 향후 multi-user/tenant IAM을 v0.6 P0에 선행 구현하지 않는다.

## 4. Access vs Information Flow

> **Read Authority + Write Authority ≠ Information Flow Authority**

예:

```text
Bot Private Memory read = allow
Channel Memory write = allow
```

이어도 `Private → Channel publication`은 자동 허용되지 않는다.

publication/internalization 시:

```text
Source Scope / Information Label
→ Target Scope
→ Principal / current Authority
→ Information-Flow Policy
→ Declassification / Approval if required
→ Memory Validation
→ Canonical Commit
```

## 5. Information Label

Context item/MemoryProposal/Artifact/external content는 필요한 범위에서:
- origin/source class
- trust class
- confidentiality/sharing class
- scope provenance
- sensitivity

를 유지한다. exact taxonomy는 Security ADR 대상이다.

## 6. Model / Tool / External Output

- LLM output은 Canonical Truth가 아니라 **Untrusted Proposal**이다.
- Tool output도 Tool/source trust metadata를 보존한다.
- web/document/external message의 instruction-like text는 authority가 아니다.
- untrusted content가 Policy/Procedure/Identity/Authority Memory로 직접 승격되지 않는다.
- 모델이 생성한 citation/verification 주장만으로 Verified를 만들지 않는다.
- poisoning/provenance 단절/retracted dependency는 quarantine/revalidation 대상이 될 수 있다.

## 7. Durable ActionGrant

ActionGrant는 기존 Membership/Authority/Permission/Approval을 대체하지 않는다. 이미 허용된 권한/승인을 특정 Canonical Action과 제한된 실행 예산에 묶는 bounded artifact다.

논리적 의미:

```text
ActionGrantRef
├─ Principal / Audience
├─ Canonical Action Digest
├─ Target Resource / Scope
├─ Allowed Operation
├─ Execution / Cost Budget
├─ Expiry
├─ Maximum Use / Remaining Use
├─ Delegation Attenuation
├─ Policy / Approval Revision
└─ Consumption / Revocation State
```

## 8. Semantic Replay 방지

다음이 동일 승인을 무한 반복하지 못한다.
- crash retry
- LLM replan
- Manager→Researcher delegation
- Provider retry/fallback
- resume/new Execution
- duplicate message/delivery

새 Execution/Provider attempt를 만들 수 있어도 grant의 action digest/remaining use/budget이 허용 semantic action 수를 제한한다. 기존 idempotency/Side Effect Ledger를 대체하지 않는다.

## 9. PEP 위치

- Command Gateway
- Channel dispatch
- Memory recall/commit/promotion
- Project/Channel publication
- Task/Live Control
- Provider Host
- Tool/Side Effect preflight
- Artifact/history read

각 PEP는 current policy/revision을 검증한다. stale cached decision이 authority가 아니다.

## 10. Revocation / Declassification / Grant Race

- membership/authority revoke 후 신규 access/control denied
- declassification approval revoke/expiry 후 stale publication commit denied
- ActionGrant revoke/exhaust 후 신규 covered action 시작 denied
- 이미 시작된 immutable Execution Context는 retroactive rewrite하지 않음
- external effect 직전 current authorization/grant 재확인이 필요한 class는 policy가 지정

## 11. Audit / Secret

필요 범위에서 principal/security domain/resource/action/policy revision/decision reason/grant ref/consumption/declassification/source-target scope를 감사한다. secret/token 원문, sensitive Memory content를 metric label/diagnostic에 넣지 않는다.

## 12. Sandbox 비회귀

v0.5 sandbox unavailable → unsandboxed fallback 금지, Provider/Plugin/Tool least privilege, high-risk Side Effect preflight 의미를 유지한다.

## 13. 검증 기준

- 기존 AT-SEC-001/002 유지.
- AT-SEC-003 Private→Shared unauthorized information flow 0.
- AT-SEC-004 grant budget보다 많은 semantic action/Side Effect 0.
- Role/Prompt/Model/Tool output이 authority를 생성하지 않음.
- 각 PEP의 Allow/Deny 의미가 Common Authorization Decision과 drift하지 않음.
