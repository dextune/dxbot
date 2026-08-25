---
title: "범위, 근거 수준, 차용 판단 원칙"
document_id: "DXB-ADP-000"
version: "0.1.0"
status: "Reference Snapshot"
normative: false
priority: "P0"
last_updated: "2026-08-25"
depends_on: ["DXB-ADP-INDEX"]
target_owners: ["docs/plan"]
package_path: "docs/plan/20260825-0930-grok-architecture-adoption-plan"
source_baseline:
  dxbot_commit: "e43739614631c95752482c2ad2ec53cb7ebd251f"
  grok_reconstructed_commit: "a9f633e09d49a85829b8236331b9e21f7e612634"
---
# 범위, 근거 수준, 차용 판단 원칙

## 1. 목적

외부 프로젝트의 구현을 그대로 모방하지 않고, DXBOT에서 이미 정의된 Identity·Memory·State·Application Contract·Dynamic Core·Provider Host의 경계를 강화하는 패턴만 선별한다.

평가 질문은 다음으로 고정한다.

1. DXBOT에 같은 의미가 이미 존재하는가.
2. 존재한다면 문서에만 있고 실행 가능한 contract/test가 없는가.
3. 새 의미가 아니라 기존 invariant를 더 강하게 만드는가.
4. canonical owner를 단일하게 유지하는가.
5. 제거·교체·복구·관찰 가능성을 개선하는가.
6. 사용자-facing 기능 확장 없이도 도입 가능한가.
7. 메모리·캐시·동시성·failure bound를 명시할 수 있는가.

## 2. 근거 수준

### 2.1 사용할 수 있는 근거

- pinned commit의 코드에 존재하는 lifecycle, graph validation, atomic write, health check, sandbox replacement, async messaging, provider routing, packaging verification
- DXBOT의 활성 plan, Agent Guide, workspace와 현재 구현 간 차이
- 두 저장소에서 재현 가능한 파일 경로와 테스트 진입점

### 2.2 사용할 수 없는 근거

- 재구성본 파일명이 원본 제품의 공식 모듈 경계라는 가정
- UI 동작만 보고 내부 invariant를 추정한 내용
- README의 기능 설명만 있고 코드 또는 테스트로 확인되지 않는 의미
- Grok Bot에 존재한다는 이유만으로 DXBOT에 필요하다고 보는 판단

## 3. 분류 체계

| 분류 | 의미 | 처리 |
|---|---|---|
| `Adopt` | DXBOT에 없고 기존 방향을 강화하며 owner가 명확함 | 계획에 포함 |
| `Close` | DXBOT 문서에 있으나 code/test/failure semantics가 덜 닫힘 | 기존 owner에서 실행 가능하게 폐쇄 |
| `Observe` | 장기적으로 유용하나 현재 milestone 범위를 넘음 | 인터페이스만 보존하고 구현 보류 |
| `Reject` | DXBOT 책임 경계를 오염시키거나 제품 특수 구현임 | 비차용 경계에 기록 |

## 4. 비협상 원칙

### 4.1 Canonical Owner 우선

- Runtime composition과 host lifecycle은 `runtime-host`가 소유한다.
- startup artifact와 endpoint publication은 `runtime-bootstrap`이 소유한다.
- sandbox policy, secret grant, permission, isolation은 `runtime-security`가 소유한다.
- Provider lifecycle, transport adaptation, usage normalization은 `provider-host`가 소유한다.
- durable Message/Task/Delegation/Execution 의미는 `application`과 Domain Owner가 소유한다.
- public DTO는 evidence 이후 `application-contract`만 소유한다.
- CLI는 입력·표시·local journal을 소유할 뿐 runtime/domain 의미를 만들지 않는다.

### 4.2 기존 추상화 우선

새 `extension-manager`, `sandbox-framework`, `event-platform`, `projection-service` crate를 기본안으로 제안하지 않는다. 먼저 현재 crate 내부의 모듈과 private type으로 vertical slice를 구현한다.

### 4.3 Derived State 방화벽

다음은 모두 canonical state가 아니다.

- 사람용 Markdown/JSON projection
- search index
- readiness summary
- diagnostic aggregation
- provider usage projection
- runtime artifact cache

각 derived state는 source revision 또는 watermark, schema version, digest를 가져야 하며 삭제 후 재생성 가능해야 한다.

### 4.4 Failure를 정상 상태로 취급

모든 제안은 다음을 설계 범위로 포함한다.

- partial startup
- cancellation과 deadline
- response loss와 duplicate delivery
- provider crash/credential expiry
- projection stale/corruption
- sandbox replacement 중 실패
- diagnostic sink failure
- shutdown 중 teardown failure

## 5. Scope Gate

| 항목 | P0 허용 | P1 이후 |
|---|---|---|
| Extension graph validation/rollback | 허용 | 해당 없음 |
| Provider dual-shape conformance | 허용 | 실제 Provider 확대 |
| Runtime diagnostics/backstop | 허용 | 고급 dashboard |
| Generic sandbox contract | 문서·private prototype | Docker/VM implementation |
| Human-readable projection | 최소 inspect/export | editable import workflow |
| Multi-Bot wake semantics | 기존 command 의미 폐쇄 | 새 channel UX |
| Artifact provenance | 외부 artifact가 들어오는 경로만 | 전사 SBOM/attestation |

## 6. 승격 조건

이 패키지의 항목을 normative plan에 반영하려면 다음이 모두 필요하다.

1. Canonical Owner가 하나로 결정된다.
2. 기존 public contract를 변경하지 않는 private vertical slice가 있다.
3. 정상·실패·복구 경로 테스트가 있다.
4. resource bound와 redaction이 검증된다.
5. 제거 시나리오가 정의된다.
6. 문서와 코드의 두 차례 재검수가 완료된다.
