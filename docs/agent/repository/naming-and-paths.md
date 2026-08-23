# Naming and Paths Guide

## Applies When

새 file/directory/crate/module/type의 이름을 만들거나 기존 이름을 변경할 때 적용한다.

## Core Rules

- 사용자 정의 Repository file/directory: lowercase kebab-case.
- Rust source module `*.rs`: snake_case.
- Rust type/trait/enum/variant: Rust 생태계의 PascalCase 관례.
- function/method/local/module: snake_case.
- ID/Ref/Key/Handle/Snapshot은 의미가 다르면 이름에서도 구분한다.
- 약어는 프로젝트 용어로 안정된 경우에만 사용하고 같은 개념에 여러 축약형을 만들지 않는다.
- 외부 도구가 강제한 고정 이름은 명시적 예외다.

## Decision Rules

이름은 구현 기술보다 소유 개념과 수명을 드러내야 한다. `Manager`, `Util`, `Helper`, `Data`, `Info`처럼 책임이 불명확한 이름은 더 구체적인 Owner/역할로 바꾼다.

## Forbidden Patterns

대소문자만 다른 path, 공백 중심 naming, invisible/confusable Unicode, `common/utils/helpers/misc` dumping ground, version 숫자를 장기 일반 규칙 이름에 붙이는 것을 금지한다.

## Verification

rename 후 import/link/config/schema/fixture/CI reference와 case-sensitive filesystem 영향을 확인한다.

## Related Guides

[repository-layout.md](repository-layout.md), [../rust/type-design.md](../rust/type-design.md)
