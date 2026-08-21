---
title: "보안·권한·승인·Sandbox"
document_id: "DXB-RUN-032"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-012", "DXB-ARC-016", "DXB-ARC-017", "DXB-DOM-025", "DXB-RUN-031"]
---

# 보안·권한·승인·Sandbox

## 1. 목적

지속형 Bot과 Provider/Plugin의 장기 권한 위험을 최소 권한·격리·감사 정책으로 통제하고 Provider Host를 capability invocation의 공통 보안 enforcement point로 둔다.

## 2. 기본 원칙

- default deny
- 모델/Prompt/Provider가 권한을 생성하지 못함
- Core는 immutable 최소 grant 사용
- Provider/Plugin은 ambient host authority를 받지 않음
- secret 원문은 Domain/Memory/Event/Provider config export에 저장하지 않음
- stale Lease/Provider/Plugin generation은 write 권한을 잃음
- sandbox unavailable 시 unsandboxed fallback 금지
- Provider가 Host permission/approval path를 우회하는 별도 HTTP/Tool execution path를 만들지 않음

## 3. Principal / Resource

Principal: User/Operator, Bot, Core Execution, Service, Provider, Plugin, Remote Worker, break-glass operator.

Resource: Bot/Memory/Task/Artifact, workspace path, Tool/Skill, Model/Provider, Plugin, network destination, SecretRef, Control operation.

Decision: `allow | deny | ask | allow-with-constraints`.

## 4. Provider Host Security Pipeline

```text
Schema/Trust Normalize
→ Provider selection + version/generation pin
→ Permission/Grant evaluation
→ Approval if required
→ Resource Admission
→ Side Effect Guard
→ Scoped Credential/Transport preparation
→ Provider call
→ Output/Trust validation
→ Audit finalize
```

Provider Call Context에는 승인된 최소 handle만 포함한다. raw Domain Store, global Secret Store, unrestricted filesystem/network handle, Registry mutator를 포함하지 않는다.

## 5. Credential / Secret

SecretRef/opaque handle만 Common에 보존하고 실제 external call 직전에 scope/expiry가 제한된 credential material 또는 approved transport를 준비한다.

- Provider A credential을 B에 자동 재사용하지 않는다.
- Provider config에 secret 원문을 넣지 않는다.
- subprocess/Plugin host에는 필요한 값만 일회성 주입한다.
- output/log/trace에 redaction canary를 둔다.

## 6. Provider Security

- Provider package/config/contract version validation
- external destination/credential scope 최소화
- process/network/filesystem 권한을 capability 요구에 맞게 제한
- raw payload trust/sensitivity label
- Provider revoke/remove 시 신규 Host selection/admission 즉시 차단
- custom permission cache가 Common revision을 우회하지 않음
- Provider가 `permission granted` 문자열을 실제 Decision으로 반환할 수 없음

## 7. Plugin Security

Plugin manifest는 requested permissions를 선언한다. Plugin Provider는 Plugin package grant와 Provider call grant를 모두 충족해야 한다. Plugin Host isolation이 Provider Host authorization을 대체하지 않는다.

## 8. Tool / Side Effect 보안 흐름

`Schema → Normalize → Permission → Approval → Resource Admission → Side Effect Intent → Sandbox/Secret → Provider/Tool Execute → Output Limit → Outcome/Audit`

비멱등 외부 변경은 permission 승인만으로 write-ahead ledger를 생략하지 않는다.

## 9. Untrusted Content

Web/file/message/tool/provider/plugin output은 기본 untrusted다. instruction과 data channel을 구분하고 sensitive action 전에 policy/approval을 재평가한다.

## 10. 예외

- audit sink unavailable: 민감 write fail-closed 또는 protected bounded spool
- Provider/Plugin host crash: 해당 scope quarantine, Runtime 지속
- permission revoke during execution: 신규 call 차단, active high-risk call은 policy에 따라 cancel
- config/signature/version invalid: lifecycle validation에서 Ready 진입 금지

## 11. 검증 기준

- AT-SPI-003/004에서 Provider Host bypass와 ambient authority가 차단된다.
- Provider가 Domain Store/Secret Store/Scheduler handle을 얻지 못한다.
- revoke 후 신규 Execution이 이전 grant를 사용하지 않는다.
- secret canary가 prompt/log/event/provider/plugin export에 나타나지 않는다.
- sandbox/host 장애가 unsandboxed fallback으로 전환되지 않는다.
