---
title: "CLI Reference Interface와 P0 Operation Registry"
document_id: "DXB-IFC-041"
version: "0.8.10"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-IFC-040", "DXB-IFC-042", "DXB-RUN-035"]
---
# CLI Reference Interface와 P0 Operation Registry

`dxb`는 parser, local endpoint selection, bounded preflight, `CliInput → CommandPayload` projection, Control/Host client, rendering, machine/safe writer와 non-canonical local journal만 소유한다. Domain, Receipt lifecycle, authorization, canonical state를 소유하지 않는다.

## Local invocation과 discovery

- `dxb`, `dxb --help`, `dxb <group> --help`, `dxb <command> --help`는 Runtime 접근 없이 help를 stdout에 쓰고 exit 0이다.
- missing/invalid argument는 exit 2이며 concise usage와 field error를 stderr에 쓴다.
- unknown command는 실행하지 않고 bounded, distance-thresholded suggestion만 제공한다.
- 다른 command가 Runtime을 자동 시작하지 않는다.
- local selection은 `DXB-RUN-035`의 `--instance/profile/exactly-one endpoint` 순서를 따르며 silent fallback하지 않는다.

## Selector와 CAS materialization

사용자는 domain selector와 optional `--if-*` CAS를 제공한다. canonical ID, revision, generation, Main/Channel ConversationId는 `Prepared` 이전 bounded preflight에서 materialize한다.

- selector 0개/복수/deny는 journal을 만들지 않는다.
- explicit canonical selector와 complete CAS는 preflight를 생략할 수 있다.
- server는 materialized value를 다시 resolve/authorize/CAS 검증한다.
- stale conflict를 CLI가 자동 재시도하지 않는다.

## Local journal

경로는 `$XDG_STATE_HOME/dxbot/cli/<instance-id>/operations/<command-id>.jsonl`이다. profile은 discovery hint이며 journal identity가 아니다.

versioned header/record는 InstanceId, local principal scope, operation key/schema, CommandId, IdempotencyKey format/epoch, RequestDigest, sequence, previous-record digest, record digest를 가진다. raw secret, Authority, content body, canonical outcome은 저장하지 않는다.

상태는 `Prepared → Dispatching → Observed → Terminal` 또는 `Prepared → Abandoned`다.

- command file은 owner-only directory에서 descriptor-relative exclusive create한다.
- `Prepared` record와 parent directory를 durable하게 만든다.
- command별 OS append lock이 single writer를 강제한다.
- 동일 immutable header의 후속 process는 lock 획득 전 observation-only다. 이전 writer 종료 뒤 OS lock이 해제된 경우에만 takeover한다.
- `Dispatching` append+fsync 뒤에만 첫 network byte를 보낸다.
- truncated final record는 마지막 완전한 record까지 허용하지만 invalid middle record, chain mismatch, unknown version은 fail closed다.
- Dispatching 이상은 capacity cleanup 대상이 아니다.
- lock-free, integrity-valid, minimum-retention을 지난 Prepared만 bounded policy가 prune할 수 있다.

## Local termination과 output

SIGINT, SIGTERM, broken pipe, pager 종료, local timeout은 관찰·렌더링만 종료한다. `Dispatching` 이상 operation을 별도 typed cancel 없이 취소하지 않고 journal을 `Abandoned`로 바꾸지 않는다.

stdout에는 result 또는 stream data만, stderr에는 diagnostic/progress만 쓴다. prompt는 controlling TTY에서만 가능하며 non-TTY에서 열지 않는다.

- default format: human
- `--format json`: one complete result/error document
- `--format jsonl`: subscription/diagnostic stream event lines
- color/progress/pager는 local-only이며 non-TTY 또는 machine output을 오염시키지 않는다.
- `--output`은 bounded temp write + fsync + atomic no-replace를 사용하며 stdout에는 artifact/result summary를 유지한다.

## Wait/output 수렴

P0 `target-terminal`은 없다. Application Command는 `out-operation-v1`을 사용한다. `applied`는 Task control Directive와 Host Action에만 허용한다. ApprovalRef가 반환되면 requested `accepted`가 충족돼도 exit 8이다.

## Registry

Columns: `command_key | cli_path/mode | operation | kind | input_schema | wait | output_schema | security_class | Acceptance`.

