---
title: "공통 계약과 Capability/Provider 확장 모델"
document_id: "DXB-ARC-012"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-ARC-011"]
---

# 공통 계약과 Capability/Provider 확장 모델

## 1. 목적과 책임

v0.6의 Model/Tool/Sandbox/Memory Index/Transport/Executor 등 교체 가능한 기능에 대한 **Capability Contract/Provider Host** 의미를 유지하면서, v0.7의 **Application Control Contract**와 혼합되지 않도록 경계를 고정한다.

Capability/Provider Framework의 Canonical 의미는 기존 문서를 그대로 상속하며 `DXB-IFC-040`은 사용자/운영 use-case를 노출하는 별도 public contract다.

## 2. 두 Contract의 구분

```text
Application Contract (`DXB-IFC-040`)
= 사용자/운영 Command · Query · Subscription

Capability Contract (`DXB-ARC-012`/`DXB-ARC-017`)
= Runtime이 교체 가능한 Provider 기능을 호출하는 stable semantic
```

금지:
- CLI가 Provider Capability Request를 public control command로 직접 사용
- public Control DTO를 Provider DTO로 재사용
- Provider optional feature를 CLI가 concrete Provider type/downcast로 판정
- Capability discovery response가 authorization을 생성

## 3. Provider Framework 비회귀

Capability Contract Package는 기존처럼 Request/Response/Streaming Event/Stable Error/Config/Feature Negotiation/Cancellation/Deadline/Resource/Idempotency/Side Effect/Version/Conformance를 가진다.

Provider selection/binding/lifecycle/removal은 Provider Host가 소유하며 CLI/Application Contract가 Registry mutator가 되지 않는다.

## 4. Public Value Boundary

CLI가 필요로 하는 public data는 stable Wire/Public Value type으로 노출한다.

```text
Domain Struct ─X→ CLI
Persistence Row ─X→ CLI
Provider DTO ─X→ CLI

Domain/Application owner
→ explicit mapping
→ Public Contract DTO
→ Control Client
→ CLI
```

large immutable payload는 public contract에서 Artifact/reference/stream을 우선하며 Domain object graph 복사를 요구하지 않는다.

## 5. Capability Discovery 관계

Application Contract의 read-only capability discovery는 다음을 safe summary로 노출할 수 있다.
- enabled public capability/feature
- Provider availability의 추상 상태
- Plugin availability의 추상 상태
- supported protocol/schema feature
- Runtime degraded/pressure state

이는 Provider Registry 내부 object/generation mutator를 공개하지 않으며 권한을 생성하지 않는다.

## 6. 검증 기준

- Provider 교체가 Application Contract semantic을 바꾸지 않는다.
- CLI가 Domain/Provider concrete type을 import/downcast하지 않는다.
- Control Contract와 Provider Contract의 DTO/error/lifecycle owner가 섞이지 않는다.
- Provider Host/Conformance/Tier A-B/removal gate가 v0.6과 동일하게 유지된다.
