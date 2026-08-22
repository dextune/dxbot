---
title: "CLI Reference Interface와 P0 Command Registry"
document_id: "DXB-IFC-041"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-IFC-040", "DXB-IFC-042", "DXB-RUN-035"]
---
# CLI Reference Interface와 P0 Command Registry

## 1. 책임

`dxb`는 parser, typed input collection, local lifecycle client, rendering, machine writer, safe file writer와 non-canonical per-command journal을 소유한다. Domain, Authorization, Resource, Provider selection 정책을 소유하지 않는다.

## 2. 전역 옵션

| option | 적용 |
|---|---|
| `--profile`, `--instance` | local discovery; Authority 아님 |
| `--json`, `--jsonl` | bounded machine output |
| `--wait <accepted|committed|terminal>` | Application Command만 |
| `--timeout` | local request/wait timeout; rollback 아님 |
| `--command-id`, `--idempotency-key` | mutation recovery identity; caller 지정 가능 |
| `--request-id` | transmission correlation only |
| `--non-interactive`, `--yes` | local prompt only; Approval 우회 불가 |
| `--output` | 허용된 Query/Artifact만 |
| `--page-size`, `--cursor`, `--all` | paged operation만 |

`runtime start`는 `--ready-at`을 사용한다. `runtime stop --host-stop`은 explicit host escalation이며 자동 fallback이 아니다.

## 3. Machine contract

- stdout에는 result JSON/JSONL만 쓴다.
- stderr에는 diagnostic/progress/sanitizer warning만 쓴다.
- JSONL terminal record 없이 exit 0 complete를 반환하지 않는다.
- local timeout/SIGINT/broken pipe는 target lifecycle을 변경하지 않는다.
- exit registry 0, 2~18의 기존 의미를 유지한다.

## 4. Durable local submission

journal path:

```text
$XDG_STATE_HOME/dxbot/cli/<profile>/<instance-id>/operations/<command-id>.jsonl
```

directory `0700`, file `0600`, exclusive create, append+fsync를 사용한다. state는 `Prepared → Sent → Observed → Terminal`이며 compaction은 per-file lock과 atomic replace를 사용한다. global map last-writer-wins 구조는 금지한다.

`operation show`는 exactly one of `--operation-id`, `--command-id`, `--idempotency-key`를 받는다. `--request-id`로 복구한다고 약속하지 않는다.

## 5. P0 Command Matrix

