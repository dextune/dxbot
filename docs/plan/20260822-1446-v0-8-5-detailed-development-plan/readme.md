---
title: "DXBOT v0.8.5 상세 개발기획 문서 집합"
document_id: "DXB-INDEX"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-BASE-000", "DXB-DEL-060", "DXB-DEL-061", "DXB-DEL-062", "DXB-DEL-063"]
package_path: "docs/plan/20260822-1446-v0-8-5-detailed-development-plan"
review_revision: 5
source_baseline_commit: "5b55b67fbaaf0f3192c126865e8b665ff26fc5aa"
---
# DXBOT 상세 개발기획 v0.8.5

적대적 CLI 구현 검토에서 발견된 기준선, recovery, bootstrap, storage, Harness, Multi-Bot, export, stop, CI 공백을 닫은 active package다.

## Active path

```text
Persistent Bot Runtime
→ M1A Storage Proof
→ typed Application Contract
→ authenticated local Control
→ crash-safe CLI
→ Multi-Bot delegation
→ real Harness canary
```

## 핵심 변경

- 원문 보존본 비규범화 및 active package version 분리
- actual read-only plan validator와 negative self-test
- fsync-before-send per-command journal
- server-derived local Principal과 atomic default policy bootstrap
- P0 Storage executable profile
- complete 62-command input registry
- Project/Channel membership + Task delegation CLI
- real Harness Adapter canary
- atomic no-replace export
- explicit host-stop escalation
- P0 Bot delete state 제거

## Canonical Owner Map

| Responsibility | Owner |
|---|---|
| baseline/version | BASE/GOV-001 |
| storage profile | ARC-018 |
| harness | ARC-013 |
| public submission/receipt | IFC-040 |
| CLI command/output | IFC-041 |
| typed input grammar | IFC-042 |
| bootstrap/stop | RUN-035 |
| principal/export security | RUN-032 |
| actual CI | ENG-054 |
| roadmap/evidence | DEL-060~063 |

## Inventory

Governance 6, Architecture 9, Domains 10, Runtime 9, Interfaces 3, Engineering 5, Delivery 5: plan documents 47. `readme.md`와 `manifest.md`를 포함해 Markdown 49개다.
