---
title: "CLI Reference Interface와 P0 Command Matrix"
document_id: "DXB-IFC-041"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-IFC-040", "DXB-RUN-035"]
---

# CLI Reference Interface와 P0 Command Matrix

## 1. 목적

`dxb`를 Headless Application Contract의 첫 공식 Reference Interface이자 안정된 automation surface로 정의한다. CLI는 parser·selector input·local lifecycle·rendering·machine output·safe file writer를 소유하고 Domain/Authorization/Resource 정책을 소유하지 않는다.

## 2. Process / Instance Boundary

```text
                 ┌─ Host Lifecycle Client ─→ user service manager
                 │
dxb CLI ──────────┤
                 │
                 └─ Control Client ─→ Control Endpoint ─→ Runtime Instance
```

- CLI process/shell/session lifetime ≠ Runtime/Bot/Task/Process lifetime
- SIGINT/disconnect/broken pipe ≠ target cancel
- connection loss ≠ committed mutation rollback
- Host Lifecycle Client는 start/status/stop/readiness만 수행
- Runtime `ControlReady` 이후 Domain operation은 Control Client/Application Contract만 사용

## 3. 전역 옵션

| 옵션 | 의미 |
|---|---|
| `--profile <name>` | local endpoint/config profile 선택; Authority 아님 |
| `--instance <id>` | expected InstanceId 고정; mismatch는 실패 |
| `--json` | 단일 bounded result machine JSON |
| `--jsonl` | multi-record/stream machine JSON Lines |
| `--wait <accepted|committed|terminal>` | 요청별 허용 상태까지 local wait |
| `--follow` | subscription으로 후속 상태 관찰 |
| `--timeout <duration>` | local host/request/wait 제한; Runtime rollback 아님 |
| `--request-id <id>` | ClientRequestId 지정; CommandId/IdempotencyKey와 별개 |
| `--non-interactive` | prompt 금지; 필요한 confirmation은 명시적 오류 |
| `--yes` | local confirmation만 승인; Runtime Approval/ActionGrant 아님 |
| `--output <file>` | safe file writer가 적용되는 명령만 사용 |

`--json`과 streaming `--follow/--jsonl` 조합은 operation schema가 단일 terminal object를 명시하지 않는 한 validation error다. human output은 automation contract가 아니다.

## 4. Wait 의미

- `accepted`: Runtime이 command를 durable receipt로 수락한 상태
- `committed`: canonical mutation outcome이 결정된 상태
- `terminal`: Task/Process와 같이 command commit 뒤 장기 target lifecycle terminal까지 관찰하는 operation에서만 허용

각 command matrix가 default wait를 고정한다. local `--timeout`은 wait만 종료한다. 반환 result는 `operation_id`, observed state, target state, 계속 실행 가능성, reconciliation command를 포함한다. timeout 뒤 새 idempotency key로 자동 재시도하지 않는다.

## 5. Machine Output

### 5.1 단일 JSON

```json
{
  "schema_version": "1.0",
  "request_id": "...",
  "instance_id": "...",
  "result": {},
  "operation": {},
  "resolved_target": {},
  "observed_at": "..."
}
```

필드는 operation schema에 따라 optional하되 human text를 parse할 필요가 없게 한다.

### 5.2 JSONL

record 종류:

```text
header     schema/instance/request/operation/snapshot metadata
item       paged query record
 event     subscription event
progress   structured progress; operation이 허용할 때만
terminal   Complete/Partial/CancelledLocally/Gap/Failed + last_safe_cursor
```

terminal record 없이 정상 complete로 간주하지 않는다. stdout에는 machine record만 출력한다. diagnostic, retry/backoff, sanitizer warning, progress human text는 stderr다.

## 6. Exit Registry

숫자와 의미의 SSOT는 본 표이며 generated registry/help/schema가 동일 source에서 생성되어야 한다.

| Code | Stable class | 의미 |
|---:|---|---|
| 0 | success | 요청한 wait/observation 조건 충족 |
| 2 | usage | parser/option/combination 오류 |
| 3 | validation | typed input/schema validation 실패 |
| 4 | not-found | target/resource 없음 |
| 5 | ambiguous-target | selector 후보가 하나가 아님 |
| 6 | conflict | stale revision/generation/idempotency conflict |
| 7 | forbidden | authorization deny |
| 8 | approval-required | Runtime Approval/ActionGrant 필요 |
| 9 | resource-rejected | admission/resource/payload ceiling |
| 10 | runtime-unavailable | host/control transport/readiness 불가 |
| 11 | incompatible-version | protocol/schema/feature incompatibility |
| 12 | local-wait-timeout | Runtime rollback 없이 local wait 종료 |
| 13 | partial-output | 일부 item/event 출력 후 incomplete terminal |
| 14 | recovery-required | reconcile/repair가 필요한 canonical 상태 |
| 15 | internal | invariant/corruption/unclassified internal failure |
| 16 | local-observation-cancelled | SIGINT 등으로 local watch 종료 |
| 17 | operation-expired | receipt retention 이후 복구 불가/제한 |
| 18 | confirmation-required | non-interactive에서 local confirmation 부족 |

