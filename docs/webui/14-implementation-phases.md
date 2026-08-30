---
title: "Web UI 구현 단계와 우선순위"
document_id: "DXB-WEB-014"
status: "Accepted"
normative: true
priority: "web-P0"
plan_baseline: "0.8.10"
last_updated: "2026-08-30"
owner: "Web Control Center 문서 패키지"
depends_on: ["DXB-WEB-002", "DXB-WEB-013"]
---
# Web UI 구현 단계와 우선순위

## 목적

구현 순서와 각 단계의 완료 조건을 소유한다. 단계 구분의 기준은 "시안의 어느 부분이 확정되는가"다.

## 1. 현재 구현 상태와 단계 원칙

현재 Frontend source/package manifest는 없으며 **Phase 0 착수 전**이다. 현 코드로 가능한 것은 문서·PNG 기반 visual decomposition과 typed mock fixture 준비까지다. 실제 Runtime 연결은 Web/BFF 범위 결정, browser gateway, wire schema generator, `WEB-OQ-016` contract gap 해소 없이는 시작할 수 없다.

시안이 단일 화면이므로 단계는 화면이 아니라 영역으로 나눈다

이전 계획은 4개 화면을 순차 구현하는 구조였다. 시안은 단일 Cockpit이므로([02-screen-inventory.md](02-screen-inventory.md)) 단계는 **Cockpit의 영역 단위**로 나눈다. 화면별 순차 구현은 적용하지 않는다.

## 2. Phase 0 — Visual Decomposition (선행, 코드 없음)

산출물:

- `visual-spec` 근거 표 ([00-visual-source-of-truth.md](00-visual-source-of-truth.md) §4)
- screen inventory 확정 (이미 [02-screen-inventory.md](02-screen-inventory.md)에 있음, 측정값으로 보강)
- component inventory 확정 ([05-component-inventory.md](05-component-inventory.md))
- token draft ([04-design-tokens.md](04-design-tokens.md))

완료 조건: 모든 token에 sample 출처 좌표가 있다.

## 3. Phase 1 — Shell + 스택 적합성 검증

`WEB-OQ-007`을 닫기 위한 bounded spike를 먼저 수행한다. Expo/RNW는 후보일 뿐이므로 승인 전 dependency/bootstrap을 production layout으로 고정하지 않는다. 후보가 통과하면 theme provider, token, typography, `app-shell`, `nav-rail`, `global-header`, `mobile-app-header`, `mobile-tab-bar`, 반응형 유틸을 구현한다.

spike에는 최소 `data-table`(Run Queue 열 구성)과 `workflow-graph`(Desktop/Mobile 차이를 포함한 병렬 그룹)를 포함한다. 결과가 PNG 밀도·keyboard/focus·mobile layout을 만족할 때만 [13-frontend-repository-layout.md](13-frontend-repository-layout.md)를 Accepted로 올린다.

완료 조건:

- Desktop 4-column과 Mobile 세로 구조가 시안 비율과 일치한다.
- Dark ↔ Light 전환이 구조 변화 없이 동작한다.
- 테이블과 그래프의 시안 재현 가능성이 확인되었다. 불가하면 여기서 스택 결정을 재검토한다.

## 4. Phase 2 — Runtime 상태 영역

구현: `card`, `key-value-row`, `entity-status-row`, `event-row`, `resource-row`, `status-dot`, `badge`, `pill`, `progress-bar`, `sparkline` + `bot-state-card`, `agent-list-card`, `recent-events-card`, `resource-health-card`.

이 단계에서 상태 표현 매핑([04-design-tokens.md](04-design-tokens.md) §5)을 확정한다. 이후 모든 영역이 이 매핑을 재사용한다.

완료 조건: 우측 컬럼 4개 카드가 Desktop 시안과 일치하고, Mobile `System / Agents` 타일과 stat strip이 동작한다. 추가로 **Project 없음 / Durable Process 없음 / Agent 없음 fixture에서도 Cockpit이 동작**해야 한다(`INV-012`, [18-domain-invariant-compliance.md](18-domain-invariant-compliance.md) §2).

`Active Agents` 카드는 대상 의미가 확정되기 전(`WEB-OQ-003`)까지 실제 데이터를 결박하지 않는다.

## 5. Phase 3 — Workflow Overview

구현: `workflow-graph`, `workflow-node`, `workflow-io-node`, `workflow-parallel-group`, `workflow-edge`, `legend`, 확대 동작.

완료 조건:

- 시안의 노드 배치·연결선·병렬 그룹·legend가 재현된다.
- 노드 수 변화에 배치가 대응한다(좌표 하드코딩 없음).
- Mobile 압축 배치가 그래프 형태를 유지한다.

## 6. Phase 4 — Run Queue

구현: `data-table`, `tab-bar`, 검색/필터, 행 액션, 선택 행 강조, `data-table-compact-row`, footer 링크.

