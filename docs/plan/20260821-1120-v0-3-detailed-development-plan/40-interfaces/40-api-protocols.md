---
title: "Control API와 프로토콜"
document_id: "DXB-IFC-040"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-026", "DXB-ARC-014", "DXB-ARC-016", "DXB-ARC-017", "DXB-RUN-032"]
---

# Control API와 프로토콜

## 1. 목적

CLI/TUI/Web/IDE/자동화가 동일 Runtime 계약을 사용하고 Routine/Capability/Provider/Plugin/Recovery 상태를 내부 Rust trait나 DB schema 노출 없이 관리하게 한다.

## 2. 프로토콜 표면

### Command
- Bot lifecycle/identity
- Goal/Task/Execution action
- Routine create/update/enable/disable/run-now
- Memory create/correct/archive/forget
- Core/Runtime control
- Bot Message/delegation
- approval/policy/config
- Provider validate/restart/deprecate/drain/detach/revalidate operation
- Plugin install/validate/enable/disable/upgrade/rollback/uninstall
- Side Effect reconciliation

### Query
- Bot/Task/Execution/Continuation/Routine
- Memory/provenance/retention
- Core/queue/resource/health
- Capability Contract/catalog/version
- Provider lifecycle/health/generation/config/SDK/conformance evidence
- selector preview/activity/drain/removal diagnostics
- Plugin manifest/lifecycle/data ownership/provided Provider
- audit/operation/migration diagnostics

### Event Stream
Domain projection, operation progress, Runtime state, approval, Common Provider lifecycle/generation, Plugin lifecycle, Routine occurrence, recovery alert를 제공한다.

## 3. Wire 경계

- Domain Rust struct를 persistence/wire에 그대로 derive해 공개하지 않는다.
- 내부 Rust trait/Provider SPI는 Plugin public ABI가 아니다.
- public Plugin/remote Provider contract가 필요하면 versioned wire schema를 사용한다.
- DeepSeek ACP/JSON-RPC 등 upstream protocol은 Harness Adapter 내부다.
- Provider Host 내부 permission/resource handle은 API로 직렬화하지 않는다.

## 4. Command Envelope

command ID/type/version, principal, target, expected revision/generation, idempotency key/scope, correlation/causation, deadline, dry-run, payload, client metadata를 가진다.

응답은 committed/accepted/rejected/conflict, resulting revision/events, Operation ID, stable error, warnings, server/schema version을 반환한다.

## 5. Routine API

RoutineId/BotId/revision, enabled, schedule/trigger summary, overlap/missed policy, next/last occurrence, generated Task refs를 노출한다. `run-now`는 일반 Task 생성 Command를 사용한다.

## 6. Capability / Provider API

Capability와 Provider를 별도 resource로 표현한다.

Capability:
- capability ID
- contract semantic version/range
- optional feature metadata
- request/response/error schema reference
- Conformance suite version

Provider:
- provider ID/version
- provided capability/version range
- Provider SDK compatibility
- config schema version/digest
- registry generation
- lifecycle state
- health observation
- in-flight/activity/drain progress
- deprecation/removal status
- last Conformance evidence reference

Standard lifecycle vocabulary:
`Declared | Validated | Starting | Ready | Draining | Stopped | Degraded | Failed | Quarantined | Incompatible`.

호환성을 위한 UI summary로 `available/unavailable/removed`를 표시할 수 있으나 이는 Common Lifecycle state를 대체하지 않는다.

selection preview는 capability/Bot/Task policy에 대해 selector가 고를 Provider와 이유 class를 보여주되 실제 Execution commit으로 오인하지 않는다.

## 7. Provider Operation

validate/start/restart/deprecate/drain/detach/revalidate/quarantine 관련 변경은 Common Lifecycle/Registry Command로 실행하며 long-running `Operation`이 될 수 있다.

Operation에는 target provider/version/generation, requested action, validation/compatibility result, progress, quiescence, terminal state, cleanup result가 포함된다. API가 Provider implementation hook을 직접 호출하지 않는다.

## 8. Plugin API

Plugin resource는 PluginId/version/package digest, lifecycle state, public API compatibility, provided/required capabilities, requested/granted permissions, config/data schema, resource status, uninstall policy를 노출한다.

Plugin이 Provider를 제공하면 Plugin lifecycle과 별도로 각 Provider의 Common lifecycle/conformance를 표시한다. actual Capability execution은 Control API가 아니라 Runtime Provider Host 경로에서 수행된다.

## 9. Unsupported / Removed / Incompatible Semantics

제거·비호환 기능을 silent ignore하거나 임의 Provider로 fallback하지 않는다.

stable error/metadata 예:
- `capability-unavailable`
- `provider-not-ready`
- `provider-degraded`
- `provider-quarantined`
- `provider-incompatible`
- `provider-deprecated`
- `provider-removed`
- `plugin-disabled`
- `plugin-incompatible`
- `configuration-stale`
- `reconciliation-required`

정확한 code registry는 구현 schema에서 고정하되 의미를 합치지 않는다.

## 10. Continuation / Recovery Query

Waiting Task의 wait reason, pending child count, deadline, continuation revision/checkpoint refs, recovery warning을 조회한다. Side Effect Unknown은 external ref/action digest/reconciliation을 조회하되 secret/raw request는 redaction한다.

## 11. Versioning

API, request/response/event, Capability Contract, Provider SDK, Plugin public API, remote Provider wire는 독립 version 범위를 가질 수 있다. additive optional field를 우선하며 unknown major/critical variant는 명시 reject한다. deprecation window와 capability discovery를 제공한다.

## 12. 성능 / Backpressure

request/response size cap, stable cursor pagination, per-client stream buffer, Artifact streaming, connection/subscription limits, slow-client resync를 사용한다. Provider catalog/conformance evidence 전체 payload를 무조건 heap materialize하지 않는다.

## 13. 검증 기준

- CLI/TUI/Web이 같은 Routine/Capability/Provider/Plugin schema/client를 사용한다.
- API Provider state가 Common Lifecycle 의미와 일치한다.
- health와 lifecycle/generation을 별도 field로 표현한다.
- removed/incompatible Provider config/action이 명시 error로 보인다.
- Plugin internal trait 변경이 API schema에 자동 전파되지 않는다.
- Waiting/Side Effect recovery 상태를 추적할 수 있다.
- slow event client가 server memory를 무제한 증가시키지 않는다.
