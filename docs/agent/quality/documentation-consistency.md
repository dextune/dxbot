# Documentation Consistency Guide

## Applies When

문서 추가·이동·삭제, public contract/state/owner 변경, Agent Guide 구조 변경에 적용한다.

## Core Rules

같은 규칙을 여러 문서에 복사하지 않고 Owner와 relative link를 사용한다. Agent Guide는 작업 방법, 활성 Plan은 버전별 제품 의미를 소유한다. Router는 링크/선택 기준 중심으로 유지한다.

Guide 구조는 `AGENTS → guide-index → category index → detailed guide → canonical source`를 따른다. orphan Guide와 category를 건너뛰는 무분별한 root 링크를 피한다.

## Verification

- lowercase kebab-case와 예외
- broken/escaping relative link
- duplicate rule/Canonical Owner
- Router→category→detail 도달 가능성
- detail의 Related/Canonical link
- code/API/schema/config/test와 모순
- 이동 후 stale inbound reference
- version-specific 사실이 장기 Guide에 고정되지 않았는지

문서 구조 변경도 [review-completion.md](review-completion.md)의 두 재검수를 수행한다.