accepted/pending receipt를 성공 code 0으로 반환할 수 있는 것은 caller가 요청한 wait가 `accepted`일 때뿐이다. `--wait committed`에서 Pending은 success가 아니다.

## 7. Selector UX

- `--id` 또는 canonical ID positional input을 우선한다.
- name/alias 사용 시 scope option을 제공하고 resolved ID/revision을 출력한다.
- ambiguous error는 bounded candidate ID/name/scope를 machine field로 제공한다.
- destructive/high-risk command는 ID와 expected revision을 pin한다.
- implicit last-used target, fuzzy match, interactive candidate selection을 P0 mutation authority로 사용하지 않는다.

## 8. P0 Command Matrix

Legend: `C/Q/S/H` = Command/Query/Subscription/Host lifecycle. Wait의 `A/C/T` = accepted/committed/target terminal.

| CLI path | Operation | Kind | Target / revision | Idempotency | Default wait | Machine | Exit/Security/Resource | Acceptance |
|---|---|---|---|---|---|---|---|---|
| `runtime start` | `StartRuntimeHost` | H | InstanceId/HostGeneration | host request key | ControlReady | JSON | 10/11/12; peer/path/readiness bound | AT-HOST-001 |
| `runtime status` | `GetHostAndRuntimeStatus` | H+Q | InstanceId | no | immediate | JSON | host vs canonical state; bounded | AT-HOST-001 |
| `runtime stop` | `RequestRuntimeShutdown` | C/H | InstanceId+HostGeneration | receipt | C | JSON | approval/confirmation policy; no Task cancel shortcut | AT-HOST-001 |
| `runtime doctor` | `DiagnoseRuntime` | H+Q | InstanceId | no | immediate | JSON/JSONL | read-only, redacted, bounded | AT-SEC-005 |
| `version` | `GetVersionCompatibility` | Q | InstanceId optional | no | immediate | JSON | safe discovery only | AT-SCHEMA-001 |
| `bot create` | `CreateBot` | C | create scope; expected namespace rev | receipt | C | JSON | confirmation as policy; bounded input | AT-CLI-008 |
| `bot list` | `ListBots` | Q | instance scope | no | immediate | JSON/JSONL | snapshot page | AT-APP-006 |
| `bot show` | `GetBot` | Q | Bot selector | no | immediate | JSON | ambiguity safe | AT-CLI-009 |
| `bot activate` | `ChangeBotLifecycle` | C | BotId+revision | receipt | C | JSON | auth/revision | AT-CLI-008 |
| `bot deactivate` | `ChangeBotLifecycle` | C | BotId+revision | receipt | C | JSON | may await safe point | AT-APP-005 |
| `bot archive` | `ChangeBotLifecycle` | C | BotId+revision | receipt | C | JSON | local confirmation + Runtime auth | AT-CLI-010 |
| `bot restore` | `ChangeBotLifecycle` | C | BotId+revision | receipt | C | JSON | explicit conflicts | AT-CLI-008 |
| `conversation show` | `GetConversation` | Q | Conversation/Bot parent selector | no | immediate | JSON | resolved parent | AT-CLI-009 |
| `conversation send` | `SendMessage` | C | ConversationId+revision | receipt | C | JSON | typed content/artifact bounds | AT-APP-005 |
| `conversation history` | `ListMessages` | Q | ConversationId | no | immediate | JSONL for all | snapshot page/export safety | AT-APP-006 |
| `thread create` | `CreateThread` | C | parent ID+revision | receipt | C | JSON | source/parent pin | AT-CLI-008 |
| `thread list` | `ListThreads` | Q | parent scope | no | immediate | JSON/JSONL | snapshot page | AT-APP-006 |
| `thread show` | `GetThread` | Q | Thread selector+parent | no | immediate | JSON | ambiguity safe | AT-CLI-009 |
| `thread send` | `SendMessage` | C | ThreadId+revision | receipt | C | JSON | same canonical send use-case | AT-APP-005 |
| `thread history` | `ListMessages` | Q | ThreadId | no | immediate | JSONL for all | snapshot page | AT-APP-006 |
| `thread branch` | `BranchThread` | C | ThreadId+source revision/message | receipt | C | JSON | immutable branch point | AT-CLI-008 |
| `task submit` | `SubmitTask` | C | owner/scope ID+revision | receipt | C | JSON | admission/approval; bounded refs | AT-APP-005 |
| `task list` | `ListTasks` | Q | Bot/Project/Channel scope | no | immediate | JSON/JSONL | snapshot page | AT-APP-006 |
| `task show` | `GetTask` | Q | Task selector | no | immediate | JSON | canonical vs projection state | AT-CLI-009 |
| `task watch` | `WatchTask` | S | TaskId | no | follow | JSONL | cursor/gap/terminal | AT-APP-007 |
| `task cancel` | `ControlTask.Cancel` | C | TaskId+revision/generation | receipt | C | JSON | approval; side-effect unknown possible | AT-CLI-010 |
| `task suspend` | `ControlTask.Suspend` | C | TaskId+revision/generation | receipt | C | JSON | may be AwaitingSafePoint | AT-CLI-010 |
| `task resume` | `ControlTask.Resume` | C | TaskId+revision | receipt | C | JSON | reauth/readmission | AT-APP-005 |
| `task redirect` | `ControlTask.Redirect` | C | TaskId+revision | receipt | C | JSON | new revision/execution; bounded input | AT-CLI-010 |
| `task result` | `GetTaskResult` | Q | TaskId | no | immediate | JSON/stream/file | safe artifact output | AT-SEC-005 |
| `memory get` | `GetMemory` | Q | MemoryId/scope | no | immediate | JSON | current auth/epistemic state | AT-CLI-009 |
| `memory search` | `SearchMemory` | Q | scope selector | no | immediate | JSONL for all | snapshot page/candidate bound | AT-APP-006 |
| `memory history` | `GetMemoryHistory` | Q | MemoryId | no | immediate | JSONL | revision snapshot | AT-APP-006 |
| `memory propose` | `ProposeMemory` | C | target scope+revision | receipt | C | JSON | untrusted proposal; info label | AT-APP-005 |
| `memory promote` | `PromoteMemory` | C | proposal+target scope revisions | receipt | C | JSON | declassification/approval | AT-SEC-005 |
| `project create` | `CreateProject` | C | namespace revision | receipt | C | JSON | auth/bounds | AT-CLI-008 |
| `project list` | `ListProjects` | Q | instance scope | no | immediate | JSON/JSONL | snapshot page | AT-APP-006 |
| `project show` | `GetProject` | Q | Project selector | no | immediate | JSON | ambiguity safe | AT-CLI-009 |
| `project archive` | `ChangeProjectLifecycle` | C | ProjectId+revision | receipt | C | JSON | confirmation/auth; child policy explicit | AT-CLI-010 |
| `project restore` | `ChangeProjectLifecycle` | C | ProjectId+revision | receipt | C | JSON | conflict explicit | AT-CLI-008 |
| `channel create` | `CreateChannel` | C | ProjectId+revision | receipt | C | JSON | parent authorization | AT-CLI-008 |
| `channel list` | `ListChannels` | Q | Project scope | no | immediate | JSON/JSONL | snapshot page | AT-APP-006 |
| `channel show` | `GetChannel` | Q | Channel selector+Project | no | immediate | JSON | ambiguity safe | AT-CLI-009 |
| `channel join` | `JoinChannel` | C | ChannelId+revision | receipt | C | JSON | membership policy | AT-APP-005 |
| `channel leave` | `LeaveChannel` | C | ChannelId+membership generation | receipt | C | JSON | target Task unaffected | AT-CLI-010 |
| `channel members` | `ListChannelMembers` | Q | ChannelId | no | immediate | JSON/JSONL | snapshot/auth | AT-APP-006 |
| `channel send` | `SendMessage` | C | Channel ConversationId+revision | receipt | C | JSON | same canonical send; info flow | AT-SEC-005 |
| `channel history` | `ListMessages` | Q | Channel ConversationId | no | immediate | JSONL | snapshot/current auth | AT-APP-006 |
| `process show` | `GetProcess` | Q | ProcessId | no | immediate | JSON | child refs, no duplicated state | AT-CLI-009 |
| `process watch` | `WatchProcess` | S | ProcessId | no | follow | JSONL | cursor/gap/terminal | AT-APP-007 |
| `operation show` | `GetOperation` | Q | OperationId/CommandId | no | immediate | JSON | receipt safe view | AT-APP-005 |
| `operation reconcile` | `ReconcileOperation` | C | OperationId+receipt revision | receipt | C | JSON | recovery auth; no blind replay | AT-APP-005 |
| `approval list` | `ListApprovals` | Q | principal/scope | no | immediate | JSON/JSONL | redacted snapshot | AT-SEC-005 |
| `approval show` | `GetApproval` | Q | ApprovalId | no | immediate | JSON | action digest/target safe view | AT-SEC-005 |
| `approval approve` | `DecideApproval.Approve` | C | ApprovalId+revision | receipt | C | JSON | Runtime auth; `--yes` irrelevant | AT-SEC-005 |
| `approval deny` | `DecideApproval.Deny` | C | ApprovalId+revision | receipt | C | JSON | Runtime auth | AT-SEC-005 |
| `provider list` | `ListProviders` | Q | capability optional | no | immediate | JSON/JSONL | read-only safe summary | AT-CLI-008 |
| `provider show` | `GetProvider` | Q | ProviderId/generation | no | immediate | JSON | no secret/internal registry | AT-CLI-009 |
| `reconcile operation` | `ReconcileOperation` | C | OperationId+revision | receipt | C | JSON | typed recovery only | AT-APP-005 |
| `reconcile side-effect` | `ReconcileSideEffect` | C | SideEffectId+revision | receipt | C | JSON | high-risk approval/audit | AT-SEC-005 |

