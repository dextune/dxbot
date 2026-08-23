# Crate and Module Layout Guide

## Applies When

새 Rust crate/module을 만들거나 기존 책임을 분리·병합할 때 적용한다.

## Core Rules

- crate는 독립 책임, dependency boundary, compilation/lifecycle 가치가 있을 때만 만든다.
- module은 한 Owner 안의 응집된 하위 책임을 표현한다.
- Domain은 Infrastructure/UI/concrete Provider를 의존하지 않는다.
- Interface는 Store/Runtime internal mutation을 직접 호출하지 않는다.
- Provider A가 Provider B를 직접 의존하지 않는다.
- public DTO, Domain type, Persistence schema, Wire schema는 편의상 하나의 struct로 합치지 않는다.

## Decision Rules

새 crate가 `독립 배포/제거/의존성 격리/Stable Contract` 중 실질적 경계를 제공하지 못하면 기존 crate의 module을 우선한다. 반대로 concrete Provider/Plugin처럼 제거와 dependency 격리가 필요한 경우 Core module에 숨기지 않는다.

## Forbidden Patterns

crate-per-concept, mega-crate의 무제한 internal coupling, cyclic dependency, private internals re-export로 firewall 우회, 테스트 편의를 위한 무의미한 trait/crate 분리를 금지한다.

## Verification

`cargo metadata` 기반 dependency 방향, 제거 build, forbidden dependency, public surface를 검사한다.

## Related Guides

[../architecture/dependency-boundaries.md](../architecture/dependency-boundaries.md), [../architecture/extension-removal.md](../architecture/extension-removal.md)

## Canonical Contracts

실제 workspace 구성이 필요하면 활성 [docs/plan](../../plan/)의 Rust workspace/module Owner를 확인한다.
