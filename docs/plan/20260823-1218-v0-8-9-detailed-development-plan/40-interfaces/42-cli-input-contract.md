---
title: "P0 CLI Typed Input Grammar와 Dual Projection Snapshot"
document_id: "DXB-IFC-042"
version: "0.8.9"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-IFC-040", "DXB-RUN-032", "DXB-RUN-035"]
---
# P0 CLI Typed Input Grammar와 Dual Projection Snapshot

## Field DSL

`!` required-one, `?` optional-one, `*` zero-or-more, `+` one-or-more다. modifier 결합(`*?`, `+?`)은 금지한다. source는 `@argv/@stdin/@file/@artifact/@local/@oneof{...}`다.

하나의 `in-*` metadata source는 다음 두 generated view를 만든다.

```text
CliInput       = argv/local/source-handle/rendering/wait/output를 포함한 parser view
CommandPayload = @local과 source handle을 제거한 typed wire/digest view
```

- `@local` field는 `CliInput`에만 존재한다.
- `@oneof{text,input-file,stdin,artifact-ref}`는 source 선택 문법이다. text/input-file/stdin은 `Prepared` 이전 bounded bytes로 materialize하고 artifact-ref는 typed ref+digest로 normalize한다.
- file path, file descriptor, source variant는 `CommandPayload`, RequestDigest, Local Journal payload에 들어가지 않는다.
- known local field `ready_at`, `timeout`, `all`, `output`, `confirmation`은 반드시 `@local`이다.
- parser DTO와 wire DTO를 독립 수작업 정의하지 않는다. generated exact diff와 projection fixture가 drift를 차단한다.

## Registry

Columns: `command_key | input_schema | target_schema | typed_fields | Acceptance`.

