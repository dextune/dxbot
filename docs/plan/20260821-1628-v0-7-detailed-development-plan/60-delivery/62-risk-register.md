---
title: "위험 등록부"
document_id: "DXB-DEL-062"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DEL-060", "DXB-ARC-017", "DXB-DOM-022", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-032", "DXB-RUN-037", "DXB-RUN-038", "DXB-IFC-040", "DXB-IFC-041", "DXB-ENG-051"]
---

# 위험 등록부

## 1. 목적

v0.6의 R-001~077과 R-SPI-*를 유지하면서 v0.7 Application Contract/CLI/Runtime Host 계열 Risk를 Acceptance/Test/OQ/Metric에 연결한다.

`DXB-IFC-042/043`에만 해당하는 과거 Interface-specific Risk는 v0.7 active release blocker로 상속하지 않는다. Backend/common contract Risk는 계속 유효하다.

## 2. 기존 위험 유지

특히 Session/Thread 혼합, unbounded queue/cache, stale authorization, Side Effect duplicate, Provider Domain 침투, Project/Channel Brain화, Shared Memory 복제, Channel fan-out, Role/Authority 혼합, Process replay, epistemic/retraction, Private→Shared leak, ActionGrant replay, Runtime Memory/OOM/leak/recovery storm 위험은 v0.7에서도 직접 regression gate다.

## 3. v0.7 신규 Risk

| ID | 위험 | 영향 | 핵심 완화/Gate |
|---|---|---|---|
| **R-078** | CLI가 Domain/permission/scheduler/memory policy를 재구현 | Critical | IFC-040/041 boundary + forbidden dependency + AT-CLI-005 |
| **R-079** | Application Contract가 특정 local transport에 종속 | High | transport-neutral semantic + adapter separation + ADR-0077 |
| **R-080** | CLI 종료가 Runtime/Task/Process lifecycle 종료로 연결 | Critical | Runtime Host independence + AT-CLI-002 |
| **R-081** | human output 변화가 automation을 파괴 | High | JSON/JSONL + stable exit/schema + AT-CLI-003 |
| **R-082** | `--all`/export/watch가 CLI 또는 Runtime OOM 유발 | Critical | paging/streaming/byte cap + AT-CLI-004 |
| **R-083** | retry/response loss가 duplicate mutation/Side Effect 생성 | Critical | CommandId/idempotency/receipt + AT-APP-003 |
| **R-084** | event gap/reconnect 후 stale 상태를 current로 표시 | High | cursor/watermark/resync + AT-APP-004 |
| **R-085** | CLI argv/config/log/trace를 통한 secret leakage | Critical | credential boundary/redaction/security tests |
| **R-086** | CLI/Runtime version skew가 semantic corruption 유발 | Critical | explicit negotiation/matrix + AT-APP-002 |
| **R-087** | superseded Interface 문서가 normative inheritance로 active scope에 잔존 | High | explicit supersession + inventory/reference audit + AT-CLI-007 |
| **R-088** | CLI가 Control Contract 대신 internal Rust API에 결합 | High | architecture test + host-path E2E |
| **R-089** | slow stdout/pipe가 Runtime stream/resource를 고갈 | High | bounded buffers/backpressure/gap policy + AT-CLI-004 |

## 4. Risk Cluster / Owner

### Contract Correctness — R-078/079/083/084/086/088
Owner: IFC-040 + ARC-010/014 + ENG-053. Gate: M1/M2/M4.

### CLI Lifecycle / Automation — R-080/081/089
Owner: IFC-041 + RUN-030/033 + ENG-050/051. Gate: M2~M5.

### Security — R-085
Owner: RUN-032 + IFC-041. Gate: M2/M5.

### Scope / Governance — R-087
Owner: BASE-000 + ENG-052/054 + Manifest. Gate: M0/M5.

### Resource — R-082
Owner: RUN-031 + ENG-051. Gate: M4/M5.

## 5. 선행 신호

- CLI crate가 Domain/Runtime/Provider internal dependency 추가
- CLI local Role/Authority branch 증가
- same idempotency key인데 canonical write count 증가
- response-loss 후 duplicate task/action 발생
- cursor gap인데 stale/current 표시 없음
- slow pipe에서 subscriber bytes/count 계속 증가
- CLI RSS가 exported dataset과 선형 증가
- argv/config/trace에서 secret canary 검출
- version mismatch인데 command가 실행됨
- v0.7 inventory에 42/43 문서 또는 active reference 등장

## 6. Phase Blocker

- M0: R-087/088
- M1: R-078/079/083/084/086
- M2: R-080/085/086/088
- M3: R-078/081/085
- M4: R-082/083/084/089
- M5: R-080~089 전체 residual evidence

Critical Risk owner/evidence 없이 해당 Gate를 종료하지 않는다.

## 7. 검증 기준

- R-078~089 모두 Acceptance/Test/OQ/Metric 중 하나 이상에 연결된다.
- 기존 R-001~077/R-SPI mitigation regression 0.
- superseded TUI/Web 전용 Risk를 active blocker로 유지하지 않는다.