P0 matrix 밖의 명령을 P0 구현에 조용히 추가하지 않는다. `runtime status`처럼 host+query가 결합되는 경우에도 output은 source를 구분한다.

## 9. P1 / P2 Backlog

P1/P2로 분리하며 P0 Release Gate를 차단하지 않는다.

- Goal/Routine 전체 CRUD/occurrence 운영
- Core list/show/watch 상세 운영
- Provider select-preview/deprecate/drain/detach
- Plugin install/validate/enable/disable/upgrade/rollback/uninstall
- Capability test
- Memory put/correct/archive/forget/compact/index 운영
- Task retry/reprioritize/delegate/graph/inspect/report 상세 alias
- bot hard delete, project/channel hard delete
- 광범위 trace follow/export
- 범용 repair/reconcile mutation
- shell completion/man page
- raw terminal rendering

P1/P2도 도입 시 typed Application operation과 같은 security/resource/schema 규칙을 따라야 한다.

## 10. Human Rendering / Terminal Safety

human renderer는 untrusted text의 ANSI/OSC/C0/C1을 기본 neutralize한다. ID, scope, revision, lifecycle을 명시해 truncation/name spoofing으로 잘못된 target처럼 보이지 않게 한다. color/TTY width가 machine semantic을 바꾸지 않는다. `--raw`는 P0 비지원이다.

