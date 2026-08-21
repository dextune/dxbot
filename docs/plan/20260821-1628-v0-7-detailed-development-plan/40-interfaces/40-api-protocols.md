---
title: "Headless Application Contract와 Control Protocol"
document_id: "DXB-IFC-040"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-026", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-036", "DXB-RUN-037", "DXB-RUN-038", "DXB-ARC-014", "DXB-ARC-016", "DXB-ARC-017", "DXB-RUN-032"]
---

# Headless Application Contract와 Control Protocol

## 1. 목적

본 문서는 v0.7의 **Headless Application Contract / Control Protocol Canonical Owner**다. v0.6 Runtime의 Bot/Conversation/Thread/Project/Channel/Task/Memory/Process/Control/Provider/Resource 의미를 Domain internal API 없이 안정적으로 노출한다.

v0.7의 공식 Reference Consumer는 CLI 하나다. Contract 자체는 CLI rendering/terminal semantics에 종속되지 않는다.

> **Runtime 의미는 Backend가 소유하고, Application Contract는 그 의미를 노출하며, consumer는 이를 재해석하지 않는다.**

## 2. Contract 범주

```text
Command      = Canonical mutation intent
Query        = bounded read/projection request
Subscription = durable state/event observation
```

다음은 같은 envelope로 합치지 않는다.
- Message
- Command
- Query
- Domain Event
- public Subscription Event
- Provider stream
- internal Work Queue / Control Channel

## 3. Command Contract

mutation Command는 필요한 범위에서 다음 의미를 가진다.

```text
CommandId
PrincipalRef
Target ResourceRef
Action
Expected Revision / Generation
IdempotencyKey
Correlation / Causation
Deadline
Optional Approval / ActionGrantRef
Payload
Protocol / Schema Context
```

### 3.1 Command Outcome

단순 `ok/error` 대신 최소 다음 의미를 구분할 수 있어야 한다.

- accepted
- committed
- rejected / validation-failed
- forbidden
- approval-required
- conflict
- stale revision/generation
- pending / awaiting-safe-point
- superseded
- recovery-required
- resource-exhausted / admission-rejected
- incompatible-version

exact wire code/name/number는 ADR에서 freeze하며 Domain error를 문자열 비교로 재구성하지 않는다.

### 3.2 Idempotency / Lost Response

- logical mutation은 retry 시 동일 IdempotencyKey를 보존한다.
- Runtime commit 후 response가 손실되어도 retry가 새 Canonical effect를 만들지 않는다.
- existing command receipt/outcome 또는 reconciliation query로 결과를 회수할 수 있어야 한다.
- idempotency key는 authorization/expected revision을 대체하지 않는다.
- external Side Effect Unknown은 기존 Side Effect Ledger/Reconciliation을 우회하지 않는다.

## 4. Query Contract

Query는 Canonical read facade 또는 Projection을 사용한다.

원칙:
- pagination 기본
- bounded page/window/items/bytes
- total count를 위해 full materialization하지 않음
- `observed_at`, source revision/watermark, stale/degraded를 필요한 범위에서 제공
- Derived 상태를 Canonical truth처럼 표시하지 않음
- authorization을 cache/index/projection에 위임하지 않음
- large content는 Artifact/reference/stream을 우선

Query response schema가 Domain struct나 persistence row를 그대로 노출하지 않는다.

## 5. Subscription / Event Contract

watch/follow consumer를 위해 public event stream은 최소 다음 의미를 가진다.

- at-least-once delivery 허용
- stable event identity
- cursor / watermark
- source revision/observed_at where applicable
- gap detection
- resume / explicit resync
- bounded server/client buffers
- slow consumer/backpressure policy
- terminal/disconnect reason

Runtime internal queue/event bus/control channel을 그대로 외부에 노출하지 않는다.

### 5.1 Reconnect

- reconnect 시 current authorization/version을 다시 검증한다.
- retained cursor가 유효하면 resume한다.
- expired/missing cursor는 explicit resync/query로 전환한다.
- silent event loss 후 current라고 표시하지 않는다.

## 6. Protocol / Schema / Runtime / Client Version

최소 다음을 구분한다.

```text
protocol_version
schema_version
runtime_version
client_version
minimum/compatible range where applicable
supported feature set
```

Data storage schema version은 별도다.

원칙:
- incompatible pair는 explicit error
- unknown mutation semantic을 client가 추측해 호출하지 않음
- unknown optional read field는 가능한 범위에서 forward-compatible 처리
- incompatible command schema silent fallback 금지
- runtime binary version과 data schema/protocol version을 동일 숫자로 가정하지 않음

## 7. Capability / Feature Discovery

read-only discovery는 권한상 허용되는 safe summary만 제공한다.

예:
- supported public commands/queries/subscriptions/features
- enabled capability class
- Provider/Plugin availability의 추상 상태
- protocol/schema compatibility
- Runtime pressure/degraded state

Capability discovery는 permission/Authority/ActionGrant를 생성하지 않는다. Provider Registry internal object를 노출하지 않는다.

## 8. Runtime Host / Control Endpoint

```text
Long-lived Runtime Host
        ↑
Control Endpoint
        ↑
Control Client
```

