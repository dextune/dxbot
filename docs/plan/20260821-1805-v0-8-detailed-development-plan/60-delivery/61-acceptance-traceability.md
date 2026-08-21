---
title: "수용 기준과 원칙 추적성"
document_id: "DXB-DEL-061"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DEL-060", "DXB-ENG-052", "DXB-IFC-040", "DXB-IFC-041"]
---

# 수용 기준과 원칙 추적성

## 1. 목적

제품 원칙, Canonical Owner, 구현 Gate, deterministic fixture, Risk를 하나의 Acceptance inventory로 연결한다. 문서 존재와 실행 PASS를 구분한다.

## 2. Acceptance Inventory

| Acceptance | 요구 | Canonical Owner | 핵심 Fixture | Milestone |
|---|---|---|---|---|
| AT-BASE-001 | self-contained effective baseline | GOV-001/INDEX/MANIFEST | package validator | M0 |
| AT-APP-001 | headless use-case completeness | IFC-040 | representative public-only E2E | M1/M3/M4 |
| AT-APP-002 | version compatibility | IFC-040/ENG-053 | compatible/incompatible matrix | M1/M6 |
| AT-APP-003 | lost-response idempotency | ARC-014/IFC-040 | commit-response-drop | M1 |
| AT-APP-004 | subscription resume/gap | IFC-040/RUN-033 | cursor disconnect/gap | M5 |
| AT-APP-005 | receipt/key binding | IFC-040 | same/different digest, restart | M1/M3 |
| AT-APP-006 | snapshot/cursor | IFC-040 | concurrent mutation/foreign cursor | M1/M5 |
| AT-APP-007 | partial/terminal/resume | IFC-040 | JSONL partial/gap/local cancel | M5 |
| AT-HOST-001 | Runtime instance/concurrent start | RUN-035 | 100-way start/stale endpoint | M2 |
| AT-CLI-001 | CLI reference E2E | IFC-041 | Bot + collaboration flows | M3/M4 |
| AT-CLI-002 | process independence | IFC-041 | CLI crash/SIGINT | M3 |
| AT-CLI-003 | machine output stability | IFC-041 | JSON/JSONL golden | M3/M5 |
| AT-CLI-004 | bounded large output | IFC-041/RUN-031 | 1M scan/slow pipe | M5/M6 |
| AT-CLI-005 | no client policy duplicate | IFC-041/RUN-032 | stale role/pressure | M3/M4 |
| AT-CLI-006 | restart/reconnect identity | RUN-033/IFC-041 | Runtime restart | M3/M5 |
| AT-CLI-007 | active Interface scope | BASE/ENG-054 | inventory/dependency | M0/M6 |
| AT-CLI-008 | P0 matrix completeness | IFC-041 | generated command registry | M0/M3 |
| AT-CLI-009 | selector ambiguity safety | IFC-040/041 | same-name/alias scopes | M1/M3 |
| AT-CLI-010 | wait/timeout/partial | IFC-041 | accepted/pending/timeout | M3/M5 |
| AT-SEC-005 | endpoint/terminal/export safety | RUN-032/IFC-041 | hijack/OSC/symlink/disk-full | M2/M5 |
| AT-SCHEMA-001 | schema SSOT/drift | IFC-040/ENG-054 | deterministic generation | M1/M6 |
| AT-CI-001 | actual Gate execution | ENG-054 | intentionally broken fixture | M0/M6 |

기존 Bot/Brain/Conversation/Thread/Task/Execution/Core/Provider/Plugin/Project/Channel/Memory/Process/Security/Resource/Recovery Acceptance는 v0.8 active docs의 해당 owner 검증 기준으로 materialize되어 있으며 구현 단계에서 regression suite로 유지한다.

## 3. 상세 신규 Acceptance

### AT-BASE-001
Given active v0.8 package만 제공될 때, When validator와 구현자가 owner/dependency/scope를 판정하면, Then prior package를 읽지 않고 결정 가능하고 hidden inheritance/duplicate owner/cycle가 0이다.

### AT-APP-005
Given command commit 후 response loss 또는 CLI restart, When same key/digest로 조회/재전송하면, Then 기존 receipt/outcome을 반환하고 effect 1회다. different principal/action/target/payload/schema digest는 typed conflict다.

### AT-CLI-009
Given 같은 name/alias를 가진 여러 resource, When scope 없는 mutation을 시도하면, Then silent selection 없이 exit 5와 bounded candidates를 반환한다. canonical ID+revision mutation은 정확한 target 하나만 변경한다.

### AT-HOST-001
Given same data root에 100 concurrent start와 stale lock/socket/descriptor, When startup/recovery를 수행하면, Then active InstanceId/HostGeneration/writer/endpoint가 하나고 다른 data root 연결/삭제가 없다.

### AT-APP-006
Given stable sort dataset을 paging하는 동안 concurrent insert/update/delete, When snapshot cursor로 전 페이지를 읽으면, Then snapshot 내 duplicate/missing이 없고 foreign/filter/scope/schema/expired/reauth cursor가 explicit error다.

### AT-APP-007
Given at-least-once duplicate, disconnect, retention gap, slow consumer, broken pipe, When JSONL watch를 수행하면, Then event duplicate 식별, resume 또는 explicit resync, terminal status와 last safe cursor가 존재하고 target lifecycle 변화가 없다.

### AT-CLI-010
Given command가 Accepted→AwaitingSafePoint→Committed로 진행하거나 local timeout/partial output이 발생할 때, When automation이 exit+machine envelope를 읽으면, Then Runtime operation state와 local observation state를 구분하고 timeout 뒤 duplicate effect가 없다.

### AT-SEC-005
Given malicious endpoint/symlink/peer, ANSI/OSC/clipboard text, traversal/symlink/FIFO/device/existing destination, disk-full/cancel, When CLI/Runtime이 처리하면, Then fail closed/sanitized/atomic cleanup되고 secret/partial sensitive file이 노출되지 않는다.

### AT-SCHEMA-001
Given Rust contract source가 변경될 때, When generator/check를 수행하면, Then schema/golden/operation/help/exit가 deterministic하게 갱신되고 manual drift는 CI failure다.

### AT-CI-001
Given duplicate ID, cycle, missing command row, forbidden dependency, stale generated fixture를 각각 주입할 때, When PR CI를 실행하면, Then 각 Gate가 실패하고 manifest의 PASS 문구로 우회되지 않는다.

## 4. Cross-Layer Trace

```text
Product invariant
→ GOV/ARC/DOM/RUN Canonical Owner
→ IFC-040 typed contract
→ IFC-041 P0 command/output
→ ENG-050 implementation rule
→ ENG-052 deterministic fixture
→ Acceptance
→ DEL-062 Risk
→ DEL-060 Milestone
```

## 5. Release Evidence 상태

각 Acceptance는 다음 상태 중 하나다.
- `Specified`: 문서 계약/fixture가 정의됨
- `Executable`: test/validator 구현됨
- `Passed`: 지정 commit/artifact에서 실행 PASS
- `Blocked`: owner/fixture/implementation 부족

v0.8 문서 package 생성 시 신규 Acceptance는 `Specified`이며 문서 validator 관련 항목만 실제 실행할 수 있다. Rust binary가 없는데 `Passed`로 표시하지 않는다.

## 6. 검증 기준

- 신규 11개 Acceptance orphan 0.
- 각 Critical Risk가 하나 이상의 Acceptance와 owner를 가짐.
- P0 Command Matrix 모든 row가 Acceptance에 연결.
- legacy product invariant가 Interface 변경으로 삭제/축소되지 않음.
- release evidence가 status/commit/artifact를 명시.
