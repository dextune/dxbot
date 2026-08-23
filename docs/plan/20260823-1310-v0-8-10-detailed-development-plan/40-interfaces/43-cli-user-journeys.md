---
title: "CLI 사용자 여정·Discovery·Rendering 계약"
document_id: "DXB-IFC-043"
version: "0.8.10"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042", "DXB-RUN-032", "DXB-RUN-033", "DXB-RUN-035"]
---
# CLI 사용자 여정·Discovery·Rendering 계약

이 문서는 Domain/API 의미를 다시 정의하지 않는다. 기존 typed operation을 사람이 처음 배우고, 반복 사용하고, 자동화하고, 실패 후 복구할 수 있도록 local CLI projection의 Canonical Owner를 제공한다.

## 1. Global local option registry

다음 option은 local selection 또는 projection이며 Authority, CommandPayload, RequestDigest를 만들지 않는다.

<!-- cli-global-option-registry:start -->
- `profile` | `--profile <name>` | verified endpoint binding 선택 | `local-only`
- `instance` | `--instance <selector>` | explicit local Instance 선택 | `local-only`
- `format` | `--format <human,json,jsonl>` | result/stream rendering | `local-only`
- `color` | `--color <auto,always,never>` | ANSI rendering | `local-only`
- `wait` | `--wait <allowed-predicate>` | local observation return point | `local-only`
- `timeout` | `--timeout <duration>` | local request/observation deadline | `local-only`
- `yes` | `--yes` | allowed confirmation prompt skip | `local-only`
<!-- cli-global-option-registry:end -->

unknown profile/Instance, stale descriptor, permission failure는 다른 Instance로 fallback하지 않는다. selected InstanceId는 human mutation output에 표시하고 machine result에는 항상 포함한다.

## 2. Help와 첫 실행

- `dxb`와 `dxb --help`는 top-level groups, 가장 짧은 first-run path, machine format 사용법을 보여준다.
- group/command help는 metadata에서 생성하고 required/optional/local/source/CAS를 구분한다.
- help example은 secret 또는 긴 content를 argv에 넣도록 권장하지 않고 stdin/input-file을 우선한다.
- `dxb version`은 client version과 supported protocol/schema를 Runtime 없이 출력한다. endpoint가 있으면 remote compatibility를 추가한다.
- verified Instance가 전혀 없을 때 `dxb runtime start`는 safe default를 bootstrap한다.
- 다른 command는 auto-start하지 않고 exit 10과 typed `runtime-start` action을 반환한다.

## 3. User selector와 automatic preflight

사람은 Bot/Conversation/Thread/Task/Project/Channel/Memory/Approval/Operation/Provider/Process selector를 입력한다. CLI가 내부 ID/revision/generation을 찾기 위해 수행하는 preflight는 다음 제약을 가진다.

1. same authenticated Principal과 selected Instance를 사용한다.
2. bounded query이며 0개/복수/deny에서 중단한다.
3. target canonical ID와 required CAS를 `Prepared` 전에 materialize한다.
4. Runtime이 다시 resolve/authorize/CAS 검증한다.
5. conflict 발생 시 최신 revision으로 자동 retry하지 않는다.
6. fuzzy match는 help suggestion에만 사용하고 mutation target으로 실행하지 않는다.

explicit canonical selector와 complete `--if-*` CAS를 준 automation은 preflight를 생략할 수 있다.

## 4. First useful path

```text
dxb runtime start
dxb bot create <name>
dxb conversation send <bot-selector> --stdin
dxb task submit --owner <bot-selector> --stdin
dxb task show <task-selector>
dxb task result <task-selector>
```

CreateBot committed payload는 BotRef/MainConversationRef를 포함한다. `conversation send <bot-selector>`는 Bot Main Conversation으로 resolve하며 사용자가 ConversationId를 먼저 조회할 필요가 없다.

provider-unavailable은 Bot identity 생성 실패가 아니다. Activate/Task admission 실패는 `runtime doctor --section provider` action과 required capability를 redacted하게 제공한다.

## 5. Human rendering

human result는 다음 순서를 유지한다.

```text
what happened
canonical visible target and selected Instance
operation/receipt state
committed resource refs or observation state
next safe action
```

content body, secret, credential, hidden target existence, unredacted policy internals을 echo하지 않는다. progress/spinner는 TTY stderr에만 쓰고 stdout을 오염시키지 않는다.

