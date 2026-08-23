# Ownership and Borrowing Guide

## Applies When

API가 데이터를 borrow/own/share해야 하는지 결정하거나 lifetime/Arc/Box 선택이 필요한 경우 적용한다.

## Core Rules

기본 순서는 `borrow → owned value → shared immutable ownership`이다. callee가 소유권을 필요로 하지 않으면 borrow한다. 장기 보관, task 이동, retry/recovery boundary처럼 명확한 수명이 있을 때만 ownership을 이전한다.

`Arc<T>`는 다수 task/owner가 장기 공유하는 immutable 또는 명시적으로 동기화된 값에 사용한다. 단순 borrow 문제를 피하기 위해 Arc를 기본값으로 쓰지 않는다. `Rc<T>`는 Send/Sync가 필요 없는 단일 thread 경계에서만 고려한다. `Box<T>`는 size/indirection/trait object 같은 명시적 이유가 있을 때 사용한다.

## Decision Rules

clone이 필요하면 먼저 `왜 새 소유자가 필요한가`, `payload 크기`, `빈도`, `retained lifetime`을 설명할 수 있어야 한다. clone을 없애기 위해 lifetime 복잡도가 과도해지면 측정 가능한 비용과 유지보수성을 함께 비교한다.

## Forbidden Patterns

API 편의를 위한 무차별 `Arc<Mutex<_>>`, 호출 경계마다 clone, global shared mutable context, reference cycle을 금지한다.

## Verification

clone hot path, Arc strong-count 장기 유지, lock owner, task 종료 후 retained object를 profile/test에서 확인한다.

## Related Guides

[memory-allocation.md](memory-allocation.md), [concurrency-async.md](concurrency-async.md)
