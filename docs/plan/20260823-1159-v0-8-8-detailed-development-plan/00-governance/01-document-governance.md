---
title: "문서 거버넌스와 active package 통제"
document_id: "DXB-GOV-001"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-BASE-000"]
---
# 문서 거버넌스와 active package 통제

## 1. 버전·상태 축

- `plan_version`: active package semantic version
- `review_revision`: 같은 plan line에 대한 단조 증가 교정 번호이며 review pass 수가 아니다.
- `adversarial_review_rounds`: 서로 다른 공격 관점으로 수행한 검토 수
- `final_rechecks`: 변경 완료 후 구조·실행 흐름을 다시 확인한 횟수
- `document_version`: 개별 Canonical Owner 의미 버전

`Resolved`는 문서 의미가 닫혔다는 뜻이고, `Executable`은 fixture가 존재한다는 뜻이며, `Passed`는 그 fixture가 특정 commit에서 실행됐다는 뜻이다. 세 상태를 섞지 않는다.

## 2. Active package 발견

Repository validator는 `docs/plan/*-detailed-development-plan/readme.md`에서 다음 조건을 만족하는 package 하나를 선택한다.

1. `document_id=DXB-INDEX`
2. `status=Accepted`
3. 가장 높은 semantic version
4. `package_path`와 실제 경로 일치
5. 동일 최고 버전 후보 하나

active package의 normative dependency는 package 안에서 닫혀야 한다. 과거 package 문장을 전제로 한 `기존 의미`, `이전 표`, `원래 코드` 참조를 금지한다.

## 3. Canonical Owner와 변경 세트

같은 의미의 상태·정책·default·schema는 Owner 하나가 전문을 소유한다. 다른 문서는 ID와 상대 링크로 참조한다.

Tier A 변경은 다음을 같은 변경 세트에서 추적한다.

```text
Baseline → ADR → Owner state machine
→ Application operation metadata → CLI input/output/error/exit
→ Storage/Security/Recovery/Resource
→ Test/Acceptance/Risk/Roadmap
→ Manifest/Validator/Workflow
```

## 4. Review evidence

review revision 숫자만으로 검수를 주장하지 않는다. 매니페스트는 각 pass마다 다음을 기록한다.

```text
review_id / 독립 목적 / 검사 범위 / 발견·수정 / 결과
```

최소 재검수는 별도의 Structural Review와 Cross-Layer Executability Review다. 사용자 요청이 더 많은 review를 요구하면 목적이 겹치지 않는 evidence를 추가한다.

## 5. Validator의 정확한 역할

Validator가 자동 판정하는 것:

- required frontmatter와 allowed status/priority/date
- active package path/version/review metadata
- lowercase kebab-case, unique ID, dependency DAG
- manifest inventory와 review evidence count
- Canonical Owner registry key uniqueness
- command/input registry row schema와 exact key/schema/Acceptance relation
- exit registry 완전성
- Acceptance ID 존재와 status field
- internal relative link
- negative fixtures

Validator가 자동으로 증명하지 않는 것:

- 상태기계의 실제 Rust 구현
- crash atomicity, authorization correctness, bounded memory
- real Harness 동작
- 문장 간 모든 semantic contradiction

따라서 validator PASS를 release evidence 전체로 확대 해석하지 않는다.

## 6. 변경 완료 조건

문서 변경은 active package, validator, workflow, Acceptance/Risk/Manifest를 함께 갱신하고 5개 적대적 검토와 2개 최종 재검수 evidence가 요구된 경우 모두 기록한 뒤 완료한다.
