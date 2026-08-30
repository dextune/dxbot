---
title: "Web UI 2회 재검수와 완료 기준"
document_id: "DXB-WEB-016"
status: "Accepted"
normative: true
priority: "web-P0"
plan_baseline: "0.8.10"
last_updated: "2026-08-30"
owner: "Web Control Center 문서 패키지"
depends_on: ["DXB-WEB-015"]
---
# Web UI 2회 재검수와 완료 기준

## 목적

[AGENTS.md](../../AGENTS.md) §1.10과 §4의 2회 재검수를 Web UI에 적용한 항목과 완료 판정을 소유한다. 절차의 일반 규칙은 [quality/review-completion.md](../agent/quality/review-completion.md)가 소유한다.

## 1. Review 1 — Structural / Consistency

| 항목 | 확인 내용 |
|---|---|
| 컴포넌트 중복 | 같은 의미의 컴포넌트가 두 곳에 구현되지 않았는가 |
| token 중복 | 같은 값이 다른 key로 중복 정의되지 않았는가 |
| token 근거 | 모든 token에 sample 출처가 있는가 |
| 스타일 리터럴 | 화면/컴포넌트에 hex·px 리터럴이 없는가 |
| 상태 복제 | 같은 서버 상태를 캐시와 store에 이중 보관하지 않는가 |
| Domain 정책 누출 | 화면이 권한·실행 가능 여부를 재계산하지 않는가 |
| 컴포넌트 책임 | `variant` 난립으로 의미가 다른 것을 합치지 않았는가 |
| naming | UI 라벨과 Domain 타입 이름 규칙([03-information-architecture.md](03-information-architecture.md) §4)을 지키는가 |
| 계층 경계 | route / feature / api / view-model 경계가 유지되는가 |
| 플랫폼 누수 | Web 전용 코드가 공통 파일에 있지 않은가 |
| 파일 naming | kebab-case 규칙을 지키는가 |
| dead code | 사용되지 않는 컴포넌트·토큰·mock이 남아 있지 않은가 |
| 문서 정합성 | 실제 트리·route·컴포넌트가 이 문서 패키지와 일치하는가 |

발견 사항을 수정한 뒤 관련 테스트를 다시 실행한다.

## 2. Review 2 — Cross-Layer Executability

| 흐름 | 확인 내용 |
|---|---|
| 계약 → 화면 | Rust contract → 생성 타입 → client → view model → UI가 끊기지 않는가 |
| 실시간 | 이벤트 → 캐시 갱신 → 부분 재렌더가 동작하는가 |
| 중복/순서 | 중복 이벤트·역순 이벤트에서 상태가 어긋나지 않는가 |
| 재연결 | 끊김 → 재연결 → cursor 재개 → 표시 복구가 되는가 |
| 실패/취소/재시도 | 각 경로가 UI에서 종결되는가 |
| 승인 | approval-required가 성공으로 표시되지 않는가 |
| 충돌 | conflict에서 자동 재시도 없이 사용자 결정으로 이어지는가 |
| 폼팩터 | Desktop / Tablet / Mobile에서 동일 기능에 도달 가능한가 |
| 색 모드 | Dark / Light에서 구조 차이가 없는가 |
| deep link | 직접 진입과 새로고침 후 상태 복구가 되는가 |
| 오프라인 | Runtime/Provider 미가동 상태가 정확히 표시되는가 |
| 스트리밍 | 중단·복구가 표현되는가 |
| 성능 | 대량 데이터·고빈도 이벤트에서 예산을 지키는가([12-performance-budget.md](12-performance-budget.md)) |
| 불변조건 | `INV-001`~`INV-015` 준수 항목이 통과하는가([18-domain-invariant-compliance.md](18-domain-invariant-compliance.md)) |
| Bot-only | Project·Process·Agent 없음 상태에서 Cockpit이 동작하는가(`INV-012`) |
| 상태 축 분리 | Receipt 상태와 Domain outcome을 하나의 실패로 합치지 않는가 |
| 수명 경계 | 화면 이탈·구독 해제가 실행을 취소하지 않는가 |
| 관측 안전 | 메시지 본문·secret·경로가 클라이언트 로그/telemetry로 나가지 않는가(`DXB-RUN-034`) |

