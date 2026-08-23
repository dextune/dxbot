# Rust Type Design Guide

## Applies When

public/internal Rust type, trait, enum, DTO, identifier를 설계할 때 적용한다.

## Core Rules

- 의미가 다른 식별자는 Newtype으로 분리한다.
- bytes/tokens/duration/revision/generation 같은 단위를 타입으로 드러낸다.
- 모호한 boolean 조합보다 enum/value object를 우선한다.
- 불가능한 상태를 표현하기 어렵게 한다.
- 상태 전이는 public field mutation보다 명시적 command/method를 사용한다.
- external DTO, Domain type, Persistence/Wire schema를 분리한다.
- public surface는 최소화한다.

## Decision Rules

닫힌 Domain 선택지는 enum을 우선하고, trait은 실제 교체 지점/Port가 있을 때 사용한다. Generic과 dynamic dispatch는 API 안정성, monomorphization/code size, runtime cost를 보고 선택한다.

## Forbidden Patterns

테스트 편의를 위한 trait 남발, mega-interface, primitive obsession으로 서로 다른 ID 혼용, Persistence derive를 public wire contract로 고정하는 것을 금지한다.

## Verification

invalid state construction, serialization boundary, public field leakage, provider-specific type contamination을 검사한다.
