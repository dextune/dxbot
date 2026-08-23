# Testing Strategy Guide

## Applies When

기능/계약 변경의 검증 수준을 정하거나 새 test suite를 만들 때 적용한다.

## Core Rules

테스트는 구현 세부보다 contract와 invariant를 검증한다. 기본 우선순위는 deterministic unit/state → property/model → component/contract → Provider Host/Conformance → integration → fault/recovery/concurrency → performance/soak → real Provider canary/E2E다.

실제 외부 모델의 비결정성을 Core correctness acceptance 기준으로 사용하지 않는다. system clock/random/timing은 fake/injectable source와 deterministic barrier를 우선한다.

## Decision Rules

모든 변경에 모든 test layer를 강제하지 않는다. 변경한 Owner와 failure semantics에 가장 가까운 layer에서 최소 deterministic proof를 만들고 cross-layer boundary가 바뀌면 integration evidence를 추가한다.

## Forbidden Patterns

sleep 길이로 race test, happy path만 존재, direct Provider test만 있고 Host path 없음, flaky external E2E를 invariant proof로 사용하는 것을 금지한다.

## Verification

변경 classification/Tier에 필요한 state, error, cancel, retry, security, resource, removal, migration scenario가 누락되지 않았는지 확인한다.
