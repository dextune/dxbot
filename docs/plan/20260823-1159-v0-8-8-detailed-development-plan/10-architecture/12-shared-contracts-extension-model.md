---
title: "공유 계약과 Extension 모델"
document_id: "DXB-ARC-012"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-ARC-010", "DXB-ARC-011"]
---
# 공유 계약과 Extension 모델

## 1. 계약과 Owner

| 계약 | Consumer | Canonical document | Logical owner | 금지 |
|---|---|---|---|---|
| Application Contract | CLI/Control/향후 Interface | `DXB-IFC-040` | `application-contract` | Provider-specific type, Domain aggregate |
| P0 Harness Capability | Application/Runtime | `DXB-ARC-013` | `provider-host` | public selector/receipt schema 소유 |
| Provider SPI | Provider Host | `DXB-ARC-017` | `provider-host` | Runtime service locator |
| Plugin API | Plugin Host | `DXB-ARC-016` | `plugin-host` | 내부 Rust trait를 안정 ABI로 간주 |

`Application/IFC-040`, `Common owner` 같은 복수·익명 Owner 표현을 사용하지 않는다.

## 2. Common enforcement

Provider Host는 lifecycle/generation, selection, permission, resource admission, deadline/cancel, side-effect guard, telemetry/audit, compatibility, recovery, conformance를 소유한다. Provider는 typed config, external adapter, capability logic, DTO/error mapping에 집중한다.

## 3. Schema 방화벽

- public Application DTO는 Provider schema에서 생성하지 않는다.
- Provider availability는 safe discovery DTO로 mapping한다.
- Provider Registry object, credential, capability-internal session은 public schema에 노출하지 않는다.
- Provider가 Application selector/receipt/cursor/error/exit owner가 되지 않는다.

## 4. 제거 가능성

Provider/Plugin/Interface는 stop-new-use, drain, detach, config/data/reference cleanup, unsupported surface, removal build를 가진다. 제거 문제를 Reference Provider나 다른 Provider의 silent fallback으로 숨기지 않는다.
