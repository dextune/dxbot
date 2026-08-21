---
title: "보안·권한·승인·Sandbox"
document_id: "DXB-RUN-032"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-012", "DXB-ARC-016", "DXB-ARC-017", "DXB-DOM-025", "DXB-DOM-027", "DXB-RUN-031", "DXB-RUN-036"]
---

# 보안·권한·승인·Sandbox

## 1. 목적

지속형 Bot/Conversation/Thread와 Provider/Plugin의 장기 권한 위험을 최소 권한·격리·감사로 통제하고, Main Conversation의 자연어 control intent가 권한 경계를 우회하지 못하게 한다.

## 2. 기본 원칙

- default deny
- Prompt/모델/Provider/Tool/Plugin output이 권한 생성 금지
- Core는 immutable 최소 grant
- Provider/Plugin ambient host authority 금지
- secret 원문 Domain/Memory/Event/Conversation export 저장 금지
- stale Lease/Provider/Plugin/Directive generation은 write 권한 상실
- sandbox unavailable → unsandboxed fallback 금지
- destructive control은 authenticated principal + scope + 필요 시 approval

## 3. Principal / Resource

Principal: User/Operator, Bot, Core Execution, Service, Provider, Plugin, Remote Worker, break-glass operator.

Resource에 기존 Bot/Memory/Task/Artifact/Tool/Model/Plugin/Network/Secret/Control operation 외 다음을 명시한다.
- Main Conversation
- Thread / Thread-local Memory
- Conversation Message history
- Task Control target / Directive
- Thread branch/archive/export

## 4. Supervisor Control Authority

Main Conversation에 “B 작업 취소해”라는 문자열이 존재한다고 바로 cancel하지 않는다.

```text
Natural-language Intent
→ authenticated principal
→ target Bot/Thread/Task resolution
→ permission/scope evaluation
→ approval if required
→ expected revision/idempotency
→ Control Command/Directive
```

- Thread A 권한으로 Thread B를 제어하지 못한다.
- resource budget 확대/reprioritize도 ceiling/approval을 따른다.
- Prompt injection이 redirect/suspend/cancel/fork authority를 획득하지 못한다.
- Provider native steering API를 Provider가 임의 호출해 Task를 변경하지 못한다.

## 5. Provider Host Security

v0.3의 normalize → selection/pin → permission/approval → resource → side-effect guard → scoped credential → provider → output validation/audit 파이프라인을 유지한다.

Provider Session/resume token은 sensitive provider metadata로 분류할 수 있으며 Thread identity나 authorization credential로 사용하지 않는다.

## 6. Thread / Memory Security

- Thread-local Memory recall은 BotId+ThreadId scope authorization을 확인한다.
- Thread branch는 source history/Memory access 권한을 재평가한다.
- Bot-global Memory promotion은 sensitivity/trust/permission을 다시 평가한다.
- archive/export/forget는 raw conversation history와 Memory를 별도 resource로 취급한다.

## 7. Live Control + Side Effect

Suspend/cancel/redirect가 permission 승인을 받았어도 non-idempotent Side Effect ledger를 생략하지 않는다. active high-risk call에 revoke/control이 들어오면 Capability safe cancellation 가능 여부와 Unknown outcome risk를 고려한다.

## 8. Audit

actor, target Bot/Thread/Task/Execution, DirectiveId, expected/result revision, reason, approval, outcome, superseded/conflict를 기록한다. inspect/report의 민감 history 조회도 policy에 따라 audit한다.

## 9. 검증 기준

- Prompt/Provider output이 Control authority를 만들지 못함.
- unauthorized Thread에서 다른 Thread Message/Memory/Task control 불가.
- Provider Session token이 ThreadId/auth token으로 사용되지 않음.
- secret canary가 Conversation/Memory/log/trace/provider diagnostic에 나타나지 않음.
- sandbox/host 장애가 unsandboxed fallback으로 전환되지 않음.
