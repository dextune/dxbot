---
title: "문서 거버넌스와 active package 통제"
document_id: "DXB-GOV-001"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-BASE-000"]
---
# 문서 거버넌스와 active package 통제

## 1. 버전·상태 축

- `plan_version`: `DXB-INDEX`가 소유하는 active package semantic version
- `review_revision`: 동일 plan line의 적대적 검수 교정 차수
- `document version`: 개별 Canonical Owner 의미의 변경 버전
- `Accepted`: 구현자가 따라야 할 승인 계약
- `Normative Baseline`: 최상위 제품 기준
- `Reference Source/Snapshot`: 비규범 provenance·외부 조사
- `Superseded/Deprecated`: active 의미가 아닌 역사 기록

`Reference Source`는 `normative:false`여야 하며 active owner를 override할 수 없다.

## 2. Active package 발견

Repository validator는 `docs/plan/*-detailed-development-plan/readme.md`를 읽고 다음을 모두 만족하는 package 하나를 선택한다.

1. `document_id=DXB-INDEX`
2. `status=Accepted`
3. 가장 높은 semantic `version`
4. frontmatter `package_path`와 실제 경로 일치
5. 동일 최고 버전 후보가 하나

개별 문서 버전이 package version과 같을 필요는 없지만, 변경된 문서는 자신의 semantic version과 `last_updated`를 갱신한다.

## 3. Effective Baseline

- active package의 normative dependency는 package 안에서 닫혀야 한다.
- 과거 package를 읽어야만 성립하는 요구는 금지한다.
- 같은 `document_id`는 active package 안에서 하나만 존재한다.
- 규칙 전문은 Canonical Owner 하나가 소유하고 다른 문서는 ID/상대 링크로 참조한다.
- history/provenance 링크는 허용하지만 normative dependency가 아니다.

## 4. 변경 세트

Tier A 의미 변경은 다음을 함께 검토한다.

```text
Baseline → ADR → Architecture/Domain/Runtime
→ Application Contract → CLI input/output
→ Storage/Security/Recovery
→ Test/Acceptance/Risk/Roadmap
→ Manifest/actual CI
```

## 5. 실제 Gate

`python3 scripts/plan-validator.py --active`가 active package, ID/DAG, path, manifest, command/input registry, Acceptance 연결과 internal link를 검사한다. `--self-test`는 duplicate ID, cycle, missing input row, manifest drift의 negative fixture가 실제 실패하는지 검증한다.

문서에 적힌 PASS 문구는 Gate evidence가 아니다. Workflow는 read-only permission으로 PR/main에서 실행한다.

## 6. Review

- Review 1: Structural / Canonical Owner / Traceability
- Review 2: Cross-Layer Executability / Crash / Security
- Review 3: Adversarial Scope / Ambiguity / Overengineering

각 Review는 발견 수정 후 전체 active package를 처음부터 재실행한다.
