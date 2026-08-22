---
title: "위험 등록부 v0.8.7"
document_id: "DXB-DEL-062"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-DEL-060", "DXB-RUN-032", "DXB-RUN-031", "DXB-IFC-040", "DXB-IFC-041", "DXB-ARC-018"]
---
# 위험 등록부 v0.8.7

<!-- risk-registry:start -->
| ID | Risk | Owner/Mitigation | Acceptance | Milestone | Status | Evidence | Blocking Finding |
|---|---|---|---|---|---|---|---|
| R-301 | Prepared 상태에서 send 후 crash | IFC-041 Dispatching fsync | AT-SUBMIT-001 | M1B/M3 | Mitigated | v0.8.7 contract | executable crash fixture pending |
| R-302 | retry가 current selector/auth에서 실패 | IFC-040 binding-first lookup | AT-IDEMP-001 | M1A | Mitigated | v0.8.7 contract | storage fixture pending |
| R-303 | Approval 후 원 operation 고아화 | DOM-026/RUN-032 continuation | AT-SECSTATE-001 | M2 | Mitigated | pending intent contract | Runtime fixture pending |
| R-304 | Receipt Rejected와 Task Rejected 혼합 | ARC-014/DOM-023 | AT-APP-005 | M1B | Mitigated | separate outcome semantics | state fixture pending |
| R-305 | Contract가 Receipt runtime state 소유 | BASE/ARC-011/014 | AT-CONTRACT-001 | M1B | Mitigated | owner registry | Rust boundary fixture pending |
| R-306 | Membership/Authority partial commit | ARC-015/DOM-028/029 | AT-MEMBER-001 | M4 | Mitigated | atomic UoW | storage fixture pending |
| R-307 | accepted wait와 final resource schema 충돌 | IFC-040/041 | AT-DOC-CONTRACT-001 | M0A | Mitigated | out-operation convergence | generated fixture pending |
| R-308 | target-terminal/applied 의미 과확장 | RUN-036/IFC-041 | AT-DOC-CONTRACT-001 | M0A | Mitigated | wait contraction | generated fixture pending |
| R-309 | global Instance revision create serialization | IFC-042/DOM-020/028 | AT-CLI-CORE-001 | M3 | Mitigated | create input contraction | parser fixture pending |
| R-310 | M4/M5 contract 조기 freeze | DEL-060 | AT-SCHEMA-001 | M6 | Mitigated | milestone freeze | implementation evidence pending |
| R-311 | document validator를 Rust executable로 오인 | DEL-061/ENG-054 | AT-DOC-CONTRACT-001/AT-CONTRACT-001 | M0A/M1B | Mitigated | status split | CI/generator pending |
| R-312 | Bot identity가 Provider readiness에 종속 | DOM-020/RUN-035 | AT-CLI-CORE-001 | M3 | Mitigated | identity-first create | Runtime fixture pending |
<!-- risk-registry:end -->
