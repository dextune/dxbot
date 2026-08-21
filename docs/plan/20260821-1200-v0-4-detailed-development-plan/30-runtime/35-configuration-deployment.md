---
title: "구성·프로파일·배포"
document_id: "DXB-RUN-035"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-011", "DXB-ARC-012", "DXB-ARC-013", "DXB-ARC-016", "DXB-ARC-017", "DXB-RUN-032", "DXB-RUN-036"]
---

# 구성·프로파일·배포

## 1. 목적

환경/Bot/Task/Provider 설정을 typed immutable snapshot으로 관리하고 Conversation/Thread retention/context/control policy를 같은 Policy SSOT 원칙으로 연결한다.

## 2. Config Precedence

compiled safe defaults → deployment → workspace/project → runtime profile → Bot profile → Thread/Goal/Task policy override → one-shot Execution override → emergency restriction. security/resource hard ceiling은 하위 layer가 완화하지 못한다.

Thread override가 Bot-global safety/permission ceiling을 변경하지 않는다.

## 3. Entry Metadata / SSOT

모든 key는 owner, type/unit, default source, valid range, scope, reloadability, sensitivity, version/deprecation을 가진다.

새 policy family 예:
- Thread history retention/archive/compaction
- Thread-local Memory retention/promotion
- Context Plan history/retrieval budget
- control queue/admission/rate/coalescing
- report freshness/stale threshold
- suspension/checkpoint retention
- branch/lineage growth guard

정확한 수치는 Config/Policy owner에서 정의하며 문서에 복제하지 않는다.

## 4. Provider Config / Selection

v0.3 Provider ID/version/Contract/SDK/config digest/secret/lifecycle validation과 selector rule을 유지한다. Provider config가 Thread identity, Control authority, Scheduler hard ceiling을 정하지 못한다.

## 5. Provider Session Configuration

Provider Session reuse/resume가 옵션으로 존재할 수 있으나:
- off로 설정해도 Core acceptance 가능
- loss/restart가 Thread/Conversation을 손상시키지 않음
- resume compatibility는 Capability Contract metadata
- session history를 hidden Canonical context로 승격하지 않음

## 6. Hot Reload

Candidate validate → immutable generation swap → old generation drain. 진행 중 Execution은 pinned snapshot을 유지한다.

Context/control policy reload도 기존 running Execution prompt를 mutation하지 않는다. security restriction은 별도 immediate control path가 가능하다.

## 7. Deployment

P0 daemon+embedded storage+local control+optional Provider sidecar를 유지한다. Control Channel은 Runtime 내부 bounded path로 시작하고 분산 배포 시에도 ordered/durable Directive semantics를 보존한다.

## 8. 검증 기준

- 동일 config source deterministic digest.
- Thread/control policy 값이 둘 이상의 owner에서 중복 정의되지 않음.
- Provider Session disabled/lost profile에서도 AT-SESSION-002 통과.
- reload 실패 시 이전 generation 유지.
- config로 Control authority/resource ceiling을 확대하지 못함.
