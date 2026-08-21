---
title: "Repository·CI·Release 운영"
document_id: "DXB-ENG-054"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-GOV-001", "DXB-ARC-011", "DXB-ARC-017", "DXB-ENG-052", "DXB-ENG-053"]
---

# Repository·CI·Release 운영

## 1. 목적

Repository naming/dependency/docs/Provider extension discipline을 유지하고 v0.4 Conversation/Thread/Live Control의 Canonical ownership과 Acceptance traceability가 구현 중 drift하지 않게 CI/Release gate를 정의한다.

## 2. 기존 Repository 규칙 유지

- 사용자 정의 path lowercase kebab-case
- Rust `*.rs` snake_case
- tool-mandated allowlist 예외
- Provider dependency firewall
- Host bypass 금지
- Provider removal/minimal build
- docs link/ID/depends_on/naming 검사

## 3. v0.4 Architecture Gate 후보

static/doc tests:
- Domain/API에서 bare ambiguous `Session` 사용 탐지/allowlist
- Provider Session ID→ThreadId/ConversationId mapping 금지
- Conversation/Thread canonical type가 Interface/Provider crate를 의존하지 않음
- direct Core Lease mutation control API 금지
- Live Control module이 Task state/Side Effect/Scheduler owner를 복제하지 않음
- production Work Queue와 Control Channel bounded config/metric 존재

## 4. Traceability Gate

P0 변경 시 다음 연결을 검사한다.
`Requirement/ADR → Canonical Doc → Domain/Runtime/API → Test/Acceptance → Risk → Migration`.

신규 AT-CONV/THREAD/CTX/MEM/CTRL/SESSION ID가 orphan되지 않아야 한다.

## 5. Release Evidence

- migration fixture result
- deterministic concurrency/fault suite
- control starvation/performance evidence
- Provider Session loss test
- Provider Host/Conformance 기존 evidence
- 3 Review 결과와 발견/수정/재확인 기록

## 6. 검증 기준

- 문서/코드에서 Session/Thread semantic drift를 gate가 탐지할 수 있음.
- v0.4 Acceptance/Risk/Test ID가 orphan 0.
- 기존 AT-SPI/AT-MOD/Provider-free build gate가 회귀하지 않음.
- release artifact에 Review 1/2/3 evidence가 존재함.
