# Memory and Allocation Guide

## Applies When

clone, String/Vec, serialization, buffering, large payload, retained memory, allocation 최적화를 다룰 때 적용한다.

## Core Rules

- hot path의 불필요한 allocation/copy/serialization intermediate를 피한다.
- 입력은 가능한 범위에서 `&str`, `&[T]` 같은 borrowed view를 사용하고 저장이 필요할 때 owned type으로 변환한다.
- `String`/`Vec<T>`는 mutation/growth ownership이 필요할 때 사용한다. immutable finalized collection은 slice/boxed slice/shared immutable representation을 검토한다.
- 큰 payload는 Artifact/reference/streaming을 우선 검토한다.
- collection capacity는 실제 상한/분포 근거 없이 과도하게 preallocate하지 않는다.
- allocation 감소가 retained RSS/heap 증가로 바뀌지 않는지 함께 측정한다.
- bounded resource는 item 수와 필요한 경우 byte 수를 모두 고려한다.

## Decision Rules

memory 최적화 순서는 `중복 데이터 제거 → 불필요한 serialization/copy 제거 → bounded lifetime/resource → data layout/cache locality → profile 기반 specialized optimization`이다.

## Forbidden Patterns

canonical payload를 여러 layer가 독립 복사, unbounded Vec/String growth, 성능 근거 없는 object pool, clone 금지 자체를 목표로 한 난해한 lifetime 설계를 금지한다.

## Verification

allocations/op, bytes/op, peak/retained heap/RSS, payload copy count, queue/cache retained bytes를 workload별로 측정한다.

## Related Guides

[cache-derived-state.md](cache-derived-state.md), [../quality/benchmark-performance.md](../quality/benchmark-performance.md)
