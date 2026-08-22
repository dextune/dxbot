---
title: "v0.8.6 결정 기록과 후속 질문"
document_id: "DXB-DEL-063"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-GOV-003", "DXB-DEL-060", "DXB-IFC-040", "DXB-IFC-041", "DXB-ARC-018"]
---
# v0.8.6 결정 기록과 후속 질문

## Resolved semantic

- `OQ-104`: RequestDigest는 client-visible semantic, ResolvedBindingDigest는 server binding이다.
- `OQ-105`: Receipt Committed, Directive Applied, Target Terminal은 서로 다른 상태다.
- `OQ-106`: wait predicate는 operation metadata가 허용한 값만 사용한다.
- `OQ-107`: IdempotencyKey는 issuance epoch를 포함하고 expired key를 신규 mutation으로 해석하지 않는다.
- `OQ-108`: Approval/AuthorityBinding/ActionGrant와 SideEffect Ledger의 Owner/state가 정해졌다.
- `OQ-109`: default policy semantic은 deny unknown, finite resource, no auto promotion/declass, no Reference fallback이다.
- `OQ-110`: Project/Channel membership은 scope aggregate가 row/generation을 소유하고 Security owner가 AuthorityBinding을 발급한다.
- `OQ-111`: delegated sender는 current Execution 또는 authorized operator request에서 server-side resolve한다.
- `OQ-112`: P0 Dynamic Core 병렬성은 서로 다른 Execution 간 병렬성이다.
- `OQ-113`: 62 lexical path/63 operation mode와 typed input snapshot을 사용한다.
- `OQ-114`: review revision과 review pass evidence count를 분리한다.

## Deferred implementation choices

- concrete storage product and adapter layout
- idempotency/receipt/audit/snapshot의 정확한 duration/byte threshold
- external Harness product/version
- first stable compatibility window 숫자
- Windows/macOS adapter, remote IAM, stronger same-UID process isolation
- TUI/Web, Plugin lifecycle CLI

Deferred 항목은 generic fallback, unbounded default, silent Reference Provider 대체로 구현하지 않는다. executable evidence와 별도 ADR 후 freeze한다.
