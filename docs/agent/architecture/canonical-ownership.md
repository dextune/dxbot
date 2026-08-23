# Canonical Ownership Guide

## Applies When

상태, 정책, lifecycle, index, cache, projection, persistent record를 추가하거나 이동할 때 적용한다.

## Core Rules

모든 사실과 정책은 하나의 Canonical Owner를 가진다. 새 상태를 만들기 전에 생성자, mutation authority, persistence owner, retention/removal owner, crash recovery source, derived invalidation source를 답할 수 있어야 한다.

Projection/Index/Cache는 Canonical State가 아니며 재생성 또는 재조회 가능해야 한다. 같은 사실을 DB, cache, UI, Provider가 각각 원본처럼 소유하지 않는다.

## Decision Rules

`누가 제품 의미를 결정하는가?`가 Owner 선정의 첫 기준이다. 저장 위치나 호출 빈도가 Owner를 결정하지 않는다. cross-aggregate coordination은 각 aggregate state를 복제하지 않고 reference/progress만 소유한다.

## Forbidden Patterns

두 Owner의 동시 authoritative write, cache에만 존재하는 canonical 정보, Provider/UI가 Domain identity/state 소유, duplicated policy/default/state machine을 금지한다.

## Verification

write path 하나, recovery source 하나, derived rebuild path, stale invalidation, owner 없는 field가 없는지 추적한다.

## Related Guides

[dependency-boundaries.md](dependency-boundaries.md), [../rust/cache-derived-state.md](../rust/cache-derived-state.md), [../runtime/persistence-recovery.md](../runtime/persistence-recovery.md)
