# Error, Retry and Side Effect Guide

## Applies When

error taxonomy, retry, idempotency, external side effect, reconciliation을 설계·수정할 때 적용한다.

## Core Rules

validation, authorization, conflict, resource, provider transient/permanent, cancellation, timeout, corruption, invariant, security incident를 의미에 맞게 구분한다. Provider error는 stable typed error/outcome으로 mapping한다.

semantic retry는 error class, idempotency, side-effect classification, deadline, budget을 함께 판단하는 Common 책임이다. Provider-local transport retry는 Stable Contract가 허용하고 observable semantic을 바꾸지 않는 범위에만 허용한다.

외부 결과가 불명확하면 `Unknown/Reconciliation Required`로 보존하고 blind retry하지 않는다.

## Forbidden Patterns

문자열 parsing으로 retry/security 판단, 비멱등 effect 자동 retry, timeout을 cancellation 완료로 오인, unknown outcome을 transient error로 축약하는 것을 금지한다.

## Verification

duplicate delivery, response loss, timeout-before/after-effect, restart/reconcile, exhausted budget, non-idempotent retry negative fixture를 검증한다.
