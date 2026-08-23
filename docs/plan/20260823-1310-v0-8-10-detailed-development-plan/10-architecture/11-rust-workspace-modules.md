---
title: "Rust Workspace와 모듈 경계"
document_id: "DXB-ARC-011"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-ARC-010", "DXB-GOV-003"]
---
# Rust Workspace와 모듈 경계

logical boundary를 우선하고 crate-per-concept를 금지한다.

```text
kernel ← domain ← application ← runtime-composition
                   ↑
          application-contract
                   ↑
       control-client/server ← cli
```

- `application-contract`: public DTO/error/exit/operation metadata와 generated schema만 소유한다.
- `application`: use case, selector/auth 호출, operation lifecycle orchestration과 unit of work를 소유한다.
- `storage`: operation/domain/audit persistence 구현을 소유한다.
- `cli`: parser/rendering/safe writer/local journal만 소유한다.

Receipt runtime state를 contract crate에 저장하거나 mutate하지 않는다. Provider/CLI/Control은 Domain internals나 Store를 직접 호출하지 않는다.

M1B부터 cargo metadata dependency firewall을 실행한다.
