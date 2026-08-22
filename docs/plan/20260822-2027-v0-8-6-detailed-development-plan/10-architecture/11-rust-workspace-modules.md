---
title: "Rust Workspace와 모듈 경계"
document_id: "DXB-ARC-011"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-ARC-010", "DXB-GOV-003"]
---
# Rust Workspace와 모듈 경계

## 1. 원칙

logical owner를 먼저 정하고 독립 compile/test/dependency firewall이 필요한 경우에만 crate로 분리한다. `common`, `utils`, service locator, mega context, crate-per-concept를 금지한다.

## 2. P0 logical modules

| Module | 단일 책임 | Persistent owner |
|---|---|---|
| `kernel` | typed IDs, revision, time, bounded units | none |
| `domain` | Bot/Memory/Conversation/Task/Project/Channel invariants | domain stores |
| `application` | typed use case, ContextPlan builder, authorization 호출, unit of work | operation/directive refs |
| `application-contract` | public operation metadata, DTO, error, exit, schema snapshot | contract snapshot store |
| `runtime` | scheduler, resource, recovery, composition | runtime metadata |
| `provider-host` | capability enforcement, provider lifecycle/selection | provider registry |
| `control` | authenticated server adapter/client/subscription | none |
| `host-lifecycle` | Linux process/service bootstrap and explicit host action | host descriptor |
| `cli` | parser/rendering/machine writer/local journal | CLI local state only |
| `storage` | canonical journal/state/outbox/projection implementations | canonical storage |
| `xtask` | schema/docs/dependency/validator generation | generated artifact |

## 3. Dependency direction

```text
kernel ← domain ← application ← runtime-composition
                   ↑
        application-contract adapters
                   ↑
       control-client/server ← cli

provider-host → capability-contract + provider-sdk
runtime-composition → application + storage + provider-host + control-server
host-lifecycle ← cli only through narrow host-action contract
```

`application-contract`는 Domain aggregate를 소유하지 않는다. Application이 Domain result를 public DTO로 mapping한다. CLI와 Control은 Contract에 의존하고 Domain/Storage/Provider concrete type에 의존하지 않는다.

## 4. Executable firewall

| From | 허용 | 금지 |
|---|---|---|
| CLI | application-contract, control-client, host-action-client | domain, storage, runtime internals, provider |
| Provider | capability-contract, provider-sdk, external SDK | domain/application/runtime/storage/interface |
| Domain | kernel | transport, DB, provider, UI |
| Host lifecycle | host-action contract | Domain mutation, storage row, provider registry |

M1B부터 cargo metadata 또는 동등 dependency graph fixture로 위 edge를 검사한다.

## 5. Local journal

CLI local journal은 `CommandId`, epoch-bearing `IdempotencyKey`, `RequestDigest`, operation reference, state만 저장한다. secret, full payload, Authority, canonical outcome을 저장하지 않는다. Runtime receipt가 source of truth다.
