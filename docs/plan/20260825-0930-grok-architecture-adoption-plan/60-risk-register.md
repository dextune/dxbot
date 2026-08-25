---
title: "위험 등록부와 완화 전략"
document_id: "DXB-ADP-060"
version: "0.1.0"
status: "Reference Snapshot"
normative: false
priority: "P0"
last_updated: "2026-08-25"
depends_on: ["DXB-ADP-040", "DXB-ADP-050"]
target_owners: ["docs/plan", "all"]
package_path: "docs/plan/20260825-0930-grok-architecture-adoption-plan"
source_baseline:
  dxbot_commit: "e43739614631c95752482c2ad2ec53cb7ebd251f"
  grok_reconstructed_commit: "a9f633e09d49a85829b8236331b9e21f7e612634"
---
# 위험 등록부와 완화 전략

## 1. 위험 요약

| ID | 위험 | 가능성 | 영향 | 완화 |
|---|---|---:|---:|---|
| R-01 | 외부 재구성본을 공식 원본처럼 신뢰 | 중 | 높음 | pinned code pattern만 근거로 사용, source caveat 유지 |
| R-02 | Extension 보완이 범용 plugin framework로 팽창 | 중 | 높음 | private contract, 새 crate 금지, public schema freeze |
| R-03 | Diagnostics가 audit와 중복 저장 | 높음 | 중 | 목적·owner·보존 분리, correlation ref만 공유 |
| R-04 | Backstop snapshot이 최신 canonical truth 또는 자동 복구기로 오인 | 중 | 높음 | source watermark/digest 검증, explicit restore operation, receipt 없는 repair 금지 |
| R-05 | Sandbox가 Docker 종속 제품 기능으로 선행 | 중 | 중 | generic contract → subprocess prototype → adapter 후행 |
| R-06 | Credential 편의 구현이 secret boundary를 우회 | 중 | 매우 높음 | whole-dir mount 금지, SecretGrant, expiry/revoke |
| R-07 | Human projection이 두 번째 canonical state가 됨 | 중 | 높음 | read-only, watermark/digest, startup source 금지 |
| R-08 | Message wake가 ping-pong/폭주를 유발 | 중 | 높음 | causal depth, dedupe, admission, fan-out bound |
| R-09 | Provider conformance가 provider 수 확장으로 변질 | 중 | 중 | reference adapter 2개만, 목적을 SPI 증명으로 고정 |
| R-10 | Usage 수치를 invoice truth로 오인 | 중 | 중 | source/completeness/estimated 구분 |
| R-11 | Artifact verification이 과도한 supply-chain 플랫폼으로 팽창 | 낮음 | 중 | 실제 외부 artifact 경로에만 최소 manifest 적용 |
| R-12 | 새 lifecycle layer가 runtime-host와 provider-host owner를 중복 | 중 | 높음 | orchestration=runtime-host, adapter lifecycle=provider-host |
| R-13 | readiness projection이 authoritative state와 불일치 | 중 | 높음 | generation/watermark binding, rebuild, stale 표시 |
| R-14 | shutdown rollback이 무한 대기 | 중 | 높음 | deadline, bounded teardown, failure aggregation |
| R-15 | 문서가 구현보다 다시 앞서감 | 높음 | 높음 | 단계별 code+test+doc 동시 변경, evidence 없는 승격 금지 |

## 2. 주요 완화 상세

### 2.1 Scope creep 통제

각 단계에서 다음 질문 중 하나라도 “예”이면 중단한다.

- 새 user-facing command가 필요한가.
- generic workflow/plugin/event platform이 필요한가.
- public schema를 먼저 고정하려는가.
- 한 항목을 위해 세 개 이상의 새 crate가 필요한가.
- 현재 owner가 불분명한 상태로 구현을 시작하는가.

### 2.2 Canonical/Derived 분리

Projection, diagnostics, usage, readiness, artifact cache는 모두 다음을 가져야 한다.

- source owner
- source revision/watermark
- schema version
- digest 또는 integrity check
- stale/partial 상태
- rebuild/delete semantics

### 2.3 Security 완화

- principal은 transport/control boundary에서 server-derived 상태 유지
- sandbox와 provider에 최소 SecretGrant만 전달
- diagnostic write 전 redaction
- external artifact execute 전 digest/permission 검증
- cross-Bot message에 information-flow policy 적용

### 2.4 Rollback 전략

| 항목 | rollback |
|---|---|
| Extension composition | 기존 단일 host path를 test-only fallback으로 유지하되 public 의미는 동일 |
| Diagnostics | sink 비활성화 가능, canonical logic 독립 |
| Sandbox | adapter 제거 후 in-process/reference provider 경로 유지 |
| Projection | 디렉터리 삭제 가능, canonical store 영향 없음 |
| Message wake | wake worker 중지, Message commit/query 유지 |
| Provider adapter | adapter 제거 build, Common SPI 유지 |
| Artifact cache | cache 삭제 후 재검증/재획득 |

Rollback을 silent fallback으로 사용하지 않는다. 기능이 unsupported/degraded가 되면 명시한다.

## 3. 잔여 위험

- Grok 재구성 코드가 원본에서 추정된 구조라는 한계
- DXBOT 실제 구현이 빠르게 변해 gap inventory가 stale해질 가능성
- subprocess Provider의 플랫폼별 signal/process semantics
- 파일시스템별 atomic rename/fsync 차이
- container/VM별 isolation 강도 차이

이 위험은 문서로 제거할 수 없으며, platform matrix와 fault test evidence로만 줄일 수 있다.
