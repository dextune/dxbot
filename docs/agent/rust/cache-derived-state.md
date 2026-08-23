# Cache and Derived State Guide

## Applies When

cache, memoization, projection, index, snapshot-derived lookup을 추가·변경할 때 적용한다.

## Core Rules

Cache/Projection/Index는 Canonical Owner가 아니다. owner, canonical source, key/version, max entries/bytes, invalidation, stale 허용 범위, eviction/TTL, rebuild path, metrics를 정의한다.

cache eviction으로 제품 상태가 손실되면 안 된다. 동일 canonical data를 여러 cache가 서로 다른 원본처럼 mutation하지 않는다.

## Decision Rules

cache 도입 전 `재계산/조회 비용이 실제 병목인가`, `stale 허용 가능성`, `invalidation source`, `retained memory 비용`을 확인한다. 정확성에 필요한 정보는 cache가 아니라 Canonical State에 둔다.

## Forbidden Patterns

unbounded cache, key에 revision/generation 없는 mutable result cache, invalidation 없는 global memoization, cache miss를 silent semantic fallback으로 사용하는 것을 금지한다.

## Verification

eviction/rebuild, stale generation, concurrent update/invalidation, retained bytes, hit ratio와 canonical recovery를 검사한다.

## Related Guides

[../architecture/canonical-ownership.md](../architecture/canonical-ownership.md), [memory-allocation.md](memory-allocation.md)
