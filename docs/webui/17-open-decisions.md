---
title: "Web UI 미결정 사항"
document_id: "DXB-WEB-017"
status: "Accepted"
normative: false
priority: "web-P0"
plan_baseline: "0.8.10"
last_updated: "2026-08-30"
owner: "Web Control Center 문서 패키지"
---
# Web UI 미결정 사항

## 목적

확정되지 않은 사항을 추적한다. 미결정을 규범처럼 서술하지 않기 위해 다른 문서는 여기로 참조를 보낸다. 열려 있는 항목이 걸린 기능은 완료로 선언하지 않는다([16-review-and-definition-of-done.md](16-review-and-definition-of-done.md) §4).

## 항목

| ID | 내용 | 영향 | 결정 필요 시점 | 상태 |
|---|---|---|---|---|
| `WEB-OQ-001` | Mobile stat strip 4셀의 canonical metric 집합과 `header-metric-cell` 지원 표현. Dark 시안(Bot / State / Dynamic Core / Sessions)은 dot·version·sparkline을, Light 시안(Bot / Run / Memory / Progress)은 link·percent·progress bar를 사용한다 | Mobile 상단 정보 구성과 metric cell component contract | Phase 1 | Open |
| `WEB-OQ-002` | Mobile 중앙 FAB(`+`)가 노출할 생성 액션 집합 | Mobile 주요 동작 경로 | Phase 1 | Open |
| `WEB-OQ-003` | UI `Agent`의 Domain 대응. 후보는 admitted Execution / Core Lease / Provider registration / delegation 대상 Bot. **`INV-003`(Core는 독립 Agent 아님), `INV-014`(anonymous sub-agent 축약 금지)와 충돌 가능** | Active Agents 카드, `/agents` route, Mobile System/Agents 타일 | Phase 2 | **Open — 불변조건 충돌** |
| `WEB-OQ-004` | Global header `Current Branch`의 Domain 대응. 현재 Domain에 대응 개념이 없다 | header 셀, 값의 출처 | Phase 2 | Open |
| `WEB-OQ-005` | 브라우저용 HTTP + streaming projection gateway의 소유·설계·인증. control plane은 Unix domain socket 전용이며, **`DXB-BASE-000` §3은 Web/BFF를 Active P0 비범위로 선언**하므로 plan 범위 결정이 선행된다 | 실제 데이터 연결 전체(Phase 7) | Phase 7 시작 전 | **Open — Blocker** |
| `WEB-OQ-006` | Rust contract → TypeScript wire 타입 생성 경로. 현재 contract snapshot에는 `@local` CLI field가 포함될 수 있고 별도 wire JSON Schema exporter/artifact는 없음. generator·artifact owner와 M6 freeze 관계를 결정해야 함 | 타입 계약, mock 제거 | Phase 7 시작 전 | **Open — Blocker** |
| `WEB-OQ-007` | Expo + React Native Web으로 Desktop 밀도(테이블·그래프·hover·focus)를 재현 가능한지 | 스택 유지 여부 | Phase 1 종료 시 | Open |
| `WEB-OQ-008` | 시안 없는 화면(`Bots`, `Flows`, `Tasks`, `Memory`, `Settings`, `/runs`, `/events`, `/agents`)의 디자인 확정 방식. 추가 시안을 받을 것인가, 파생 규칙으로 갈 것인가 | 해당 화면 구현 착수 시점 | 해당 화면 착수 전 | Open |
| `WEB-OQ-009` | `sample/mobile-main-dark.png.png` 파일명 정리 여부(이중 확장자) | 문서 참조 정합성 | 아무 때나. rename 시 [00-visual-source-of-truth.md](00-visual-source-of-truth.md) §1 표 동시 갱신 | Open |
| `WEB-OQ-010` | 이 문서 패키지를 활성 `docs/plan/` 패키지의 Interface 문서로 승격할지, `docs/webui/`에 독립 유지할지. Web이 P0 비범위인 동안은 독립 유지가 자연스럽고, 범위에 편입되는 시점에 승격이 필요하다 | 문서 소유권과 검증 스크립트 적용 범위 | Web이 plan 범위에 편입될 때 | Open |
| `WEB-OQ-011` | Global header `Sessions 4 active`가 어떤 session인지. `INV-005`는 Interface Session / Provider Session / Core Lease / Conversation / Thread / Task / Execution을 서로 다른 identity로 규정한다 | header 셀, Mobile stat strip | Phase 2 | Open |
| `WEB-OQ-012` | `Resource / Health` 카드의 CPU / Memory / Disk / Network 데이터 출처. `DXB-RUN-031`은 Runtime process-wide memory/cost/concurrency와 admission을 소유하며 host disk/network 지표를 정의하지 않는다. `DXB-RUN-034`는 metric label 제약을 둔다 | Resource/Health 카드 | Phase 2 | Open |
| `WEB-OQ-013` | `Memory State Healthy 92%`의 지표 정의. `DXB-DOM-022`는 assertion/scope/provenance를 소유하며 health 퍼센트를 정의하지 않는다. `INV-007`에 따라 Conversation History를 Memory로 표시할 수 없다 | header 셀, Mobile stat strip | Phase 2 | Open |
| `WEB-OQ-014` | Desktop/Mobile Workflow의 node set과 라벨 차이. Mobile은 INPUT을 생략하고 순차 `4. Update Database`를 추가하며 `Update DB`, `Notify Team`으로 축약한다 | graph data model, responsive layout, visual fixture | Phase 1 종료 전 | **Open — visual/data ambiguity** |
| `WEB-OQ-015` | Run Queue의 total badge와 visible rows 관계. Desktop 5행, Mobile 4행, badge는 모두 5이며 stable Process list query도 현재 없음 | compact list truncation, pagination, accessibility | Phase 4 전 | Open |
| `WEB-OQ-016` | Cockpit live read contract gap. 현재 stable operation에는 Flow catalog/definition, Process list(Run Queue), audit event timeline, Agent, 시안과 동일한 host metric projection이 없음 | 전체 live Cockpit; gateway만으로 해결 불가 | Phase 7 시작 전 | **Open — Blocker** |
| `WEB-OQ-017` | Desktop Favorites 3개 항목의 Mobile 접근 경로. Mobile 시안에는 직접 대응 영역이 없으며 Menu 포함 여부도 확정되지 않음 | navigation parity | Phase 1 종료 전 | Open |

## 결정 기록 규칙

1. 결정되면 상태를 `Decided`로 바꾸고 결정 내용과 근거를 한 줄로 남긴다.
2. 결정 결과가 규범이면 **해당 Owner 문서로 이동**시키고 여기에는 링크만 남긴다. 규범을 이 문서에 복제하지 않는다.
3. 결정 없이 구현을 진행해야 할 때는 임시 선택임을 코드와 이 표에 함께 남긴다.

## 관련 문서

- [index.md](index.md) — 문서 Router
- [01-scope-and-ownership.md](01-scope-and-ownership.md) — `WEB-OQ-005`, `WEB-OQ-010`의 범위 근거
- [08-api-and-state-architecture.md](08-api-and-state-architecture.md) — `WEB-OQ-005`, `WEB-OQ-006`
- [13-frontend-repository-layout.md](13-frontend-repository-layout.md) — `WEB-OQ-007`
- [02-screen-inventory.md](02-screen-inventory.md) — `WEB-OQ-008`
- [18-domain-invariant-compliance.md](18-domain-invariant-compliance.md) — `WEB-OQ-003`, `WEB-OQ-011`, `WEB-OQ-013`의 불변조건 근거
