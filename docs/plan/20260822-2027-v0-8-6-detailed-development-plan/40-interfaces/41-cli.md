---
title: "CLI Reference Interface와 P0 Operation Registry"
document_id: "DXB-IFC-041"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-IFC-040", "DXB-IFC-042", "DXB-RUN-035"]
---
# CLI Reference Interface와 P0 Operation Registry

## 1. 책임

`dxb`는 parser, typed input collection, Control/Host client, rendering, machine writer, safe file writer와 non-canonical per-command journal을 소유한다. Domain, Authorization, Resource, Provider selection 정책을 소유하지 않는다.

## 2. 전역 옵션

- `--profile`, `--instance`: local discovery이며 Authority가 아니다.
- `--json`, `--jsonl`: `DXB-IFC-040` machine schema를 선택한다.
- `--wait <accepted|committed|applied|target-terminal>`: metadata가 허용한 Command에만 적용한다.
- `--timeout`: local request/wait deadline이며 rollback이 아니다.
- `--command-id`, `--idempotency-key`: mutation identity다.
- `--request-id`: transport correlation only다.
- `--non-interactive`, `--yes`: local prompt만 제어한다.
- `--page-size`, `--cursor`, `--all`: paged operation에만 적용한다.
- `--output`: safe export를 허용한 Query/Artifact에만 적용한다.

## 3. Machine contract

stdout에는 `dxbot.result.v1` 또는 `dxbot.stream.v1`만 기록하고 stderr에는 diagnostic/progress/sanitizer warning만 기록한다. human output parsing을 automation contract로 사용하지 않는다. JSONL terminal record가 로컬 writer에 의해 생성되지 않은 complete success는 exit 0으로 보고하지 않는다. broken pipe에서는 terminal 전달 자체를 보장하지 않는다.

## 4. Durable local journal

```text
$XDG_STATE_HOME/dxbot/cli/<profile>/<instance-id>/operations/<command-id>.jsonl
```

`PreparedUnsent → SentUnknown → Observed → Terminal` 또는 local-only `Abandoned`다. directory `0700`, file `0600`, exclusive create, append+fsync를 사용한다. global map last-writer-wins를 금지한다. SentUnknown을 자동 evict/replay하지 않는다.

## 5. Registry semantics

lexical CLI path는 62개지만 `runtime stop`의 graceful/host mode를 분리해 typed operation key는 63개다. `command_key + mode` 하나가 정확히 하나의 operation에 mapping된다. 아래 snapshot은 M1B Rust metadata source가 생기면 generated exact diff 대상이며, 그 전까지 parser freeze는 Blocked다.

Columns: `command_key | cli_path/mode | operation | kind | input_schema | wait | output_schema | security_class | Acceptance`.

