---
title: "Plugin 시스템"
document_id: "DXB-ARC-016"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P1"
last_updated: "2026-08-22"
depends_on: ["DXB-ARC-012", "DXB-ARC-015"]
---
# Plugin 시스템

Plugin은 Provider나 내부 module의 동의어가 아니라 stable package/lifecycle/permission/data boundary다.

Plugin manifest는 stable ID/version/API range/capability/permission/config/resource/data lifecycle을 가진다. Plugin Host는 package isolation, Provider Host는 capability invocation enforcement를 소유한다.

`Discovered → Validated → Enabled → Draining → Disabled → Uninstalled`와 incompatible/quarantined 상태를 구분한다. uninstall은 persisted data의 retain/export/migrate/purge/block 결정을 가진다.

Plugin ecosystem, install/upgrade/rollback/uninstall CLI는 P0가 아니다. 본 문서는 향후 Extension이 Core 경계를 오염시키지 않도록 보존하는 P1 architecture contract이며 P0 CLI Gate에 포함하지 않는다.

검증 기준:

- Plugin이 Domain Store/Registry mutator를 직접 획득하지 않는다.
- Plugin Provider가 Provider Host/Conformance를 우회하지 않는다.
- disable/uninstall 뒤 activity/resource/reference leak가 없다.
