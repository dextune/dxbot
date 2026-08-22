---
title: "v0.8.5 결정 기록과 후속 질문"
document_id: "DXB-DEL-063"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-GOV-003", "DXB-DEL-060", "DXB-IFC-040", "DXB-IFC-041", "DXB-ARC-018"]
---
# v0.8.5 결정 기록과 후속 질문

## Resolved

- `OQ-090`: active package는 v0.8.5, review revision 5다.
- `OQ-091`: source original은 nonnormative provenance다.
- `OQ-092`: `ClientRequestId`는 correlation 전용; recovery는 CommandId/IdempotencyKey다.
- `OQ-093`: per-command journal은 fsync-before-send이며 payload를 저장하지 않는다.
- `OQ-094`: P0 principal은 server-derived UID+Instance mapping이다.
- `OQ-095`: first init은 default policy generations를 atomic create한다.
- `OQ-096`: Storage adapter는 M1A profile을 통과한 뒤 freeze한다.
- `OQ-097`: Project/Channel membership과 delegation은 P0 typed CLI 경로를 가진다.
- `OQ-098`: real Harness Adapter canary는 P0 release evidence다.
- `OQ-099`: export는 atomic no-replace다.
- `OQ-100`: host stop은 explicit escalation이다.
- `OQ-101`: command input grammar는 operation metadata SSOT에서 생성한다.
- `OQ-102`: stale write workflow는 제거하고 read-only validator로 대체한다.
- `OQ-103`: Bot delete states는 P0 enum에서 제외한다.

## Deferred

구체 storage product, external Harness 제품/버전, numeric snapshot retention/disk threshold, Windows/macOS adapters, remote IAM, TUI/Web, Plugin lifecycle CLI는 executable evidence와 별도 ADR 후 결정한다. Deferred 항목을 generic fallback이나 unsafe default로 구현하지 않는다.
