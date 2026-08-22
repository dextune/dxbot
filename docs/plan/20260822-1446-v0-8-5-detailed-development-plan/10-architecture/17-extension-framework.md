---
title: "Common Extension Framework"
document_id: "DXB-ARC-017"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-011", "DXB-ARC-012", "DXB-ARC-016"]
---

# Common Extension Framework

## 1. 목적

Optional Capability와 Provider 구현이 공통 correctness를 재구현하지 않도록 Stable Contract, Host, SDK, Testkit, Reference Provider, Dependency Firewall을 정의한다.

## 2. Common 책임

lifecycle/generation, selection, permission, resource admission/accounting, deadline/cancel, semantic retry, side-effect reconciliation, telemetry/audit, compatibility, recovery, conformance를 소유한다.

## 3. Provider surface

Provider는 typed config, external SDK adapter, capability logic, DTO/error mapping만 받는다. raw Domain Store, Scheduler/Registry mutator, global Secret Store, unrestricted filesystem/network, generic service lookup을 받지 않는다.

## 4. Bounded execution

Provider output sink, queue, stream은 item+byte cap과 cancellation/cleanup을 가진다. Provider-local retry는 stable contract가 허용하고 observable semantic을 바꾸지 않는 transport retry에 한한다.

## 5. v0.8 Contract 관계

Application selector/receipt/page/stream schema는 Extension Framework가 소유하지 않는다. Provider availability는 Application discovery DTO로 안전하게 mapping한다.

## 6. 검증 기준

- production Host bypass 0
- 동등 Provider Conformance 통과
- deterministic Reference Provider 존재
- provider 제거 build와 dependency firewall PASS
