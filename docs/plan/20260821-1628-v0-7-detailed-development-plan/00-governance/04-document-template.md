---
title: "상세 설계 문서 표준 템플릿"
document_id: "DXB-GOV-004"
version: "0.7.0"
status: "Accepted Template"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-GOV-001", "DXB-ARC-017"]
---

# 상세 설계 문서 표준 템플릿

새로운 Domain/Runtime/Application Contract/Interface/Capability/Provider/Plugin 문서는 아래 구조를 기본으로 사용한다.

## 1. 목적
해결하려는 문제와 제품 의미를 정의한다.

## 2. Architecture Classification / Quality Tier

- Classification: `Core Domain | Optional Capability | Provider | Interface | Plugin`
- Quality Tier: `A(Common/Contract) | B(Extension Implementation)`
- Canonical Owner
- Lifecycle Owner
- Persistent State Owner/없음
- 제거 가능 여부
- Provider/Plugin 여부

## 3. 책임 범위

- 포함 책임
- 명시적 제외 책임
- 관련 문서와 충돌 시 우선순위
- Common vs Extension 소유권
- 허용/금지 dependency 방향

## 4. 구성요소

| 구성요소 | 책임 | 수명 | Owner | 제거성 | 금지사항 |
|---|---|---|---|---|---|

## 5. 데이터 흐름

Command 입력부터 validation, Application Contract/Application, Domain decision, persistence, Scheduler/Runtime, Provider Host, Provider/Side Effect, Recovery, Event/Projection/Interface까지 정상 경로를 정의한다. 큰 데이터의 ownership/copy/Artifact 전환 지점을 표시한다.

## 6. 상호작용 계약

- input/output/streaming event
- stable error/outcome
- sync/async
- deadline/cancellation
- idempotency/side-effect classification
- consistency/revision
- authorization/approval
- resource budget/accounting
- observability/audit
- version/compatibility
- Provider selection/config generation snapshot

Optional Capability라면 위 항목을 Contract Package로 완결하고 Conformance를 연결한다. Application/Public Contract라면 Command/Query/Subscription의 의미와 Domain/internal type 경계를 명시한다.

## 7. Provider Host / Extension Surface

Provider가 관련되면 다음을 명시한다.

- Host pre/post enforcement stage
- Provider Call Context 최소 surface
- Provider SDK 사용 범위
- 금지 Runtime/Domain/Storage dependency
- Provider-local retry가 허용되는 정확한 조건
- lifecycle/activity/quiescence owner
- direct invocation 금지 여부

Provider가 관련되지 않으면 `N/A`와 이유를 기록한다.

## 8. 상태기계와 불변조건

허용 전이, terminal 상태, race 승자, expected revision, restart 후 상태를 정의한다. Waiting이면 Continuation, Provider이면 Standard Lifecycle과 generation/activity 관계를 포함한다.

## 9. Side Effect / 외부 상태

외부 변경이 있으면 idempotency key/action digest, write-ahead intent, confirmed/failed/unknown, reconciliation, retry 조건을 정의한다.

## 10. 예외상황과 복구

validation, conflict, timeout, cancellation, Provider absence/incompatible/failure, partial success, crash/restart, reconnect, overload, cleanup failure, feature disabled/removed/superseded를 포함한다.

## 11. 메모리·성능·캐시

item/byte cap, allocation/copy ownership, cache source/key/invalidation, queue/backpressure, hot/cold data, benchmark를 명시한다. Provider Host/SDK, Control/Interface streaming overhead와 buffer도 bounded 원칙을 따른다.

## 12. 보안과 권한

principal, resource, action, permission/approval, secret, untrusted input, audit, Plugin/Provider isolation과 Host enforcement 지점을 명시한다. Interface는 Authorization Decision Owner가 아님을 명시한다.

## 13. 확장성 및 Provider/Plugin

- 대체/복수 Provider selector
- local/remote 확장
- Provider SDK/Conformance/Reference 영향
- Plugin으로 노출할 필요 여부
- Plugin Host와 Provider Host 구분
- 내부 trait와 public wire contract 구분

## 14. Removal / Migration / Supersession Contract

Optional Capability/Provider/Interface/Plugin이면 최소 다음을 작성한다.

`deprecate/supersede → stop new use → drain(if applicable) → detach → compatibility → data/config/docs cleanup → derived cleanup → dependency/code removal`

stale config/reference, persistent data ownership, rollback/re-entry condition, unsupported API 상태를 정의한다. 단순 파일 누락을 supersession으로 간주하지 않는다.

## 15. 구현 우선순위

P0/P1/P2/P3와 선후 의존성, Phase/Milestone exit gate를 명시한다. Tier B가 Tier A 변경을 요구하는 escalation 조건을 기록한다.

## 16. 검증 기준

Given/When/Then 또는 측정 예산으로 작성하고 `AT-*`에 연결한다. Provider면 Conformance, Host-path, dependency firewall, replacement/removal/multi-selection을 검토한다.

## 17. Cross-Layer Scenario 영향

다음 경로에서 영향 여부를 명시한다.

`Client/Command → Application Contract → Application → Domain/Persistence → Scheduler/Runtime → Provider Host → Provider/Plugin → Recovery → Projection/Event → Interface`

Capability/Provider 개발 흐름도 `Contract → Host/SDK/Testkit → Provider → Conformance/CI → Runtime → Recovery → Removal`로 추적한다.

## 18. 미결정 사항

질문, 권장 기본값, 결정 증거, ADR Gate를 기록한다.

## 19. 변경 체크리스트

- 5개 분류와 Quality Tier가 명확한가
- Canonical/Lifecycle/Persistent State Owner가 하나인가
- Common/Extension 책임이 겹치지 않는가
- Domain이 concrete Provider/Plugin/Interface를 참조하지 않는가
- Provider가 Host/SDK/Dependency Firewall을 우회하지 않는가
- Interface가 Application Contract/Authorization/Resource owner를 우회하지 않는가
- 새로운 중복 state/policy/cache/retry가 없는가
- allocation/queue/cache/stream 상한이 있는가
- Waiting/Side Effect/Provider lifecycle/reconnect recovery가 완결되는가
- disable/remove/supersede/migration 경로가 있는가
- stale config/API가 silent ignore되지 않는가
- docs/test/conformance/traceability/risk가 함께 갱신되는가
- Review 1 `Structural / Consistency` 대상이 정의됐는가
- Review 2 `Cross-Layer Executability / Compatibility` 대상이 정의됐는가
