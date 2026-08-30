---
title: "DXBOT Web Control Center 문서 Index"
document_id: "DXB-WEB-INDEX"
status: "Accepted"
normative: false
priority: "web-P0"
plan_baseline: "0.8.10"
last_updated: "2026-08-30"
owner: "Web Control Center 문서 패키지"
---
# DXBOT Web Control Center 문서 Index

DXBOT Web UI의 설계·구현·검증 문서 Router다. 규칙 본문을 이 문서에 복제하지 않고 Owner 문서로 보낸다.

## 먼저 읽어야 하는 것

작업 종류와 무관하게 **[00-visual-source-of-truth.md](00-visual-source-of-truth.md)를 먼저 읽는다.** 시각 기준은 `sample/` 디렉터리의 PNG 파일이며, 그 파일들이 유일한 정답이다. 대화 기억·요약·일반적인 Dashboard 관례는 기준이 아니다.

```text
sample/desktop-main-dark.png       Desktop / Dark  (Primary)
sample/desktop-main-light.png      Desktop / Light
sample/mobile-main-dark.png.png    Mobile  / Dark
sample/mobile-main-light.png       Mobile  / Light
```

이 4개는 **4개의 화면이 아니다.** 단일 `workspace-cockpit` 화면 × 2 폼팩터 × 2 색상 모드다. 자세한 구조는 [02-screen-inventory.md](02-screen-inventory.md)를 읽는다.

## 작업별 진입점

| 작업 | 읽을 문서 |
|---|---|
| 시각 기준·시안 해석 판정 | [00-visual-source-of-truth.md](00-visual-source-of-truth.md) |
| 범위·소유권·Tier 판정 | [01-scope-and-ownership.md](01-scope-and-ownership.md) |
| 화면 구조 파악 | [02-screen-inventory.md](02-screen-inventory.md) |
| route·내비게이션·용어 매핑 | [03-information-architecture.md](03-information-architecture.md) |
| 색·간격·타이포·상태 표현 | [04-design-tokens.md](04-design-tokens.md) |
| 컴포넌트 추가·분리 판단 | [05-component-inventory.md](05-component-inventory.md) |
| 반응형·breakpoint | [06-responsive-layout.md](06-responsive-layout.md) |
| Dark / Light | [07-theme-and-color-mode.md](07-theme-and-color-mode.md) |
| 데이터 접근·캐시·실시간 | [08-api-and-state-architecture.md](08-api-and-state-architecture.md) |
| 타입 생성·계약 사용 | [09-type-contract.md](09-type-contract.md) |
| 로딩·빈 상태·실패·재연결 | [10-error-and-exceptional-state.md](10-error-and-exceptional-state.md) |
| 키보드·라벨·대비·터치 | [11-accessibility.md](11-accessibility.md) |
| 렌더·메모리·네트워크 예산 | [12-performance-budget.md](12-performance-budget.md) |
| 코드 위치·스택 | [13-frontend-repository-layout.md](13-frontend-repository-layout.md) |
| 구현 순서·첫 커밋 범위 | [14-implementation-phases.md](14-implementation-phases.md) |
| 시각 일치 검증·테스트 계층 | [15-verification-and-visual-regression.md](15-verification-and-visual-regression.md) |
| 재검수·완료 판정 | [16-review-and-definition-of-done.md](16-review-and-definition-of-done.md) |
| 미결정 사항 확인 | [17-open-decisions.md](17-open-decisions.md) |
| Domain 불변조건과의 충돌 판단 | [18-domain-invariant-compliance.md](18-domain-invariant-compliance.md) |

## 문서 목록

