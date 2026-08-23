---
title: "위험 등록부 v0.8.10"
document_id: "DXB-DEL-062"
version: "0.8.10"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-DEL-060", "DXB-RUN-032", "DXB-RUN-031", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-043", "DXB-ARC-018"]
---
# 위험 등록부 v0.8.10

<!-- risk-registry:start -->
| ID | Risk | Owner/Mitigation | Acceptance | Milestone | Status | Evidence | Blocking Finding |
|---|---|---|---|---|---|---|---|
| R-501 | 첫 실행에서 존재하지 않는 InstanceId를 요구 | RUN-035 default bootstrap and exact selection | AT-CLI-DISCOVERY-001 | M2 | Mitigated | first-run contract | executable fixture pending |
| R-502 | 다른 command가 Runtime을 몰래 start하거나 다른 Instance로 fallback | RUN-035/IFC-043 no auto-start/no fallback | AT-CLI-DISCOVERY-001 | M2 | Mitigated | selection precedence | executable fixture pending |
| R-503 | 사용자가 every mutation마다 internal ID/revision/generation 복사 | IFC-042 bounded preflight and optional if-CAS | AT-CLI-INTERACTIVE-001 | M3 | Mitigated | input registry contraction | generated fixture pending |
| R-504 | conversation/channel command가 내부 ConversationId를 요구 | DOM-027/029 selector materialization | AT-CLI-CORE-001 | M3 | Mitigated | domain target contract | end-to-end fixture pending |
| R-505 | preflight race를 CLI가 최신 revision으로 자동 retry | IFC-040/043 one-shot CAS conflict | AT-CLI-INTERACTIVE-001 | M3 | Mitigated | no-auto-retry contract | race fixture pending |
| R-506 | human prompt/progress가 JSON stdout을 오염 | IFC-040/041/043 channel separation | AT-CLI-AUTOMATION-001 | M3 | Mitigated | machine projection contract | golden fixture pending |
| R-507 | nonzero machine error가 parse 불가능하거나 next action이 문자열뿐 | IFC-040/043 complete result + typed action | AT-CLI-AUTOMATION-001 | M3 | Mitigated | error schema contract | generated fixture pending |
| R-508 | provider-unavailable인데 M5 provider command 전 진단 불가 | RUN-035 M2 doctor provider section | AT-CLI-DISCOVERY-001 | M2 | Mitigated | doctor action contract | runtime fixture pending |
| R-509 | interrupt 후 operation이 계속되는데 사용자가 실패로 오판 | RUN-033/IFC-043 operation_may_continue and refs | AT-CLI-RECOVERY-001 | M3 | Mitigated | recovery projection | signal fixture pending |
| R-510 | stale Prepared가 영구 capacity를 점유하거나 unsafe prune | RUN-033 lock/integrity/retention eligibility | AT-CLI-RECOVERY-001 | M3 | Mitigated | Prepared policy | multi-process fixture pending |
| R-511 | --all이 unbounded 또는 일부만 받고 성공으로 끝남 | IFC-043 local ceiling and resume cursor | AT-CLI-AUTOMATION-001 | M3 | Mitigated | bounded all contract | pagination fixture pending |
| R-512 | selector suggestion이 fuzzy mutation으로 실행 | IFC-043 exact mutation target only | AT-CLI-009 | M3 | Mitigated | ambiguity contract | selector fixture pending |
| R-513 | user journey GO를 실제 product usability PASS로 오인 | DEL-061 status separation | AT-DOC-CONTRACT-001 | M0A | Mitigated | design/executable distinction | CI and Rust evidence pending |
<!-- risk-registry:end -->

`Mitigated`는 문서 계약상 차단 경로가 생겼다는 뜻이다. executable fixture가 `Passed`가 되기 전 구현 위험이 제거됐다고 해석하지 않는다.
