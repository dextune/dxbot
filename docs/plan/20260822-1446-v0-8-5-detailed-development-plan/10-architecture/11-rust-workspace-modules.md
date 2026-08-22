---
title: "Rust Workspace와 모듈 경계"
document_id: "DXB-ARC-011"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-GOV-003"]
---

# Rust Workspace와 모듈 경계

## 1. 원칙

logical boundary를 먼저 확정하고 독립 compile/test/dependency/firewall 필요가 증명될 때만 crate로 분리한다. `common`, `utils`, service locator, mega context, crate 폭증을 금지한다.

## 2. P0 logical modules

```text
kernel                    typed IDs, revision, time, bounded units
domain                    bot/memory/conversation/task/project/channel
application               typed use cases, unit of work, receipt owner port
application-contract      public DTO/error/schema source
runtime                    scheduler/process/resource/recovery/composition
provider-host             capability enforcement and reference provider
control                    server adapter, client, subscription, endpoint discovery
host-lifecycle             Linux user-service adapter only
cli                        parser, selector input, rendering, machine writer, local journal
storage                    journal/state/outbox/projection implementations
xtask/plan-validator       generated schema/docs/dependency gates
```

`application-contract`는 Runtime과 CLI 사이의 public schema SSOT를 강제할 독립 compile boundary가 필요하므로 P0 별도 crate 후보로 승인한다. 나머지는 책임과 build evidence 없이 별도 crate로 기계 분리하지 않는다.

## 3. Dependency 방향

```text
kernel ← domain ← application
                   ↑
application-contract (public values, no Domain internals)
                   ↑
control client/server ← cli
runtime composition → application + storage + provider-host + control server
host-lifecycle ← cli (bootstrap only; no domain/runtime internals)
```

## 4. Public type firewall

- public DTO ≠ Domain aggregate ≠ storage row
- `ResourceRef`, `OperationReceipt`, cursor/error envelope는 contract crate가 소유한다.
- Domain ID newtype의 wire 표현은 명시 mapping한다.
- Provider capability DTO와 Application discovery DTO를 공유하지 않는다.

## 5. CLI local journal

허용 field: instance/profile, CommandId, IdempotencyKey, RequestDigest, submitted_at, last receipt revision. 금지: secret, full payload, Authority, canonical outcome. item+byte cap과 atomic replace를 갖고 Runtime receipt가 source of truth다.

## 6. Async ownership

spawned task는 owner/cancel/join을 가진다. server subscriber, client receiver, stdout writer, endpoint watcher, Runtime child는 서로 cancellation token을 공유해 Domain lifecycle을 우연히 변경하지 않는다.

## 7. 검증 기준

- CLI forbidden internal edge 0
- Host Lifecycle Domain edge 0
- public DTO=Domain/Persistence shared type 0
- owner 없는 task/channel/cache 0
- application-contract 외 speculative schema crate 0
