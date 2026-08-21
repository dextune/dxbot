---
title: "상세 설계 문서 표준 템플릿"
document_id: "DXB-GOV-004"
version: "0.2.0"
status: "Accepted Template"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-GOV-001"]
---

# 상세 설계 문서 표준 템플릿

새로운 Domain/Runtime/Interface/Capability/Provider/Plugin 문서는 아래 구조를 기본으로 사용한다.

## 1. 목적
해결하려는 문제와 제품 의미를 정의한다.

## 2. Architecture Classification

- Classification: `Core Domain | Optional Capability | Provider | Interface | Plugin`
- Canonical Owner
- Lifecycle Owner
- Persistent State Owner/없음
- 제거 가능 여부
- Provider/Plugin 여부

## 3. 책임 범위

- 포함 책임
- 명시적 제외 책임
- 관련 문서와 충돌 시 우선순위
- 허용/금지 dependency 방향

## 4. 구성요소

| 구성요소 | 책임 | 수명 | Owner | 제거성 | 금지사항 |
|---|---|---|---|---|---|

## 5. 데이터 흐름

Command 입력부터 validation, Domain decision, persistence, provider/side effect, Event/Projection까지 정상 경로를 정의한다. 큰 데이터의 ownership/copy/Artifact 전환 지점을 표시한다.

## 6. 상호작용 계약

- input/output
- sync/async
- deadline/cancellation
- idempotency
- consistency/revision
- authorization
- resource budget
- observability
- versioning
- provider selection snapshot

## 7. 상태기계와 불변조건

허용 전이, terminal 상태, race 승자, expected revision, restart 후 상태를 정의한다. Waiting 상태가 있으면 Continuation 계약을 포함한다.

## 8. Side Effect / 외부 상태

외부 변경이 있으면 다음을 정의한다.

- idempotency key/action digest
- write-ahead intent
- confirmed/failed/unknown 의미
- reconciliation
- retry 조건

## 9. 예외상황과 복구

validation, conflict, timeout, cancellation, provider absence, partial success, crash/restart, overload, cleanup failure, feature disabled/removed를 포함한다.

## 10. 메모리·성능·캐시

item/byte cap, allocation/copy ownership, cache source/key/invalidation, queue/backpressure, hot/cold data, benchmark를 명시한다. Canonical Memory 삭제와 cache eviction을 혼동하지 않는다.

## 11. 보안과 권한

principal, resource, action, permission/approval, secret, untrusted input, audit, plugin/provider isolation을 명시한다.

## 12. 확장성 및 Provider/Plugin

- 대체 Provider
- 복수 Provider selector
- local/remote 확장
- Plugin으로 노출할 필요 여부
- 내부 trait와 public wire contract 구분

## 13. Removal / Migration Contract

Optional Capability/Provider/Interface/Plugin이면 최소 다음을 작성한다.

`deprecate → stop new use → drain → detach → compatibility → data/config cleanup → derived cleanup → dependency/code removal`

stale config, persistent data ownership, rollback, unsupported API 상태를 정의한다.

## 14. 구현 우선순위

P0/P1/P2/P3와 선후 의존성, exit gate를 명시한다.

## 15. 검증 기준

Given/When/Then 또는 측정 예산으로 작성하고 `AT-*` ID와 연결한다. Provider replacement/removal/multi-selection이 관련되면 별도 test를 둔다.

## 16. Cross-Layer Scenario 영향

다음 경로에서 영향 여부를 명시한다.

`Command → Application → Domain → Persistence → Scheduler → Provider → Recovery → Projection → API`

## 17. 미결정 사항

질문, 권장 기본값, 결정 증거, ADR Gate를 기록한다.

## 18. 변경 체크리스트

- 5개 분류 중 하나인가
- Canonical/Lifecycle/Persistent State Owner가 하나인가
- Domain이 concrete Provider/Plugin을 참조하지 않는가
- 새로운 중복 state/policy/cache/retry가 없는가
- allocation/queue/cache 상한이 있는가
- Waiting continuation이 완결되는가
- Side Effect unknown/reconcile 의미가 있는가
- disable/remove/migration 경로가 있는가
- stale config/API가 silent ignore되지 않는가
- security/permission/resource gate를 우회하지 않는가
- docs/test/traceability/risk가 함께 갱신되는가
- Structural Review와 Cross-Layer Review 대상이 정의됐는가
