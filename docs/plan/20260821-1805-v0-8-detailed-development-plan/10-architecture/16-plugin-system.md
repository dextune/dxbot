---
title: "Plugin 시스템"
document_id: "DXB-ARC-016"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-012", "DXB-ARC-015"]
---

# Plugin 시스템

## 1. 목적

Plugin을 Provider나 내부 module의 동의어로 사용하지 않고 stable package/lifecycle/permission/data boundary로 정의한다.

## 2. 책임

Plugin manifest는 stable ID/version/API range/capability/permission/config/resource/data lifecycle을 가진다. Plugin Host는 package isolation, Provider Host는 capability invocation enforcement를 소유한다.

## 3. Lifecycle

`Discovered → Validated → Enabled → Draining → Disabled → Uninstalled`와 incompatible/quarantined 상태를 구분한다. uninstall은 persisted data의 retain/export/migrate/purge/block 결정을 가진다.

## 4. v0.8 scope

Plugin ecosystem 전체와 install/upgrade/rollback CLI는 P0가 아니다. 기존 Plugin 경계를 보존하되 CLI P0 Command Matrix에 억지로 포함하지 않는다.

## 5. 검증 기준

- Plugin이 Domain Store/Registry mutator를 직접 획득하지 않음
- Plugin Provider가 Provider Host/Conformance 우회 0
- disable/uninstall 후 activity/resource/reference leak 0