ApprovalRequired, Conflict, RecoveryRequired, ProviderUnavailable은 success처럼 보이는 green message를 사용하지 않는다.

## 6. Machine rendering

- `json`: 단일 result/error document. nonzero exit에서도 parse 가능한 complete document를 stdout에 남긴다.
- `jsonl`: Subscription/diagnostic stream event당 한 줄과 terminal record.
- stderr: optional human diagnostic/progress only. `--format json|jsonl`에서 schema-bearing payload를 stderr에 쓰지 않는다.
- TTY 여부, color, pager, locale가 machine field 이름이나 shape를 바꾸지 않는다.
- next action은 `action_code`, `command_key`, typed arguments로 표현하며 raw shell command string을 stable schema로 삼지 않는다.

## 7. Failure action registry

<!-- cli-failure-action-registry:start -->
- `runtime-unavailable` | `runtime-status,runtime-start,runtime-doctor` | endpoint 선택과 startup 진단
- `provider-unavailable` | `runtime-doctor` | `section=provider`
- `approval-required` | `approval-show,approval-approve,approval-deny` | ApprovalRef와 visible revision
- `ambiguous-target` | `show-or-list` | bounded visible candidate canonical refs
- `conflict` | `show-and-resubmit` | visible current revision/generation, no automatic retry
- `recovery-required` | `operation-show,operation-reconcile,runtime-doctor` | original IDs와 journal state
- `partial-or-resync` | `resume-or-restart` | cursor 또는 explicit restart reason
- `incompatible` | `version` | client/runtime/protocol/schema direction
<!-- cli-failure-action-registry:end -->

## 8. Local journal user experience

CLI는 selected Instance journal을 bounded scan한다.

- matching Prepared: same semantic digest면 IDs를 재사용해 continue 가능.
- stale Prepared: provably-unsent, lock-free, integrity-valid, retention-eligible일 때만 policy prune 가능.
- Dispatching/Observed: server binding lookup을 먼저 수행.
- both absent: same IDs/digest/payload replay 가능.
- uncertain/corrupt/unknown version: auto replay/delete 금지, read-only/help/version 허용, doctor action 제공.

사용자가 local interrupt를 Runtime cancel로 오해하지 않도록 human/machine result에 `operation_may_continue=true`와 OperationRef를 제공한다.

## 9. Bounded list/output

`--all`은 server unbounded flag가 아니라 bounded page loop다. local item/byte ceiling과 cancellation을 적용하며 완료하지 못하면 partial-or-resync, resume cursor와 written item count를 반환한다.

`--output`은 destination 존재 시 overwrite하지 않는다. artifact/data file과 stdout result metadata를 분리한다.

## 10. User journey readiness registry

`GO`는 active 문서에서 입력, 정상 결과, 실패, 복구, automation 의미가 모두 닫혔다는 설계 판정이다. executable product usability PASS를 뜻하지 않는다.

<!-- user-journey-registry:start -->
- `UJ-001` | local help/version과 첫 Runtime bootstrap | `AT-CLI-DISCOVERY-001` | `GO`
- `UJ-002` | Bot 생성과 Main Conversation 첫 메시지 | `AT-CLI-INTERACTIVE-001` | `GO`
- `UJ-003` | selector 기반 일상 mutation과 CAS conflict | `AT-CLI-INTERACTIVE-001` | `GO`
- `UJ-004` | non-TTY JSON/JSONL automation | `AT-CLI-AUTOMATION-001` | `GO`
- `UJ-005` | approval-required 확인과 결정 | `AT-SECSTATE-001` | `GO`
- `UJ-006` | provider-unavailable 진단 | `AT-CLI-DISCOVERY-001` | `GO`
- `UJ-007` | ambiguous target와 visible candidate 선택 | `AT-CLI-INTERACTIVE-001` | `GO`
- `UJ-008` | interrupt/crash 이후 journal·operation 복구 | `AT-CLI-RECOVERY-001` | `GO`
- `UJ-009` | pagination, --all, safe output과 resume | `AT-CLI-AUTOMATION-001` | `GO`
- `UJ-010` | multi-instance 선택과 compatibility 확인 | `AT-CLI-DISCOVERY-001` | `GO`
<!-- user-journey-registry:end -->

## 11. 비범위

shell completion subsystem, interactive wizard, hidden current Bot/Project authority, auto-start, auto mutation retry, TUI/Web, Plugin management CLI를 P0에 추가하지 않는다.