<!-- p0-command-registry:start -->
- `runtime-start` | `runtime start` | `StartRuntimeHost` | `H` | `in-runtime-start-v1` | `default=applied;allowed=applied` | `out-host-action-v1` | `host-admin` | `AT-HOST-001`
- `runtime-status` | `runtime status` | `GetHostAndRuntimeStatus` | `H/Q` | `in-runtime-status-v1` | `default=immediate;allowed=immediate` | `out-host-status-v1` | `read` | `AT-HOST-001`
- `runtime-stop-graceful` | `runtime stop` | `RequestRuntimeShutdown` | `C` | `in-runtime-stop-graceful-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `host-admin` | `AT-HOST-001`
- `runtime-stop-host` | `runtime stop --host-stop` | `StopRuntimeHost` | `H` | `in-runtime-stop-host-v1` | `default=applied;allowed=applied` | `out-host-action-v1` | `host-admin` | `AT-HOST-001`
- `runtime-doctor` | `runtime doctor` | `DiagnoseRuntime` | `H/Q` | `in-runtime-doctor-v1` | `default=immediate;allowed=immediate` | `out-diagnostic-stream-v1` | `read` | `AT-SEC-005`
- `version` | `version` | `GetVersionCompatibility` | `Q` | `in-version-v1` | `default=immediate;allowed=immediate` | `out-version-v1` | `read` | `AT-VERSION-001`
- `bot-create` | `bot create` | `CreateBot` | `C` | `in-bot-create-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `mutate` | `AT-CLI-CORE-001`
- `bot-list` | `bot list` | `ListBots` | `Q` | `in-bot-list-v1` | `default=immediate;allowed=immediate` | `out-bot-page-v1` | `read` | `AT-APP-006`
- `bot-show` | `bot show` | `GetBot` | `Q` | `in-bot-show-v1` | `default=immediate;allowed=immediate` | `out-bot-v1` | `read` | `AT-CLI-009`
- `bot-activate` | `bot activate` | `ChangeBotLifecycle.Activate` | `C` | `in-bot-activate-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `bot-deactivate` | `bot deactivate` | `ChangeBotLifecycle.Deactivate` | `C` | `in-bot-deactivate-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `bot-archive` | `bot archive` | `ChangeBotLifecycle.Archive` | `C` | `in-bot-archive-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `destructive` | `AT-CLI-CONFIRM-001`
- `bot-restore` | `bot restore` | `ChangeBotLifecycle.Restore` | `C` | `in-bot-restore-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `conversation-show` | `conversation show` | `GetConversation` | `Q` | `in-conversation-show-v1` | `default=immediate;allowed=immediate` | `out-conversation-v1` | `read` | `AT-CLI-009`
- `conversation-send` | `conversation send` | `SendMessage` | `C` | `in-conversation-send-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `mutate` | `AT-CLI-CORE-001`
- `conversation-history` | `conversation history` | `ListMessages` | `Q` | `in-conversation-history-v1` | `default=immediate;allowed=immediate` | `out-message-page-v1` | `read` | `AT-APP-006`
- `thread-create` | `thread create` | `CreateThread` | `C` | `in-thread-create-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `thread-list` | `thread list` | `ListThreads` | `Q` | `in-thread-list-v1` | `default=immediate;allowed=immediate` | `out-thread-page-v1` | `read` | `AT-APP-006`
- `thread-show` | `thread show` | `GetThread` | `Q` | `in-thread-show-v1` | `default=immediate;allowed=immediate` | `out-thread-v1` | `read` | `AT-CLI-009`
- `thread-send` | `thread send` | `SendMessage` | `C` | `in-thread-send-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `thread-history` | `thread history` | `ListMessages` | `Q` | `in-thread-history-v1` | `default=immediate;allowed=immediate` | `out-message-page-v1` | `read` | `AT-APP-006`
- `thread-branch` | `thread branch` | `BranchThread` | `C` | `in-thread-branch-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `task-submit` | `task submit` | `SubmitTask` | `C` | `in-task-submit-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `task-list` | `task list` | `ListTasks` | `Q` | `in-task-list-v1` | `default=immediate;allowed=immediate` | `out-task-page-v1` | `read` | `AT-APP-006`
- `task-show` | `task show` | `GetTask` | `Q` | `in-task-show-v1` | `default=immediate;allowed=immediate` | `out-task-v1` | `read` | `AT-CLI-CORE-001`
- `task-watch` | `task watch` | `WatchTask` | `S` | `in-task-watch-v1` | `default=follow;allowed=follow` | `out-task-stream-v1` | `read` | `AT-APP-007`
- `task-cancel` | `task cancel` | `ControlTask.Cancel` | `C` | `in-task-cancel-v1` | `default=committed;allowed=accepted,committed,applied` | `out-operation-v1` | `destructive` | `AT-CLI-010`
- `task-suspend` | `task suspend` | `ControlTask.Suspend` | `C` | `in-task-suspend-v1` | `default=committed;allowed=accepted,committed,applied` | `out-operation-v1` | `mutate` | `AT-CLI-010`
- `task-resume` | `task resume` | `ControlTask.Resume` | `C` | `in-task-resume-v1` | `default=committed;allowed=accepted,committed,applied` | `out-operation-v1` | `mutate` | `AT-CLI-010`
- `task-redirect` | `task redirect` | `ControlTask.Redirect` | `C` | `in-task-redirect-v1` | `default=committed;allowed=accepted,committed,applied` | `out-operation-v1` | `destructive` | `AT-CLI-010`
- `task-result` | `task result` | `GetTaskResult` | `Q` | `in-task-result-v1` | `default=immediate;allowed=immediate` | `out-task-result-v1` | `read` | `AT-CLI-CORE-001`
- `memory-get` | `memory get` | `GetMemory` | `Q` | `in-memory-get-v1` | `default=immediate;allowed=immediate` | `out-memory-v1` | `read` | `AT-CLI-009`
- `memory-search` | `memory search` | `SearchMemory` | `Q` | `in-memory-search-v1` | `default=immediate;allowed=immediate` | `out-memory-page-v1` | `read` | `AT-APP-006`
- `memory-history` | `memory history` | `GetMemoryHistory` | `Q` | `in-memory-history-v1` | `default=immediate;allowed=immediate` | `out-memory-history-v1` | `read` | `AT-APP-006`
- `memory-propose` | `memory propose` | `ProposeMemory` | `C` | `in-memory-propose-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `memory-promote` | `memory promote` | `PromoteMemory` | `C` | `in-memory-promote-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `high-risk` | `AT-SEC-005`
- `project-create` | `project create` | `CreateProject` | `C` | `in-project-create-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `project-list` | `project list` | `ListProjects` | `Q` | `in-project-list-v1` | `default=immediate;allowed=immediate` | `out-project-page-v1` | `read` | `AT-APP-006`
- `project-show` | `project show` | `GetProject` | `Q` | `in-project-show-v1` | `default=immediate;allowed=immediate` | `out-project-v1` | `read` | `AT-CLI-009`
- `project-archive` | `project archive` | `ChangeProjectLifecycle.Archive` | `C` | `in-project-archive-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `destructive` | `AT-CLI-CONFIRM-001`
- `project-restore` | `project restore` | `ChangeProjectLifecycle.Restore` | `C` | `in-project-restore-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `project-member-set` | `project member set` | `SetScopeMembership.Project` | `C` | `in-project-member-set-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `high-risk` | `AT-MEMBER-001`
- `project-member-remove` | `project member remove` | `RemoveScopeMembership.Project` | `C` | `in-project-member-remove-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `high-risk` | `AT-MEMBER-001`
- `project-member-list` | `project member list` | `ListScopeMemberships.Project` | `Q` | `in-project-member-list-v1` | `default=immediate;allowed=immediate` | `out-membership-page-v1` | `read` | `AT-MEMBER-001`
- `channel-create` | `channel create` | `CreateChannel` | `C` | `in-channel-create-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `mutate` | `AT-APP-005`
- `channel-list` | `channel list` | `ListChannels` | `Q` | `in-channel-list-v1` | `default=immediate;allowed=immediate` | `out-channel-page-v1` | `read` | `AT-APP-006`
- `channel-show` | `channel show` | `GetChannel` | `Q` | `in-channel-show-v1` | `default=immediate;allowed=immediate` | `out-channel-v1` | `read` | `AT-CLI-009`
- `channel-member-set` | `channel member set` | `SetScopeMembership.Channel` | `C` | `in-channel-member-set-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `high-risk` | `AT-MEMBER-001`
- `channel-member-remove` | `channel member remove` | `RemoveScopeMembership.Channel` | `C` | `in-channel-member-remove-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `high-risk` | `AT-MEMBER-001`
- `channel-member-list` | `channel member list` | `ListScopeMemberships.Channel` | `Q` | `in-channel-member-list-v1` | `default=immediate;allowed=immediate` | `out-membership-page-v1` | `read` | `AT-MEMBER-001`
- `channel-send` | `channel send` | `SendMessage` | `C` | `in-channel-send-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `high-risk` | `AT-SEC-005`
- `channel-history` | `channel history` | `ListMessages` | `Q` | `in-channel-history-v1` | `default=immediate;allowed=immediate` | `out-message-page-v1` | `read` | `AT-APP-006`
- `process-show` | `process show` | `GetProcess` | `Q` | `in-process-show-v1` | `default=immediate;allowed=immediate` | `out-process-v1` | `read` | `AT-CLI-009`
- `process-watch` | `process watch` | `WatchProcess` | `S` | `in-process-watch-v1` | `default=follow;allowed=follow` | `out-process-stream-v1` | `read` | `AT-APP-007`
- `operation-show` | `operation show` | `GetOperation` | `Q` | `in-operation-show-v1` | `default=immediate;allowed=immediate` | `out-receipt-v1` | `recovery` | `AT-SUBMIT-001`
- `operation-reconcile` | `operation reconcile` | `ReconcileOperation` | `C` | `in-operation-reconcile-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `recovery` | `AT-APP-005`
- `approval-list` | `approval list` | `ListApprovals` | `Q` | `in-approval-list-v1` | `default=immediate;allowed=immediate` | `out-approval-page-v1` | `read` | `AT-SECSTATE-001`
- `approval-show` | `approval show` | `GetApproval` | `Q` | `in-approval-show-v1` | `default=immediate;allowed=immediate` | `out-approval-v1` | `read` | `AT-SECSTATE-001`
- `approval-approve` | `approval approve` | `DecideApproval.Approve` | `C` | `in-approval-approve-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `security-decision` | `AT-SECSTATE-001`
- `approval-deny` | `approval deny` | `DecideApproval.Deny` | `C` | `in-approval-deny-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `security-decision` | `AT-SECSTATE-001`
- `provider-list` | `provider list` | `ListProviders` | `Q` | `in-provider-list-v1` | `default=immediate;allowed=immediate` | `out-provider-page-v1` | `read` | `AT-HARNESS-001`
- `provider-show` | `provider show` | `GetProvider` | `Q` | `in-provider-show-v1` | `default=immediate;allowed=immediate` | `out-provider-v1` | `read` | `AT-HARNESS-001`
- `side-effect-reconcile` | `side-effect reconcile` | `ReconcileSideEffect` | `C` | `in-side-effect-reconcile-v1` | `default=committed;allowed=accepted,committed` | `out-operation-v1` | `high-risk` | `AT-SEC-005`
<!-- p0-command-registry:end -->

## Milestone freeze registry

<!-- milestone-freeze:start -->
- `M2` | `runtime-start,runtime-status,runtime-stop-graceful,runtime-stop-host,runtime-doctor,approval-list,approval-show,approval-approve,approval-deny`
- `M3` | `version,bot-create,bot-list,bot-show,bot-activate,bot-deactivate,bot-archive,bot-restore,conversation-show,conversation-send,conversation-history,thread-create,thread-list,thread-show,thread-send,thread-history,thread-branch,task-submit,task-list,task-show,task-result,operation-show,operation-reconcile`
- `M4` | `project-create,project-list,project-show,project-archive,project-restore,project-member-set,project-member-remove,project-member-list,channel-create,channel-list,channel-show,channel-member-set,channel-member-remove,channel-member-list,channel-send,channel-history`
- `M5` | `task-watch,task-cancel,task-suspend,task-resume,task-redirect,memory-get,memory-search,memory-history,memory-propose,memory-promote,process-show,process-watch,provider-list,provider-show,side-effect-reconcile`
<!-- milestone-freeze:end -->

M2/M3/M4/M5는 `DXB-DEL-060/061`의 누적 Gate PASS 이후 해당 subset만 freeze한다. command의 Acceptance 최초 milestone이 command milestone보다 늦으면 freeze할 수 없다. destructive confirmation은 M3 `AT-CLI-CONFIRM-001`, Task Directive observation은 M5 `AT-CLI-010`이 소유한다. 전체 63 operation freeze는 M6에서만 수행한다.
