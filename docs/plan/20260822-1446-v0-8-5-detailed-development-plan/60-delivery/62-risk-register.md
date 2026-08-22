---
title: "위험 등록부 v0.8.5"
document_id: "DXB-DEL-062"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-DEL-060", "DXB-RUN-032", "DXB-RUN-031", "DXB-IFC-040", "DXB-IFC-041", "DXB-ARC-018"]
---
# 위험 등록부 v0.8.5

| ID | Risk | Owner/Mitigation | Acceptance | Blocker |
|---|---|---|---|---|
| R-101 | source가 active owner를 override | GOV-001/source nonnormative | AT-BASE-001 | M0 |
| R-102 | package/review/doc version 혼용 | GOV-001 active discovery | AT-PLAN-002 | M0 |
| R-103 | pre-send crash로 recovery key 손실 | IFC-040 per-command fsync journal | AT-SUBMIT-001 | M3 |
| R-104 | multi-process journal last-writer loss | CLI per-command file/lock | AT-SUBMIT-001 | M3 |
| R-105 | client Principal spoofing | RUN-032 server-derived principal | AT-BOOT-001 | M2 |
| R-106 | default policy/provider 미정으로 create divergence | RUN-035 atomic defaults | AT-BOOT-001 | M2 |
| R-107 | storage choice가 receipt/snapshot을 재설계 | ARC-018 M1A proof | AT-STORAGE-001 | M1A |
| R-108 | Reference Provider만으로 가짜 release | ARC-013 real canary | AT-HARNESS-001 | M5 |
| R-109 | Multi-Bot CLI 경로 부재 | typed membership/delegation | AT-MEMBER-001/AT-DELEGATE-001 | M4 |
| R-110 | export check/rename TOCTOU overwrite | atomic no-replace | AT-EXPORT-001 | M5 |
| R-111 | endpoint 장애가 implicit host kill로 전환 | RUN-035 explicit escalation | AT-HOST-001 | M2 |
| R-112 | parser가 incomplete manual contract 고착 | operation/input metadata SSOT | AT-SCHEMA-001 | M1B/M3 |
| R-113 | stale workflow green을 release evidence로 오인 | ENG-054 read-only actual CI | AT-PLAN-002 | M0 |
| R-114 | P0 delete state가 premature migration 분기 생성 | DOM-020 state removal | Review 3 | M1B |

Critical/High risk는 executable fixture와 milestone evidence 없이 close하지 않는다.
