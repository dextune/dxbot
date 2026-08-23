# Repository Layout Guide

## Applies When

파일·디렉터리·crate·test·fixture·generated artifact의 위치를 추가·이동·삭제할 때 적용한다.

## Does Not Apply When

기존 파일 내부의 의미만 바꾸고 layout이 변하지 않는 경우에는 해당 구현 Guide만 읽는다.

## Core Rules

- directory는 책임과 소유권을 반영한다.
- 독립 deployment/lifecycle/dependency 경계가 없으면 편의상 package/crate를 늘리지 않는다.
- concrete Provider/Plugin처럼 dependency와 lifecycle을 분리해야 하는 Extension은 Core에 섞지 않는다.
- generated output은 source-of-truth와 생성 방법을 명확히 하고 수동 수정하지 않는다.
- test/fixture는 검증 대상 owner와 가까운 위치를 우선하되 공통 Conformance fixture는 공통 Testkit Owner가 소유한다.
- 장기 Agent 규칙은 `docs/agent/`, 버전별 설계는 `docs/plan/`이 소유한다.

## Decision Rules

새 위치가 필요하면 `기존 Owner 위치 존재? → 독립 책임? → 독립 lifecycle/dependency?` 순서로 판단한다. 셋 중 하나라도 불명확하면 새 top-level directory를 만들지 않는다.

## Forbidden Patterns

`temp`, `new`, `final2`, `misc`, 소유권 없는 `shared` directory, generated/source 혼재, Provider-specific dependency를 Core directory에 배치하는 것을 금지한다.

## Verification

새 orphan path, broken link/import, stale config/fixture, generated drift, unrelated diff가 없는지 확인한다.

## Related Guides

[naming-and-paths.md](naming-and-paths.md), [crate-module-layout.md](crate-module-layout.md), [../architecture/canonical-ownership.md](../architecture/canonical-ownership.md)

## Canonical Contracts

제품별 실제 package/crate 구성이 필요하면 활성 [docs/plan](../../plan/)의 workspace/module Owner를 확인한다.
