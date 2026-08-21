---
title: "보안·권한·정보 흐름·승인·Sandbox"
document_id: "DXB-RUN-032"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-012", "DXB-ARC-016", "DXB-ARC-017", "DXB-DOM-025", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-031", "DXB-RUN-036"]
---

# 보안·권한·정보 흐름·승인·Sandbox

## 1. 목적

v0.6의 Common Authorization Decision Owner, Information Flow/Declassification, Durable ActionGrant, Untrusted Proposal, Sandbox 의미를 유지하고, v0.7 CLI principal/credential/approval boundary를 추가한다.

## 2. Common Authorization Owner 비회귀

Authorization policy SSOT는 계속 `DXB-RUN-032` 하나다. CLI, Control Endpoint, Application Layer는 PEP/consumer일 뿐 Role→Authority/Permission/Approval policy를 재구현하지 않는다.

## 3. CLI Principal Resolution

local single-user Runtime에서도 모든 control request는 Runtime이 이해하는 `PrincipalRef`/`SecurityDomainRef`로 귀결되어야 한다.

- OS/local principal과 DXBOT principal relation을 명시적으로 resolve한다.
- ambient shell 권한을 DXBOT Authority와 동일시하지 않는다.
- endpoint/profile 변경이 privilege escalation을 만들지 않는다.
- client가 보낸 Role 문자열을 current Authority로 신뢰하지 않는다.
- target mutation 직전 current policy/revision을 Runtime이 검증한다.

## 4. Credential / Secret Boundary

기본 원칙:
- secret를 command history에 평문으로 넣도록 요구하지 않는다.
- `--password xxx`, `--token xxx` 같은 argv secret를 기본 UX로 만들지 않는다.
- env/config/keyring/agent 등 exact 저장 backend는 Security ADR 대상이다.
- config show/diff, trace, doctor, export, error context가 secret를 노출하지 않는다.
- debug/verbose flag가 redaction을 자동 해제하지 않는다.
- secret/token/raw sensitive content를 metric label로 사용하지 않는다.

## 5. Approval / ActionGrant

CLI confirmation과 Authorization approval을 혼합하지 않는다.

```text
interactive confirmation
= 사용자의 로컬 UX guard

Approval / ActionGrant
= Runtime이 검증하는 Canonical security semantic
```

`dxb approval approve` 같은 command가 있어도 target action digest/principal/revision/expiry/budget/use를 Runtime이 검증한다. `--yes`는 ActionGrant를 생성하지 않는다.

## 6. Information Flow 비회귀

Private/Sensitive→Project/Channel Shared publication은 CLI가 read/write 권한을 모두 갖더라도 Information-Flow/Declassification 검사를 통과해야 한다. `memory promote`, `export`, `channel send` 등의 convenience command가 이를 우회하지 않는다.

## 7. Control Endpoint PEP

Control Endpoint는 최소 다음을 enforcement한다.
- authenticated/resolved principal
- current authorization
- request size/decode limits
- version compatibility
- required approval/grant reference validation path

그러나 Domain policy의 Canonical 의미를 adapter 내부에 복제하지 않는다.

## 8. 검증 기준

- CLI local Role/OS privilege가 Runtime Authority를 생성하지 않는다.
- argv/config/log/trace/export secret leakage 0.
- `--yes`/non-interactive가 approval/ActionGrant를 우회하지 않는다.
- stale grant/revision mutation이 denied/conflict된다.
- AT-SEC-001~004 및 Sandbox 비회귀 0.