<!-- p0-input-registry:start -->
- `runtime-start` | `in-runtime-start-v1` | `tgt-instance-bootstrap-v1` | `instance:InstanceSelector!@argv;ready_at:ReadyAt?@local{process,storage,runtime,control}=control;timeout:Duration?@local` | `AT-HOST-001`
- `runtime-status` | `in-runtime-status-v1` | `tgt-instance-v1` | `instance:InstanceSelector!@argv` | `AT-HOST-001`
- `runtime-stop-graceful` | `in-runtime-stop-graceful-v1` | `tgt-runtime-current-v1` | `instance:InstanceSelector!@argv;reason:BoundedText?@argv` | `AT-HOST-001`
- `runtime-stop-host` | `in-runtime-stop-host-v1` | `tgt-runtime-generation-v1` | `instance:InstanceSelector!@argv;host_stop:True!@argv;host_generation:HostGeneration!@argv;confirmation:Confirmation!@local` | `AT-HOST-001`
- `runtime-doctor` | `in-runtime-doctor-v1` | `tgt-instance-v1` | `instance:InstanceSelector!@argv;section:DoctorSection?@argv;page_size:PageSize?@argv;cursor:Cursor?@argv` | `AT-SEC-005`
- `version` | `in-version-v1` | `tgt-instance-optional-v1` | `instance:InstanceSelector?@argv` | `AT-VERSION-001`
- `bot-create` | `in-bot-create-v1` | `tgt-instance-v1` | `name:BotName!@argv;brain_policy:PolicyRef?@argv;permission_policy:PolicyRef?@argv;resource_policy:PolicyRef?@argv;provider_policy:PolicyRef?@argv` | `AT-CLI-CORE-001`
- `bot-list` | `in-bot-list-v1` | `tgt-instance-v1` | `page_size:PageSize?@argv;cursor:Cursor?@argv;all:Bool?@local` | `AT-APP-006`
- `bot-show` | `in-bot-show-v1` | `tgt-bot-selector-v1` | `bot:BotSelector!@argv{canonical-id-or-scoped-exact}` | `AT-CLI-009`
- `bot-activate` | `in-bot-activate-v1` | `tgt-bot-revision-v1` | `bot_id:BotId!@argv;expected_revision:Revision!@argv` | `AT-APP-005`
- `bot-deactivate` | `in-bot-deactivate-v1` | `tgt-bot-revision-v1` | `bot_id:BotId!@argv;expected_revision:Revision!@argv` | `AT-APP-005`
- `bot-archive` | `in-bot-archive-v1` | `tgt-bot-revision-v1` | `bot_id:BotId!@argv;expected_revision:Revision!@argv;confirmation:Confirmation!@local` | `AT-CLI-CONFIRM-001`
- `bot-restore` | `in-bot-restore-v1` | `tgt-bot-revision-v1` | `bot_id:BotId!@argv;expected_revision:Revision!@argv` | `AT-APP-005`
- `conversation-show` | `in-conversation-show-v1` | `tgt-conversation-selector-v1` | `conversation_id:ConversationId?@argv;bot_id:BotId?@argv{exactly-one}` | `AT-CLI-009`
- `conversation-send` | `in-conversation-send-v1` | `tgt-conversation-revision-v1` | `conversation_id:ConversationId!@argv;expected_revision:Revision!@argv;content:ContentSource!@oneof{text,input-file,stdin,artifact-ref}` | `AT-CLI-CORE-001`
- `conversation-history` | `in-conversation-history-v1` | `tgt-conversation-v1` | `conversation_id:ConversationId!@argv;page_size:PageSize?@argv;cursor:Cursor?@argv;all:Bool?@local;output:SafeOutputPath?@local` | `AT-APP-006`
- `thread-create` | `in-thread-create-v1` | `tgt-conversation-revision-v1` | `conversation_id:ConversationId!@argv;expected_revision:Revision!@argv;title:ThreadTitle?@argv` | `AT-APP-005`
- `thread-list` | `in-thread-list-v1` | `tgt-conversation-v1` | `conversation_id:ConversationId!@argv;page_size:PageSize?@argv;cursor:Cursor?@argv;all:Bool?@local` | `AT-APP-006`
- `thread-show` | `in-thread-show-v1` | `tgt-thread-parent-v1` | `thread_id:ThreadId!@argv;conversation_id:ConversationId!@argv` | `AT-CLI-009`
- `thread-send` | `in-thread-send-v1` | `tgt-thread-revision-v1` | `thread_id:ThreadId!@argv;conversation_id:ConversationId!@argv;expected_revision:Revision!@argv;content:ContentSource!@oneof{text,input-file,stdin,artifact-ref}` | `AT-APP-005`
- `thread-history` | `in-thread-history-v1` | `tgt-thread-parent-v1` | `thread_id:ThreadId!@argv;conversation_id:ConversationId!@argv;page_size:PageSize?@argv;cursor:Cursor?@argv;all:Bool?@local;output:SafeOutputPath?@local` | `AT-APP-006`
- `thread-branch` | `in-thread-branch-v1` | `tgt-thread-revision-v1` | `thread_id:ThreadId!@argv;conversation_id:ConversationId!@argv;source_revision:Revision!@argv;message_id:MessageId!@argv;title:ThreadTitle?@argv` | `AT-APP-005`
- `task-submit` | `in-task-submit-v1` | `tgt-task-owner-revision-v1` | `owner_scope:ScopeRef!@argv;expected_scope_revision:Revision!@argv;intent:ContentSource!@oneof{text,input-file,stdin,artifact-ref};delegate_to_bot:BotId?@argv;requested_sender_bot_id:BotId?@argv{operator-request-only};deadline:Deadline?@argv;budget:BudgetRef?@argv` | `AT-APP-005`
- `task-list` | `in-task-list-v1` | `tgt-scope-v1` | `scope:ScopeRef!@argv;state:TaskState?@argv;page_size:PageSize?@argv;cursor:Cursor?@argv;all:Bool?@local` | `AT-APP-006`
- `task-show` | `in-task-show-v1` | `tgt-task-selector-v1` | `task:TaskSelector!@argv{canonical-id-or-scoped-exact}` | `AT-CLI-CORE-001`
- `task-watch` | `in-task-watch-v1` | `tgt-task-v1` | `task_id:TaskId!@argv;cursor:Cursor?@argv` | `AT-APP-007`
- `task-cancel` | `in-task-cancel-v1` | `tgt-task-generation-v1` | `task_id:TaskId!@argv;expected_revision:Revision!@argv;execution_generation:Generation!@argv;reason:BoundedText?@argv` | `AT-CLI-010`
- `task-suspend` | `in-task-suspend-v1` | `tgt-task-generation-v1` | `task_id:TaskId!@argv;expected_revision:Revision!@argv;execution_generation:Generation!@argv;reason:BoundedText?@argv` | `AT-CLI-010`
- `task-resume` | `in-task-resume-v1` | `tgt-task-revision-v1` | `task_id:TaskId!@argv;expected_revision:Revision!@argv` | `AT-CLI-010`
- `task-redirect` | `in-task-redirect-v1` | `tgt-task-revision-v1` | `task_id:TaskId!@argv;expected_revision:Revision!@argv;replacement:ContentSource!@oneof{text,input-file,stdin,artifact-ref}` | `AT-CLI-010`
- `task-result` | `in-task-result-v1` | `tgt-task-v1` | `task_id:TaskId!@argv;artifact_id:ArtifactId?@argv;output:SafeOutputPath?@local` | `AT-CLI-CORE-001`
- `memory-get` | `in-memory-get-v1` | `tgt-memory-v1` | `memory_id:MemoryId!@argv;scope:MemoryScopeRef?@argv` | `AT-CLI-009`
- `memory-search` | `in-memory-search-v1` | `tgt-memory-scope-v1` | `scope:MemoryScopeRef!@argv;query:BoundedText!@argv;page_size:PageSize?@argv;cursor:Cursor?@argv;all:Bool?@local` | `AT-APP-006`
- `memory-history` | `in-memory-history-v1` | `tgt-memory-v1` | `memory_id:MemoryId!@argv;page_size:PageSize?@argv;cursor:Cursor?@argv` | `AT-APP-006`
- `memory-propose` | `in-memory-propose-v1` | `tgt-memory-scope-revision-v1` | `scope:MemoryScopeRef!@argv;expected_scope_revision:Revision!@argv;statement:ContentSource!@oneof{text,input-file,stdin,artifact-ref};evidence:EvidenceRef*@argv` | `AT-APP-005`
- `memory-promote` | `in-memory-promote-v1` | `tgt-memory-promotion-v1` | `proposal_id:MemoryId!@argv;proposal_revision:Revision!@argv;target_scope:MemoryScopeRef!@argv;target_scope_revision:Revision!@argv;evidence:EvidenceRef+@argv;declassification_ref:DecisionRef?@argv` | `AT-SEC-005`
- `project-create` | `in-project-create-v1` | `tgt-instance-v1` | `name:ProjectName!@argv;owner_bot_id:BotId!@argv` | `AT-APP-005`
- `project-list` | `in-project-list-v1` | `tgt-instance-v1` | `page_size:PageSize?@argv;cursor:Cursor?@argv;all:Bool?@local` | `AT-APP-006`
- `project-show` | `in-project-show-v1` | `tgt-project-selector-v1` | `project:ProjectSelector!@argv{canonical-id-or-visible-exact}` | `AT-CLI-009`
- `project-archive` | `in-project-archive-v1` | `tgt-project-revision-v1` | `project_id:ProjectId!@argv;expected_revision:Revision!@argv;confirmation:Confirmation!@local` | `AT-CLI-CONFIRM-001`
- `project-restore` | `in-project-restore-v1` | `tgt-project-revision-v1` | `project_id:ProjectId!@argv;expected_revision:Revision!@argv` | `AT-APP-005`
- `project-member-set` | `in-project-member-set-v1` | `tgt-project-membership-v1` | `project_id:ProjectId!@argv;expected_project_revision:Revision!@argv;member_bot_id:BotId!@argv;role_ref:RoleRef!@argv;if_membership_generation:GenerationOrAbsent!@argv` | `AT-MEMBER-001`
- `project-member-remove` | `in-project-member-remove-v1` | `tgt-project-membership-v1` | `project_id:ProjectId!@argv;expected_project_revision:Revision!@argv;member_bot_id:BotId!@argv;expected_membership_generation:Generation!@argv` | `AT-MEMBER-001`
- `project-member-list` | `in-project-member-list-v1` | `tgt-project-v1` | `project_id:ProjectId!@argv;page_size:PageSize?@argv;cursor:Cursor?@argv;all:Bool?@local` | `AT-MEMBER-001`
- `channel-create` | `in-channel-create-v1` | `tgt-project-revision-v1` | `project_id:ProjectId!@argv;expected_project_revision:Revision!@argv;name:ChannelName!@argv` | `AT-APP-005`
- `channel-list` | `in-channel-list-v1` | `tgt-project-v1` | `project_id:ProjectId!@argv;page_size:PageSize?@argv;cursor:Cursor?@argv;all:Bool?@local` | `AT-APP-006`
- `channel-show` | `in-channel-show-v1` | `tgt-channel-project-v1` | `channel:ChannelSelector!@argv;project_id:ProjectId!@argv` | `AT-CLI-009`
- `channel-member-set` | `in-channel-member-set-v1` | `tgt-channel-membership-v1` | `channel_id:ChannelId!@argv;expected_channel_revision:Revision!@argv;member_bot_id:BotId!@argv;role_ref:RoleRef!@argv;if_membership_generation:GenerationOrAbsent!@argv` | `AT-MEMBER-001`
- `channel-member-remove` | `in-channel-member-remove-v1` | `tgt-channel-membership-v1` | `channel_id:ChannelId!@argv;expected_channel_revision:Revision!@argv;member_bot_id:BotId!@argv;expected_membership_generation:Generation!@argv` | `AT-MEMBER-001`
- `channel-member-list` | `in-channel-member-list-v1` | `tgt-channel-v1` | `channel_id:ChannelId!@argv;page_size:PageSize?@argv;cursor:Cursor?@argv;all:Bool?@local` | `AT-MEMBER-001`
- `channel-send` | `in-channel-send-v1` | `tgt-conversation-revision-v1` | `conversation_id:ConversationId!@argv;expected_revision:Revision!@argv;content:ContentSource!@oneof{text,input-file,stdin,artifact-ref}` | `AT-SEC-005`
- `channel-history` | `in-channel-history-v1` | `tgt-conversation-v1` | `conversation_id:ConversationId!@argv;page_size:PageSize?@argv;cursor:Cursor?@argv;all:Bool?@local;output:SafeOutputPath?@local` | `AT-APP-006`
- `process-show` | `in-process-show-v1` | `tgt-process-v1` | `process_id:ProcessId!@argv` | `AT-CLI-009`
- `process-watch` | `in-process-watch-v1` | `tgt-process-v1` | `process_id:ProcessId!@argv;cursor:Cursor?@argv` | `AT-APP-007`
- `operation-show` | `in-operation-show-v1` | `tgt-operation-key-v1` | `operation_id:OperationId?@argv;command_id:CommandId?@argv;idempotency_key:IdempotencyKey?@argv{exactly-one}` | `AT-SUBMIT-001`
- `operation-reconcile` | `in-operation-reconcile-v1` | `tgt-operation-revision-v1` | `operation_id:OperationId!@argv;expected_receipt_revision:Revision!@argv` | `AT-APP-005`
- `approval-list` | `in-approval-list-v1` | `tgt-principal-scope-v1` | `scope:ScopeRef?@argv;state:ApprovalState?@argv;page_size:PageSize?@argv;cursor:Cursor?@argv;all:Bool?@local` | `AT-SECSTATE-001`
- `approval-show` | `in-approval-show-v1` | `tgt-approval-v1` | `approval_id:ApprovalId!@argv` | `AT-SECSTATE-001`
- `approval-approve` | `in-approval-approve-v1` | `tgt-approval-revision-v1` | `approval_id:ApprovalId!@argv;expected_revision:Revision!@argv` | `AT-SECSTATE-001`
- `approval-deny` | `in-approval-deny-v1` | `tgt-approval-revision-v1` | `approval_id:ApprovalId!@argv;expected_revision:Revision!@argv;reason:BoundedText?@argv` | `AT-SECSTATE-001`
- `provider-list` | `in-provider-list-v1` | `tgt-instance-v1` | `capability:CapabilityId?@argv;page_size:PageSize?@argv;cursor:Cursor?@argv;all:Bool?@local` | `AT-HARNESS-001`
- `provider-show` | `in-provider-show-v1` | `tgt-provider-generation-v1` | `provider_id:ProviderId!@argv;generation:Generation!@argv` | `AT-HARNESS-001`
- `side-effect-reconcile` | `in-side-effect-reconcile-v1` | `tgt-side-effect-revision-v1` | `side_effect_id:SideEffectId!@argv;expected_revision:Revision!@argv;action:ReconcileAction!@argv{confirm,fail,manual};evidence:EvidenceRef+@argv` | `AT-SEC-005`
<!-- p0-input-registry:end -->

## Target rules

mutation은 target schema가 요구하는 revision/generation/CAS를 사용한다. create namespace는 global Instance revision 대신 uniqueness/idempotency를 사용한다. graceful runtime stop의 HostGeneration은 verified endpoint descriptor에서 local resolve하며 argv가 아니다.

`runtime start`의 `ready_at/timeout`, list/history의 `all`, file destination의 `output`, destructive confirmation은 CLI 관찰·UX field다. 이 값의 변경은 동일 CommandPayload/RequestDigest를 변경하지 않는다.

## Freeze Gate

각 milestone subset은 해당 command Acceptance, milestone platform Acceptance, 모든 선행 Gate, generated Rust metadata exact diff가 PASS한 뒤에만 freeze한다. full 63-operation freeze는 M6 전 금지다.
