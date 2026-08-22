---
title: "보안·Principal·Approval·Authority·Local CLI Threat Model"
document_id: "DXB-RUN-032"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-RUN-031", "DXB-DOM-023", "DXB-DOM-025", "DXB-DOM-029", "DXB-ARC-012"]
---
# 보안·Principal·Approval·Authority·Local CLI Threat Model

## 1. Principal과 trust boundary

```text
AuthenticatedPeerContext(OS UID, process peer facts)
+ InstanceId
→ LocalPrincipalRef
```

P0는 같은 OS UID의 모든 process를 동일 LocalPrincipal로 취급한다. 이는 process sandbox가 아니라 **OS account boundary**다. 같은 UID의 악성 process를 서로 격리한다고 주장하지 않는다. stronger isolation/impersonation은 후속 IAM 계약이 필요하다.

client의 profile, PrincipalRef, BotId, `--yes`는 Authority가 아니다.

## 2. AuthorityBinding

```text
AuthorityBindingId / Generation
SubjectRef / ScopeRef / AllowedActionSet
IssuerDecisionRef / PolicyGeneration
State: Active | Superseded | Revoked | Expired
```

Security owner가 생성·revoke하고 Membership은 ref만 저장한다. RoleRef 자체로 권한을 부여하지 않는다.

## 3. Approval과 ActionGrant

```text
Approval: Requested → Approved | Denied | Expired | Revoked
ActionGrant: Active → Consumed | Expired | Revoked
```

Approval은 Principal, action digest, resolved target, policy generation, expiry, revision에 결박된다. 고위험 operation이 Approval을 요구하면 Runtime이 Approval을 생성한다. approve/deny는 expected revision으로 winner 하나를 결정한다.

ActionGrant는 Approved Approval 또는 current AuthorityDecision에서 발급하며 action/target/budget/deadline/use-count를 제한한다. CLI가 임의 생성하거나 raw token을 log/journal에 저장하지 않는다.

## 4. Default security semantic

unknown action/target은 deny한다. owner principal의 local administration도 declassification, authority issuance, destructive/high-risk external side effect는 explicit policy/Approval을 요구한다. `--yes`는 local prompt만 생략한다.

## 5. Endpoint/secret/terminal/export

runtime directory는 user-owned `0700`, endpoint는 no-follow/peer credential/InstanceId/HostGeneration을 검증한다. secret를 argv/plain log/receipt/local journal/metric label에 저장하지 않는다.

human terminal은 C0/C1, ANSI CSI/OSC/DCS/APC/PM, bidi/invisible 위험을 neutralize한다. Linux export는 descriptor-relative no-follow, exclusive temp, bounded write, file fsync, atomic no-replace, parent fsync를 사용하고 보장 불가 시 fail closed한다.

## 6. Information flow

Private/Sensitive → Project/Channel/Export는 Declassification decision과 audit를 요구한다. Model/Tool output이 principal, selector, path, shell command, permission을 스스로 승인하지 못한다.
