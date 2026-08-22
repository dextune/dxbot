---
title: "외부 참조 스냅샷"
document_id: "DXB-DEL-064"
version: "0.8.0"
status: "Reference Snapshot"
normative: false
priority: "P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-013"]
---

# 외부 참조 스냅샷

## 1. 목적

Harness와 외부 구현 자료는 DXBOT 제품 의미를 결정하지 않고 Adapter/Provider 설계의 참고·위험 탐색·compatibility evidence로만 사용한다. 본 문서는 normative하지 않다.

## 2. Snapshot

- 확인 기준일: 2026-08-21
- 주요 참고: DeepSeek Harness 공식 repository/architecture/tool/sandbox/defensive pattern/automation 자료
- 채택 범위: capability seam, configuration composition, append-only execution trajectory, tool pipeline, lifecycle/cancellation/teardown 방어 원칙
- 비채택: upstream Session/Agent/Subagent/Plugin/Storage type을 DXBOT Bot/Core/Memory/Domain public API로 직접 사용

## 3. DXBOT 재해석

| 외부 개념 | DXBOT 적용 |
|---|---|
| Session/event trajectory | Provider/Harness execution record; Bot Domain Journal과 분리 |
| Agent/Subagent | Bot/Core로 자동 매핑하지 않음 |
| Plugin tree | Stable Capability/Provider/Plugin 경계로 재해석 |
| storage hook | canonical Bot/Memory storage 직접 소유 금지 |
| stdio/control protocol | DXBOT public Contract를 대체하지 않는 adapter 참고 |
| sandbox/tool execution | Common permission/resource/side-effect boundary 아래 배치 |

## 4. 구현 규칙

- production dependency는 exact version/commit digest와 license notice를 기록한다.
- upstream type/event를 Domain/public schema로 re-export하지 않는다.
- upgrade는 compatibility matrix, Conformance, fault/security regression을 요구한다.
- developer-preview/불안정 상태면 silent latest 추종을 금지한다.
- 외부 자료와 DXBOT Canonical Owner가 충돌하면 DXBOT owner를 따른다.

## 5. 재검토 Trigger

- stable release/API major/license/security advisory
- ACP/SDK/sidecar compatibility 변화
- Sandbox/permission 모델 변화
- Adapter Conformance 실패
- 다른 Harness Provider가 운영/성능 우위를 증명

## 6. 검증 기준

- release artifact가 외부 dependency version/digest를 출력.
- external upgrade PR에 snapshot/compatibility/conformance evidence.
- upstream type이 Domain/Application public schema를 오염시키지 않음.
- 외부 자료가 제품 원칙을 암묵 변경하지 않음.
