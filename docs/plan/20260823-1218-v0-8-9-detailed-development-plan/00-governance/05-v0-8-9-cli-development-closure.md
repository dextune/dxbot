---
title: "v0.8.9 CLI 구현 관계성 수렴"
document_id: "DXB-GOV-005"
version: "0.8.9"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-BASE-000", "DXB-GOV-003", "DXB-ARC-014", "DXB-ARC-015", "DXB-ARC-018", "DXB-RUN-030", "DXB-RUN-032", "DXB-RUN-033", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042", "DXB-ENG-052", "DXB-ENG-054", "DXB-DEL-060", "DXB-DEL-061"]
---
# v0.8.9 CLI 구현 관계성 수렴

본 문서는 5회 적대적 검토의 delta와 수렴 이유를 기록한다. 실제 규범 의미는 각 Canonical Owner가 소유하며, 이 문서는 다른 Owner의 규칙을 override하는 보조 overlay가 아니다.

## 1. 5회 적대적 검토 결과

| Round | 공격 관점 | 확인된 결함 | 최소 교정 |
|---|---|---|---|
| R1 | Baseline·Owner closure | self-contained를 선언했지만 INV/ADR/security/recovery 의미를 과거 package에 위임 | Baseline과 ADR을 active package 안에 전부 열거하고 fail-closed 의미를 Owner 문서에 복원 |
| R2 | CLI input·wire projection | `--all`, `--output`, readiness/timeout이 `@argv`로 남고 ContentSource path/fd 제거 규칙이 생성 계약으로 닫히지 않음 | 하나의 `in-*` source에서 `CliInput`과 `CommandPayload`를 생성하고 local field를 `@local`로 고정 |
| R3 | Submission·recovery·concurrency | Principal-scoped CommandId만으로는 다른 UID recovery가 신규 operation을 만들 수 있고 compaction/journal takeover/interrupt 의미가 불완전 | Instance-global CommandId, principal-bound key, tombstone horizon, single-writer takeover, no implicit cancel을 명시 |
| R4 | Milestone·Acceptance | command가 요구하는 Acceptance보다 이른 milestone에서 subset freeze 가능 | Acceptance의 최초 milestone을 단일화하고 누적 Gate와 machine registry를 추가 |
| R5 | Validator·CI evidence | self-test가 `markdown_count: 49`를 하드코딩해 active v0.8.8에서 실제 변이를 만들지 못함 | current value를 regex로 변이하고 local leak, invariant/ADR coverage, milestone ordering negative fixture를 추가 |

## 2. 수렴된 CLI submission path

```text
parse CliInput
→ bounded ContentSource materialization
→ strip @local/source handle and build CommandPayload
→ allocate CommandId + principal-bound IdempotencyKey
→ RequestDigest
→ exclusive Prepared + fsync + parent durability
→ single-writer Dispatching append + fsync
→ first network byte
→ authenticate Principal
→ validate key scope
→ global CommandId + principal Idempotency binding lookup
→ [existing] stored digest compare and Receipt projection
→ [absent] selector/auth/policy resolve
→ atomic Receipt/binding/state/event/audit commit
```

다른 UID나 Instance가 local journal을 읽더라도 동일 key/CommandId로 신규 operation을 만들 수 없다. 존재 여부를 노출하지 않는 conflict 또는 expired/recovery-required 결과로 끝난다.

## 3. Local/Wire 경계

`@local` field와 path, file descriptor, source selector, rendering, output destination은 `CliInput`에만 존재한다. `CommandPayload`는 materialized bytes 또는 typed ArtifactRef와 semantic selector만 가진다. 두 projection은 하나의 metadata source에서 생성하므로 parser DTO와 wire DTO를 수작업으로 중복 정의하지 않는다.

## 4. Gate 수렴

Acceptance는 하나의 `first_required_milestone`만 가진다. Gate는 누적되므로 M4 command는 M0A~M3 evidence와 M4 신규 evidence를 모두 요구한다. `version` query의 최소 호환성 검증은 M3의 `AT-VERSION-001`, 전체 63-operation final schema freeze는 M6의 `AT-SCHEMA-001`이 소유한다.

## 5. 비범위

새 CLI command, 새 Domain aggregate, 별도 session manager, generic RPC/IDL/workflow framework, DB/Harness 제품 선택, 임의 retention/timeout 숫자, TUI/Web/Plugin lifecycle CLI는 추가하지 않는다.

## 6. 구현 진입 판정

Workspace와 M1A spike는 GO다. M1B contract source prototype도 GO지만 public freeze는 금지한다. M2 이후 subset freeze는 `DXB-DEL-060/061`의 누적 Gate가 실제 executable evidence로 PASS한 뒤에만 허용한다.
