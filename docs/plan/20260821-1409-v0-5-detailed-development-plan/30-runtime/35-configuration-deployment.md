---
title: "구성·프로파일·배포"
document_id: "DXB-RUN-035"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-011", "DXB-ARC-012", "DXB-ARC-013", "DXB-ARC-016", "DXB-ARC-017", "DXB-RUN-032", "DXB-RUN-036"]
---

# 구성·프로파일·배포

## 1. 목적

v0.4 typed immutable Config/Policy generation 원칙을 유지하면서 Project/Channel/Memory Scope/Orchestration 정책을 SSOT로 관리한다.

## 2. Config / Policy Precedence

개념적 precedence:
`compiled safe defaults → deployment → workspace/tenant → project policy → channel policy → Bot profile → Thread/Goal/Task override → Execution override → emergency restriction`.

적용 가능한 layer만 존재한다. Bot-only path에는 Project/Channel layer가 없다. 하위 layer가 security/resource/permission hard ceiling을 완화하지 못한다.

## 3. 신규 Policy Family

- Project/Channel lifecycle/retention operational policy
- membership/Role/Authority admission policy
- per-message participant activation/fan-out
- Channel response concurrency/backpressure
- Shared Memory write/promotion/retention
- scope-aware recall/context budgets
- membership revoke/high-risk reauthorization
- Channel history/index/cache limits
- manager routing/failover policy

정확한 숫자 threshold는 Policy owner/benchmark/ADR에서 결정한다.

## 4. Config Entry Metadata

owner, scope, type/unit, safe default source, valid range, reloadability, sensitivity, version/deprecation을 가진다. Role/Authority와 permission grant를 free-form prompt/config 문자열로 정의하지 않는다.

## 5. Hot Reload

Candidate validate → immutable generation swap → old generation drain을 유지한다.

- running Execution Context/Provider binding mutation 금지
- routing은 new generation을 신규 admission에 사용
- membership/authority/security revoke는 별도 current authorization path로 즉시 강화 가능
- cache key/invalidation은 affected generation을 포함

## 6. Deployment Independence

P0 local daemon/embedded storage에서도 Project/Channel semantic을 완전히 구현한다. M7 remote/HA에서도 다음은 변하지 않는다.
- Project/Channel Canonical identity
- Membership/Authority revision semantics
- Memory ScopeRef/provenance
- Scheduler Core Lease authority
- durable Directive/Task delegation

remote transport가 Channel identity나 authority가 아니다.

## 7. Provider Session

Provider Session reuse/resume는 optional optimization이다. Project/Channel/Thread context source 또는 membership truth가 아니다.

## 8. 검증 기준

- 동일 config source가 deterministic digest/generation을 생성.
- Project/Channel limit이 문서/CLI/Web에 중복 hardcode되지 않음.
- reload 중 stale Authority generation으로 신규 control commit 불가.
- local/remote deployment가 Scope/Channel semantic을 변경하지 않음.
