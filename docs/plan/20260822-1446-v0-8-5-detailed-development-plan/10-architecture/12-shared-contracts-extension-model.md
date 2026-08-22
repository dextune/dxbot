---
title: "공유 계약과 Extension 모델"
document_id: "DXB-ARC-012"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-ARC-011"]
---

# 공유 계약과 Extension 모델

## 1. 목적

Application Contract, Capability Contract, Provider SPI, Plugin API를 서로 다른 책임으로 유지한다.

## 2. 계약 구분

| 계약 | Consumer | Owner | 금지 |
|---|---|---|---|
| Application Contract | CLI/향후 Interface | Application/IFC-040 | Provider-specific type |
| Capability Contract | Application/Runtime | Common Capability owner | Domain aggregate 노출 |
| Provider SPI | Provider Host | Capability owner | Runtime service locator |
| Plugin API | Plugin Host | Plugin framework | 내부 Rust trait를 안정 ABI로 가정 |

## 3. Common enforcement

Provider Host가 lifecycle, selection, permission, resource, deadline/cancel, side-effect guard, telemetry, compatibility, recovery, conformance를 소유한다. Provider는 external adapter와 capability logic에 집중한다.

## 4. Schema 오염 방지

public Application DTO를 Provider schema에서 생성하지 않는다. capability discovery는 safe summary이며 Provider Registry object나 authority를 노출하지 않는다.

## 5. 제거 가능성

Provider/Plugin/Interface는 stop-new-use, drain, detach, config/data/reference cleanup, unsupported 상태, removal build를 가진다. 제거 문제를 silent fallback으로 숨기지 않는다.

## 6. 검증 기준

- Application Contract와 Provider SPI generic envelope 병합 0
- Provider가 public selector/receipt/cursor owner가 되는 경로 0
- concrete Provider dependency의 Domain/CLI 침투 0
