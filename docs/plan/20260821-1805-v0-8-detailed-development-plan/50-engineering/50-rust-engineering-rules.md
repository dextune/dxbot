---
title: "Rust 구현 규칙"
document_id: "DXB-ENG-050"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-011", "DXB-IFC-040", "DXB-IFC-041", "DXB-RUN-030"]
---

# Rust 구현 규칙

## 1. 목적

v0.8 계약을 Rust로 구현할 때 책임 경계, 타입 분리, bounded resource, cancellation, schema generation, safe local I/O를 공통 구현 규칙으로 고정한다. 구현 편의를 위한 shortcut·중복 정책·crate 폭증을 금지한다.

## 2. Logical Boundary

최소 logical responsibility:

```text
kernel/value types
Domain aggregates
Application use cases + Operation Receipt owner
Runtime composition/scheduler/resource/recovery
Provider Host + stable capability contracts
Application public contract/schema source
Control server/client + Host Lifecycle adapter
CLI parser/render/machine/safe writer/local journal
```

독립 compile/dependency/lifecycle/test 근거가 없으면 각 책임을 별도 crate로 기계적으로 분리하지 않는다. 반대로 CLI/Provider/Storage concrete dependency가 Domain을 오염시키지 않게 firewall을 유지한다.

## 3. Dependency Firewall

허용:

```text
CLI → public application-contract values
CLI → control client
CLI → narrow host-lifecycle client
Control adapter → application ports/public mapping
Runtime composition → Provider Host
```

금지:

```text
CLI → Domain/Runtime/Storage internals
CLI → concrete Provider/Registry/Scheduler mutator
Host Lifecycle Client → Bot/Task/Memory/Provider state
Control endpoint → Domain Store direct mutation bypassing Application
Provider → Application/Runtime/Storage internals
Public DTO = Domain struct = Persistence row
```

workspace metadata/architecture test로 forbidden edge를 검출한다.

## 4. Public Schema Source

Rust `application-contract` logical boundary가 Command/Query/Subscription DTO, error, operation registry, exit metadata의 source다.

- Domain/Persistence type을 `derive`로 그대로 public wire에 노출하지 않는다.
- deterministic schema/golden generator는 stable field ordering과 generator version을 가진다.
- generated output을 수동 수정하지 않는다.
- operation name/string registry를 CLI와 서버에 복사하지 않고 generated/typed metadata를 재사용한다.
- 범용 RPC/IDL framework를 P0에 새로 만들지 않는다.

## 5. Resource Selector / Receipt 구현

- 식별자는 `BotId`, `TaskId`, `OperationId`, `InstanceId` 등 의미별 newtype을 사용한다.
- selector와 resolved ref를 다른 타입으로 분리해 unresolved input을 Domain mutation에 전달하지 않는다.
- RequestDigest canonicalization은 locale/map iteration/serialization incidental order에 의존하지 않는다.
- idempotency lookup과 canonical commit/receipt write는 crash-safe boundary를 가진다.
- same key/different binding을 type/validation 단계에서 explicit conflict로 처리한다.
- receipt payload는 full request/result clone보다 digest/outcome reference를 우선한다.

## 6. Runtime Instance / Host

- endpoint path와 data root를 untyped `String`으로 섞지 않고 validated path/value type을 사용한다.
- file/socket/lock open은 no-follow, owner/mode, exclusive create를 적용한다.
- HostGeneration fencing을 storage/control write path에서 검증한다.
- shell command string 조합 대신 structured argv/service API를 사용한다.
- PID/process handle을 Task/Execution control handle로 재사용하지 않는다.

## 7. Async / Cancellation / Ownership

- spawn된 모든 task는 supervisor/owner/cancel/join을 가진다.
- unbounded channel과 fire-and-forget 금지.
- CLI local cancellation token과 Runtime Task cancellation token을 공유하지 않는다.
- request deadline, local wait timeout, host operation timeout, target Task deadline을 다른 타입/상태로 구분한다.
- subscriber/client writer drop 시 producer unregister/cleanup을 보장한다.
- async lock guard를 잡은 채 slow I/O/serialization/file write를 await하지 않는다.
- blocking filesystem/fsync는 bounded blocking boundary를 사용한다.

## 8. Pagination / Streaming / Output

- page/chunk를 incremental decode→map→serialize→write 후 release한다.
- `--all`에서 full `Vec`, giant JSON array/String, unbounded serde intermediate를 금지한다.
- cursor는 integrity-protected opaque bytes/string이고 decoded internals를 client authority로 신뢰하지 않는다.
- JSONL terminal record를 normal item/event와 typed enum으로 구분한다.
- stdout/stderr writer를 분리하고 machine stdout에 logging subscriber를 연결하지 않는다.
- partial writer는 last safe cursor와 terminal/error를 보존한다.

## 9. CLI Local Journal

- max entries와 max encoded bytes를 config/constant source에서 하나로 관리한다.
- atomic replace/append+compaction과 crash recovery를 정의한다.
- secret/full payload/result/Authority/ActionGrant를 저장하지 않는다.
- Runtime receipt를 대체하거나 operation state를 local guess로 commit하지 않는다.

## 10. Terminal / File Safe Writer

- sanitizer는 untrusted text를 streaming 처리하고 output expansion cap을 가진다.
- ANSI/OSC parser를 ad-hoc regex 여러 곳에 복제하지 않는다.
- safe file writer는 exclusive temp, no-follow, mode, bounded chunk, flush/fsync, atomic rename, cleanup을 하나의 owned component로 제공한다.
- path validation 후 open 사이 race를 file descriptor 기반 방식으로 다시 검증한다.
- overwrite/force semantics를 P0에 암묵 추가하지 않는다.

## 11. Error / Retry

- ApplicationError와 HostLifecycleError를 typed mapping한다.
- 문자열 matching으로 retry/authorization/exit code를 결정하지 않는다.
- retry는 RetryDisposition, idempotency, deadline, receipt, Side Effect 상태를 함께 본다.
- `Unknown/RecoveryRequired`를 transient error로 downgrade하지 않는다.
- panic/invariant violation을 user validation error로 숨기지 않는다.

## 12. Ownership·Copy·Allocation

- caller-owned input은 가능한 범위에서 borrow한다.
- large immutable payload는 `Arc`/reference/Artifact/stream을 비용 근거에 따라 사용한다.
- hot path의 `String`↔bytes↔JSON 반복 변환을 측정한다.
- `clone()`을 기계적으로 금지하지 않지만 소유 의미와 bytes cost를 설명할 수 있어야 한다.
- receipt/page/event DTO가 Canonical payload를 중복 retain하지 않게 한다.

## 13. 금지 패턴

- generic `execute(method: String, payload: Value)` public mutation
- service locator / mega RuntimeContext
- `common`, `utils`, `helpers`, `misc` dumping ground
- Interface별 duplicate selector/authorization/retry
- crate-per-command 또는 crate-per-DTO
- speculative TUI/Web/BFF/remote transport module
- raw `unsafe` endpoint/file optimization without safety contract

## 14. 검증 기준

- forbidden dependency 0.
- public DTO와 Domain/Persistence direct type sharing 0.
- selector unresolved type이 mutation owner에 도달하는 path 0.
- receipt commit/lost-response failpoint 통과.
- large output full aggregation 0.
- local/Runtime cancellation 혼용 0.
- sanitizer/file writer security fixture 통과.
- schema/help/exit/operation registry generated drift 0.
