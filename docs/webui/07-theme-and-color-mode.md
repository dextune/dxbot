---
title: "Web UI Theme와 Color Mode"
document_id: "DXB-WEB-007"
status: "Accepted"
normative: true
priority: "web-P1"
plan_baseline: "0.8.10"
last_updated: "2026-08-30"
owner: "Web Control Center 문서 패키지"
depends_on: ["DXB-WEB-004"]
---
# Web UI Theme와 Color Mode

## 목적

Dark / Light 모드의 결정 규칙과 구현 제약을 소유한다.

## 1. Canonical Owner

theme 선택은 **Frontend UI preference**다. Runtime Domain 상태가 아니다. 따라서 Rust Runtime에 theme을 저장하거나 조회하지 않는다.

지원 값:

```text
system | light | dark
```

해석 우선순위:

1. 사용자가 명시적으로 선택한 값
2. OS / 브라우저 system 설정
3. 기본값 = `dark`

기본값 `dark`는 Primary 시안에서 출발한 **Web 표현 계층 결정**이다. [00-visual-source-of-truth.md](00-visual-source-of-truth.md)는 파일 우선순위만 소유하며 theme preference 정책을 소유하지 않는다.

## 2. 구현 규칙

1. Dark와 Light를 별도 스타일셋으로 작성하지 않는다. 동일 semantic token key에 값만 교체한다([04-design-tokens.md](04-design-tokens.md) §2).
2. 레이아웃·간격·타이포·컴포넌트 구조는 모드 간 동일하다. 모드에 따라 요소를 추가·제거하지 않는다.
3. 초기 paint 전에 theme을 해석해 **flash를 만들지 않는다.** Web에서는 앱 부트스트랩 단계에서 해석하고, 그 전에 색이 있는 콘텐츠를 그리지 않는다.
4. 사용자 선택은 로컬에 영속화하고, 새로고침 후 동일 모드로 복구한다.
5. system 모드에서 OS 설정이 런타임에 바뀌면 즉시 반영한다.
6. 이미지·아이콘은 모드별 별도 asset을 만들지 않고 currentColor 또는 token 기반 색을 사용한다. 불가피한 경우에만 모드별 asset을 두고 근거를 남긴다.

## 3. 검증

- 두 모드에서 동일 viewport 스크린샷의 **구조 diff가 0**인지 확인한다. 색 외 차이가 있으면 결함이다.
- 각 모드에서 텍스트 대비 요구를 만족하는지 확인한다([11-accessibility.md](11-accessibility.md)).
- 모드 전환 시 재렌더 범위가 전체 트리로 번지지 않는지 확인한다([12-performance-budget.md](12-performance-budget.md)).
- 새로고침·초기 로드에서 flash가 없는지 확인한다.

## 관련 문서

- [04-design-tokens.md](04-design-tokens.md) — token 값 쌍
- [15-verification-and-visual-regression.md](15-verification-and-visual-regression.md) — 모드별 baseline
