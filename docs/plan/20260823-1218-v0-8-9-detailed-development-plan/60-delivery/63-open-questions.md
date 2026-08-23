---
title: "v0.8.9 결정 기록과 후속 질문"
document_id: "DXB-DEL-063"
version: "0.8.9"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-GOV-003", "DXB-DEL-060", "DXB-IFC-040", "DXB-IFC-041", "DXB-ARC-018"]
---
# v0.8.9 결정 기록과 후속 질문

## Resolved

- active package가 INV-001~015와 ADR-0095~0132를 직접 열거한다.
- security/recovery fail-closed 의미를 과거 package 문장에 위임하지 않는다.
- 하나의 `in-*` metadata에서 `CliInput`과 `CommandPayload`를 생성한다.
- `ready_at`, `timeout`, `all`, `output`, `confirmation`은 local-only다.
- ContentSource의 path/fd/source variant는 wire와 RequestDigest에서 제거한다.
- `CommandId`는 Instance-global, IdempotencyKey는 Principal/issuance epoch-bound다.
- full Receipt compaction 뒤에도 acceptance/recovery horizon 동안 최소 tombstone을 유지한다.
- local journal은 command별 single writer이며 writer 종료 뒤 검증된 takeover만 허용한다.
- SIGINT/broken pipe/local timeout은 observation 종료이며 implicit Runtime cancel이 아니다.
- Acceptance는 하나의 최초 milestone만 가지며 Gate는 누적된다.
- `AT-VERSION-001`과 M6 `AT-SCHEMA-001`을 분리한다.
- validator negative fixture는 current value를 동적으로 변이한다.

## Deferred implementation choices

concrete storage/Harness 제품, numeric retention/byte/timeout/compatibility window, stronger same-UID isolation, TUI/Web/Plugin lifecycle CLI는 executable evidence와 별도 ADR 대상이다. 이 항목들은 Workspace/M1A/M1B 진입을 막지 않으며 현재 CLI contract에 새 기능을 추가하지 않는다.

## Blocking semantic questions

CLI까지의 기존 P0 scope를 구현하기 위한 미결 규범 질문은 없다. 실제 Acceptance가 PASS하기 전 M2 이후 subset freeze는 계속 BLOCKED다.
