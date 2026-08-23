# Dependency Boundaries Guide

## Applies When

새 dependency, Port/Adapter, Host boundary, cross-layer call을 추가할 때 적용한다.

## Core Rules

의존성은 제품 의미와 policy를 소유한 안쪽 경계를 향한다. Domain은 DB/HTTP/UI/concrete Provider를 모른다. Provider는 Domain/Application/Runtime/Storage/UI internals를 직접 의존하지 않는다. Interface는 stable Application/Control surface를 사용한다.

Provider의 기본 허용 dependency는 해당 Capability Contract, 승인된 Provider SDK, 공개 Kernel/value type, 실제 integration에 필요한 external SDK다.

## Decision Rules

cross-layer 편의 호출이 필요해 보이면 `기존 Port 존재? → stable capability로 일반화 가능한가? → Tier A contract 변경인가?` 순서로 판단한다. service locator나 RuntimeContext 확장으로 우회하지 않는다.

## Forbidden Patterns

Provider↔Provider 직접 의존, Domain→Infrastructure, UI→Store mutation, SDK가 private internal을 re-export, generic `get<T>()` service lookup을 금지한다.

## Verification

cargo dependency graph, Host bypass, forbidden import, removal build, test-only direct SPI와 production path 분리를 검사한다.

## Related Guides

[common-extension.md](common-extension.md), [../repository/crate-module-layout.md](../repository/crate-module-layout.md)