발견 사항 수정 후 관련 검증을 다시 실행한다.

## 3. Definition of Done

완료 판정은 두 층으로 나눈다. **Visual Fixture Complete**는 PNG/token/component/mock 검증이며 Runtime 연결을 의미하지 않는다. **Live Control Center Complete**는 Visual Fixture Complete에 더해 gateway·wire types·필요 read contract·security·recovery 검증을 모두 요구한다. 두 판정을 한 개의 "UI 완료"로 합치지 않는다.

### 3.1 Visual Fixture Complete

시각:

- Cockpit Desktop / Mobile layout이 `sample/*.png`와 일치한다([15-verification-and-visual-regression.md](15-verification-and-visual-regression.md) §1 기준).
- Dark / Light 완전 지원, 구조 차이 없음.
- Desktop / Mobile은 승인된 reference viewport에서 일치하고, Tablet은 reference PNG 없이 [06-responsive-layout.md](06-responsive-layout.md) §3 파생 규칙·interaction reachability로 검증된다.
- Desktop / Tablet / Mobile에서 가로 스크롤·overflow 없음.
- 대표 viewport visual regression baseline 확보.

구조:

- token / primitive / composite / feature 계층 분리 완료.
- hardcoded 스타일 없음.
- 컴포넌트 인벤토리와 실제 구현 일치.

Visual fixture 품질:

- token/component/view-model unit test와 대표 visual regression이 통과한다.
- fixture shape는 `mocks/`에 격리되고 generated wire type 또는 live 연결로 표시되지 않는다.
- keyboard/focus/contrast와 reduced-motion 기준을 fixture에서 검증한다.

### 3.2 Live Control Center Complete

Visual Fixture Complete의 모든 조건에 더해 다음을 만족한다.

상태:

- Domain 상태의 중복 소유 없음.
- 생성된 계약 타입 사용.
- Domain 상태기계(Task / Process / Bot / Receipt)별 매핑 완비([03-information-architecture.md](03-information-architecture.md) §5).
- loading / empty / partial / failure / offline / reconnecting / approval / conflict 상태 구현.
- Receipt 축과 Domain outcome 축을 분리해 표시.
- Bot-only 상태(Project·Process 없음)에서 정상 동작.
- 실시간 갱신과 중복 이벤트 방어 동작.
- 주요 목록 가상화.

품질:

- Unit / Component / Integration / Contract 테스트 통과.
- 접근성 최소 기준 충족([11-accessibility.md](11-accessibility.md)).
- Review 1 완료 및 수정 반영.
- Review 2 완료 및 수정 반영.
- 코드·문서 간 구조·이름 일치.

## 4. 완료로 선언할 수 없는 조건

다음 중 하나라도 해당하면 완료가 아니다.

- 시안과의 구조적 차이를 "의도적 개선"으로 남겨둔 경우
- Visual Fixture Complete를 Live Control Center Complete로 표시한 경우
- gateway 없이 mock 상태로 실제 연결 항목을 통과 처리한 경우
- gateway만 추가하고 `WEB-OQ-016`의 read contract gap을 Frontend 추론/CLI scraping으로 메운 경우
- plan 범위 결정 없이 gateway를 도입한 경우([01-scope-and-ownership.md](01-scope-and-ownership.md) §0)
- 불변조건 충돌 항목(`WEB-OQ-003` 등)을 미결정 상태로 두고 해당 UI를 실제 데이터에 결박한 경우
- Bot-only 상태에서 Cockpit이 동작하지 않는 경우
- 예외 상태 중 일부만 구현한 경우
- token 근거 표에 출처 없는 값이 남은 경우
- 시안 없는 화면의 시각 규범을 임의로 확정한 경우

## 관련 문서

- [15-verification-and-visual-regression.md](15-verification-and-visual-regression.md) — 검증 절차
- [17-open-decisions.md](17-open-decisions.md) — 미결정 사항(열려 있으면 해당 항목은 완료 불가)
