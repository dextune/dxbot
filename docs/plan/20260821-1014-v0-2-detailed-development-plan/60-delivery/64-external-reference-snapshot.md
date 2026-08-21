---
title: "외부 참조 스냅샷"
document_id: "DXB-DEL-064"
version: "0.1.0"
status: "Reference Snapshot"
normative: false
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-013"]
---


# 외부 참조 스냅샷

## 1. 목적

DXBOT Harness 설계에 참고한 외부 자료의 확인 시점과 채택 범위를 기록한다. 외부 자료는 「DXBOT 최상위 기준선」을 변경하지 않는다.

## 2. 책임 범위

- DeepSeek Harness 공식 자료
- 확인일과 상태
- 채택한 개념
- 채택하지 않거나 격리한 구현 세부
- 재검토 조건

## 3. 확인일

- Snapshot date: **2026-08-21**
- DeepSeek Harness status: 공식 저장소와 소개 페이지 기준 **developer preview**
- 주의: 공식 저장소가 호환성 파괴 변경 가능성을 명시하므로 exact version/commit pin과 Adapter conformance가 필요하다.

## 4. 공식 참조

1. DeepSeek Harness 공식 저장소  
   <https://github.com/deepseek-ai/deepseek-harness>

2. 공식 제품/개발자 프리뷰 소개  
   <https://deepseek.com/harness/en/>

3. 공식 Architecture 문서  
   <https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/architecture.md>

4. 공식 ACP automation 예시  
   <https://github.com/deepseek-ai/deepseek-harness/blob/master/examples/acp-agent/readme.md>

5. 공식 Tool execution pipeline  
   <https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/tool-execution-pipeline.md>

6. 공식 Sandbox subsystem 문서  
   <https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/subsystems/sandbox.md>

7. 공식 Defensive patterns  
   <https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/defensive-patterns.md>

8. 공식 License  
   <https://github.com/deepseek-ai/deepseek-harness/blob/master/LICENSE>

## 5. 참고한 핵심 개념

- Everything is a plugin
- Service/Provider/Consumer capability seam
- 구성 기반 합성
- append-only execution/session events
- model-visible input 재현
- turn/step/tool pipeline 분리
- live hook와 durable event 구분
- reversible registration/lifecycle
- automation-oriented stdio protocol
- cancellation/error/teardown defensive rules

## 6. DXBOT에서 재해석한 항목

| 외부 개념 | DXBOT 적용 |
|---|---|
| Session event log | Harness trajectory로 사용; Bot Domain Journal과 분리 |
| Agent | DXBOT Bot과 자동 동일시하지 않음 |
| Subagent | Core 또는 다른 Bot으로 자동 매핑하지 않음 |
| Plugin tree | Rust Capability Registry/Adapter 원칙으로 재해석 |
| Profiles/Bundles | DXBOT typed profile/config snapshot으로 재해석 |
| Session fork/resume | Harness 실행 기능; Bot clone/lifecycle과 분리 |
| UI plugin | Control API 이후 독립 Interface로 제한 |
| Storage plugin | Bot canonical storage는 DXBOT 소유 |

## 7. 채택하지 않은 기본 가정

- upstream 내부 package/type/event를 DXBOT public API로 사용
- developer preview 최신판 자동 추종
- Harness Session이 Bot Memory를 소유
- upstream의 agent/subagent hierarchy가 DXBOT Bot/Core 의미를 결정
- TypeScript runtime을 DXBOT Core Runtime으로 간주
- 외부 plugin이 Domain Store를 직접 수정
- Harness trajectory 전체를 Domain Event에 중복 저장

## 8. 안전·성숙도 참고

비공식/연구 자료는 위험 탐색에만 사용하고 Normative 근거로 삼지 않는다. 예:
- DeepSeek Harness 간접 prompt injection 평가 논문  
  <https://arxiv.org/abs/2608.16393>

해당 자료의 특정 수치를 제품 보증으로 해석하지 않으며, DXBOT은 별도 threat model과 security regression suite를 유지한다.

## 9. 재검토 트리거

- DeepSeek Harness 첫 안정 release
- protocol/API major 변경
- ACP/SDK 안정성 선언
- license/third-party notice 변경
- sandbox/permission 모델 변경
- DXBOT sidecar conformance 실패
- Rust-native Harness가 성능/운영 우위를 보임
- upstream 보안 권고

## 10. 데이터 흐름과 상호작용

외부 자료 확인 → Reference Snapshot 갱신 → Adapter compatibility matrix → spike/conformance → ADR → 구현. 외부 변경이 Domain 문서에 직접 반영되지 않는다.

## 11. 예외상황

- URL/branch 내용이 변할 수 있으므로 실제 구현은 commit digest를 별도 compatibility manifest에 기록한다.
- 외부 문서와 코드가 충돌하면 코드/테스트를 확인하고 upstream issue를 기록한다.
- 공식 상태가 안정으로 바뀌어도 DXBOT 경계를 제거하지 않는다.
- 라이선스가 허용적이어도 production suitability를 자동 의미하지 않는다.

## 12. 확장성

향후 다른 Harness/Model/Tool/Sandbox Provider도 같은 snapshot 형식으로 기록한다. 각 Provider는 채택 개념, 비채택 개념, 버전, license, conformance를 가진다.

## 13. 구현 우선순위

- **P0:** DeepSeek commit/version pin, ACP/sidecar spike, license notice
- **P1:** compatibility matrix와 scheduled canary
- **P2:** 다중 Harness reference catalog

## 14. 검증 기준

- release build가 사용한 Harness version/digest를 출력한다.
- 외부 upgrade PR에 snapshot/compatibility/conformance 결과가 포함된다.
- upstream 타입이 Domain crate에 노출되지 않는다.
- 외부 자료가 최상위 제품 원칙을 암묵적으로 변경하지 않는다.
