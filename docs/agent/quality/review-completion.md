# Review and Completion Guide

## Applies When

의미 있는 변경을 완료 판정하기 직전에 항상 적용한다.

## Recheck 1 — Structural / Consistency

파일/경로/naming, crate dependency, Canonical Owner, duplicated state/policy/default, config/schema/migration, Guide/문서 링크, test/Acceptance/Risk 관계, orphan dependency/feature/config를 검사한다. 발견 사항 수정 후 해당 범위를 다시 확인한다.

## Recheck 2 — Cross-Layer Executability

변경과 관련된 실제 흐름을 `Input/Command → Application → Domain → Persistence → Scheduler/Runtime → Provider Host → Provider/Plugin → Recovery → Projection → Interface` 순서로 추적한다.

Provider 없음/교체/복수/제거, lifecycle drain/restart, cancellation/timeout, crash/restart, duplicate/partial success, migration/rollback, stale config/data, resource pressure를 적용 가능한 범위에서 대입한다.

## Completion Criteria

- 요청 변경이 실제 적용됨
- 영향받는 docs/test/schema/config/migration 갱신
- 실행 가능한 검증 수행
- 수행하지 못한 검증 명시
- 두 재검수 완료
- 발견 사항 수정 및 재확인
- unrelated 변경 보존

사용자 또는 범위별 규범이 추가 review를 요구하면 두 기본 재검수와 별도로 목적과 증거가 다른 review를 수행한다.
