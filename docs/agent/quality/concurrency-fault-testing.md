# Concurrency and Fault Testing Guide

## Applies When

race, crash window, cancellation, timeout, restart, partial success, duplicate delivery가 영향을 받는 변경에 적용한다.

## Core Rules

race winner와 terminal invariant를 명시적으로 검증한다. timing은 sleep 대신 barrier/event probe/fake clock을 사용한다. commit/external side effect/outcome 기록/migration/provider generation swap/shutdown의 직전·직후 crash window를 검토한다.

실제 process crash 의미가 중요하면 mock만으로 대체하지 않는다. restart는 persisted state와 Stable Contract에서 판정한다.

## Forbidden Patterns

확률적 race test만 존재, unknown side effect를 transient retry로 기대, cancellation 요청 즉시 resource cleanup 완료로 가정, crash 후 in-memory state를 복구 source로 사용하는 것을 금지한다.

## Verification

cancel/complete, drain/new-call, stop/late-callback, stale revision/generation, duplicate delivery, partial commit, disk-full, shutdown-under-load를 포함한다.
