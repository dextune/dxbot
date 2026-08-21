---
title: "Repository·CI·Release 운영"
document_id: "DXB-ENG-054"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-GOV-001", "DXB-ARC-011", "DXB-ARC-017", "DXB-ENG-052", "DXB-ENG-053"]
---

# Repository·CI·Release 운영

## 1. 목적

Repository naming/dependency/docs/Provider extension discipline과 v0.6 Backend gates를 유지하면서, v0.7 Headless Application Contract/CLI를 build/test/package/release하고 superseded Interface의 stale artifact/dependency를 남기지 않는다.

## 2. Repository 규칙

- 사용자 정의 path lowercase kebab-case
- Rust `*.rs` snake_case
- tool-mandated allowlist 예외
- Provider dependency firewall/Host bypass 금지
- docs link/ID/depends_on/naming 검사
- 제거 기능의 stale config/schema/reference/orphan dependency 금지

## 3. v0.7 Build Artifact 후보

- Runtime Host binary/service artifact
- `dxb` CLI binary

두 artifact가 하나의 installer로 배포되는지 여부와 process/lifecycle 독립성을 혼합하지 않는다.

v0.7 active release artifact에 TUI/Web binary/package/static bundle을 포함하지 않는다.

## 4. Architecture / Docs Gate

static/doc tests:
- CLI→Domain/Runtime/Storage/Provider forbidden dependency
- Control Adapter→Domain Store bypass
- Host Lifecycle Client→Domain/Task/Memory/Scheduler/Provider mutation/internal dependency
- Runtime Ready 이후 Domain use-case의 Host Lifecycle shortcut
- public DTO/Domain/Persistence type sharing 금지
- direct Core Lease mutation control API 금지
- active `DXB-IFC-042/043` path/dependency/release gate 0
- TUI/Web 전용 crate/module/package manifest 0
- document inventory/count/link/ID/depends_on 검증

## 5. Contract / CLI CI

- Runtime Host/CLI build
- Runtime absent→host bootstrap→readiness→Control connect smoke
- Application Contract schema tests
- compatibility matrix fixture
- command idempotency/lost-response fixture
- subscription resume/gap fixture
- CLI machine output golden/schema test
- stable exit class test
- shell/platform smoke where supported
- SIGINT/broken pipe/slow consumer fixture
- large output streaming/RSS benchmark
- secret redaction/security test

## 6. Traceability Gate

P0 변경은 다음 연결을 검사한다.

`Requirement/ADR → Canonical Doc → Application/Domain/Runtime → Contract/CLI → Test/Acceptance → Risk → Migration`.

AT-APP/AT-CLI ID와 R-078~089/OQ-068~079가 orphan되지 않아야 한다.

## 7. Release Evidence

- v0.6 Backend regression suite
- Runtime Host bootstrap/lifecycle fixture
- protocol compatibility matrix
- CLI E2E/scripting output evidence
- large-output/slow-consumer performance evidence
- forbidden dependency result
- secret redaction result
- Review 1 Structural/Consistency evidence
- Review 2 Cross-Layer Executability/Compatibility evidence

## 8. 검증 기준

- Runtime Host/`dxb` CLI build/release 경로 존재.
- Host Lifecycle Client의 Domain shortcut 0.
- TUI/Web active build/package/release artifact 0.
- `DXB-IFC-042/043` active docs/dependency/gate 0.
- AT-APP/CLI traceability orphan 0.
- 기존 Provider-free/minimal build/Backend Acceptance gate regression 0.
- release artifact에 2회 Review evidence가 존재한다.
