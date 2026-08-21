---
title: "Plugin 시스템"
document_id: "DXB-ARC-016"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P1/P2"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-012", "DXB-ARC-017", "DXB-GOV-003"]
---

# Plugin 시스템

## 1. 목적

Plugin을 DXBOT 내부 모듈이나 Provider의 동의어로 사용하지 않고, 제3자 또는 독립 패키지가 안정된 계약을 통해 기능을 제공하는 외부 확장 단위로 정의한다.

> Plugin은 안정된 DXBOT 확장 계약을 통해 독립 패키지가 Capability Provider 또는 허용된 확장 기능을 제공하는 배포 단위다.

## 2. Architecture Classification

- Classification: `Plugin`
- Lifecycle Owner: Plugin Manager
- Persistent State Owner: manifest가 명시한 plugin-owned namespace
- 제거 가능: 필수. disable/upgrade/rollback/uninstall 계약 필요

## 3. Provider Host와 Plugin Host의 경계

두 Host는 같은 개념이 아니다.

| 경계 | 책임 |
|---|---|
| Plugin Manager/Host | package install/validate/enable/disable/upgrade/uninstall, isolation, plugin-owned data |
| Provider Host | Capability invocation의 lifecycle/selection/security/resource/deadline/error/telemetry enforcement |

Plugin이 Capability Provider를 제공하면:

1. Plugin Manager가 package/permission/config를 검증한다.
2. Plugin Host가 isolation/resource boundary를 준비한다.
3. Provider candidate가 Standard Provider Lifecycle validation을 통과한다.
4. `Ready` candidate만 Registry generation에 publish한다.
5. 실제 Capability call은 built-in Provider와 동일하게 Provider Host를 거친다.

Plugin이 별도 direct invocation path를 만들어 Provider Host를 우회하지 않는다.

## 4. Provider와의 관계

- Plugin은 하나 이상의 Capability Provider를 등록할 수 있다.
- built-in Provider는 Plugin일 필요가 없다.
- Plugin은 Core Domain type이나 storage table을 직접 mutate하지 않는다.
- Plugin이 Domain 변경을 요청해야 한다면 안정 Command/API를 사용한다.
- 내부 Rust trait object를 Plugin ABI로 직접 노출하지 않는다.
- Plugin Provider도 Provider SDK/Conformance/Dependency Firewall 적용 대상이다.

## 5. Plugin Manifest

최소 논리 필드:
- PluginId/name/version/package format
- required DXBOT public API range
- provides/requires capability + contract versions
- requested permissions
- config/data schema version
- lifecycle hooks
- resource requirements
- entrypoint/host mode
- digest/signature metadata
- dependencies/conflicts
- upgrade/migration/uninstall policy

## 6. Lifecycle

```text
Package Available → Installed → Validated → Enabled → Draining → Disabled → Removed
```

Plugin package lifecycle와 그 안의 Provider lifecycle을 원자적으로 동일시하지 않는다. enable candidate가 준비되어도 각 Provider가 `Ready`가 되기 전 Registry에 노출하지 않는다.

## 7. Enable / Disable

Enable:
1. isolated host/resource 준비
2. plugin config snapshot pin
3. permission grant
4. Provider/Hook candidate 생성
5. Provider lifecycle validate/start + conformance/health probe
6. immutable registry generation publish

Disable:
1. 신규 Plugin command/Provider selection 차단
2. 제공 Provider를 Draining으로 전환
3. Common activity 기준 in-flight quiescence
4. provider unregister/stop
5. subscription/hook 차단
6. Plugin host resource quiesce
7. durable lifecycle marker

## 8. Upgrade / Rollback

- 진행 중 Execution은 시작 시점 Plugin/Provider version을 pin한다.
- old/new mutable shared state를 동시에 사용하지 않는다.
- config/data migration은 versioned transaction/marker를 사용한다.
- 신규 Execution부터 새 Provider generation을 사용한다.
- irreversible migration은 backup/export와 operator approval이 필요하다.

## 9. Uninstall과 Data Ownership

Plugin data policy는 `retain | export-and-remove | purge | migrate | block` 중 명시한다. Plugin 제거를 이유로 Bot Identity, Canonical Memory, Task Journal을 임의 삭제하지 않는다.

## 10. Isolation과 권한

Plugin은 ambient authority를 갖지 않는다. filesystem/network/process/secret/control 접근은 manifest+policy grant가 필요하다. Plugin Provider가 받는 실제 call context도 Minimum Provider Surface를 따른다.

구체 host 방식(process/WASM/native)은 별도 ADR에서 선택하며 in-process native를 무조건 기본값으로 가정하지 않는다.

## 11. Public API / SDK

Plugin public SDK/Wire와 Provider SDK를 구분한다. Plugin SDK는 package/public wire 호환성을, Provider SDK는 DXBOT 내부 Capability implementation surface를 담당한다. 둘 다 internal Runtime mutable API를 공개하지 않는다.

## 12. Failure Isolation

startup/protocol/resource/crash/migration/security failure는 Plugin scope에서 quarantine/disable/rollback하며 Core Runtime을 임의 종료시키지 않는다. Plugin Provider 오류는 Provider Host에서 stable outcome으로 normalize한다.

## 13. Observability / Audit

install/enable/disable/upgrade/uninstall과 함께 제공 Provider의 lifecycle/registration/selection/drain도 상관관계로 기록한다.

## 14. 구현 우선순위

- P1: manifest, Plugin Manager lifecycle, reference Plugin, permission/data ownership, Provider Host 연계, disable/uninstall
- P2: isolated host, signed package, public Plugin SDK, certification suite
- P3: remote registry/discovery, third-party ecosystem

## 15. 검증 기준

- Plugin install 실패 후 registry/resource가 남지 않는다.
- disable 후 신규 Provider selection이 해당 Plugin Provider를 선택하지 않는다.
- Plugin Provider가 Provider Host를 우회하지 않는다.
- in-flight Execution은 version pinning을 유지한다.
- uninstall 후 Core Domain build/schema가 변하지 않는다.
- internal Rust trait 변경이 public Plugin wire compatibility를 자동으로 깨뜨리지 않는다.
