---
title: "Common Extension Framework"
document_id: "DXB-ARC-017"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-ARC-011", "DXB-ARC-012", "DXB-ARC-016"]
---
# Common Extension Framework

Optional Capability와 Provider가 공통 correctness를 재구현하지 않도록 Stable Contract, Provider Host, SDK, Testkit, Reference Provider, Dependency Firewall을 정의한다.

## Common 책임

lifecycle/generation, selection, permission, resource admission/accounting, deadline/cancel, semantic retry, side-effect reconciliation guard, telemetry/audit, compatibility, recovery, conformance를 `provider-host`가 소유한다.

## Capability completeness

Capability는 trait 이름만으로 완료되지 않는다. Request/Response/Stream Event/Stable Error/Config/Metadata/Cancellation/Deadline/Resource/Idempotency/Side Effect/Version/Conformance가 닫혀야 한다. P0 Harness Capability의 concrete semantic은 `DXB-ARC-013`이 소유한다.

## Provider surface

Provider는 typed config, external SDK adapter, capability logic, DTO/error mapping만 받는다. raw Domain Store, Scheduler/Registry mutator, global Secret Store, unrestricted filesystem/network, generic service lookup을 받지 않는다.

## Bounded execution

Provider output sink, queue, stream은 item+byte cap과 cancellation/cleanup을 가진다. Provider-local retry는 observable semantic을 바꾸지 않는 허용된 transport retry에 한한다.

검증 기준:

- production Host bypass 0
- 동등 Provider Conformance 통과
- deterministic Reference Provider 존재
- real Harness canary는 같은 Host pipeline 사용
- provider 제거 build와 dependency firewall PASS
