---
title: "수용 기준과 원칙 추적성 v0.8.7"
document_id: "DXB-DEL-061"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-DEL-060", "DXB-ENG-052", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042"]
---
# 수용 기준과 원칙 추적성 v0.8.7

`Executable`은 실제 fixture가 현재 repository에서 실행 가능할 때만 사용한다. 문서 registry 검증과 Rust generated contract 검증을 분리한다.

<!-- acceptance-registry:start -->
| Acceptance | Requirement | Owner | Fixture | Milestone | Status | Evidence Ref | Last Verified | Blocking Finding |
|---|---|---|---|---|---|---|---|---|
| AT-BASE-001 | active self-contained baseline | BASE/GOV | package validator | M0A | Executable | plan-validator active | SELF | CI result pending |
| AT-PLAN-002 | read-only validator + negative fixtures | ENG-054 | built-in self-test | M0A | Executable | plan-validator self-test | SELF | CI result pending |
| AT-DOC-CONTRACT-001 | document registry/wait/output/DSL relation | IFC-040/041/042 | plan validator | M0A | Executable | document validator | SELF | CI result pending |
| AT-CONTRACT-001 | Rust metadata generated exact contract | IFC-040/041/042 | generator+golden | M1B | Blocked | none | none | Rust generator absent |
| AT-STORAGE-001 | atomic/snapshot/disk profile | ARC-018 | process crash spike | M1A | Blocked | none | none | storage adapter absent |
| AT-IDEMP-001 | binding lookup/horizon duplicate safety | IFC-040/ARC-015 | crash/restart fixture | M1A | Blocked | none | none | storage adapter absent |
| AT-SUBMIT-001 | Dispatching fsync-before-send + recovery | IFC-040/041 | crash matrix | M1B/M3 | Blocked | none | none | CLI/runtime absent |
| AT-JOURNAL-001 | local journal abandon/capacity | IFC-041 | multi-process fixture | M1B/M3 | Blocked | none | none | CLI absent |
| AT-BOOT-001 | principal/default bootstrap | RUN-035/RUN-032 | concurrent first init | M2 | Blocked | none | none | Runtime absent |
| AT-SECSTATE-001 | Approval/Authority/Grant continuation | RUN-032/DOM-026 | decision/restart/revoke | M2 | Blocked | none | none | security store absent |
| AT-AUDIT-001 | durable required audit | RUN-034 | atomic intent/capacity | M2 | Blocked | none | none | audit store absent |
| AT-HOST-001 | single instance/split stop | RUN-035 | concurrent start/stale generation | M2 | Blocked | none | none | Runtime absent |
| AT-CLI-CORE-001 | Bot-only vertical slice without watch | IFC-041/042 | create/send/submit/show/result/restart | M3 | Blocked | none | none | CLI/runtime absent |
| AT-MEMBER-001 | scope membership atomic CAS | DOM-028/029 | binding/audit atomic race | M4 | Blocked | none | none | Domain absent |
| AT-DELEGATE-001 | persistent Bot delegation | DOM-025/026 | two-Bot flow | M4 | Blocked | none | none | Domain absent |
| AT-HARNESS-001 | real Host-path Adapter | ARC-013 | success/failure/cancel | M5 | Blocked | none | none | Adapter absent |
| AT-EXPORT-001 | atomic no-replace output | RUN-032 | destination race/disk-full | M5 | Blocked | none | none | writer absent |
| AT-APP-005 | receipt/domain outcome semantics | IFC-040/DOM-023 | digest/outcome matrix | M1B/M3 | Blocked | none | none | Application absent |
| AT-APP-006 | snapshot/cursor | IFC-040/ARC-018 | concurrent mutation/expiry | M5 | Blocked | none | none | storage absent |
| AT-APP-007 | stream terminal/resume | IFC-040 | duplicate/gap/partial | M5 | Blocked | none | none | subscription absent |
| AT-CLI-009 | ambiguity safety | IFC-040/042 | same-name scope | M3 | Blocked | none | none | parser absent |
| AT-CLI-010 | directive wait/timeout semantics | IFC-040/041 | safe-point/local timeout | M3/M5 | Blocked | none | none | CLI absent |
| AT-SEC-005 | endpoint/terminal/export/info-flow | RUN-032 | hijack/OSC/path/revoke | M2/M5 | Blocked | none | none | Runtime absent |
| AT-SCHEMA-001 | schema compatibility/final freeze | IFC-040/ENG-053 | generated drift | M6 | Blocked | none | none | generator absent |
<!-- acceptance-registry:end -->
