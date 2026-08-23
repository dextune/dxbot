# Common and Extension Guide

## Applies When

공통 helper/framework를 만들거나 validation/retry/security/resource/telemetry 로직을 여러 구현에서 공유하려 할 때 적용한다.

## Core Rules

공통화는 코드 모양이 아니라 **같은 의미와 같은 invariant**를 공유할 때만 한다. Common Framework는 lifecycle, generation/selection, permission/approval, resource admission/accounting, deadline/cancellation, semantic retry, side-effect guard, telemetry/audit, compatibility, recovery, Conformance 같은 cross-cutting correctness를 소유한다.

Extension은 Provider-specific config, external SDK adapter, capability logic, DTO/error mapping에 집중한다.

## Decision Rules

공통화 후보는 `같은 의미인가 → 같은 invariant인가 → 같은 Owner인가 → 제거/교체에도 공통인가`를 모두 만족해야 한다. 단순 copy-paste 감소만으로 Common으로 승격하지 않는다.

## Forbidden Patterns

`common/utils/helpers/misc` dumping ground, Provider마다 global retry/security/scheduler/persistence 재구현, Extension 편의를 위한 provider-specific Common Contract, mega-context를 금지한다.

## Verification

중복 policy/state/default, Host bypass, Extension 제거 시 Common 잔여 dependency, Provider-specific type의 Common surface 노출을 검사한다.

## Related Guides

[capability-provider-plugin.md](capability-provider-plugin.md), [dependency-boundaries.md](dependency-boundaries.md)
