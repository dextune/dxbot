---
title: "통합 구현 로드맵"
document_id: "DXB-ADP-040"
version: "0.1.0"
status: "Reference Snapshot"
normative: false
priority: "P0"
last_updated: "2026-08-25"
depends_on: ["DXB-ADP-020", "DXB-ADP-021", "DXB-ADP-022", "DXB-ADP-023", "DXB-ADP-024", "DXB-ADP-025", "DXB-ADP-026", "DXB-ADP-030"]
target_owners: ["docs/plan", "runtime-host", "runtime-security", "provider-host", "application"]
package_path: "docs/plan/20260825-0930-grok-architecture-adoption-plan"
source_baseline:
  dxbot_commit: "e43739614631c95752482c2ad2ec53cb7ebd251f"
  grok_reconstructed_commit: "a9f633e09d49a85829b8236331b9e21f7e612634"
---
# 통합 구현 로드맵

## 1. 활성 v0.8.10과의 관계

이 로드맵은 v0.8.10 milestone을 대체하지 않는다. 기존 M0~M6의 Acceptance gate 안에 삽입 가능한 보완 작업을 정의한다.

원칙:

- M1B public freeze 금지 유지
- M2/M3 subset freeze는 Acceptance evidence 전까지 BLOCKED 유지
- 새 CLI command, TUI/Web/Plugin scope 추가 금지
- 먼저 private contract/test로 증명

## 2. 단계 개요

```text
A0 Decision lock
  ↓
A1 Runtime composition closure
  ↓
A2 Diagnostics/backstop closure
  ├────────────┐
  ↓            ↓
A3 Provider    A4 Sandbox contract
conformance      prototype
  └──────┬─────┘
         ↓
A5 Multi-Bot message/wake closure
         ↓
A6 Human projection/recovery evidence
         ↓
A7 Artifact provenance/publication check
```

A1과 A2가 나머지 단계의 공통 기반이다.

## 3. A0 — Decision lock

### 산출물

- 이 패키지의 `Adopt/Close/Observe/Reject` 결정 승인
- 각 항목의 Canonical Owner 확인
- current code inventory와 gap list
- 새 crate 없음 원칙 확인

### Gate

- owner 중복 없음
- 활성 plan과 command/schema 충돌 없음
- 비차용 경계 합의

## 4. A1 — Runtime composition closure

### 범위

- private ExtensionDescriptor
- graph validation
- deterministic resolution
- partial start rollback
- reverse teardown
- readiness aggregate

### 영향을 받는 계층

- `runtime-host`
- `runtime-bootstrap`
- `runtime-audit`
- test-only provider extension

### 완료 기준

`DXB-ADP-020` Acceptance 전부 통과. 아직 public extension schema를 만들지 않는다.

## 5. A2 — Diagnostics와 backstop closure

### 범위

- DiagnosticEvent private schema
- family/reason registry
- redaction/bounds
- runtime health projection
- quarantine prototype

### 완료 기준

- lifecycle/provider/storage 세 계층에서 최소 한 family씩 연결
- sink failure와 corruption fault test
- audit와 diagnostics owner 중복 없음

## 6. A3 — Provider conformance

### 범위

- adapter-independent conformance suite
- HTTP Reference Adapter
- subprocess Reference Adapter
- streaming/cancel/deadline/tool/usage/credential lifecycle

### 완료 기준

두 adapter가 같은 suite를 통과하고 Application/CLI에 provider branch가 없다.

### 기존 milestone 연결

M5 real Harness canary의 선행 증거로 사용한다. provider 수를 늘리는 단계가 아니다.

## 7. A4 — Sandbox contract prototype

### 범위

- Docker 비종속 SandboxSpec/lifecycle
- local subprocess isolation
- generation/ownership/resource/cleanup
- content-addressed runtime artifact

### 완료 기준

- canonical store/credential directory 직접 노출 없음
- crash/cancel/deadline 후 누수 없음
- stale generation fencing

Docker adapter는 이 단계의 필수 산출물이 아니다.

## 8. A5 — Multi-Bot Message/Wake closure

### 범위

- Message와 Delegation 의미 분리
- durable send acknowledgement
- delivery intent
- recipient fresh wake Execution
- idempotency/recovery
- bounded Channel fan-out

### 완료 기준

single recipient vertical slice와 crash-between-delivery-and-wake fault test 통과.

### 기존 milestone 연결

M4 Project/Channel membership + typed delegation과 함께 검증하되, 새로운 chat UX를 추가하지 않는다.

## 9. A6 — Human projection과 recovery evidence

### 범위

- instance summary 하나
- manifest/digest/watermark
- atomic publication
- local safe reader
- stale/corrupt/partial 명시

### 완료 기준

projection 삭제·corruption·partial-write 후 canonical store에서 수렴한다.

## 10. A7 — Artifact provenance

### 범위

- 외부 subprocess artifact 하나
- checksum/manifest/staging
- runtime binding
- archive publication check

### 완료 기준

tamper와 partial download가 startup 전에 차단된다.

## 11. 변경 단위 원칙

각 단계는 다음의 가장 작은 완결 단위로 제출한다.

```text
owner document update
+ private code
+ unit/conformance/fault tests
+ safe diagnostic
+ removal note
+ validation evidence
```

문서만 먼저 장기간 누적하거나, 코드만 추가해 owner 문서를 뒤늦게 맞추지 않는다.

## 12. 우선순위와 중단 조건

| 단계 | 우선순위 | 중단 조건 |
|---|---|---|
| A0 | P0 | owner 충돌 또는 v0.8.10 scope 확장 발견 |
| A1 | P0 | composition이 generic plugin framework로 확대됨 |
| A2 | P0 | audit/diagnostic 중복 저장 발생 |
| A3 | P0 | provider branch가 Application에 유출됨 |
| A4 | P1 | Docker 도입이 선행 조건으로 변함 |
| A5 | P1 | Message를 범용 workflow로 확장함 |
| A6 | P1 | projection이 canonical write path가 됨 |
| A7 | P1 | 조직 전역 supply-chain 플랫폼으로 확대됨 |

## 13. 예상 파일 영향 범위

구현 시 재검토할 후보이며 확정 파일 목록이 아니다.

```text
crates/runtime-host/
crates/runtime-bootstrap/
crates/runtime-security/
crates/runtime-audit/
crates/provider-host/
crates/application/
crates/application-contract/   # evidence 이후만
crates/cli/                    # 기존 surface 연결만
docs/plan/<active-package>/
docs/agent/                    # 보편 구현 규칙이 바뀌는 경우만
```

## 14. 완료 판정

A1~A3가 통과해야 이번 플랜의 P0 보완이 완료된다. A4~A7은 실제 milestone 필요성과 evidence에 따라 순차 채택하며, 사용되지 않는 architecture placeholder를 미리 public contract에 남기지 않는다.
