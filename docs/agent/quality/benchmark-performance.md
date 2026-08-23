# Benchmark and Performance Guide

## Applies When

성능 최적화, memory/cache/channel tuning, abstraction 비용을 평가할 때 적용한다.

## Core Rules

성능 변경은 latency p50/p95/p99, throughput, allocations/op, bytes/op, retained heap/RSS, queue fill, cache hit/eviction/retained bytes, Provider time과 Host/Runtime overhead를 구분해 측정한다.

최적화 순서는 ownership/data flow → duplicate work/serialization 제거 → bounded resource → cache locality/batch I/O → profile/benchmark → specialized optimization이다.

correctness/security/recovery/Host enforcement를 약화해 성능을 얻지 않는다.

## Decision Rules

구체 자료구조, cache size, batch/timeout 숫자는 benchmark/workload evidence가 있을 때 선택한다. microbenchmark 개선이 retained memory나 tail latency를 악화시키지 않는지 함께 본다.

## Forbidden Patterns

측정 없는 lock-free/unsafe/object pool, 평균값만 보고 tail 무시, allocation 감소를 위해 global retained cache 증가, external Provider latency를 Runtime overhead로 오인하는 것을 금지한다.

## Verification

대표 workload와 worst-case bounded workload, soak에서 선형 resource 증가가 없는지 확인한다.
