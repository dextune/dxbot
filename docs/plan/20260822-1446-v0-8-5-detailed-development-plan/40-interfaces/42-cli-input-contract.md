---
title: "P0 CLI Typed Input Contract"
document_id: "DXB-IFC-042"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-IFC-040", "DXB-RUN-032", "DXB-RUN-035"]
---
# P0 CLI Typed Input Contract

## 1. 공통 문법

- selector는 `--*-id` 또는 scoped exact name/alias 중 operation이 허용한 하나만 사용한다.
- mutation은 required expected revision/generation을 생략할 수 없다.
- content/input source는 적용 가능한 경우 `--text`, `--input-file`, `--stdin`, `--artifact-ref` 중 정확히 하나다.
- mutation identity는 caller 제공 또는 CLI 생성 `CommandId`/`IdempotencyKey`다.
- `--request-id`는 recovery identity가 아니다.
- `--all`은 JSONL incremental paging만 사용하며 full JSON array를 만들지 않는다.
- secret/credential input은 argv가 아니라 scoped credential mechanism을 사용한다.
- option conflict/default는 Application operation metadata에서 생성한다.

## 2. Command input registry

| CLI path | Required typed input / conflicts |
|---|---|
| `runtime start` | `--instance`; optional `--ready-at control`/timeout. Application wait flags 금지. |
| `runtime status` | `--instance`; no mutation identity. |
| `runtime stop` | `--instance`; graceful default. host mode requires `--host-stop --host-generation` and confirmation. |
| `runtime doctor` | `--instance`; optional bounded section/page flags. |
| `version` | optional instance; safe discovery only. |
| `bot create` | `--name`; optional brain/permission/resource/provider policy refs. Omission resolves server defaults. |
| `bot list` | page flags only. |
| `bot show` | exactly one Bot selector. |
| `bot activate` | `--bot-id --expected-revision`. |
| `bot deactivate` | `--bot-id --expected-revision`. |
| `bot archive` | `--bot-id --expected-revision` + confirmation. |
| `bot restore` | `--bot-id --expected-revision`. |
| `conversation show` | BotId or ConversationId; ambiguous name prohibited. |
| `conversation send` | `--conversation-id --expected-revision`; exactly one content source. |
| `conversation history` | ConversationId + page/export flags. |
| `thread create` | parent ID+revision; optional title. |
| `thread list` | parent scope + page. |
| `thread show` | ThreadId and parent scope. |
| `thread send` | ThreadId+revision; exactly one content source. |
| `thread history` | ThreadId + page/export. |
| `thread branch` | ThreadId+source revision+message ID; optional title. |
| `task submit` | owner/scope ID+revision; exactly one intent/input source; optional `--delegate-to-bot` and sender/correlation/budget. |
| `task list` | one scope selector + page/filter. |
| `task show` | TaskId or scoped exact selector. |
| `task watch` | TaskId; optional cursor. |
| `task cancel` | TaskId+expected revision/execution generation; optional reason. |
| `task suspend` | TaskId+expected revision/execution generation; optional reason. |
| `task resume` | TaskId+expected revision. |
| `task redirect` | TaskId+expected revision; exactly one replacement input source. |
| `task result` | TaskId; optional artifact ID and safe output. |
| `memory get` | MemoryId and scope if required. |
| `memory search` | one Memory scope + query + page. |
| `memory history` | MemoryId + page. |
| `memory propose` | scope+expected revision; statement or artifact ref; provenance/evidence. |
| `memory promote` | proposal ID+revision, target scope+revision, evidence/declassification ref as required. |
| `project create` | name+namespace revision+owner BotId. |
| `project list` | page. |
| `project show` | Project selector. |
| `project archive` | ProjectId+revision+confirmation. |
| `project restore` | ProjectId+revision. |
| `project member set` | ProjectId+revision, member BotId, RoleRef, AuthorityBindingRef, `--if-membership-generation <n|absent>`. |
| `project member remove` | ProjectId+revision, member BotId, expected membership generation. |
| `project member list` | ProjectId + page. |
| `channel create` | ProjectId+revision+name; initial member bindings optional and bounded. |
| `channel list` | ProjectId + page. |
| `channel show` | Channel selector+Project scope. |
| `channel member set` | ChannelId+revision, member BotId, RoleRef, AuthorityBindingRef, CAS generation/absent. |
| `channel member remove` | ChannelId+revision, member BotId, expected membership generation. |
| `channel member list` | ChannelId + page. |
| `channel send` | Channel ConversationId+revision; exactly one content source. |
| `channel history` | Channel ConversationId + page/export. |
| `process show` | ProcessId. |
| `process watch` | ProcessId; optional cursor. |
| `operation show` | exactly one operation/command/idempotency key. Idempotency lookup uses authenticated principal. |
| `operation reconcile` | OperationId+receipt revision. |
| `approval list` | scope/filter+page. |
| `approval show` | ApprovalId. |
| `approval approve` | ApprovalId+revision; Runtime authorization required. |
| `approval deny` | ApprovalId+revision; optional reason. |
| `provider list` | optional capability + page. |
| `provider show` | ProviderId+generation. |
| `side-effect reconcile` | SideEffectId+revision; typed evidence/action; approval as policy requires. |