<!-- p0-command-registry:start -->
- `runtime-start` | `runtime start` | `StartRuntimeHost` | `H` | `in-runtime-start-v1` | `default=applied;allowed=applied` | `out-host-action-v1` | `host-admin` | `AT-HOST-001`
- `runtime-status` | `runtime status` | `GetHostAndRuntimeStatus` | `H/Q` | `in-runtime-status-v1` | `default=immediate;allowed=immediate` | `out-host-status-v1` | `read` | `AT-HOST-001`
- `runtime-stop-graceful` | `runtime stop` | `RequestRuntimeShutdown` | `C` | `in-runtime-stop-graceful-v1` | `default=committed;allowed=accepted,committed,applied` | `out-operation-v1` | `host-admin` | `AT-HOST-001`
- `runtime-stop-host` | `runtime stop --host-stop` | `StopRuntimeHost` | `H` | `in-runtime-stop-host-v1` | `default=applied;allowed=applied` | `out-host-action-v1` | `host-admin` | `AT-HOST-001`
- `runtime-doctor` | `runtime doctor` | `DiagnoseRuntime` | `H/Q` | `in-runtime-doctor-v1` | `default=immediate;allowed=immediate` | `out-diagnostic-stream-v1` | `read` | `AT-SEC-005`
- `version` | `version` | `GetVersionCompatibility` | `Q` | `in-version-v1` | `default=immediate;allowed=immediate` | `out-version-v1` | `read` | `AT-SCHEMA-001`
- `bot-create` | `bot create` | `CreateBot` | `C` | `in-bot-create-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `mutate` | `AT-BOOT-001`
- `bot-list` | `bot list` | `ListBots` | `Q` | `in-bot-list-v1` | `default=immediate;allowed=immediate` | `out-bot-page-v1` | `read` | `AT-APP-006`
- `bot-show` | `bot show` | `GetBot` | `Q` | `in-bot-show-v1` | `default=immediate;allowed=immediate` | `out-bot-v1` | `read` | `AT-CLI-009`
- `bot-activate` | `bot activate` | `ChangeBotLifecycle.Activate` | `C` | `in-bot-activate-v1` | `default=committed;allowed=accepted,committed,applied` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `bot-deactivate` | `bot deactivate` | `ChangeBotLifecycle.Deactivate` | `C` | `in-bot-deactivate-v1` | `default=committed;allowed=accepted,committed,applied` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `bot-archive` | `bot archive` | `ChangeBotLifecycle.Archive` | `C` | `in-bot-archive-v1` | `default=committed;allowed=accepted,committed,applied` | `out-operation-v1` | `destructive` | `AT-CLI-010`
- `bot-restore` | `bot restore` | `ChangeBotLifecycle.Restore` | `C` | `in-bot-restore-v1` | `default=committed;allowed=accepted,committed,applied` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `conversation-show` | `conversation show` | `GetConversation` | `Q` | `in-conversation-show-v1` | `default=immediate;allowed=immediate` | `out-conversation-v1` | `read` | `AT-CLI-009`
- `conversation-send` | `conversation send` | `SendMessage` | `C` | `in-conversation-send-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `conversation-history` | `conversation history` | `ListMessages` | `Q` | `in-conversation-history-v1` | `default=immediate;allowed=immediate` | `out-message-page-v1` | `read` | `AT-APP-006`
- `thread-create` | `thread create` | `CreateThread` | `C` | `in-thread-create-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `thread-list` | `thread list` | `ListThreads` | `Q` | `in-thread-list-v1` | `default=immediate;allowed=immediate` | `out-thread-page-v1` | `read` | `AT-APP-006`
- `thread-show` | `thread show` | `GetThread` | `Q` | `in-thread-show-v1` | `default=immediate;allowed=immediate` | `out-thread-v1` | `read` | `AT-CLI-009`
- `thread-send` | `thread send` | `SendMessage` | `C` | `in-thread-send-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `thread-history` | `thread history` | `ListMessages` | `Q` | `in-thread-history-v1` | `default=immediate;allowed=immediate` | `out-message-page-v1` | `read` | `AT-APP-006`
- `thread-branch` | `thread branch` | `BranchThread` | `C` | `in-thread-branch-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `task-submit` | `task submit` | `SubmitTask` | `C` | `in-task-submit-v1` | `default=committed;allowed=accepted,committed,target-terminal` | `out-operation-v1` | `mutate` | `AT-DELEGATE-001`
- `task-list` | `task list` | `ListTasks` | `Q` | `in-task-list-v1` | `default=immediate;allowed=immediate` | `out-task-page-v1` | `read` | `AT-APP-006`
- `task-show` | `task show` | `GetTask` | `Q` | `in-task-show-v1` | `default=immediate;allowed=immediate` | `out-task-v1` | `read` | `AT-CLI-009`
- `task-watch` | `task watch` | `WatchTask` | `S` | `in-task-watch-v1` | `default=follow;allowed=follow` | `out-task-stream-v1` | `read` | `AT-APP-007`
- `task-cancel` | `task cancel` | `ControlTask.Cancel` | `C` | `in-task-cancel-v1` | `default=committed;allowed=accepted,committed,applied,target-terminal` | `out-operation-v1` | `destructive` | `AT-CLI-010`
- `task-suspend` | `task suspend` | `ControlTask.Suspend` | `C` | `in-task-suspend-v1` | `default=committed;allowed=accepted,committed,applied` | `out-operation-v1` | `mutate` | `AT-CLI-010`
- `task-resume` | `task resume` | `ControlTask.Resume` | `C` | `in-task-resume-v1` | `default=committed;allowed=accepted,committed,applied` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `task-redirect` | `task redirect` | `ControlTask.Redirect` | `C` | `in-task-redirect-v1` | `default=committed;allowed=accepted,committed,applied` | `out-operation-v1` | `destructive` | `AT-CLI-010`
- `task-result` | `task result` | `GetTaskResult` | `Q` | `in-task-result-v1` | `default=immediate;allowed=immediate` | `out-task-result-v1` | `read` | `AT-SEC-005`
- `memory-get` | `memory get` | `GetMemory` | `Q` | `in-memory-get-v1` | `default=immediate;allowed=immediate` | `out-memory-v1` | `read` | `AT-CLI-009`
- `memory-search` | `memory search` | `SearchMemory` | `Q` | `in-memory-search-v1` | `default=immediate;allowed=immediate` | `out-memory-page-v1` | `read` | `AT-APP-006`
- `memory-history` | `memory history` | `GetMemoryHistory` | `Q` | `in-memory-history-v1` | `default=immediate;allowed=immediate` | `out-memory-history-v1` | `read` | `AT-APP-006`
- `memory-propose` | `memory propose` | `ProposeMemory` | `C` | `in-memory-propose-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `memory-promote` | `memory promote` | `PromoteMemory` | `C` | `in-memory-promote-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `high-risk` | `AT-SEC-005`
- `project-create` | `project create` | `CreateProject` | `C` | `in-project-create-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `project-list` | `project list` | `ListProjects` | `Q` | `in-project-list-v1` | `default=immediate;allowed=immediate` | `out-project-page-v1` | `read` | `AT-APP-006`
- `project-show` | `project show` | `GetProject` | `Q` | `in-project-show-v1` | `default=immediate;allowed=immediate` | `out-project-v1` | `read` | `AT-CLI-009`
- `project-archive` | `project archive` | `ChangeProjectLifecycle.Archive` | `C` | `in-project-archive-v1` | `default=committed;allowed=accepted,committed,applied` | `out-operation-v1` | `destructive` | `AT-CLI-010`
- `project-restore` | `project restore` | `ChangeProjectLifecycle.Restore` | `C` | `in-project-restore-v1` | `default=committed;allowed=accepted,committed,applied` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `project-member-set` | `project member set` | `SetScopeMembership.Project` | `C` | `in-project-member-set-v1` | `default=committed;allowed=accepted,committed` | `out-membership-v1` | `high-risk` | `AT-MEMBER-001`
- `project-member-remove` | `project member remove` | `RemoveScopeMembership.Project` | `C` | `in-project-member-remove-v1` | `default=committed;allowed=accepted,committed` | `out-membership-v1` | `high-risk` | `AT-MEMBER-001`
- `project-member-list` | `project member list` | `ListScopeMemberships.Project` | `Q` | `in-project-member-list-v1` | `default=immediate;allowed=immediate` | `out-membership-page-v1` | `read` | `AT-MEMBER-001`
- `channel-create` | `channel create` | `CreateChannel` | `C` | `in-channel-create-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `channel-list` | `channel list` | `ListChannels` | `Q` | `in-channel-list-v1` | `default=immediate;allowed=immediate` | `out-channel-page-v1` | `read` | `AT-APP-006`
- `channel-show` | `channel show` | `GetChannel` | `Q` | `in-channel-show-v1` | `default=immediate;allowed=immediate` | `out-channel-v1` | `read` | `AT-CLI-009`
- `channel-member-set` | `channel member set` | `SetScopeMembership.Channel` | `C` | `in-channel-member-set-v1` | `default=committed;allowed=accepted,committed` | `out-membership-v1` | `high-risk` | `AT-MEMBER-001`
- `channel-member-remove` | `channel member remove` | `RemoveScopeMembership.Channel` | `C` | `in-channel-member-remove-v1` | `default=committed;allowed=accepted,committed` | `out-membership-v1` | `high-risk` | `AT-MEMBER-001`
- `channel-member-list` | `channel member list` | `ListScopeMemberships.Channel` | `Q` | `in-channel-member-list-v1` | `default=immediate;allowed=immediate` | `out-membership-page-v1` | `read` | `AT-MEMBER-001`
- `channel-send` | `channel send` | `SendMessage` | `C` | `in-channel-send-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `high-risk` | `AT-SEC-005`
- `channel-history` | `channel history` | `ListMessages` | `Q` | `in-channel-history-v1` | `default=immediate;allowed=immediate` | `out-message-page-v1` | `read` | `AT-APP-006`
- `process-show` | `process show` | `GetProcess` | `Q` | `in-process-show-v1` | `default=immediate;allowed=immediate` | `out-process-v1` | `read` | `AT-CLI-009`
- `process-watch` | `process watch` | `WatchProcess` | `S` | `in-process-watch-v1` | `default=follow;allowed=follow` | `out-process-stream-v1` | `read` | `AT-APP-007`
- `operation-show` | `operation show` | `GetOperation` | `Q` | `in-operation-show-v1` | `default=immediate;allowed=immediate` | `out-receipt-v1` | `recovery` | `AT-SUBMIT-001`
- `operation-reconcile` | `operation reconcile` | `ReconcileOperation` | `C` | `in-operation-reconcile-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `recovery` | `AT-APP-005`
- `approval-list` | `approval list` | `ListApprovals` | `Q` | `in-approval-list-v1` | `default=immediate;allowed=immediate` | `out-approval-page-v1` | `read` | `AT-SEC-005`
- `approval-show` | `approval show` | `GetApproval` | `Q` | `in-approval-show-v1` | `default=immediate;allowed=immediate` | `out-approval-v1` | `read` | `AT-SEC-005`
- `approval-approve` | `approval approve` | `DecideApproval.Approve` | `C` | `in-approval-approve-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `security-decision` | `AT-SEC-005`
- `approval-deny` | `approval deny` | `DecideApproval.Deny` | `C` | `in-approval-deny-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `security-decision` | `AT-SEC-005`
- `provider-list` | `provider list` | `ListProviders` | `Q` | `in-provider-list-v1` | `default=immediate;allowed=immediate` | `out-provider-page-v1` | `read` | `AT-HARNESS-001`
- `provider-show` | `provider show` | `GetProvider` | `Q` | `in-provider-show-v1` | `default=immediate;allowed=immediate` | `out-provider-v1` | `read` | `AT-HARNESS-001`
- `side-effect-reconcile` | `side-effect reconcile` | `ReconcileSideEffect` | `C` | `in-side-effect-reconcile-v1` | `default=committed;allowed=accepted,committed` | `out-side-effect-v1` | `high-risk` | `AT-SEC-005`
<!-- p0-command-registry:end -->

## 6. 금지

- generic JSON mutation
- unresolved selector의 Domain 전달
- SIGINT implicit Task cancel
- machine stdout log/spinner
- full dataset aggregation
- `runtime stop` endpoint failure의 implicit host kill
- Reference Provider silent production fallback