| CLI path | Operation | Kind | Target | Input summary | Default wait | Machine | Acceptance |
|---|---|---|---|---|---|---|---|
| `runtime start` | `StartRuntimeHost` | `H` | InstanceId | --ready-at, --timeout | ControlReady | JSON | AT-HOST-001 |
| `runtime status` | `GetHostAndRuntimeStatus` | `H+Q` | InstanceId | none | immediate | JSON | AT-HOST-001 |
| `runtime stop` | `RequestRuntimeShutdown` | `C/H` | InstanceId+HostGeneration | --host-stop is explicit | committed/control or host terminal | JSON | AT-HOST-001 |
| `runtime doctor` | `DiagnoseRuntime` | `H+Q` | InstanceId | --section?, paging | immediate | JSON/JSONL | AT-SEC-005 |
| `version` | `GetVersionCompatibility` | `Q` | InstanceId? | none | immediate | JSON | AT-SCHEMA-001 |
| `bot create` | `CreateBot` | `C` | create scope | name, optional policy refs | committed | JSON | AT-BOOT-001 |
| `bot list` | `ListBots` | `Q` | instance | page | immediate | JSON/JSONL | AT-APP-006 |
| `bot show` | `GetBot` | `Q` | Bot selector | none | immediate | JSON | AT-CLI-009 |
| `bot activate` | `ChangeBotLifecycle` | `C` | BotId+revision | none | committed | JSON | AT-APP-005 |
| `bot deactivate` | `ChangeBotLifecycle` | `C` | BotId+revision | none | committed | JSON | AT-APP-005 |
| `bot archive` | `ChangeBotLifecycle` | `C` | BotId+revision | confirmation | committed | JSON | AT-CLI-010 |
| `bot restore` | `ChangeBotLifecycle` | `C` | BotId+revision | none | committed | JSON | AT-APP-005 |
| `conversation show` | `GetConversation` | `Q` | Bot/Conversation selector | none | immediate | JSON | AT-CLI-009 |
| `conversation send` | `SendMessage` | `C` | ConversationId+revision | one content source | committed | JSON | AT-APP-005 |
| `conversation history` | `ListMessages` | `Q` | ConversationId | page/export | immediate | JSONL | AT-APP-006 |
| `thread create` | `CreateThread` | `C` | parent+revision | title? | committed | JSON | AT-APP-005 |
| `thread list` | `ListThreads` | `Q` | parent scope | page | immediate | JSON/JSONL | AT-APP-006 |
| `thread show` | `GetThread` | `Q` | Thread+parent | none | immediate | JSON | AT-CLI-009 |
| `thread send` | `SendMessage` | `C` | ThreadId+revision | one content source | committed | JSON | AT-APP-005 |
| `thread history` | `ListMessages` | `Q` | ThreadId | page/export | immediate | JSONL | AT-APP-006 |
| `thread branch` | `BranchThread` | `C` | ThreadId+revision+message | title? | committed | JSON | AT-APP-005 |
| `task submit` | `SubmitTask` | `C` | owner/scope+revision | intent/input, optional delegation | committed | JSON | AT-DELEGATE-001 |
| `task list` | `ListTasks` | `Q` | Bot/Project/Channel scope | page/filter | immediate | JSON/JSONL | AT-APP-006 |
| `task show` | `GetTask` | `Q` | Task selector | none | immediate | JSON | AT-CLI-009 |
| `task watch` | `WatchTask` | `S` | TaskId | cursor | follow | JSONL | AT-APP-007 |
| `task cancel` | `ControlTask.Cancel` | `C` | TaskId+revision/generation | reason? | committed | JSON | AT-CLI-010 |
| `task suspend` | `ControlTask.Suspend` | `C` | TaskId+revision/generation | reason? | committed | JSON | AT-CLI-010 |
| `task resume` | `ControlTask.Resume` | `C` | TaskId+revision | none | committed | JSON | AT-APP-005 |
| `task redirect` | `ControlTask.Redirect` | `C` | TaskId+revision | one input source | committed | JSON | AT-CLI-010 |
| `task result` | `GetTaskResult` | `Q` | TaskId | artifact selector/output | immediate | JSON/stream/file | AT-SEC-005 |
| `memory get` | `GetMemory` | `Q` | MemoryId/scope | none | immediate | JSON | AT-CLI-009 |
| `memory search` | `SearchMemory` | `Q` | scope | query/page | immediate | JSONL | AT-APP-006 |
| `memory history` | `GetMemoryHistory` | `Q` | MemoryId | page | immediate | JSONL | AT-APP-006 |
| `memory propose` | `ProposeMemory` | `C` | scope+revision | statement/artifact/evidence | committed | JSON | AT-APP-005 |
| `memory promote` | `PromoteMemory` | `C` | proposal+scope revisions | evidence/declass ref | committed | JSON | AT-SEC-005 |
| `project create` | `CreateProject` | `C` | namespace revision | name, owner bot | committed | JSON | AT-APP-005 |
| `project list` | `ListProjects` | `Q` | instance | page | immediate | JSON/JSONL | AT-APP-006 |
| `project show` | `GetProject` | `Q` | Project selector | none | immediate | JSON | AT-CLI-009 |
| `project archive` | `ChangeProjectLifecycle` | `C` | ProjectId+revision | confirmation | committed | JSON | AT-CLI-010 |
| `project restore` | `ChangeProjectLifecycle` | `C` | ProjectId+revision | none | committed | JSON | AT-APP-005 |
| `project member set` | `SetScopeMembership` | `C` | ProjectId+revision | member/role/authority/CAS | committed | JSON | AT-MEMBER-001 |
| `project member remove` | `RemoveScopeMembership` | `C` | ProjectId+revision+membership generation | member | committed | JSON | AT-MEMBER-001 |
| `project member list` | `ListScopeMemberships` | `Q` | ProjectId | page | immediate | JSON/JSONL | AT-MEMBER-001 |
| `channel create` | `CreateChannel` | `C` | ProjectId+revision | name, initial members? | committed | JSON | AT-APP-005 |
| `channel list` | `ListChannels` | `Q` | ProjectId | page | immediate | JSON/JSONL | AT-APP-006 |
| `channel show` | `GetChannel` | `Q` | Channel+Project | none | immediate | JSON | AT-CLI-009 |
| `channel member set` | `SetScopeMembership` | `C` | ChannelId+revision | member/role/authority/CAS | committed | JSON | AT-MEMBER-001 |
| `channel member remove` | `RemoveScopeMembership` | `C` | ChannelId+revision+membership generation | member | committed | JSON | AT-MEMBER-001 |
| `channel member list` | `ListScopeMemberships` | `Q` | ChannelId | page | immediate | JSON/JSONL | AT-MEMBER-001 |
| `channel send` | `SendMessage` | `C` | Channel ConversationId+revision | one content source | committed | JSON | AT-SEC-005 |
| `channel history` | `ListMessages` | `Q` | Channel ConversationId | page/export | immediate | JSONL | AT-APP-006 |
| `process show` | `GetProcess` | `Q` | ProcessId | none | immediate | JSON | AT-CLI-009 |
| `process watch` | `WatchProcess` | `S` | ProcessId | cursor | follow | JSONL | AT-APP-007 |
| `operation show` | `GetOperation` | `Q` | one recovery key | operation/command/idempotency key | immediate | JSON | AT-SUBMIT-001 |
| `operation reconcile` | `ReconcileOperation` | `C` | OperationId+receipt revision | none | committed | JSON | AT-APP-005 |
| `approval list` | `ListApprovals` | `Q` | principal/scope | page | immediate | JSON/JSONL | AT-SEC-005 |
| `approval show` | `GetApproval` | `Q` | ApprovalId | none | immediate | JSON | AT-SEC-005 |
| `approval approve` | `DecideApproval.Approve` | `C` | ApprovalId+revision | none | committed | JSON | AT-SEC-005 |
| `approval deny` | `DecideApproval.Deny` | `C` | ApprovalId+revision | reason? | committed | JSON | AT-SEC-005 |
| `provider list` | `ListProviders` | `Q` | capability? | page | immediate | JSON/JSONL | AT-HARNESS-001 |
| `provider show` | `GetProvider` | `Q` | ProviderId+generation | none | immediate | JSON | AT-HARNESS-001 |
| `side-effect reconcile` | `ReconcileSideEffect` | `C` | SideEffectId+revision | evidence/action | committed | JSON | AT-SEC-005 |