## 11. Safe File Output

`--output`은 matrix에서 file output을 허용한 Query/Artifact에만 적용한다.

- existing destination overwrite 금지
- traversal/symlink/special file 거부
- same-dir exclusive temp, sensitive mode `0600`
- bounded incremental write
- flush/fsync 가능한 범위 후 atomic rename
- partial/cancel cleanup
- JSONL partial은 terminal record를 파일에 기록할 수 있을 때 기록하고 exit 13

mutation command result를 file write 성공과 하나의 canonical transaction으로 가장하지 않는다.

## 12. Retry / Reconcile UX

- transport error 전송 여부가 불확실하면 local journal의 CommandId/RequestDigest로 `operation show`를 먼저 시도한다.
- Runtime이 operation을 모른다고 확인한 경우에만 같은 logical key/digest로 재전송한다.
- operation-expired, recovery-required, side-effect unknown은 자동 retry하지 않는다.
- retry decision은 structured RetryDisposition을 따른다.

## 13. 금지 Dependency / Pattern

```text
dxb-cli -X-> Domain/Runtime/Storage internals
dxb-cli -X-> concrete Provider
Host Lifecycle Client -X-> Bot/Task/Memory/Scheduler state
dxb-cli -X-> local authorization/resource policy
```

추가 금지:
- generic JSON mutation
- human string parsing automation
- full dataset aggregation
- implicit fuzzy/last-used destructive target
- SIGINT implicit Task cancel
- machine stdout spinner/log

## 14. 검증 기준

### AT-CLI-001~007
기존 Reference E2E, process independence, machine output, bounded output, no client policy, recovery, active Interface scope 의미를 유지한다.

### AT-CLI-008 — P0 Command Matrix Completeness
모든 P0 row가 operation/schema/owner/test를 가지며 matrix 밖 P0 command와 중복 alias 0.

### AT-CLI-009 — Resource Selector Ambiguity Safety
동일 이름 fixture에서 silent mutation 0, machine error에 bounded candidates와 resolved target schema가 존재.

### AT-CLI-010 — Machine Wait / Timeout / Partial Semantics
accepted/pending/committed/target terminal/local timeout/partial을 script가 구분하며 timeout 후 duplicate effect 0.

### AT-SEC-005 — Local Endpoint / Terminal / Export Safety
endpoint hijack, ANSI/OSC, traversal/symlink/special file, partial sensitive output fixture가 fail closed다.
