---
title: "수용 기준과 원칙 추적성 v0.8.6"
document_id: "DXB-DEL-061"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-DEL-060", "DXB-ENG-052", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042"]
---
# 수용 기준과 원칙 추적성 v0.8.6

Status는 `Specified | Executable | Passed | Blocked`다. `SELF`는 이 package가 포함된 commit을 뜻하며 CI 결과 전에는 Passed로 간주하지 않는다.

<!-- acceptance-registry:start -->
| Acceptance | Requirement | Owner | Fixture | Milestone | Status | Evidence Ref | Last Verified | Blocking Finding |
|---|---|---|---|---|---|---|---|---|
| AT-BASE-001 | active self-contained baseline | BASE/GOV | package validator | M0A | Executable | plan-validator active | SELF | CI result pending |
| AT-PLAN-002 | read-only validator + negative fixtures | ENG-054 | built-in self-test | M0A | Executable | plan-validator self-test | SELF | CI result pending |
| AT-CONTRACT-001 | digest/receipt/wait/schema/registry exact relation | IFC-040/041/042 | registry+golden fixture | M0A/M1B | Executable | document validator subset | SELF | Rust generator absent |
| AT-STORAGE-001 | atomic/snapshot/disk profile | ARC-018 | process crash storage spike | M1A | Blocked | none | none | storage adapter absent |
| AT-IDEMP-001 | bounded key horizon duplicate safety | IFC-040/ARC-015 | horizon/restart fixture | M1A | Blocked | none | none | storage adapter absent |
| AT-SUBMIT-001 | crash-safe CLI submission | IFC-040/041 | every journal/send/commit crash point | M1B/M3 | Blocked | none | none | CLI/runtime absent |
| AT-JOURNAL-001 | local journal abandon/capacity/recovery | IFC-041/RUN-031 | multi-process crash fixture | M1B/M3 | Blocked | none | none | CLI absent |
| AT-BOOT-001 | principal/default policy bootstrap | RUN-035/RUN-032 | concurrent first init | M2 | Blocked | none | none | Runtime absent |
| AT-SECSTATE-001 | Approval/Authority/ActionGrant lifecycle | RUN-032 | state/concurrency/revoke fixture | M2 | Blocked | none | none | security store absent |
| AT-AUDIT-001 | durable required audit | RUN-034 | atomic intent/capacity/recovery | M2 | Blocked | none | none | audit store absent |
| AT-HOST-001 | single instance and split stop modes | RUN-035 | concurrent start/stale generation | M2 | Blocked | none | none | Runtime absent |
| AT-MEMBER-001 | typed scope membership CAS | DOM-028/029 | set/remove/revoke race | M4 | Blocked | none | none | Domain absent |
| AT-DELEGATE-001 | persistent Bot delegation sender binding | DOM-025/026 | two Bot operator/execution flow | M4 | Blocked | none | none | Domain absent |
| AT-HARNESS-001 | real Host-path Adapter | ARC-013 | success/failure/cancel canary | M5 | Blocked | none | none | Adapter absent |
| AT-EXPORT-001 | atomic no-replace output | RUN-032 | destination race/disk-full | M5 | Blocked | none | none | writer absent |
| AT-APP-005 | receipt/key binding and commit semantics | IFC-040 | same/different digest | M1B/M3 | Blocked | none | none | Application absent |
| AT-APP-006 | snapshot/cursor | IFC-040/ARC-018 | concurrent mutation/expiry | M1A/M5 | Blocked | none | none | storage absent |
| AT-APP-007 | stream terminal/resume | IFC-040 | duplicate/gap/partial | M5 | Blocked | none | none | subscription absent |
| AT-CLI-009 | ambiguity safety | IFC-040/042 | same-name scope fixture | M3 | Blocked | none | none | parser absent |
| AT-CLI-010 | wait/timeout/partial semantics | IFC-040/041 | directive/local timeout fixture | M3/M5 | Blocked | none | none | CLI absent |
| AT-SEC-005 | endpoint/terminal/export/information flow | RUN-032 | hijack/OSC/path/revoke | M2/M5 | Blocked | none | none | Runtime absent |
| AT-SCHEMA-001 | schema metadata SSOT and compatibility | IFC-040/ENG-053 | generated drift fixture | M1B/M6 | Blocked | none | none | generator absent |
<!-- acceptance-registry:end -->
