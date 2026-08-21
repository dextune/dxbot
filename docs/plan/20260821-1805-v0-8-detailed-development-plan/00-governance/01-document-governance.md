---
title: "문서 거버넌스와 변경 통제"
document_id: "DXB-GOV-001"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000"]
---

# 문서 거버넌스와 변경 통제

## 1. 목적

문서·schema·코드·테스트가 서로 다른 계약으로 분기하지 않도록 Effective Baseline, Canonical Owner, 승인 상태, 변경 세트, 자동 Gate와 v0.8 전용 3회 Review를 정의한다.

## 2. 문서 상태

- `Draft`: P0 구현 Gate를 통과시키지 못하는 미확정 문서
- `Accepted`: 구현과 테스트가 따라야 하는 승인 계약
- `Normative Baseline`: 최상위 제품 기준
- `Accepted Template`: 승인된 문서 형식
- `Source of Truth`: 변경하지 않는 원문 보존본
- `Reference Snapshot`: 비규범 외부 조사 기록
- `Superseded` / `Deprecated`: 대체·제거 계약을 가진 비활성 상태

P0 normative 문서는 owner, deterministic Acceptance, Risk와 구현 단계가 연결된 경우에만 `Accepted`가 된다.

## 3. Effective Baseline 규칙

1. active package는 현재 유효한 계약을 패키지 안에 직접 포함한다.
2. 과거 패키지를 읽어야만 의미가 완성되는 참조 상속을 금지한다.
3. 동일 `document_id`의 현재 의미는 active package의 한 파일만 소유한다.
4. 보존·명확화·변경·supersession 결과는 `manifest.md` resolution matrix에 기록한다.
5. 규칙 전문은 한 Canonical Owner에만 두고 다른 문서는 ID와 상대 링크로 참조한다.
6. 원문 보존본의 역사적 TUI/Web 표현은 active milestone로 해석하지 않는다.

## 4. 변경 등급과 변경 세트

v0.8은 Tier A — Common/Application Contract 변경이다. 의미 변경은 다음을 같은 change set으로 검토한다.

`Baseline → ADR → Architecture/Domain/Runtime Owner → public schema → CLI surface → Test/Acceptance → Risk/OQ → Compatibility/Removal → Manifest`

Interface 편의를 위한 Domain rule, Authorization, Resource policy, receipt, cursor의 중복 구현은 금지한다.

## 5. Freeze Gate

다음 문서는 P0 Freeze 대상이다.

`DXB-BASE-000`, `DXB-GOV-001~004`, `DXB-ARC-010/011/014/015`, `DXB-RUN-030~035`, `DXB-IFC-040/041`, `DXB-ENG-050/052/053/054`, `DXB-DEL-060~063`, `DXB-INDEX`, `DXB-MANIFEST`.

P0 semantic Open Question이 남아 있으면 Accepted로 승격하지 않는다. Deferred P1/P2 질문은 P0 Gate와 분리한다.

## 6. 추적성 규칙

- Requirement `FR-*`, `NFR-*`, `INV-*`
- ADR `ADR-*`
- Acceptance `AT-*`
- Risk `R-*`
- Open/Resolved Question `OQ-*`

P0 Requirement는 `Owner → Acceptance → Risk → Milestone` 연결을 가져야 한다. orphan은 CI 실패다.

## 7. v0.8 3회 전체 Review

각 Review는 전체 active package를 독립적으로 검사하고, 발견 사항 수정 후 같은 범위를 처음부터 재실행한다.

### Review 1 — Effective Baseline / Structural / Traceability

file/path/ID/inventory, depends_on 존재·cycle, owner 중복, Accepted 상태, resolution matrix, Command Matrix, Requirement→Acceptance→Risk→OQ, TUI/Web active residue를 검사한다.

### Review 2 — Cross-Layer Executability / Fault / Compatibility

```text
Shell → CLI parser/selector → Host Lifecycle or Control Client
→ Application Contract → Application → Domain/Persistence
→ Runtime/Scheduler/Provider Host → Receipt/Event/Projection
→ Pagination/Subscription → Human/Machine Output
```

Runtime absent, concurrent start, stale endpoint, ambiguous selector, stale revision, permission/approval, response loss, key mismatch, crash/SIGINT, restart, cursor gap, slow/broken pipe, partial file, version mismatch, pressure를 삽입한다.

### Review 3 — Scope / Overengineering / Implementation Ambiguity

실제 P0 use-case 없는 abstraction, speculative transport/UI framework, crate 폭증, generic mutation escape hatch, policy 발명 공백, P1/P2 scope 유입을 검사한다.

## 8. 자동 Gate

CI는 active package 자동 발견, Markdown inventory, ID/depends_on/cycle, status, resolution drift, Command Matrix completeness, Acceptance/Risk/OQ orphan, schema/help/exit snapshot drift, forbidden dependency를 실행한다. 문서의 자체 PASS 문구만으로 Gate를 통과시키지 않는다.

## 9. 변경 완료 조건

- 세 Review 결과·발견·수정·재실행 evidence가 Manifest에 있다.
- code/schema/test가 아직 없으면 미실행 사실을 명확히 분리한다.
- 이전 패키지나 외부 문서를 active 계약으로 요구하지 않는다.
