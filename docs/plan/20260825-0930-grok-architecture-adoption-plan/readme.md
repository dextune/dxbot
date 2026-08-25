---
title: "Grok Bot 재구성본 기반 DXBOT 아키텍처 보완 플랜"
document_id: "DXB-ADP-INDEX"
version: "0.1.0"
status: "Reference Snapshot"
normative: false
priority: "P0"
last_updated: "2026-08-25"
depends_on: []
target_owners: ["docs/plan", "runtime-host", "runtime-security", "provider-host", "application"]
package_path: "docs/plan/20260825-0930-grok-architecture-adoption-plan"
source_baseline:
  dxbot_commit: "e43739614631c95752482c2ad2ec53cb7ebd251f"
  grok_reconstructed_commit: "a9f633e09d49a85829b8236331b9e21f7e612634"
---
# Grok Bot 재구성본 기반 DXBOT 아키텍처 보완 플랜

## 1. 문서 성격

이 패키지는 `b-nnett/grok-bot-0.18-reconstructed`에서 확인되는 실용적 런타임 패턴을 DXBOT의 기존 방향과 책임 경계에 맞게 재해석한 **비규범적 보완 계획**이다. 현재 활성 기준인 DXBOT v0.8.10을 대체하거나 새로운 제품 범위를 확정하지 않는다.

Grok Bot 저장소는 공개 배포 앱을 바탕으로 한 비공식 재구성본이다. 따라서 해당 저장소의 파일·모듈 경계를 원본 제품의 정식 아키텍처로 간주하지 않고, 실제 코드에서 검증 가능한 운영 패턴만 참고한다.

## 2. 결론

DXBOT의 Domain/Application/Control/Runtime/Provider 책임 분리는 유지한다. 차용 대상은 다음 일곱 가지로 제한한다.

| 우선순위 | 차용 또는 보완 대상 | DXBOT 현재 상태 | 결론 |
|---|---|---|---|
| P0 | Extension 조립, dependency graph, partial-start rollback, reverse teardown | 설계 원칙은 있으나 실행 가능한 composition contract가 덜 닫힘 | 즉시 보완 |
| P0 | 구조화된 diagnostics와 bounded recovery backstop | audit/security는 있으나 runtime 이상 상태의 공통 관찰과 제한된 recovery snapshot 계약이 분산됨 | 즉시 보완 |
| P0 | Provider SPI의 서로 다른 전송형 구현에 대한 conformance | Provider Host와 Harness가 있으나 HTTP형·subprocess형 이중 증명이 부족함 | 구현 증명 |
| P1 | 로컬 실행 격리와 sandbox lifecycle | 보안·resource 원칙은 있으나 artifact/mount/network/secret/replacement 계약이 덜 구체적 | 계약 우선, 구현 후행 |
| P1 | 사람이 검사 가능한 read-only projection | Projection 개념은 있으나 운영자가 파일 수준에서 검증할 표준 산출물이 없음 | derived projection로 도입 |
| P1 | 비동기 Bot message → fresh wake/Execution 의미 | Conversation·Delegation은 있으나 즉시 ack와 후속 wake의 경계가 덜 명시적 | 기존 의미를 폐쇄 |
| P1 | 외부 runtime/artifact provenance와 검증 | Rust 빌드는 안정적이나 외부 Provider/실행 artifact 유입 시 공통 검증 규칙이 없음 | 조건부 도입 |

다음은 차용하지 않는다.

- Agent directory 또는 `store.db`를 Bot의 canonical aggregate로 삼는 구조
- Agent별 Promise queue를 Dynamic Core Scheduler로 대체하는 구조
- Electron Main 또는 UI process가 Domain state를 소유하는 구조
- Provider별 분기를 Application/Coordinator에 확산시키는 구조
- 사람이 편집하는 Markdown/JSON 파일을 Memory·Identity의 원본으로 삼는 구조
- 고정 sleep, 주기 pulse, 오류 삼키기, transcript에 오류문을 삽입하는 우회책

## 3. 적용 원칙

1. 새 crate를 먼저 만들지 않는다. 기존 Canonical Owner가 의미를 소유할 수 없는 것이 증명될 때만 분리한다.
2. public contract 변경은 Acceptance evidence 전까지 금지한다.
3. 차용은 구현 형태가 아니라 invariant, lifecycle, failure semantics를 대상으로 한다.
4. canonical state와 projection/cache/index를 혼합하지 않는다.
5. local/remote, HTTP/subprocess, human/automation 경로가 동일 Application Contract를 사용해야 한다.
6. 모든 queue, stream, retry, fan-out, diagnostic payload, artifact는 item·byte·time bound를 가진다.
7. 구현 후 Structural/Consistency와 Cross-Layer Executability 재검수를 각각 수행한다.

## 4. 읽기 순서

1. [범위와 판단 원칙](00-scope-and-decision-principles.md)
2. [격차 평가](10-gap-assessment.md)
3. 제안별 상세 문서
   - [Extension lifecycle과 runtime composition](20-extension-lifecycle-and-runtime-composition.md)
   - [로컬 실행 격리와 sandbox lifecycle](21-local-execution-isolation-and-sandbox-lifecycle.md)
   - [Observability, diagnostics, state backstop](22-observability-diagnostics-and-state-backstop.md)
   - [사람이 검사 가능한 projection과 recovery evidence](23-human-readable-projections-and-recovery-evidence.md)
   - [Multi-Bot messaging과 wake semantics](24-multi-bot-messaging-and-wake-semantics.md)
   - [Provider adapter conformance와 usage accounting](25-provider-adapter-conformance-and-usage-accounting.md)
   - [Build provenance와 artifact verification](26-build-provenance-and-artifact-verification.md)
4. [비차용 경계](30-non-adoption-boundaries.md)
5. [통합 로드맵](40-integrated-roadmap.md)
6. [Acceptance와 테스트 계획](50-acceptance-and-test-plan.md)
7. [위험 등록부](60-risk-register.md)
8. [근거 소스 맵](90-source-map.md)
9. [최종 재검수 보고](99-validation-report.md)

## 5. 이 패키지가 하지 않는 일

- v0.8.10의 milestone, command set, wire schema를 변경하지 않는다.
- TUI/Web/Electron/Desktop UI를 도입하지 않는다.
- Docker를 필수 runtime으로 결정하지 않는다.
- Plugin 범위를 P0으로 끌어오지 않는다.
- 범용 workflow engine, service locator, dynamic ABI, distributed consensus를 추가하지 않는다.
- Grok Bot의 제품 UX나 파일 구조를 복제하지 않는다.

## 6. 최종 산출 조건

이 문서의 제안은 각 Canonical Owner 문서에 반영되고, 최소 vertical slice와 fault evidence가 생긴 뒤에만 규범으로 승격할 수 있다. 이 패키지 자체는 의사결정 입력이며 제품 의미의 최종 Owner가 아니다.
