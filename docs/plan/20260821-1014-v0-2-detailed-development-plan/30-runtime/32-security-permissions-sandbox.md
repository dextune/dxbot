---
title: "보안·권한·승인·Sandbox"
document_id: "DXB-RUN-032"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-012", "DXB-ARC-016", "DXB-DOM-025", "DXB-RUN-031"]
---

# 보안·권한·승인·Sandbox

## 1. 목적

지속형 Bot과 Provider/Plugin이 장기간 권한을 보유하는 위험을 최소 권한·격리·감사 가능한 정책으로 통제한다.

## 2. 기본 원칙

- default deny
- 모델/Prompt가 권한을 생성하지 못함
- Core는 immutable 최소 grant 사용
- Provider/Plugin은 ambient host authority를 받지 않음
- secret 원문은 Domain/Memory/Event/Plugin config export에 저장하지 않음
- stale Lease/Plugin generation은 write 권한을 잃음
- sandbox unavailable 시 unsandboxed fallback 금지

## 3. Principal / Resource

Principal: User/Operator, Bot, Core Execution, Service, Provider, Plugin, Remote Worker, break-glass operator.

Resource: Bot/Memory/Task/Artifact, workspace path, Tool/Skill, Model/Provider, Plugin, network destination, SecretRef, Control operation.

Decision: `allow | deny | ask | allow-with-constraints`.

## 4. Core Capability Grant

allowed tools/actions, workspace roots/mode, network allowlist, model/provider, secret handles, max process/resource, expiry, lease/fencing, delegation scope를 포함한다. 추가 권한은 새 grant revision으로 발급한다.

## 5. Provider Security

- Provider manifest/config validation
- credential scope 최소화
- process/network/filesystem 권한을 capability 요구에 맞게 제한
- provider raw payload는 trust/sensitivity label
- Provider A 권한을 Provider B에 자동 재사용하지 않음
- Provider 제거/credential revoke 시 신규 selection 즉시 차단

## 6. Plugin Security

Plugin manifest는 requested permissions를 선언한다.

필수 통제:
- package digest/signature policy
- required/provided capability validation
- filesystem/network/process/secret/control access 명시 grant
- config/data schema validation
- resource budget
- install/enable/upgrade/uninstall audit
- native/unsafe code 별도 review
- failure/process isolation
- dependency/SBOM/vulnerability review

Plugin이 `Control Plane internal` 또는 `DB handle`을 ambient capability로 받지 않는다.

## 7. Plugin Lifecycle Approval

다음은 risk에 따라 structured approval 대상이 될 수 있다.
- 신규 high-risk permission grant
- package digest 변경 upgrade
- irreversible data migration
- plugin-owned data purge
- unsigned/untrusted source
- break-glass enable

Action digest가 바뀌면 기존 approval을 재사용하지 않는다.

## 8. Tool / Side Effect 보안 흐름

`Schema → Untrusted Normalize → Permission → Approval → Resource Admission → Side Effect Intent Commit → Sandbox/Secret Resolve → Execute → Output Limit → Outcome/Audit`

비멱등 외부 변경은 permission 승인만으로 write-ahead ledger를 생략하지 않는다.

## 9. Secret

SecretRef/opaque handle만 전달하고 실제 Provider/Tool 호출 직전에 resolve한다. subprocess/Plugin host에는 필요한 값만 일회성 주입하며 output/log redaction canary를 둔다. Bot Message와 Plugin-owned long-term data에 secret 원문 전달을 금지한다.

## 10. Untrusted Content

Web/file/message/tool/plugin output은 기본 untrusted. instruction과 data channel을 구분하고 sensitive action 전에 policy/approval을 재평가한다. Plugin이 반환한 “approved” 문자열은 실제 Approval Decision이 아니다.

## 11. 예외

- audit sink unavailable: 민감 write fail-closed 또는 durable protected spool
- Plugin host crash: Plugin scope quarantine, Runtime 지속
- signature invalid: install/upgrade reject
- permission revoke during execution: 신규 call 차단, high-risk active call cancel policy
- data migration failure: old version 유지/rollback 또는 disabled

## 12. 검증 기준

- AT-PLUGIN-001에서 permission 없는 Plugin의 fs/network/secret 접근이 거부된다.
- Provider/Plugin revoke 후 신규 Execution이 해당 grant를 사용하지 않는다.
- secret canary가 prompt/log/event/plugin export에 나타나지 않는다.
- sandbox/Plugin host 장애가 host unsandboxed execution으로 fallback하지 않는다.