<!-- p0-input-registry:start -->
- `runtime start` | `--instance`; optional `--ready-at control`/timeout. Application wait flags 금지.
- `runtime status` | `--instance`; no mutation identity.
- `runtime stop` | `--instance`; graceful default. host mode requires `--host-stop --host-generation` and confirmation.
- `runtime doctor` | `--instance`; optional bounded section/page flags.
- `version` | optional instance; safe discovery only.
- `bot create` | `--name`; optional brain/permission/resource/provider policy refs. Omission resolves server defaults.
- `bot list` | page flags only.
- `bot show` | exactly one Bot selector.
- `bot activate` | `--bot-id --expected-revision`.
- `bot deactivate` | `--bot-id --expected-revision`.
- `bot archive` | `--bot-id --expected-revision` + confirmation.
- `bot restore` | `--bot-id --expected-revision`.
- `conversation show` | BotId or ConversationId; ambiguous name prohibited.
- `conversation send` | `--conversation-id --expected-revision`; exactly one content source.
- `conversation history` | ConversationId + page/export flags.
- `thread create` | parent ID+revision; optional title.
- `thread list` | parent scope + page.
- `thread show` | ThreadId and parent scope.
- `thread send` | ThreadId+revision; exactly one content source.
- `thread history` | ThreadId + page/export.
- `thread branch` | ThreadId+source revision+message ID; optional title.
- `task submit` | owner/scope ID+revision; exactly one intent/input source; optional `--delegate-to-bot` and sender/correlation/budget.
- `task list` | one scope selector + page/filter.
- `task show` | TaskId or scoped exact selector.
- `task watch` | TaskId; optional cursor.
- `task cancel` | TaskId+expected revision/execution generation; optional reason.
- `task suspend` | TaskId+expected revision/execution generation; optional reason.
- `task resume` | TaskId+expected revision.
- `task redirect` | TaskId+expected revision; exactly one replacement input source.
- `task result` | TaskId; optional artifact ID and safe output.
- `memory get` | MemoryId and scope if required.
- `memory search` | one Memory scope + query + page.
- `memory history` | MemoryId + page.
- `memory propose` | scope+expected revision; statement or artifact ref; provenance/evidence.
- `memory promote` | proposal ID+revision, target scope+revision, evidence/declassification ref as required.
- `project create` | name+namespace revision+owner BotId.
- `project list` | page.
- `project show` | Project selector.
- `project archive` | ProjectId+revision+confirmation.
- `project restore` | ProjectId+revision.
- `project member set` | ProjectId+revision, member BotId, RoleRef, AuthorityBindingRef, `--if-membership-generation <n|absent>`.
- `project member remove` | ProjectId+revision, member BotId, expected membership generation.
- `project member list` | ProjectId + page.
- `channel create` | ProjectId+revision+name; initial member bindings optional and bounded.
- `channel list` | ProjectId + page.
- `channel show` | Channel selector+Project scope.
- `channel member set` | ChannelId+revision, member BotId, RoleRef, AuthorityBindingRef, CAS generation/absent.
- `channel member remove` | ChannelId+revision, member BotId, expected membership generation.
- `channel member list` | ChannelId + page.
- `channel send` | Channel ConversationId+revision; exactly one content source.
- `channel history` | Channel ConversationId + page/export.
- `process show` | ProcessId.
- `process watch` | ProcessId; optional cursor.
- `operation show` | exactly one operation/command/idempotency key. Idempotency lookup uses authenticated principal.
- `operation reconcile` | OperationId+receipt revision.
- `approval list` | scope/filter+page.
- `approval show` | ApprovalId.
- `approval approve` | ApprovalId+revision; Runtime authorization required.
- `approval deny` | ApprovalId+revision; optional reason.
- `provider list` | optional capability + page.
- `provider show` | ProviderId+generation.
- `side-effect reconcile` | SideEffectId+revision; typed evidence/action; approval as policy requires.
<!-- p0-input-registry:end -->
## 3. Parser freeze Gate

다음이 모두 충족된 뒤 parser/help/schema snapshot을 freeze한다.

1. `DXB-ARC-018` M1A Storage Proof PASS
2. operation metadata source와 command/input registry set equality
3. submission crash-window fixture PASS
4. bootstrap principal/default policy fixture PASS
5. selector/revision/option conflict golden PASS