| 문서 | ID | 소유 범위 |
|---|---|---|
| [00-visual-source-of-truth.md](00-visual-source-of-truth.md) | `DXB-WEB-000` | 시각 기준 파일과 값 추출 규칙 |
| [01-scope-and-ownership.md](01-scope-and-ownership.md) | `DXB-WEB-001` | 목표·비범위·Canonical Ownership·Tier |
| [02-screen-inventory.md](02-screen-inventory.md) | `DXB-WEB-002` | 화면 영역 구조와 구성 요소 |
| [03-information-architecture.md](03-information-architecture.md) | `DXB-WEB-003` | 내비게이션·route·UI↔Domain 용어 매핑 |
| [04-design-tokens.md](04-design-tokens.md) | `DXB-WEB-004` | token key 체계와 상태 표현 매핑 |
| [05-component-inventory.md](05-component-inventory.md) | `DXB-WEB-005` | 컴포넌트 계층과 분리 규칙 |
| [06-responsive-layout.md](06-responsive-layout.md) | `DXB-WEB-006` | breakpoint·폼팩터 전환·검증 viewport |
| [07-theme-and-color-mode.md](07-theme-and-color-mode.md) | `DXB-WEB-007` | theme 결정 규칙 |
| [08-api-and-state-architecture.md](08-api-and-state-architecture.md) | `DXB-WEB-008` | 전송 경로 선행 조건·상태 분류·실시간 |
| [09-type-contract.md](09-type-contract.md) | `DXB-WEB-009` | 타입 생성 경로와 금지 사항 |
| [10-error-and-exceptional-state.md](10-error-and-exceptional-state.md) | `DXB-WEB-010` | 예외 상태 목록과 표현 위치 |
| [11-accessibility.md](11-accessibility.md) | `DXB-WEB-011` | 접근성 최소 기준 |
| [12-performance-budget.md](12-performance-budget.md) | `DXB-WEB-012` | 렌더·메모리·네트워크 규칙 |
| [13-frontend-repository-layout.md](13-frontend-repository-layout.md) | `DXB-WEB-013` | 코드 배치와 스택 (status: Proposed) |
| [14-implementation-phases.md](14-implementation-phases.md) | `DXB-WEB-014` | Phase 0–9와 우선순위 |
| [15-verification-and-visual-regression.md](15-verification-and-visual-regression.md) | `DXB-WEB-015` | 시각 일치 판정과 테스트 계층 |
| [16-review-and-definition-of-done.md](16-review-and-definition-of-done.md) | `DXB-WEB-016` | 2회 재검수와 완료 기준 |
| [17-open-decisions.md](17-open-decisions.md) | `DXB-WEB-017` | 미결정 사항 추적 |
| [18-domain-invariant-compliance.md](18-domain-invariant-compliance.md) | `DXB-WEB-018` | `INV-001`~`INV-015`가 UI에 부과하는 제약 |

## 상위 규약과의 관계

- 저장소 전체 규약: [AGENTS.md](../../AGENTS.md)
- Agent Guide Router: [docs/agent/guide-index.md](../agent/guide-index.md)
- 문서 작성 규칙: [docs/agent/documentation-rules.md](../agent/documentation-rules.md)
- 제품 의미·상태기계·계약의 Canonical Owner: 활성 [docs/plan](../plan/) 패키지

이 패키지는 **Web 표현 계층**을 소유한다. Domain 상태·정책·계약 의미를 소유하지 않으며 override하지 않는다. 충돌 시 활성 plan의 Owner 문서가 우선한다.

### 범위 위치 — Web은 활성 P0 비범위다

활성 기준선 `DXB-BASE-000` §3은 `TUI, Web, BFF, remote multi-tenant transport, ...`를 Active P0 비범위로 선언한다. 따라서

- 이 패키지의 `priority: web-P0`는 **Web 패키지 내부 우선순위**이며 plan의 P0 Gate와 같은 층위가 아니다.
- `normative: true`는 **Web 표현 계층에 한정**된 규범성이다.
- gateway 도입처럼 P0 비범위 표면을 실제로 만드는 결정은 plan 범위 결정을 먼저 받는다.

자세한 근거는 [01-scope-and-ownership.md](01-scope-and-ownership.md) §0이 소유한다.

## 현재 상태 요약

- 시안 확정 범위: `workspace-cockpit` 1개 화면 (Desktop / Mobile layout, Dark / Light)
- 구현 코드: 없음. Frontend 배치는 [13-frontend-repository-layout.md](13-frontend-repository-layout.md)의 제안 상태
- 실제 데이터 연결: **차단됨.** 브라우저용 HTTP/streaming gateway와 wire schema generator가 없고, Flow catalog/Process list/event/Agent/metric read contract도 일부 부재한다. Web/BFF는 활성 P0 비범위다 (`WEB-OQ-005`, `006`, `016`)
- Task/Process watch operation은 존재하지만 browser streaming transport는 없고, command별 freeze 지점과 전체 M6 freeze를 구분해야 한다 ([08-api-and-state-architecture.md](08-api-and-state-architecture.md) §1.4)
- 불변조건 충돌 미해결: `Active Agents` 의미 (`WEB-OQ-003`)
- 열린 결정: 17건 ([17-open-decisions.md](17-open-decisions.md))
