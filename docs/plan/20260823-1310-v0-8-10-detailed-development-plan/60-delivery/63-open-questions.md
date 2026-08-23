---
title: "v0.8.10 사용자 실사용 결정 기록과 후속 질문"
document_id: "DXB-DEL-063"
version: "0.8.10"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-GOV-003", "DXB-GOV-005", "DXB-DEL-060", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042", "DXB-IFC-043", "DXB-ARC-018"]
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

## Deferred implementation choices

concrete storage/Harness 제품, numeric retention/byte/timeout/compatibility window, stronger same-UID isolation, TUI/Web/Plugin lifecycle CLI는 executable evidence와 별도 ADR 대상이다. 이 항목들은 Workspace/M1A/M1B 진입을 막지 않는다.

## Blocking semantic questions

CLI까지의 기존 P0 scope를 사람이 사용하기 위한 미결 규범 질문은 없다. 실제 Acceptance가 PASS하기 전 M2 이후 subset freeze와 product usability GO는 계속 BLOCKED다.
