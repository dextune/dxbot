---
title: "DXBOT 상세 개발 기획 문서 집합"
document_id: "DXB-INDEX"
version: "0.1.0"
status: "Working Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000"]
---


# DXBOT 상세 개발 기획 문서 집합

이 디렉터리는 「DXBOT 1차 컨셉 기획안」을 최상위 기준으로 구체화한 개발 기준서다. 문서는 **제품 의미 → 아키텍처 경계 → 도메인 상태기계 → 실행·보안·운영 → 인터페이스 → 품질·납품** 순으로 읽는다.

## 1. 기준 우선순위

충돌 시 다음 순서로 해석한다.

1. `00-governance/00-source-concept-original.md`의 원문 제품 철학
2. `00-governance/00-normative-baseline.md`의 상세 해석
3. 승인된 ADR 및 `03-architecture-decision-baseline.md`
4. 도메인 문서의 불변조건과 공개 계약
5. 런타임·인터페이스·엔지니어링 문서
6. 구현 코드와 테스트

하위 문서는 상위 기준을 변경할 수 없다. 변경이 필요하면 기준 문서, 영향 문서, 추적성 표, 테스트를 같은 변경 단위로 갱신한다.

## 2. 권장 읽기 순서

| 순서 | 문서군 | 목적 |
|---:|---|---|
| 1 | `00-governance` | 범위, 용어, 결정 규칙 고정 |
| 2 | `10-architecture` | 레이어, 의존성, Harness·저장소 경계 정의 |
| 3 | `20-domains` | Bot·Brain·Memory·Task·Core·Bot Network 상태기계 정의 |
| 4 | `30-runtime` | 동시성, 자원, 보안, 복구, 관측, 배포 정의 |
| 5 | `40-interfaces` | 동일 Runtime을 사용하는 CLI/TUI/Web/API 정의 |
| 6 | `50-engineering` | Rust 구현·성능·검증·릴리스 규칙 정의 |
| 7 | `60-delivery` | 구현 순서, 수용 기준, 위험, 미결정 사항 관리 |

## 3. 문서 목록

### 거버넌스
- `00-source-concept-original.md`: 사용자 제공 최상위 원문 보존본
- `00-normative-baseline.md`: 최상위 제품 원칙과 비협상 조건
- `01-document-governance.md`: 변경·승인·추적 규칙
- `02-glossary-domain-model.md`: 단일 용어집과 집합 관계
- `03-architecture-decision-baseline.md`: 초기 ADR 후보와 기본 선택
- `04-document-template.md`: 후속 상세 문서 표준 구조

### 아키텍처
- `10-system-architecture.md`: 전체 런타임 분해와 의존성 방향
- `11-rust-workspace-modules.md`: Rust workspace 및 crate 경계
- `12-shared-contracts-extension-model.md`: 공통 계약과 확장 모델
- `13-harness-integration.md`: DeepSeek-inspired Harness 경계
- `14-event-state-model.md`: Command/Event/Projection 규칙
- `15-storage-data-model.md`: 영속화, 트랜잭션, 인덱스, 스냅샷

### 핵심 도메인
- `20-bot-identity-lifecycle.md`
- `21-brain-context.md`
- `22-memory-system.md`
- `23-goal-task-execution.md`
- `24-dynamic-core-scheduler.md`
- `25-multi-bot-network.md`
- `26-control-plane.md`

### 런타임·운영
- `30-concurrency-consistency.md`
- `31-resource-governance.md`
- `32-security-permissions-sandbox.md`
- `33-resilience-recovery.md`
- `34-observability-audit.md`
- `35-configuration-deployment.md`

### 인터페이스
- `40-api-protocols.md`
- `41-cli.md`
- `42-tui.md`
- `43-web-control-center.md`

### 엔지니어링
- `50-rust-engineering-rules.md`
- `51-performance-memory-cache.md`
- `52-testing-verification.md`
- `53-versioning-migration.md`
- `54-repository-ci-release.md`

### 납품
- `60-implementation-roadmap.md`
- `61-acceptance-traceability.md`
- `62-risk-register.md`
- `63-open-questions.md`
- `64-external-reference-snapshot.md`


### 패키지 보조 파일
- `all-in-one.md`: 전체 문서를 읽기 순서대로 병합한 검토용 통합본
- `manifest.md`: 파일별 크기·줄 수·SHA-256 검증표

## 4. 공통 설계 규칙

- Bot이 Identity·Memory·Persistent State를 소유한다. Session과 UI는 소유자가 아니다.
- 하나의 Bot에는 하나의 Brain이 있고, Core는 Brain이 취득하는 일시적 실행 임대다.
- Canonical State와 Derived State를 명시한다. 동일 사실의 다중 원본을 금지한다.
- 공유 쓰기는 명령 경로에서 직렬화하고, 읽기는 불변 스냅샷을 우선한다.
- 모든 큐·버퍼·캐시는 상한, 퇴출 정책, 관측 지표를 가져야 한다.
- 도메인 타입은 Harness·DB·UI 프레임워크 타입을 직접 노출하지 않는다.
- 공통화는 의미가 같은 계약에만 적용한다. 이름만 비슷한 기능을 억지로 추상화하지 않는다.
- `common`, `utils`, `helpers`와 같은 무소유 범용 저장소를 만들지 않는다.
- 기능 중복은 코드뿐 아니라 상태, 캐시, 재시도, 스케줄러, 정책 판단의 중복까지 포함한다.
- 모델·도구·네트워크 지연을 제외한 Runtime 오버헤드는 계측 가능한 예산으로 관리한다.

## 5. 완료 정의

문서 집합은 다음을 만족할 때 개발 기준으로 승인할 수 있다.

1. 16개 최상위 핵심 원칙이 `61-acceptance-traceability.md`에서 설계·구현·테스트 항목으로 추적된다.
2. 모든 주요 Aggregate에 상태기계, 불변조건, 동시성 소유자, 영속화 소유자가 정의되어 있다.
3. 외부 Harness 변경이 Bot 도메인 타입에 전파되지 않는 경계가 존재한다.
4. 실패·취소·재시작·중복 전달·부분 성공에 대한 의미가 문서화되어 있다.
5. P0 수용 테스트가 자동화 가능하며, 성능 예산과 자원 상한이 명시되어 있다.