<!-- p0-command-registry:start -->
- `runtime start` | `StartRuntimeHost` | `H` | `AT-HOST-001`
- `runtime status` | `GetHostAndRuntimeStatus` | `H+Q` | `AT-HOST-001`
- `runtime stop` | `RequestRuntimeShutdown` | `C/H` | `AT-HOST-001`
- `runtime doctor` | `DiagnoseRuntime` | `H+Q` | `AT-SEC-005`
- `version` | `GetVersionCompatibility` | `Q` | `AT-SCHEMA-001`
- `bot create` | `CreateBot` | `C` | `AT-BOOT-001`
- `bot list` | `ListBots` | `Q` | `AT-APP-006`
- `bot show` | `GetBot` | `Q` | `AT-CLI-009`
- `bot activate` | `ChangeBotLifecycle` | `C` | `AT-APP-005`
- `bot deactivate` | `ChangeBotLifecycle` | `C` | `AT-APP-005`
- `bot archive` | `ChangeBotLifecycle` | `C` | `AT-CLI-010`
- `bot restore` | `ChangeBotLifecycle` | `C` | `AT-APP-005`
- `conversation show` | `GetConversation` | `Q` | `AT-CLI-009`
- `conversation send` | `SendMessage` | `C` | `AT-APP-005`
- `conversation history` | `ListMessages` | `Q` | `AT-APP-006`
- `thread create` | `CreateThread` | `C` | `AT-APP-005`
- `thread list` | `ListThreads` | `Q` | `AT-APP-006`
- `thread show` | `GetThread` | `Q` | `AT-CLI-009`
- `thread send` | `SendMessage` | `C` | `AT-APP-005`
- `thread history` | `ListMessages` | `Q` | `AT-APP-006`
- `thread branch` | `BranchThread` | `C` | `AT-APP-005`
- `task submit` | `SubmitTask` | `C` | `AT-DELEGATE-001`
- `task list` | `ListTasks` | `Q` | `AT-APP-006`
- `task show` | `GetTask` | `Q` | `AT-CLI-009`
- `task watch` | `WatchTask` | `S` | `AT-APP-007`
- `task cancel` | `ControlTask.Cancel` | `C` | `AT-CLI-010`
- `task suspend` | `ControlTask.Suspend` | `C` | `AT-CLI-010`
- `task resume` | `ControlTask.Resume` | `C` | `AT-APP-005`
- `task redirect` | `ControlTask.Redirect` | `C` | `AT-CLI-010`
- `task result` | `GetTaskResult` | `Q` | `AT-SEC-005`
- `memory get` | `GetMemory` | `Q` | `AT-CLI-009`
- `memory search` | `SearchMemory` | `Q` | `AT-APP-006`
- `memory history` | `GetMemoryHistory` | `Q` | `AT-APP-006`
- `memory propose` | `ProposeMemory` | `C` | `AT-APP-005`
- `memory promote` | `PromoteMemory` | `C` | `AT-SEC-005`
- `project create` | `CreateProject` | `C` | `AT-APP-005`
- `project list` | `ListProjects` | `Q` | `AT-APP-006`
- `project show` | `GetProject` | `Q` | `AT-CLI-009`
- `project archive` | `ChangeProjectLifecycle` | `C` | `AT-CLI-010`
- `project restore` | `ChangeProjectLifecycle` | `C` | `AT-APP-005`
- `project member set` | `SetScopeMembership` | `C` | `AT-MEMBER-001`
- `project member remove` | `RemoveScopeMembership` | `C` | `AT-MEMBER-001`
- `project member list` | `ListScopeMemberships` | `Q` | `AT-MEMBER-001`
- `channel create` | `CreateChannel` | `C` | `AT-APP-005`
- `channel list` | `ListChannels` | `Q` | `AT-APP-006`
- `channel show` | `GetChannel` | `Q` | `AT-CLI-009`
- `channel member set` | `SetScopeMembership` | `C` | `AT-MEMBER-001`
- `channel member remove` | `RemoveScopeMembership` | `C` | `AT-MEMBER-001`
- `channel member list` | `ListScopeMemberships` | `Q` | `AT-MEMBER-001`
- `channel send` | `SendMessage` | `C` | `AT-SEC-005`
- `channel history` | `ListMessages` | `Q` | `AT-APP-006`
- `process show` | `GetProcess` | `Q` | `AT-CLI-009`
- `process watch` | `WatchProcess` | `S` | `AT-APP-007`
- `operation show` | `GetOperation` | `Q` | `AT-SUBMIT-001`
- `operation reconcile` | `ReconcileOperation` | `C` | `AT-APP-005`
- `approval list` | `ListApprovals` | `Q` | `AT-SEC-005`
- `approval show` | `GetApproval` | `Q` | `AT-SEC-005`
- `approval approve` | `DecideApproval.Approve` | `C` | `AT-SEC-005`
- `approval deny` | `DecideApproval.Deny` | `C` | `AT-SEC-005`
- `provider list` | `ListProviders` | `Q` | `AT-HARNESS-001`
- `provider show` | `GetProvider` | `Q` | `AT-HARNESS-001`
- `side-effect reconcile` | `ReconcileSideEffect` | `C` | `AT-SEC-005`
<!-- p0-command-registry:end -->
## 6. Runtime stop

기본은 graceful Control command다. ControlReady가 아니면 exit 10과 discovery summary를 반환한다. `--host-stop --host-generation`을 명시한 경우에만 user service manager stop을 수행하며 confirmation과 generation CAS를 요구한다.

## 7. Terminal / file

human renderer는 control sequence를 neutralize한다. export는 `DXB-RUN-032`의 descriptor-relative atomic no-replace writer만 사용한다. 일반 rename overwrite, `--raw`, `--force-overwrite`는 P0에 없다.

## 8. 금지

- generic JSON mutation
- human output parsing automation
- full dataset aggregation
- unresolved selector를 mutation owner로 전달
- SIGINT implicit Task cancel
- machine stdout log/spinner
- Reference Provider silent production fallback
