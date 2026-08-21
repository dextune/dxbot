---
title: "Plugin 시스템"
document_id: "DXB-ARC-016"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P1/P2"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-012", "DXB-RUN-032", "DXB-ENG-053"]
---

# Plugin 시스템

## 1. 목적

Plugin을 DXBOT 내부 모듈이나 Provider의 동의어로 사용하지 않고, 제3자 또는 독립 패키지가 안정된 계약을 통해 기능을 제공하는 **외부 확장 단위**로 정의한다.

> Plugin은 안정된 DXBOT 확장 계약을 통해 독립 패키지가 Capability Provider 또는 허용된 확장 기능을 제공하는 배포 단위다.

## 2. Architecture Classification

- Classification: `Plugin`
- Canonical Owner: Plugin package는 자체 기능 의미를 소유하되 DXBOT Core Domain 의미는 소유하지 않는다.
- Lifecycle Owner: Plugin Manager
- Persistent State Owner: manifest가 명시한 plugin-owned namespace
- 제거 가능: 필수. disable/upgrade/rollback/uninstall 계약 필요

## 3. Provider와의 관계

- Plugin은 하나 이상의 Capability Provider를 등록할 수 있다.
- built-in Provider는 Plugin일 필요가 없다.
- Plugin은 Core Domain type이나 storage table을 직접 mutate하지 않는다.
- Plugin이 Domain 변경을 요청해야 한다면 안정 Command/API를 사용한다.
- 내부 Rust trait object를 Plugin ABI로 직접 노출하지 않는다.

## 4. Plugin Manifest

최소 논리 필드:

- PluginId / name / version
- package format version
- required DXBOT API/version range
- provides capabilities + versions
- requires capabilities + versions
- requested permissions/grants
- config schema + version
- plugin data schema + version
- lifecycle hooks supported
- resource budget hints/hard requirements
- entrypoint/host mode
- package digest/signature metadata
- dependencies/conflicts
- upgrade/migration compatibility
- uninstall data policy

Manifest는 load 전에 schema와 signature/digest policy를 검증한다.

## 5. Lifecycle

```text
Package Available
   ↓ install
Installed
   ↓ validate
Validated
   ↓ enable
Enabled
   ↓ disable request
Draining
   ↓ quiesced
Disabled
   ├─ upgrade → validate → enable
   ├─ rollback → validate → enable
   └─ uninstall → Removed
```

실패 단계는 부분 등록을 남기지 않는다. enable이 완료되기 전 Capability Registry에 active Provider로 노출하지 않는다.

## 6. Install / Validate

- package digest 확인
- manifest/API compatibility
- dependency/capability resolution
- permission review
- config schema validation
- migration dry-run
- resource/host capability 확인
- conformance/certification metadata 확인

설치가 곧 enable을 의미하지 않는다.

## 7. Enable / Disable

Enable:
1. isolated host/resource 준비
2. plugin config snapshot pin
3. requested permission을 정책이 grant
4. provider/hook/schema registration candidate 생성
5. health/conformance probe
6. immutable registry generation에 publish

Disable:
1. 신규 selection/command 차단
2. in-flight work drain 또는 policy cancel
3. subscription/hook 차단
4. provider unregister
5. process/resource quiesce
6. durable lifecycle marker

## 8. Upgrade / Rollback

- old/new version을 동시에 mutable shared state에 연결하지 않는다.
- config/data migration은 versioned transaction/marker를 사용한다.
- 신규 Execution부터 새 version을 사용한다.
- rollback 가능 여부를 migration 전에 판정한다.
- irreversible migration은 backup/export와 operator approval이 필요하다.
- 진행 중 Execution은 시작 시점 Plugin/Provider version을 pin한다.

## 9. Uninstall과 Data Ownership

Plugin data는 Core Domain data와 별도 ownership을 가져야 한다. uninstall policy는 최소 다음 중 하나를 명시한다.

- `retain`: package만 제거하고 데이터 보존
- `export-and-remove`: export artifact 생성 후 제거
- `purge`: 승인 후 plugin-owned data 삭제
- `migrate`: successor plugin/schema로 변환
- `block`: 다른 active feature가 의존하면 uninstall 거부

Plugin 제거를 이유로 Bot Identity, Canonical Memory, Task Journal을 임의 삭제하지 않는다. Plugin이 생성한 Domain facts는 provenance를 남기고 일반 Domain retention을 따른다.

## 10. Isolation과 권한

Plugin은 기본적으로 ambient authority를 갖지 않는다.

- filesystem/network/process/secret/control API 접근은 manifest + policy grant 필요
- host memory/internal pointer 접근을 공개 계약으로 제공하지 않음
- resource CPU/memory/process/channel budget
- stdout/stderr/log size cap
- secret 원문 저장 금지
- Plugin failure/panic/crash가 Runtime 전체를 종료시키지 않도록 host boundary에서 격리

구체 host 방식(process/WASM/native 등)은 별도 ADR에서 선택하며 **in-process native를 무조건 기본값으로 가정하지 않는다**.

## 11. Public API / SDK

- stable wire/schema contract
- generated or versioned SDK 가능
- backwards compatibility range
- unknown field/enum handling
- cancellation/deadline/idempotency
- bounded payload/stream
- capability negotiation
- explicit error codes

내부 crate layout이나 trait signature가 바뀌어도 compatible Plugin이 재컴파일 없이 동작할 수 있는 수준을 장기 목표로 한다.

## 12. Failure Isolation

- startup failure: Plugin만 unavailable
- protocol violation: quarantine
- resource overrun: throttle/terminate
- repeated crash: circuit/open + disable
- migration failure: old version 유지 또는 rollback
- audit/security failure: sensitive action fail-closed
- uninstall cleanup failure: `RemovalPending` 상태와 operator action

Plugin failure가 Canonical Journal을 임의 수정해서 복구하는 것을 금지한다.

## 13. Observability / Audit

- install/enable/disable/upgrade/rollback/uninstall
- package digest/version
- permissions granted/denied
- provider registration/selection
- resource usage
- crash/protocol violations
- data migration/purge
를 correlation과 actor로 감사한다.

## 14. 구현 우선순위

- **P1:** manifest schema, Plugin Manager lifecycle, reference Plugin, permission/data ownership, disable/uninstall semantics
- **P2:** dynamic registry, isolated host, signed package, public SDK, certification suite
- **P3:** remote registry/discovery, third-party ecosystem, marketplace 후보

## 15. 검증 기준

- Plugin install 실패 후 registry/resource가 남지 않는다.
- disable 후 신규 Provider selection이 해당 Plugin을 선택하지 않는다.
- in-flight Execution은 version pinning을 유지한다.
- uninstall 후 Core Domain build/schema가 변하지 않는다.
- plugin-owned data 정책이 retain/export/purge/migrate 중 명시적으로 적용된다.
- 내부 Rust trait 변경이 public Plugin wire compatibility를 자동으로 깨뜨리지 않는다.
