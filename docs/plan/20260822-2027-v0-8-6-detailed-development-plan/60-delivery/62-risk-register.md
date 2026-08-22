---
title: "위험 등록부 v0.8.6"
document_id: "DXB-DEL-062"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-DEL-060", "DXB-RUN-032", "DXB-RUN-031", "DXB-IFC-040", "DXB-IFC-041", "DXB-ARC-018"]
---
# 위험 등록부 v0.8.6

<!-- risk-registry:start -->
| ID | Risk | Owner/Mitigation | Acceptance | Milestone | Status | Evidence | Blocking Finding |
|---|---|---|---|---|---|---|---|
| R-201 | RequestDigest가 server-only 값을 요구 | IFC-040 digest split | AT-CONTRACT-001 | M0A/M1B | Mitigated | v0.8.6 contract | executable digest fixture pending |
| R-202 | Receipt/Directive/Target/wait 혼합 | ARC-014/RUN-036 | AT-CONTRACT-001 | M0A/M1B | Mitigated | state-layer tables | Rust state fixture pending |
| R-203 | Approval/Authority/ActionGrant 무소유 | RUN-032 | AT-SECSTATE-001 | M2 | Mitigated | canonical model | persistence fixture pending |
| R-204 | SideEffect unknown blind retry | DOM-023 | AT-SEC-005 | M1B/M5 | Mitigated | ledger state model | runtime reconcile pending |
| R-205 | default policy ref만 존재 | RUN-035 | AT-BOOT-001 | M2 | Mitigated | semantic defaults | bootstrap fixture pending |
| R-206 | natural-language registry/parser drift | IFC-041/042 | AT-CONTRACT-001 | M1B | Mitigated | typed snapshot | Rust generator pending |
| R-207 | expired key가 새 mutation으로 재해석 | IFC-040/ARC-015 | AT-IDEMP-001 | M1A | Mitigated | epoch+horizon contract | storage fixture pending |
| R-208 | uncertain local journal eviction/replay | IFC-041/RUN-031 | AT-JOURNAL-001 | M1B/M3 | Mitigated | explicit states | CLI fixture pending |
| R-209 | same UID를 process isolation으로 오인 | RUN-032 | AT-SEC-005 | M2 | Mitigated | threat boundary | security test pending |
| R-210 | audit가 best-effort log로 퇴화 | RUN-034 | AT-AUDIT-001 | M2 | Mitigated | AuditIntent/store contract | audit store pending |
| R-211 | anonymous intra-Execution work/sub-agent | DOM-024 | Review R5 | M1B | Mitigated | P0 execution-only parallelism | architecture test pending |
| R-212 | graceful stop이 implicit host kill로 전환 | RUN-035/IFC-041 | AT-HOST-001 | M2 | Mitigated | split operation keys | host fixture pending |
| R-213 | M1A 전 parser freeze | BASE/DEL-060 | AT-STORAGE-001 | M1A | Open | explicit Blocked gate | storage proof absent |
| R-214 | Reference Provider로 가짜 release | ARC-013 | AT-HARNESS-001 | M5 | Open | real canary required | Adapter absent |
| R-215 | compatibility/retention 숫자 조기 고정 | ENG-053 | AT-SCHEMA-001 | M6 | Mitigated | freeze deferred | benchmark/maintenance ADR pending |
<!-- risk-registry:end -->

Critical/High risk는 executable fixture와 commit evidence 없이 `Verified` 또는 `Closed`로 표시하지 않는다.
