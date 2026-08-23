# Concurrency and Async Guide

## Applies When

spawn, async task, lock, atomic, cancellation, race, blocking I/O를 추가·변경할 때 적용한다.

## Core Rules

모든 spawned task는 supervisor/owner, cancellation path, shutdown/join 의미를 가진다. fire-and-forget을 만들지 않는다. cancellation 요청과 실제 종료를 구분한다.

async lock guard를 잡은 채 장시간 I/O를 await하지 않는다. blocking work는 bounded blocking execution 경계로 격리한다. race correctness는 revision/fencing/idempotency/generation/activity guard처럼 의미 있는 protocol로 해결한다.

## Decision Rules

동기화 선택은 먼저 ownership을 단순화하고 message passing/immutable snapshot이 가능한지 검토한 뒤 lock을 사용한다. atomic/lock-free는 contention/profile과 memory ordering 근거가 있을 때만 사용한다.

## Forbidden Patterns

unowned spawn, sleep으로 race 보정, global mutex로 unrelated work 직렬화, lock 내부 external I/O, shutdown 시 detached child를 금지한다.

## Verification

race winner, cancel/complete, shutdown-under-load, late callback, stale generation write, permit/activity leak을 deterministic barrier/fake clock으로 검증한다.

## Related Guides

[channels-backpressure.md](channels-backpressure.md), [../quality/concurrency-fault-testing.md](../quality/concurrency-fault-testing.md)