완료 조건: Desktop 테이블과 Mobile 압축 행이 시안과 일치하고, 가로 스크롤이 없다.

## 7. Phase 5 — Thread / Conversation

구현: `message-list`(가상화), `message-item`, `message-embed-flow-triggered`, `message-embed-run-progress`, `message-composer`, streaming-interrupted visual state, 재시도 표현. 현재 public contract에는 model token stream이 없으므로 Task watch를 token streaming으로 오해하지 않으며, live token streaming은 별도 Interface contract 없이는 fixture 범위를 넘지 않는다.

완료 조건:

- 메시지 항목 경계가 분리되어 메시지 추가가 전체 재렌더를 유발하지 않는다.
- composer가 고정되고 모바일 키보드에 가려지지 않는다.
- 스트리밍 중단 상태가 표현된다([10-error-and-exceptional-state.md](10-error-and-exceptional-state.md)).

## 8. Phase 6 — 예외 상태 전면 적용

[10-error-and-exceptional-state.md](10-error-and-exceptional-state.md) §1의 모든 상태를 각 영역에 적용한다. 섹션 단위 실패 격리를 검증한다.

완료 조건: 상태 주입 테스트가 모든 상태에 존재한다.

## 9. Phase 7 — 실제 Runtime 연결

**선행 조건 (모두 충족 필요)**:

1. plan 범위 결정 — `DXB-BASE-000` §3이 Web/BFF를 P0 비범위로 선언하므로 gateway 도입은 plan 범위 결정을 먼저 받는다([08-api-and-state-architecture.md](08-api-and-state-architecture.md) §1.3).
2. HTTP/streaming gateway 존재 + 인증 경로 확정.
3. `M5` 도달 — Task/Process watch(subscription) 표면이 M5 범위다([08-api-and-state-architecture.md](08-api-and-state-architecture.md) §1.4).
4. 타입 생성 경로 확정. 현재 wire JSON Schema exporter가 없으며 `M6` 이전에는 생성 타입 변동을 가정한다.
5. 연결하려는 Cockpit 영역에 stable read/query contract 존재. Flow catalog, Process list, event timeline, Agent/metric projection 공백은 `WEB-OQ-016`에서 먼저 해결한다.

하나라도 미충족이면 해당 영역의 live 연결을 시작하지 않는다. 기존 query가 있는 Bot/Task/Conversation 영역과 contract gap 영역을 한 번에 완료 처리하지 않는다. mock 상태에서 "연결 완료"로 표시하지 않는다.

순서:

```text
1. wire 타입 생성 경로 확정
2. current capability matrix([08-api-and-state-architecture.md](08-api-and-state-architecture.md) §1.4)에서 지원되는 read query만 연결
3. error mapping (Receipt / stable error / exit category 구조)
4. loading / empty / partial 상태 연결
5. mutation + Receipt 판정 (Receipt 축과 Domain outcome 축 분리)
6. subscription 연결
7. 재연결 / cursor 재개
8. stale·resync 복구
9. mock 제거
```

## 10. Phase 8 — Mobile Hardening

360–430px 폭에서 재검증한다. overflow, 터치 타겟, safe-area, 키보드 오버레이, 하단 composer, 긴 텍스트 줄바꿈, 화면 간 이동을 확인한다.

## 11. Phase 9 — Polish

hover / pressed / focus / transition / skeleton / reconnecting 표현을 마감한다. 애니메이션은 상태 이해에 필요한 범위로 제한하고 reduced motion을 존중한다.

## 12. 우선순위 요약

아래 `web-Px`는 Web 패키지 내부 우선순위다. 활성 plan의 P0 Gate나 milestone과 같은 층위가 아니다([01-scope-and-ownership.md](01-scope-and-ownership.md) §0).

```text
web-P0  Phase 0–1   token / shell / navigation / 스택 검증
web-P1  Phase 2–4   상태 카드 / Workflow 그래프 / Run Queue
web-P2  Phase 5     Conversation / 스트리밍
web-P3  Phase 6     예외 상태 전면 적용
web-P4  Phase 7     실제 Runtime 연결 (plan 범위 결정 + gateway + M5 선행)
web-P5  Phase 8–9   Mobile / 접근성 / 시각 회귀 마감
```

web-P0 완료 전에 영역별 개별 styling을 확장하지 않는다.

## 13. 권장 첫 커밋 범위

```text
WEB-OQ-007 stack spike + 결정 기록
+ 승인된 apps/control-center bootstrap
+ theme / token (visual-spec 근거 포함)
+ app-shell / nav-rail / global-header / mobile-tab-bar
+ 최소 data-table + 최소 workflow-graph 시험 구현
+ Dark/Light 전환
```

이 커밋에서 시안의 골격과 스택 적합성을 동시에 고정한다.

## 관련 문서

- [15-verification-and-visual-regression.md](15-verification-and-visual-regression.md) — 각 단계 검증
- [16-review-and-definition-of-done.md](16-review-and-definition-of-done.md) — 완료 판정
