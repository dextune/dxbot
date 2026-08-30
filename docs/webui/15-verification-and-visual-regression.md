---
title: "Web UI 검증과 Visual Regression"
document_id: "DXB-WEB-015"
status: "Accepted"
normative: true
priority: "web-P0"
plan_baseline: "0.8.10"
last_updated: "2026-08-30"
owner: "Web Control Center 문서 패키지"
depends_on: ["DXB-WEB-000", "DXB-WEB-006"]
---
# Web UI 검증과 Visual Regression

## 목적

시안 일치 여부를 어떻게 판정하는지, 그리고 어떤 테스트 계층을 유지하는지를 소유한다.

## 1. 시각 일치 판정 기준

각 화면·영역을 `sample/*.png`와 다음 항목으로 비교한다.

```text
전체 layout 비율
영역 폭/높이 비율
컴포넌트 위치와 정렬
spacing 단계
typography 계층 (size / weight / line-height)
색과 대비 계층
border radius / border 두께
그림자 유무와 강도
icon 크기·stroke·위치
카드 밀도(행 높이, padding)
상태 표현(아이콘 모양 + 색 + 텍스트)
반응형 전환 결과
```

허용/불허 구분:

| 차이 | 판정 |
|---|---|
| font anti-aliasing, subpixel 렌더 차이 | 허용 |
| 브라우저 기본 스크롤바 폭 | 허용 (문서화 후) |
| 위치·크기·간격·정렬 차이 | **불허 — 수정** |
| 색 계층·대비 관계 차이 | **불허 — 수정** |
| 컴포넌트 형태·아이콘 모양 차이 | **불허 — 수정** |
| 밀도(행 높이, padding) 차이 | **불허 — 수정** |

## 2. 절차

1. 시안과 동일 viewport에서 구현 스크린샷을 생성한다. Desktop은 1448 × 1086을 사용한다. Mobile은 941 × 1672 전체 device mockup을 browser viewport로 쓰지 않고 Phase 0에서 측정한 **프레임 내부 content viewport**를 사용하며, frame/status bar/home indicator는 diff 대상에서 제외한다.
2. reference와 구현을 overlay 또는 image diff로 비교한다.
3. 큰 구조 차이(영역 폭, 컬럼 수, 카드 순서)를 먼저 제거한다. 픽셀 미세 차이 조정은 그다음이다.
4. 차이를 해결할 때 시안을 바꾸지 않는다. 구현을 바꾼다.
5. 시안을 바꿔야 한다는 결론이 나오면 디자인 변경으로 취급하고 [00-visual-source-of-truth.md](00-visual-source-of-truth.md) §1 표(파일 + 해시)를 갱신한 뒤 baseline을 재승인한다.

## 3. Baseline 관리

- baseline은 viewport × 색 모드 조합으로 유지한다.
- baseline 변경은 디자인 변경이 확정된 경우에만 허용한다. "구현이 이렇게 되어서" 갱신하지 않는다.
- 시안 없는 화면([02-screen-inventory.md](02-screen-inventory.md) §4)은 pixel baseline 대상이 아니다. token 준수와 컴포넌트 재사용으로 검증한다.

검증 viewport 목록은 [06-responsive-layout.md](06-responsive-layout.md) §5가 소유한다. 이 문서에 복제하지 않는다.

## 4. 테스트 계층

**Unit**

```text
token resolver
status presentation 매핑
view-model mapper
event 적용(idempotency, 순서, cursor)
날짜·경과 시간 포맷
```

**Component**

각 컴포넌트에 대해 다음 조합을 검증한다.

```text
Dark / Light
loading / empty / error
disabled
키보드 포커스 및 라벨
긴 텍스트 / 긴 이름 / 큰 숫자
알 수 없는 상태 값
```

**Integration (Cockpit 영역 단위)**

```text
정상 데이터
빈 데이터
부분 실패 (한 섹션만 실패)
전체 실패
실시간 이벤트 반영
중복 이벤트 전달
재연결 및 cursor 재개
권한 거부 / 승인 대기
```

**Contract**

```text
wire schema generator가 도입된 뒤 생성 타입과 canonical wire artifact의 drift 0
contract snapshot의 `@local` field가 Web wire type에 포함되지 않음
알 수 없는 enum 값 허용
nullable 필드 처리
응답 호환성 회귀
```

## 5. 로컬 검증 범위

구현 후 프로젝트가 채택한 package manager와 script를 기준으로 다음 범주를 로컬에서 수행한다.

```text
format
lint
typecheck
unit test
component test
integration test
web production build / export
대표 viewport visual regression
```

GitHub Actions를 검증 수단으로 전제하지 않는다. 필요한 검증은 로컬 또는 허용된 독립 환경에서 수행한다. Frontend 검증 실패가 Rust CI를 오염시키지 않도록 스크립트 경계를 분리한다([13-frontend-repository-layout.md](13-frontend-repository-layout.md) §5).

## 관련 문서

- [00-visual-source-of-truth.md](00-visual-source-of-truth.md) — 기준 파일과 해시
- [06-responsive-layout.md](06-responsive-layout.md) — viewport 목록
- [16-review-and-definition-of-done.md](16-review-and-definition-of-done.md) — 완료 판정
