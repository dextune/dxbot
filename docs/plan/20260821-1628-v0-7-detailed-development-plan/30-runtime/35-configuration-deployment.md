---
title: "구성·프로파일·배포"
document_id: "DXB-RUN-035"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-011", "DXB-ARC-012", "DXB-ARC-013", "DXB-ARC-016", "DXB-ARC-017", "DXB-RUN-032", "DXB-RUN-036"]
---

# 구성·프로파일·배포

## 1. 목적

v0.6 typed immutable Config/Policy generation과 Project/Channel/Memory/Resource 정책 SSOT를 유지하고, v0.7 Runtime Host/Control Endpoint/CLI profile이 Domain policy와 분리되도록 한다.

## 2. Config / Policy Precedence

개념적 precedence:

`compiled safe defaults → deployment → workspace/security domain → project policy → channel policy → Bot profile → Thread/Goal/Task override → Execution override → emergency restriction`.

적용 가능한 layer만 존재한다. Bot-only path에는 Project/Channel layer가 없다. 하위 layer가 security/resource/permission hard ceiling을 완화하지 못한다.

## 3. Policy Family 비회귀

v0.6의 Project/Channel lifecycle/retention, membership/Role/Authority, collaboration fan-out/backpressure, Shared Memory, scope-aware Context, high-risk reauthorization, history/index/cache, Runtime Memory envelope/pressure 정책을 유지한다.

정확한 threshold는 Policy SSOT/benchmark/ADR이 결정한다.

## 4. Runtime Host / Control Config

v0.7에서 추가되는 configuration class는 Domain truth와 분리한다.

- local Control Endpoint address/transport adapter
- Runtime Host service/process lifecycle integration
- protocol compatibility/minimum supported range
- request/response/stream byte bounds의 deployment ceiling
- local principal/profile resolution
- client reconnect/backoff operational bounds

exact IPC/service manager/credential backend는 ADR 대상이다.

## 5. CLI Local Profile

CLI local config는 endpoint/profile/output preference 등을 담을 수 있으나 다음을 Canonical하게 소유하지 않는다.
- Bot/Project/Channel state
- Role/Authority/Permission
- Task/Process state
- Memory truth
- Runtime pressure policy
- Provider selection truth

CLI profile 변경으로 privilege escalation이나 Domain identity 변경이 발생하지 않는다.

## 6. Secret / Sensitivity

Config entry는 owner/scope/type/unit/default source/range/reloadability/sensitivity/version/deprecation metadata를 가진다.

- Role/Authority/permission grant를 free-form config 문자열로 정의하지 않는다.
- secret를 CLI argv/plain config dump의 기본 입력·출력으로 사용하지 않는다.
- `config show/diff`는 sensitive value를 기본 redaction한다.

## 7. Hot Reload

Candidate validate → immutable generation swap → old generation drain을 유지한다.

- running Execution Context/Provider binding mutation 금지
- 신규 routing/admission은 new generation 사용
- membership/authority/security revoke는 current authorization path로 즉시 강화 가능
- cache key/invalidation은 affected generation 포함
- Control endpoint config reload가 Bot/Thread/Task identity를 변경하지 않음

## 8. Deployment Independence

P0 local Runtime Host/embedded storage에서도 모든 Backend semantic을 완전하게 구현한다. 향후 remote/HA에서도 Project/Channel identity, Membership/Authority revision, Memory Scope/provenance, Scheduler Core Lease authority, Directive/Task delegation은 변하지 않는다.

transport endpoint/session이 Domain identity나 Authority가 아니다.

## 9. 검증 기준

- 동일 config source가 deterministic digest/generation을 생성.
- Project/Channel/Resource limit이 문서/Application Contract/CLI에 중복 hardcode되지 않음.
- reload 중 stale Authority/config generation으로 신규 control commit 불가.
- CLI local profile이 Canonical state/Authority를 소유하지 않음.
- local/remote deployment가 Domain semantic을 변경하지 않음.