Control Endpoint는 **실행 중인 Runtime의 Application Contract transport adapter**다. exact local IPC는 ADR 대상이지만 다음 semantic은 필수다.

- authenticated/resolved principal
- authorization enforcement path
- request/response byte cap
- deadline/local request cancellation
- connection failure semantics
- version negotiation
- streaming/backpressure
- no Domain semantic leakage

transport connection/session은 Bot/Conversation/Thread/Task/Process identity가 아니다.

### 8.1 Runtime Bootstrap / Host Lifecycle Boundary

Runtime이 아직 실행되지 않은 상태의 `start`는 Control Endpoint로 처리할 수 없다. 따라서 Runtime process/service lifecycle에는 `DXB-ARC-010/011`의 narrow Host Lifecycle Adapter/Client를 사용한다.

Host Lifecycle Boundary가 소유할 수 있는 것은 다음으로 제한한다.
- process/service start
- process/service status discovery
- graceful stop bootstrap/supervision
- endpoint/readiness discovery

이는 **Application Domain Command가 아니며** Bot/Project/Channel/Thread/Task/Memory/Process/Scheduler/Provider state를 직접 읽거나 mutate할 수 없다.

Runtime이 Ready가 된 이후의 Domain status/doctor/control/use-case는 본 Application Contract를 사용한다. Host Lifecycle Client를 Domain fallback API로 사용하지 않는다.

## 9. Existing Domain Surface

Headless Contract만으로 실행 중 Runtime의 최소 P0 Domain use-case를 완결할 수 있어야 한다.

- Runtime canonical status/resource/doctor/control where Runtime is Ready
- Bot lifecycle/status
- Main Conversation send/history
- Thread create/history/branch
- Project/Channel lifecycle/membership/Role/Authority
- Task submit/inspect/report/control/result
- Durable Process inspect/control/observe
- Scoped Memory read/propose/promote/history
- Provider/Capability/Plugin safe status
- approval/reconcile/trace
- Runtime resource/pressure observability

Runtime process/service start/host status/readiness bootstrap은 Section 8.1의 Host Lifecycle Boundary가 담당한다. Bot-only command에 ProjectId/ChannelId/ProcessId를 필수화하지 않는다.

## 10. Security / Principal

- request는 `PrincipalRef`/SecurityDomain context에 귀결된다.
- Role/Authority/Authorization은 구분한다.
- current authorization을 mutation/read 단계에서 검증한다.
- approval-required/high-risk action은 existing ActionGrant/Approval 의미를 사용한다.
- secret/token/raw sensitive content를 public diagnostics에 노출하지 않는다.
- Private→Shared publication은 Information-Flow/Declassification을 통과한다.

## 11. Resource / Memory Pressure Semantic

최소 다음 class를 stable하게 전달한다.
- resource exhausted
- admission rejected/deferred
- payload too large / decoded expansion rejected
- pressure constrained/critical
- budget exhausted
- stream terminated by cumulative byte cap
- recovery-required

OOM 자체를 정상 retryable application response로 약속하지 않는다.

## 12. Large Data / Backpressure

- request/response cumulative byte cap
- decoded/decompressed allocation preflight
- pagination/windowing
- Artifact streaming/reference
- JSONL/stream consumer 지원에 필요한 incremental contract
- slow consumer gap/resync
- full Project/Channel/history/trace materialization 금지

## 13. Interface Neutrality / Non-Goals

v0.7에서 구현하지 않는다.
- TUI/Web specific route/view/pane schema
- browser reconnect semantics
- frontend state store
- BFF
- generic UI component protocol
- speculative multi-transport abstraction 폭증

새 product Interface는 v0.7 Runtime Host/Application Contract/CLI의 M0~M5/DoD와 compatibility/resource/security evidence가 안정화된 뒤 별도 버전에서 재승인한다.

현재 파일명 `40-api-protocols.md`는 기존 link/document ID 호환성을 위해 유지한다. Canonical 책임은 본 문서 제목과 `DXB-IFC-040`으로 고정한다.

## 14. 검증 기준

### AT-APP-001 — Headless Contract Completeness
Backend representative fixture를 Domain internal API 없이 Contract만으로 수행 가능하며 direct Store/Scheduler/Provider access 0.

### AT-APP-002 — Contract Version Compatibility
compatible pair는 semantic 유지, incompatible pair는 explicit version error, silent reinterpretation 0.

### AT-APP-003 — Command Idempotency / Lost Response
commit 후 response loss에서 동일 key retry가 duplicate Canonical effect를 만들지 않고 기존 outcome을 회수한다.

### AT-APP-004 — Subscription Resume / Gap
disconnect/gap 후 resume 또는 explicit resync하며 silent event loss로 current state를 오판하지 않는다.

추가:
- Runtime bootstrap path가 Host Lifecycle Boundary를 사용하되 Domain mutation 0.
- Runtime Ready 이후 Domain operation의 Host Lifecycle shortcut 0.
- 기존 AT-PROC/MEM/SEC/COLLAB/RMEM 및 Bot-only Acceptance도 headless path로 재현 가능해야 한다.
