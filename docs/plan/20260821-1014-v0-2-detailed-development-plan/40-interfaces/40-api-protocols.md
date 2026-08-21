---
title: "Control API와 프로토콜"
document_id: "DXB-IFC-040"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-026", "DXB-ARC-014", "DXB-ARC-016", "DXB-RUN-032"]
---

# Control API와 프로토콜

## 1. 목적

CLI/TUI/Web/IDE/자동화가 동일 Runtime 계약을 사용하고 Routine/Provider/Plugin/Recovery 상태를 내부 Rust trait나 DB schema 노출 없이 관리하게 한다.

## 2. 프로토콜 표면

### Command
- Bot lifecycle/identity
- Goal/Task/Execution action
- Routine create/update/enable/disable/run-now
- Memory create/correct/archive/forget
- Core/Runtime control
- Bot Message/delegation
- approval/policy/config
- Provider deprecate/drain/detach operation
- Plugin install/validate/enable/disable/upgrade/rollback/uninstall
- Side Effect reconciliation

### Query
- Bot/Task/Execution/Continuation/Routine
- Memory/provenance/retention
- Core/queue/resource/health
- capability/provider catalog and selector preview
- Plugin manifest/lifecycle/data ownership
- audit/operation/migration diagnostics

### Event Stream
Domain projection, operation progress, Runtime state, approval, Provider/Plugin lifecycle, Routine occurrence, recovery alert를 제공한다.

## 3. Wire 경계

- Domain Rust struct를 persistence/wire에 그대로 derive해 공개하지 않는다.
- 내부 Rust trait는 Plugin public ABI가 아니다.
- Public Plugin/Provider remote contract가 필요하면 versioned wire schema를 사용한다.
- DeepSeek ACP/JSON-RPC 등 upstream protocol은 Harness Adapter 내부다.

## 4. Command Envelope

command ID/type/version, principal, target, expected revision, idempotency key/scope, correlation/causation, deadline, dry-run, payload, client metadata를 가진다.

응답은 committed/accepted/rejected/conflict, resulting revision/events, Operation ID, stable error, warnings, server/schema version을 반환한다.

## 5. Routine API

논리 resource:
- RoutineId/BotId/revision
- enabled
- schedule/trigger summary
- overlap/missed policy
- next/last occurrence
- generated Task refs

`run-now`는 별도 비밀 실행 경로가 아니라 일반 Task 생성 Command를 사용한다. update는 expected revision을 요구한다.

## 6. Provider API

Capability catalog와 Provider runtime state를 구분한다.

Provider 상태 예:
- available
- degraded
- deprecated
- draining
- unavailable
- removed

selection preview는 입력 capability/Bot/Task policy에 대해 실제 selector가 어떤 Provider를 고를지와 이유 class를 보여줄 수 있다. 이를 execution commit으로 오인하지 않는다.

## 7. Plugin API

Plugin resource는:
- PluginId/version/package digest
- lifecycle state
- API compatibility
- provided/required capabilities
- requested/granted permissions
- config/data schema versions
- resource status
- uninstall data policy
를 노출한다.

install/upgrade/uninstall은 long-running `Operation`일 수 있으며 dry-run, approval, progress, rollback 가능성을 제공한다.

## 8. Unsupported / Removed Semantics

제거된 기능을 silent ignore하거나 임의 Provider로 fallback하지 않는다.

stable error/metadata 예:
- `capability-unavailable`
- `provider-deprecated`
- `provider-removed`
- `plugin-disabled`
- `plugin-incompatible`
- `configuration-stale`
- `reconciliation-required`

정확한 code registry는 구현 schema에서 고정하되 의미를 합치지 않는다.

## 9. Continuation / Recovery Query

운영자는 Waiting Task의 wait reason, pending child count, deadline, continuation revision/checkpoint refs, recovery warning을 조회할 수 있다. 민감 payload 전체를 기본 노출하지 않는다.

Side Effect Unknown entry는 external ref, action digest, reconciliation status를 조회하되 secret/raw request는 권한에 따라 redaction한다.

## 10. Versioning

API, request/response/event, Plugin public API, Provider remote schema는 독립 version 범위를 가질 수 있다. additive optional field를 우선하며 unknown major/critical variant는 명시 reject한다. deprecation window와 capability discovery를 제공한다.

## 11. 성능 / Backpressure

request/response size cap, stable cursor pagination, per-client stream buffer, Artifact streaming, connection/subscription limits, slow-client resync를 사용한다. Control API 때문에 전체 graph/list를 heap materialize하지 않는다.

## 12. 검증 기준

- CLI/TUI/Web이 같은 Routine/Provider/Plugin schema/client를 사용한다.
- removed Provider config/action이 명시 error로 보인다.
- Plugin internal trait 변경이 API schema에 자동 전파되지 않는다.
- Waiting/Side Effect recovery 상태를 API에서 추적할 수 있다.
- slow event client가 server memory를 무제한 증가시키지 않는다.
