---
title: "위험 등록부 v0.8.9"
document_id: "DXB-DEL-062"
version: "0.8.9"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-DEL-060", "DXB-RUN-032", "DXB-RUN-031", "DXB-IFC-040", "DXB-IFC-041", "DXB-ARC-018"]
---
# 위험 등록부 v0.8.9

<!-- risk-registry:start -->
| ID | Risk | Owner/Mitigation | Acceptance | Milestone | Status | Evidence | Blocking Finding |
|---|---|---|---|---|---|---|---|
| R-401 | active package가 과거 INV/ADR/fail-closed 의미에 의존 | BASE/GOV-003/RUN-032/033 self-contained enumeration | AT-BASE-001 | M0A | Mitigated | v0.8.9 owners updated | validator and review evidence required |
| R-402 | local option이 wire payload와 RequestDigest로 누출 | IFC-042 dual projection and @local | AT-CONTRACT-001 | M1B | Mitigated | local field registry | generated projection fixture pending |
| R-403 | file/stdin path 또는 fd가 wire/journal에 저장 | IFC-040/042 bounded materialization | AT-CONTRACT-001 | M1B | Mitigated | ContentSource projection rule | Rust fixture pending |
| R-404 | 다른 Principal이 same CommandId를 신규 operation으로 생성 | ARC-014/015 global CommandId and principal key | AT-IDEMP-001 | M1A | Mitigated | binding index contract | storage fixture pending |
| R-405 | Receipt compaction 후 retry가 신규 mutation으로 재해석 | ARC-015/018 tombstone horizon | AT-STORAGE-001 | M1A | Mitigated | retention invariant | compaction fixture pending |
| R-406 | 복수 CLI process가 journal을 동시에 append | IFC-041/RUN-030 single-writer lock | AT-JOURNAL-001 | M1B | Mitigated | takeover contract | multi-process fixture pending |
| R-407 | SIGINT/broken pipe/local timeout이 Runtime cancel로 오해 | IFC-041/RUN-030/033 no implicit cancel | AT-SUBMIT-001 | M1B | Mitigated | termination contract | signal fixture pending |
| R-408 | Acceptance보다 이른 milestone에서 command freeze | DEL-060/061 cumulative gate | AT-DOC-CONTRACT-001 | M0A | Mitigated | machine milestone registry | validator fixture required |
| R-409 | negative self-test 하드코딩으로 변이가 발생하지 않음 | ENG-054 dynamic current-value mutation | AT-DOC-CONTRACT-001 | M0A | Mitigated | self-test rule | CI run required |
| R-410 | Approval decision 후 원 operation 고아화 | RUN-032/ARC-014 continuation | AT-SECSTATE-001 | M2 | Mitigated | durable wakeup/reconcile contract | Runtime fixture pending |
| R-411 | Membership와 Authority/Audit partial commit | ARC-015/DOM-028/029/RUN-032 atomic UoW | AT-MEMBER-001 | M4 | Mitigated | atomic ownership contract | storage fixture pending |
| R-412 | Bot identity가 Provider readiness에 종속 | BASE/DOM-020/RUN-035 identity-first create | AT-CLI-CORE-001 | M3 | Mitigated | provider-independent create | Runtime fixture pending |
<!-- risk-registry:end -->

`Mitigated`는 문서 계약상 차단 경로가 생겼다는 뜻이다. executable fixture가 `Passed`가 되기 전 구현 위험이 제거됐다고 해석하지 않는다.
