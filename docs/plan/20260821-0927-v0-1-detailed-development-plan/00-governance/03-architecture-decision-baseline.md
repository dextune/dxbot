---
title: "초기 아키텍처 결정 기준선"
document_id: "DXB-GOV-003"
version: "0.1.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-GOV-002"]
---


# 초기 아키텍처 결정 기준선

## 1. 목적

구현팀이 핵심 구조를 매번 재논의하거나 서로 다른 기본값을 선택하지 않도록 초기 결정을 명시한다. 각 항목은 상세 검증 후 정식 ADR로 승격한다.

## 2. 책임 범위

P0 개발을 시작하기 위해 필요한 구조적 기본값을 다룬다. 특정 crate 버전이나 운영 벤더 선택은 포함하지 않는다.

## 3. 결정 목록

| 후보 ADR | 기본 결정 | 근거 | 재검토 트리거 |
|---|---|---|---|
| ADR-0001 | Bot 중심 Aggregate와 Session 비소유성 | 제품 정체성 보존 | Session이 Bot 상태를 소유해야만 하는 증거 |
| ADR-0002 | Domain Event Journal + 현재 상태 Snapshot의 Hybrid | 추적·복구와 단순 조회 균형 | 쓰기량/복잡도 예산 초과 |
| ADR-0003 | Bot별 단일 쓰기 조정자 | 공유 상태 경쟁 축소 | 단일 조정자가 병목임이 계측됨 |
| ADR-0004 | Core는 Lease 기반 일시 실행 | 정체성 중복 방지 | 장기 독립 실행이 필요한 사례 |
| ADR-0005 | Harness는 Port/Adapter로 격리 | 상위 변경과 공급자 종속 차단 | 안정된 단일 표준이 확정됨 |
| ADR-0006 | 초기 배포는 단일 노드/프로세스 | 의미 검증과 운영 복잡도 최소화 | 원격 실행 요구가 P0가 됨 |
| ADR-0007 | Embedded Store 우선, 다중 노드 Store는 후속 | 개발·복구 단순화 | 동시 Writer/HA 요구 |
| ADR-0008 | 내구성 Inbox/Outbox, at-least-once 전달 | 메시지 유실 방지 | 종단 간 exactly-once 필요 증명 |
| ADR-0009 | 불변 Snapshot + 명령 기반 변경 | 읽기 동시성과 재현성 | Snapshot 비용이 예산 초과 |
| ADR-0010 | 모든 큐와 캐시를 bounded로 구성 | 메모리 안정성 | 없음; 비협상 |
| ADR-0011 | CLI와 모든 UI는 동일 Control API 사용 | UI 로직 중복 방지 | 없음; 비협상 |
| ADR-0012 | 외부 큰 데이터는 Artifact 참조로 전달 | 복사·로그 팽창 방지 | 작은 데이터만 존재한다는 증거 |
| ADR-0013 | 정책은 호출 시점의 불변 Policy Snapshot으로 평가 | 재현성과 감사 | 실시간 정책 변경이 실행 중 강제돼야 함 |
| ADR-0014 | 외부 오류와 Domain 오류를 분리 | 재시도·운영 판단 명확화 | 없음 |
| ADR-0015 | 성능 최적화는 계측과 예산 기반 | 위험한 조기 최적화 방지 | 없음 |

## 4. 구성요소

각 ADR은 다음 구조를 가진다.
- Context
- Decision
- Alternatives
- Consequences
- Invariants
- Rollout
- Migration/Rollback
- Verification Evidence
- Supersedes/Superseded by

## 5. 데이터 흐름과 상호작용

결정 제안은 관련 도메인 문서의 불변조건에서 시작한다. 정식 ADR이 승인되면:
1. 이 기준선의 상태를 갱신한다.
2. crate 의존성 및 스키마 계약을 갱신한다.
3. 추적성 표에 ADR과 테스트를 연결한다.
4. 기존 데이터와 API에 대한 Migration을 정의한다.
5. 관측 지표로 결정의 가정을 검증한다.

## 6. 예외상황

- 실험 코드는 결정과 다를 수 있으나 제품 경로에 포함되지 않고 feature flag와 제거 조건을 가져야 한다.
- P0 결정 위반은 성능상 편의를 이유로 허용하지 않는다. 벤치마크와 대안 ADR이 필요하다.
- 외부 Harness 구조를 그대로 복제하는 것은 자동 승인되지 않는다. DXBOT 도메인 의미와 Rust 비용 모델을 다시 검증한다.
- 임시 호환 shim은 만료일·제거 issue·계측을 갖지 않으면 추가하지 않는다.

## 7. 확장성

결정은 단일 노드 의미를 분산 환경에서도 유지할 수 있어야 한다. 분산화를 위해 Bot/Core의 의미를 바꾸기보다 실행 위치, Lease, 메시지 운송, 일관성 수준을 확장한다.

## 8. 구현 우선순위

- **P0:** ADR-0001~0014 검증 및 승인
- **P1:** 분산 준비, 고급 검색, 다중 Tenant 관련 ADR
- **P2:** 원격 Worker, HA, 샤딩 ADR

## 9. 검증 기준

- 코드 의존성 그래프가 결정된 레이어 방향을 위반하지 않는다.
- 각 P0 ADR에 최소 하나의 자동화된 Architecture Test 또는 Runtime Test가 있다.
- Decision의 가정이 계측 가능하지 않으면 Accepted 상태로 승격하지 않는다.
- 대안과 롤백이 없는 비가역 결정은 승인하지 않는다.
