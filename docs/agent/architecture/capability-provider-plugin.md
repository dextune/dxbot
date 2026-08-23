# Capability, Provider and Plugin Guide

## Applies When

새 Optional Capability, Provider, Provider SDK, Plugin을 설계·구현할 때 적용한다.

## Core Rules

Capability는 trait 이름만으로 완료되지 않는다. 적용 가능한 Request/Response/Stream Event, Stable Error, Config, Metadata/negotiation, cancellation/deadline, resource, idempotency/side-effect, version/compatibility, Conformance를 닫는다.

production Consumer는 `Consumer → Common Provider Host → Stable Provider SPI → Provider` 경로를 사용한다. Plugin Host는 package/isolation boundary이고 Provider Host는 capability invocation enforcement boundary다.

Provider lifecycle 의미는 Common이 소유하며 Ready 전 selection, Draining 후 신규 activity, stale generation late write를 금지한다.

## Decision Rules

새 Provider-specific optional feature가 Consumer downcast/branch를 요구하면 Stable Capability semantic 자체가 다른지 검토한다. 다르면 Tier A Capability/version 변경을 고려하고 concrete Provider branch로 숨기지 않는다.

## Forbidden Patterns

raw Domain Store/Scheduler/Registry/global Secret Store 전달, unrestricted filesystem/network, direct production SPI call, Reference Provider의 production fallback을 금지한다.

## Verification

Conformance, Common Host path, lifecycle start/degrade/drain/stop, unavailable/removal, multiple selection/generation, dependency firewall, bounded output/cancel을 검증한다.

## Related Guides

[extension-removal.md](extension-removal.md), [../runtime/security-authority.md](../runtime/security-authority.md), [../quality/testing-strategy.md](../quality/testing-strategy.md)
