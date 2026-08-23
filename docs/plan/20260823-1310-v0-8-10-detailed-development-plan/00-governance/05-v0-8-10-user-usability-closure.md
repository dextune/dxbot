---
title: "v0.8.10 사용자 실사용 CLI 계약 폐쇄"
document_id: "DXB-GOV-005"
version: "0.8.10"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-BASE-000", "DXB-GOV-003", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042", "DXB-IFC-043", "DXB-RUN-032", "DXB-RUN-033", "DXB-RUN-035", "DXB-DEL-060", "DXB-DEL-061"]
---
# v0.8.10 사용자 실사용 CLI 계약 폐쇄

이 문서는 v0.8.9의 내부 정확성을 유지하면서 실제 사용자가 첫 실행부터 복구까지 완료할 수 없던 관계성 결함을 기능 확장 없이 닫는다. 동일 package의 과거 문장과 충돌하면 본 문서와 `DXB-IFC-043`의 v0.8.10 규칙이 우선한다.

## 1. 사용자에게 노출할 것과 숨길 것

사용자는 domain selector, 의도, local rendering/wait/output 선택만 제공한다. 다음 값은 CLI가 bounded preflight와 verified endpoint에서 materialize한다.

```text
InstanceId / HostGeneration
canonical target ID
expected aggregate revision
execution/provider/membership generation
Bot Main ConversationId
Channel ConversationId
```

이 값은 wire contract에서 사라지는 것이 아니라 `Prepared` 이전 `CommandPayload`에 명시적으로 들어간다. Runtime은 selector, canonical ID, revision/generation, authorization을 독립적으로 검증한다.

## 2. preflight와 mutation safety

- canonical ID selector와 explicit `--if-*` CAS가 모두 있으면 불필요한 preflight를 생략할 수 있다.
- 이름·scope selector 또는 CAS 생략 시 CLI는 같은 authenticated Principal로 bounded Query를 수행한다.
- preflight 결과가 0개 또는 복수면 journal과 CommandId를 만들지 않는다.
- materialized target/CAS를 포함한 뒤 RequestDigest와 `Prepared`를 생성한다.
- preflight 이후 target이 변하면 Runtime이 conflict를 반환한다.
- CLI는 mutation conflict를 자동 재시도하거나 최신 revision으로 몰래 갱신하지 않는다.

## 3. first-run과 Instance 선택

`dxb runtime start`는 explicit local selection이 없고 기존 verified Instance도 없을 때만 deterministic default local Instance를 bootstrap한다. 성공적으로 authenticated endpoint를 검증한 뒤에만 default profile binding을 atomic write한다.

다른 command는 Runtime을 자동 시작하지 않는다. 선택 우선순위는 explicit `--instance`, selected profile binding, 단 하나의 verified local endpoint다. 0개는 runtime-unavailable, 복수는 ambiguous-target이며 다른 endpoint로 silent fallback하지 않는다.

## 4. command target 수렴

- `conversation show/send/history`는 ConversationId 또는 Bot selector를 받아 Bot Main Conversation을 resolve한다.
- `thread` command는 Thread selector만 요구하고 redundant parent ID를 사용자에게 강제하지 않는다.
- `channel send/history`는 Channel selector를 받아 Channel Conversation을 resolve한다.
- Bot/Task/Project/Channel/Approval/Operation/Provider/Memory/Process mutation은 selector와 optional `if-*` CAS를 사용한다.
- CreateBot committed payload는 최소 `BotRef`, `BotRevision`, `MainConversationRef`를 반환한다.

## 5. help·rendering·automation

- `dxb`, `dxb --help`, group/command `--help`는 local-only이며 exit 0이다.
- unknown command는 실행하지 않고 exit 2와 bounded suggestion만 제공한다.
- default rendering은 human이다. `--format json`은 단일 result, `--format jsonl`은 stream에 사용한다.
- machine output은 TTY 여부에 따라 shape가 변하지 않는다.
- progress, color, pager, prompt는 stderr 또는 controlling TTY만 사용하고 machine stdout을 오염시키지 않는다.
- non-TTY는 prompt를 열지 않는다.

## 6. actionable failure와 recovery

stable error/result에는 code, category, retryability, visible refs, field violations, optional resume cursor와 typed next action을 포함한다.

- runtime-unavailable → `runtime status/start/doctor`
- provider-unavailable → `runtime doctor --section provider`
- approval-required → `approval show/approve/deny`
- ambiguous-target → visible bounded candidates와 canonical selector
- conflict → current visible revision/generation과 show command
- recovery-required → `operation show/reconcile`
- partial-or-resync → resume cursor 또는 restart requirement

Local Journal startup scan은 current Instance에 한해 bounded하게 수행한다. `Prepared`만 provably-unsent이며 lock-free·integrity-valid·retention-eligible record는 policy에 따라 prune할 수 있다. `Dispatching` 이상은 먼저 Runtime lookup하고, both-binding-absent가 확인된 경우에만 같은 CommandId/IdempotencyKey/RequestDigest로 replay한다.

## 7. 과도설계 방지

새 command, hidden current-context authority, auto-start, auto mutation retry, fuzzy mutation target, generic prompt framework, shell completion subsystem, TUI/Web/Plugin scope를 추가하지 않는다. 기존 typed operation과 local projection만 정렬한다.
