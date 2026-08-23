---
title: "Acceptance·Traceability와 사용자 여정 최초 Milestone Registry v0.8.10"
document_id: "DXB-DEL-061"
version: "0.8.10"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-DEL-060", "DXB-ENG-052", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042", "DXB-IFC-043"]
---
# Acceptance·Traceability와 사용자 여정 최초 Milestone Registry v0.8.10

각 Acceptance는 정확히 하나의 최초 요구 milestone을 가진다. 이후 milestone은 누적 Gate로 해당 PASS를 재사용한다.

<!-- acceptance-registry:start -->
| ID | Requirement | Owner | Fixture | Milestone | Status | Artifact | Command | Last Evidence |
|---|---|---|---|---|---|---|---|---|
| AT-BASE-001 | active package self-contained invariant and ADR baseline | DXB-BASE-000/DXB-GOV-003 | validator invariant/ADR fixture | M0A | Executable | active-plan report | python3 scripts/plan-validator.py --active | local validation required |
| AT-PLAN-002 | document ID, DAG, manifest, owner, link and registry integrity | DXB-GOV-001/DXB-MANIFEST | plan structural fixture | M0A | Executable | active-plan report | python3 scripts/plan-validator.py --active | local validation required |
| AT-DOC-CONTRACT-001 | CLI output, exit, projection, journey and milestone document closure | DXB-GOV-005/DXB-IFC-040/041/042/043 | negative self-test | M0A | Executable | self-test report | python3 scripts/plan-validator.py --self-test | local validation required |
| AT-STORAGE-001 | atomic state, receipt, two binding indexes, tombstone and recovery | DXB-ARC-015/018 | process crash and compaction spike | M1A | Blocked | storage spike artifact | cargo test -p storage-spike | implementation pending |
| AT-IDEMP-001 | global CommandId and principal key conflict matrix | DXB-ARC-014/015 | idempotency conflict fixture | M1A | Blocked | idempotency report | cargo test -p storage-spike idempotency | implementation pending |
| AT-CONTRACT-001 | generated Rust contract, user selector preflight and dual projection exact diff | DXB-IFC-040/042 | generator golden fixture | M1B | Blocked | generated contract snapshot | cargo test -p application-contract | implementation pending |
| AT-SUBMIT-001 | durable Prepared and Dispatching before send with binding lookup recovery | DXB-IFC-040/041 | submission crash fixture | M1B | Blocked | submission report | cargo test -p control-client submission | implementation pending |
| AT-JOURNAL-001 | single-writer journal, takeover, hash chain and bounded retention | DXB-IFC-041/DXB-RUN-030 | multi-process journal fixture | M1B | Blocked | journal report | cargo test -p cli journal | implementation pending |
| AT-APP-005 | mutation target materialization, receipt and domain-outcome semantics | DXB-DOM-026/DXB-ARC-014/DXB-IFC-042 | application mutation state fixture | M1B | Blocked | application report | cargo test -p application mutation | implementation pending |
| AT-BOOT-001 | atomic first Instance and verified endpoint bootstrap | DXB-RUN-035 | bootstrap crash fixture | M2 | Blocked | bootstrap report | cargo test -p runtime-bootstrap | implementation pending |
| AT-SECSTATE-001 | Principal, Approval, Authority and pending operation continuation | DXB-RUN-032 | security state fixture | M2 | Blocked | security-state report | cargo test -p runtime-security | implementation pending |
| AT-AUDIT-001 | required audit intent atomicity and redacted observation | DXB-RUN-034 | audit crash fixture | M2 | Blocked | audit report | cargo test -p runtime-audit | implementation pending |
| AT-HOST-001 | runtime host start/status/graceful stop/explicit host stop fencing | DXB-RUN-035/DXB-IFC-041 | host lifecycle fixture | M2 | Blocked | host report | cargo test -p runtime-host | implementation pending |
| AT-SEC-005 | endpoint, terminal, information-flow and non-disclosure fail-closed path | DXB-RUN-032 | local boundary security fixture | M2 | Blocked | security boundary report | cargo test -p control-server security | implementation pending |
| AT-CLI-DISCOVERY-001 | local help/version, first-run bootstrap, exact Instance selection and provider doctor | DXB-IFC-043/DXB-RUN-035 | offline/discovery/first-run fixture | M2 | Blocked | cli discovery report | cargo test -p cli discovery | implementation pending |
| AT-VERSION-001 | minimum client/runtime/protocol/schema compatibility query | DXB-ENG-053/DXB-IFC-041/043 | version matrix fixture | M3 | Blocked | version report | cargo test -p control-client version | implementation pending |
| AT-CLI-CORE-001 | Bot create, Main Conversation send, Task submit/show/result path | DXB-IFC-041/DXB-DOM-020/026/027 | Bot-only end-to-end fixture | M3 | Blocked | cli core report | cargo test -p cli core | implementation pending |
| AT-APP-006 | bounded page, cursor, all-loop and resync semantics | DXB-IFC-040/043/DXB-RUN-033 | pagination snapshot fixture | M3 | Blocked | pagination report | cargo test -p application query | implementation pending |
| AT-CLI-009 | canonical ID or scoped exact selector ambiguity behavior | DXB-IFC-041/042/043 | selector ambiguity fixture | M3 | Blocked | selector report | cargo test -p cli selector | implementation pending |
| AT-CLI-CONFIRM-001 | destructive confirmation and non-TTY no-prompt semantics | DXB-IFC-041/042/043/DXB-RUN-032 | confirmation safety fixture | M3 | Blocked | confirmation report | cargo test -p cli confirmation | implementation pending |
| AT-EXPORT-001 | safe output no-follow, bounded write, fsync and no-replace | DXB-RUN-032/DXB-IFC-041/043 | safe writer fixture | M3 | Blocked | safe writer report | cargo test -p cli safe-writer | implementation pending |
| AT-CLI-INTERACTIVE-001 | domain selector preflight, optional if-CAS, actionable conflict/approval/provider result | DXB-IFC-042/043 | interactive journey fixture | M3 | Blocked | interactive journey report | cargo test -p cli interactive | implementation pending |
| AT-CLI-AUTOMATION-001 | stable JSON/JSONL, stdout/stderr, non-TTY and typed next actions | DXB-IFC-040/041/043 | machine interface fixture | M3 | Blocked | automation report | cargo test -p cli automation | implementation pending |
| AT-CLI-RECOVERY-001 | user-visible Prepared reuse/prune, Dispatching lookup/replay and operation continuation | DXB-IFC-041/043/DXB-RUN-033 | restart/re-entry fixture | M3 | Blocked | cli recovery report | cargo test -p cli recovery | implementation pending |
| AT-MEMBER-001 | Project/Channel selector membership and AuthorityBinding atomicity | DXB-DOM-028/029/DXB-RUN-032 | membership crash fixture | M4 | Blocked | membership report | cargo test -p application membership | implementation pending |
| AT-DELEGATE-001 | typed Multi-Bot delegation with membership and scope boundary | DXB-DOM-025 | delegation end-to-end fixture | M4 | Blocked | delegation report | cargo test -p application delegation | implementation pending |
| AT-CLI-010 | Task Directive applied wait, local timeout and no implicit cancel | DXB-IFC-041/DXB-RUN-030/036 | directive observation fixture | M5 | Blocked | directive report | cargo test -p cli directive | implementation pending |
| AT-HARNESS-001 | Reference Provider plus one real Harness Adapter canary | DXB-ARC-013 | real harness canary | M5 | Blocked | harness report | cargo test -p provider-host harness | implementation pending |
| AT-APP-007 | subscription cursor, reconnect, gap and explicit resync | DXB-IFC-040/DXB-RUN-033 | stream reconnect fixture | M5 | Blocked | subscription report | cargo test -p application subscription | implementation pending |
| AT-SCHEMA-001 | full 63-operation parser/help/preflight/schema/error/exit/journey final freeze | DXB-IFC-040/041/042/043 | release snapshot exact diff | M6 | Blocked | release contract snapshot | cargo test --workspace contract-freeze | all prior milestones pending |
<!-- acceptance-registry:end -->

`Executable`은 검증 명령과 fixture가 정의돼 있다는 뜻이며 실제 CI PASS가 아니다. `Passed`는 repository commit, 실행 command, artifact/hash가 모두 기록된 경우에만 사용한다. 현재 Rust/Storage/Runtime/CLI/Harness 구현 Acceptance는 `Blocked`다.

command registry의 Acceptance milestone은 command freeze milestone보다 늦을 수 없다. `DXB-DEL-060` milestone exit registry와 본 표의 milestone은 exact match여야 한다. `DXB-IFC-043`의 journey GO는 본 Acceptance를 자동 Passed로 승격하지 않는다.
