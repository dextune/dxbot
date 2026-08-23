# Dependencies, Features and Unsafe Guide

## Applies When

crate dependency, feature flag, optional compilation path, `unsafe`를 추가·변경할 때 적용한다.

## Core Rules

새 dependency는 기존 dependency/작은 owned code와 비교하고 maintenance, security/license, transitive graph, compile/binary/runtime cost, platform/MSRV 영향을 검토한다. Provider-specific dependency는 Provider boundary에 격리한다.

feature flag는 semantic owner와 default, dependency implication, disabled behavior, persisted/config compatibility, removal path를 가져야 한다. disabled feature가 silent fallback을 만들면 안 된다.

`unsafe`는 격리 module, safety contract, caller obligation, safe wrapper, 검증, 필요성/benchmark 근거를 요구한다.

## Forbidden Patterns

편의 dependency로 public Domain API 오염, feature 조합에 따라 invariant 변화가 숨겨지는 구조, undocumented unsafe block, SDK를 통한 forbidden dependency 우회를 금지한다.

## Verification

minimal/default/all-features build, feature disable/removal, dependency graph/license/security, unsafe caller obligation test를 확인한다.
