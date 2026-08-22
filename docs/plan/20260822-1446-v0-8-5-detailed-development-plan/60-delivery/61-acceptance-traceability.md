---
title: "수용 기준과 원칙 추적성 v0.8.5"
document_id: "DXB-DEL-061"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-DEL-060", "DXB-ENG-052", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042"]
---
# 수용 기준과 원칙 추적성 v0.8.5

| Acceptance | Requirement | Owner | Fixture | Milestone |
|---|---|---|---|---|
| AT-BASE-001 | active self-contained baseline | BASE/GOV | package validator | M0 |
| AT-PLAN-002 | actual read-only validator | ENG-054 | built-in negative fixtures | M0 |
| AT-STORAGE-001 | atomic/snapshot/disk profile | ARC-018 | process crash/storage spike | M1A |
| AT-SUBMIT-001 | crash-safe CLI submission | IFC-040/041 | every journal/send/commit crash point | M1B/M3 |
| AT-BOOT-001 | principal/default policy bootstrap | RUN-035/RUN-032 | concurrent first init | M2 |
| AT-MEMBER-001 | typed scope membership | DOM-026/IFC-042 | create/update/remove CAS/revoke | M4 |
| AT-DELEGATE-001 | persistent Bot delegation | DOM-025/026 | two Bot Task flow | M4 |
| AT-HARNESS-001 | real Host-path Adapter | ARC-013 | success/failure/cancel canary | M5 |
| AT-EXPORT-001 | atomic no-replace output | RUN-032 | destination race/disk-full | M5 |
| AT-APP-005 | receipt/key binding | IFC-040 | same/different digest | M1B/M3 |
| AT-APP-006 | snapshot/cursor | IFC-040/ARC-018 | concurrent mutation/expiry | M1A/M5 |
| AT-APP-007 | stream terminal/resume | IFC-040 | duplicate/gap/partial | M5 |
| AT-HOST-001 | single instance | RUN-035 | concurrent start/stale generation | M2 |
| AT-CLI-009 | ambiguity safety | IFC-040/042 | same-name scopes | M3 |
| AT-CLI-010 | wait/timeout/partial | IFC-041 | safe point/local timeout | M3/M5 |
| AT-SEC-005 | endpoint/terminal/export | RUN-032 | hijack/OSC/path race | M2/M5 |
| AT-SCHEMA-001 | schema metadata SSOT | IFC-040 | generation drift | M1B/M6 |

Status는 `Specified | Executable | Passed | Blocked`로 관리한다. 문서 package에서는 validator 관련 항목만 Executable/Passed가 될 수 있으며 Rust/Storage/Harness evidence를 미리 Passed로 표시하지 않는다.
