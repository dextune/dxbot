---
title: "v0.8.10 사용자 실사용 결정 기록과 후속 질문"
document_id: "DXB-DEL-063"
version: "0.8.10"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-30"
depends_on: ["DXB-GOV-003", "DXB-GOV-005", "DXB-DEL-060", "DXB-DEL-068", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042", "DXB-IFC-043", "DXB-ARC-018"]
---
# v0.8.10 사용자 실사용 결정 기록과 후속 질문

## Resolved

- first `runtime start`는 기존 verified Instance가 없을 때 safe default bootstrap을 제공한다.
- 다른 command는 auto-start하거나 다른 Instance로 silent fallback하지 않는다.
- profile/instance는 local discovery hint이고 authenticated endpoint가 canonical Instance를 결정한다.
- user-facing mutation은 domain selector와 optional if-CAS를 사용한다.
- CLI는 `Prepared` 이전 preflight로 canonical ID와 required CAS를 materialize하고 Runtime이 재검증한다.
- mutation conflict는 자동 retry하지 않는다.
- Bot selector로 Main Conversation을, Channel selector로 Channel Conversation을 사용한다.
- Thread command는 redundant parent ConversationId를 강제하지 않는다.
- CreateBot committed payload는 BotRef/MainConversationRef를 제공한다.
- local help/version, human/json/jsonl, non-TTY, typed next action을 닫는다.
- provider-unavailable은 M2 runtime doctor provider section으로 진단한다.
- Prepared reuse/prune와 Dispatching lookup/replay의 사용자 재진입을 닫는다.
- `--all`과 `--output`은 bounded/partial/resume/no-replace 의미를 가진다.
- `UJ-001~010`은 문서 계약 기준 GO다.
- production LLM 모델은 `MiniMax-M3`로 고정하며 다른 모델로 silent fallback하지 않는다.
- 공식 `https://github.com/deepseek-ai/deepseek-harness`를 DXBOT Harness adapter 대상으로 채택한다.
- 공식 Harness 통합은 pinned ACP v1 stdio subprocess boundary를 사용하고 Cordis/Node in-process embedding을 금지한다.

## Deferred implementation choices

concrete storage 제품, numeric retention/byte/timeout/compatibility window, stronger same-UID isolation, TUI/Web/dynamic Plugin lifecycle CLI는 executable evidence와 별도 ADR 대상이다. Harness 제품 선택은 더 이상 deferred가 아니며 exact snapshot은 `DXB-DEL-064`, 단계·gate·중단 조건은 [DXB-DEL-068](68-modular-harness-adoption-plan.md)이 소유한다. 이 구현 선택들은 Workspace/M1A/M1B의 과거 진입 판정을 소급 변경하지 않는다.

## Blocking semantic questions

CLI까지의 기존 P0 scope를 사람이 사용하기 위한 미결 규범 질문은 없다. Harness Modularization Complete는 `DXB-DEL-068`의 executable evidence 전까지 BLOCKED다.
