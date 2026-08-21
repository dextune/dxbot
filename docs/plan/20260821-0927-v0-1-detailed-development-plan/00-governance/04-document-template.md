---
title: "상세 설계 문서 표준 템플릿"
document_id: "DXB-GOV-004"
version: "0.1.0"
status: "Accepted Template"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-GOV-001"]
---


# 상세 설계 문서 표준 템플릿

새로운 도메인·런타임·인터페이스 문서는 아래 구조를 기본으로 사용한다. 단순히 제목만 채우지 말고 각 항목에 소유권, 실패 의미, 상한, 검증 증거를 포함한다.

## 1. 목적

해결하려는 문제와 제품 원칙을 한 문단으로 정의한다. 구현 수단보다 달성해야 할 의미를 먼저 적는다.

## 2. 책임 범위

- 포함 책임
- 명시적 제외 책임
- Canonical State/Policy Owner
- 관련 문서와 충돌 시 우선순위

## 3. 구성요소

| 구성요소 | 책임 | 수명 | 소유자 | 금지사항 |
|---|---|---|---|---|

외부 Provider, Domain Aggregate, Runtime Worker, Projection을 구분한다.

## 4. 데이터 흐름

Command 입력부터 검증, 상태 결정, 영속화, 외부 실행, Event/Projection까지 정상 경로를 번호와 다이어그램으로 정의한다. 큰 데이터가 복사되는 지점과 Artifact 전환을 표시한다.

## 5. 상호작용과 계약

- 입력/출력
- sync/async
- deadline/cancellation
- idempotency
- consistency
- authorization
- resource budget
- observability
- versioning

## 6. 상태기계와 불변조건

허용 전이, terminal 상태, race 승자, expected revision, 복구 후 상태를 정의한다. 불가능한 상태를 목록화한다.

## 7. 예외상황과 복구

- validation
- conflict
- timeout
- cancellation
- external failure
- partial success
- crash/restart
- overload
- security incident
- cleanup failure

오류를 성공/실패 하나로 축약하지 않고 독립 결과를 보존한다.

## 8. 메모리·성능·캐시

- 최대 item/byte
- allocation/copy ownership
- cache key/invalidation
- queue/backpressure
- hot/cold data
- benchmark workload
- regression threshold

## 9. 보안과 권한

principal, resource, action, policy, approval, secret, untrusted input, audit를 명시한다. 보안 Provider가 없을 때 fail-closed 여부를 적는다.

## 10. 확장성

단일 노드 의미를 유지한 채 remote/distributed/multi-tenant로 확장하는 경계를 정의한다. 조기 분산화 또는 무근거 추상화를 피한다.

## 11. 구현 우선순위

- P0: 제품 의미와 복구에 필수
- P1: 운영 가능성과 확장
- P2: 분산/고급 UI
- P3: 최적화/자율화

선후 의존성과 milestone exit gate를 포함한다.

## 12. 검증 기준

각 기준은 자동화 가능한 Given/When/Then 또는 측정 예산으로 작성하고 `AT-*` ID와 연결한다.

## 13. 미결정 사항

질문, 권장 기본값, 결정 증거, ADR Gate를 적는다. 질문만 남기지 않는다.

## 14. 변경 체크리스트

- 상위 기준과 충돌하지 않는가
- Canonical Owner가 하나인가
- 새 상태 중복이 생기지 않는가
- common abstraction을 재사용했는가
- allocation/queue/cache 상한이 있는가
- cancellation/teardown이 완결되는가
- migration/version 영향이 있는가
- security/permission을 우회하지 않는가
- docs/test/traceability가 함께 갱신되는가
