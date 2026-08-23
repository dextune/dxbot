# Extension Removal and Replacement Guide

## Applies When

Capability, Provider, Plugin, Interface, feature flag를 disable/remove/replace/upgrade할 때 적용한다.

## Core Rules

추가 계약과 제거 계약을 함께 설계한다. 제거는 신규 사용 차단, in-flight drain/termination, config/reference 정리, persisted data 처리, cache/index stale reference 제거, dependency/feature 제거, unrelated Core restore, unsupported 상태 표면까지 포함한다.

## Decision Rules

persisted reference가 남으면 `retain | migrate | export | purge | block removal` 중 의미를 Owner가 결정해야 한다. silent fallback으로 제거 문제를 숨기지 않는다.

## Forbidden Patterns

disabled Provider의 자동 다른 Provider 선택, stale config 무시, data orphan, generation 없는 hot swap, Plugin uninstall 후 resource/registry leak을 금지한다.

## Verification

remove-build, in-flight drain, restart, stale config/data, rollback/upgrade, registry/resource leak, no silent fallback을 검사한다.

## Related Guides

[capability-provider-plugin.md](capability-provider-plugin.md), [../runtime/lifecycle-shutdown.md](../runtime/lifecycle-shutdown.md)
