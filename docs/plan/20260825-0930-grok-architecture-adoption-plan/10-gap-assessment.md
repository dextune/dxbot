---
title: "DXBOT 대비 아키텍처 격차 평가"
document_id: "DXB-ADP-010"
version: "0.1.0"
status: "Reference Snapshot"
normative: false
priority: "P0"
last_updated: "2026-08-25"
depends_on: ["DXB-ADP-000"]
target_owners: ["docs/plan", "runtime-host", "runtime-security", "provider-host", "application"]
package_path: "docs/plan/20260825-0930-grok-architecture-adoption-plan"
source_baseline:
  dxbot_commit: "e43739614631c95752482c2ad2ec53cb7ebd251f"
  grok_reconstructed_commit: "a9f633e09d49a85829b8236331b9e21f7e612634"
---
# DXBOT 대비 아키텍처 격차 평가

## 1. 평가 전제

DXBOT은 구조적 분리와 correctness contract에서 우위가 있다. Grok Bot 재구성본은 완성된 desktop Agent 제품을 복원한 것이므로 기능 수와 UI 통합 수준이 높지만, 그 제품 구조를 DXBOT의 목표 모델로 삼을 수는 없다.

따라서 “없는 기능”이 아니라 “현재 DXBOT 설계를 실제 운영 경로에서 증명하기 위해 부족한 contract”를 찾는다.

## 2. 현재 강점과 보완점

| 영역 | DXBOT 현재 강점 | 확인된 보완점 | 분류 |
|---|---|---|---|
| Domain identity | Bot/Conversation/Thread/Task/Project/Channel/Operation 등이 분리된 typed identity | 없음. 외부 구조를 차용할 필요 없음 | 유지 |
| Application boundary | CLI/Control이 Domain concrete API를 직접 소유하지 않음 | Interface 간 동일 의미를 conformance로 계속 증명해야 함 | Close |
| Durable submission | idempotency, receipt, journal, recovery 의미가 강함 | runtime extension 및 provider side effect와 연결된 end-to-end recovery evidence 필요 | Close |
| Runtime lifecycle | start/stop/generation fencing 원칙이 존재 | extension dependency graph, partial-start rollback, readiness aggregation이 executable contract로 덜 닫힘 | Close |
| Scheduler | Dynamic Core Lease, admission, fencing 정의 | Agent별 queue 패턴은 필요 없음. 다만 동일 target ordering constraint의 위치를 scheduler/application에서 명시할 필요 | Close |
| Provider Host | SPI, protocol, harness, DeepSeek adapter 존재 | 전송 성격이 다른 두 구현으로 동일 SPI가 유지되는지 증거 부족 | Close |
| Security | server-derived principal, runtime-security, audit redaction | sandbox artifact/mount/network/secret lifecycle의 통합 계약 부족 | Observe/Close |
| Observability | audit, receipt, event/projection 존재 | runtime degraded reason, extension health, corruption quarantine의 공통 진단 모델 부족 | Adopt |
| Memory | scope, revision, provenance, epistemic state 정의 | 사람이 검사 가능한 projection 표준이 없음. canonical model은 유지 | Adopt |
| Multi-Bot | durable Delegation → recipient Task 의미가 강함 | 일반 Message의 ack, wake, fresh Execution, no-wait 의미가 명시적으로 닫혀 있지 않음 | Close |
| Build provenance | Cargo lock/toolchain/CI 존재 | 외부 runtime binary/image/CLI를 채택할 때 digest·origin·compatibility manifest 규칙 부족 | Observe |

## 3. 가장 큰 실질적 격차

### 3.1 Runtime composition이 원칙에서 구현 계약으로 내려오지 않음

DXBOT 가이드는 lifecycle과 Common Extension 책임을 이미 정의한다. 그러나 현재 runtime-host 구현은 host 상태와 generation fencing 중심이며, 여러 capability가 실제로 조립될 때 다음의 단일 Owner가 아직 충분히 드러나지 않는다.

- dependency declaration과 graph validation
- deterministic startup order
- readiness publication
- startup 중 실패 시 reverse rollback
- admission close 후 drain
- reverse teardown
- teardown 실패의 집계·보고

Grok의 extension graph는 이 간극을 메우는 작은 참고 구현이다. 차용 대상은 TypeScript API가 아니라 위 invariant다.

### 3.2 Runtime 이상 상태를 관찰하고 recovery evidence를 보존하는 공통 경로가 약함

DXBOT은 audit와 error contract가 강하지만, 다음 상황의 공통 진단 envelope가 분산될 위험이 있다.

- extension degraded
- store corruption 의심
- projection watermark 지연
- provider credential expiry
- sandbox drift
- shutdown timeout

Grok의 session diagnostics는 별도 보고 경로를 두고, state-backstop은 size cap과 debounce를 적용해 store snapshot을 별도 저장소에 보존한다. DXBOT이 가져올 부분은 raw DB 복사가 아니라 **정규화된 진단, bounded recovery snapshot, 명시적 restore 검증**이다. Quarantine은 DXBOT의 기존 fail-closed 원칙에서 추가되는 보완이며 Grok 구현을 그대로 복제하는 항목이 아니다.

### 3.3 Provider 추상화가 실제 이질성으로 검증되지 않음

HTTP API Provider와 subprocess/CLI Provider는 cancellation, credential, streaming, process lifecycle이 다르다. 둘을 같은 contract로 통과시키지 않으면 Provider SPI가 특정 구현에 편향될 수 있다.

### 3.4 운영자 inspection 경로가 canonical query에만 의존함

CLI query가 올바른 기본 경로이지만, 장애·백업·지원 상황에서는 사람이 읽을 수 있는 제한된 projection이 유용하다. Grok의 파일 기반 memory는 canonical owner로는 부적합하지만, inspectable projection 아이디어는 채택 가능하다.

## 4. 보완 우선순위

### P0 — 구현 진입 전에 닫을 것

1. Extension composition private contract와 graph/lifecycle test
2. Diagnostics envelope와 degraded/bounded-backstop semantics
3. HTTP형 + subprocess형 Reference Provider conformance

### P1 — M5 전후 vertical slice에서 증명할 것

4. Generic sandbox contract와 하나의 local isolation prototype
5. Message delivery → fresh wake Execution semantics
6. Read-only human projection과 rebuild test
7. 외부 artifact manifest와 tamper detection

## 5. 보완하지 않을 격차

다음은 DXBOT의 결함이 아니다.

- Electron/React UI 부재
- Grok Router settings surface 부재
- Agent directory cloning 부재
- shipped renderer patching 부재
- Cursor/Claude/Codex/OpenRouter 동시 지원 부재
- desktop updater/signing 부재

이는 제품 범위 차이이며 아키텍처 결손으로 처리하지 않는다.
