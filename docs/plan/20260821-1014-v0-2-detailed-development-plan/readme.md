---
title: "DXBOT 상세 개발 기획 문서 집합"
document_id: "DXB-INDEX"
version: "0.2.0"
status: "Working Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000"]
---

# DXBOT 상세 개발 기획 문서 집합 v0.2

이 디렉터리는 기존 v0.1 상세 개발 기획을 유지하면서, 문서 개선 플랜의 요구를 반영해 **모듈성·교체 가능성·제거 가능성·복구 완결성·Repository 규범성**을 강화한 신규 버전이다.

v0.2는 DXBOT의 제품 정체성을 재설계하지 않는다. 기존의 Persistent Bot, Memory-Centric Runtime, Single Brain/Dynamic Core, Port/Adapter, Canonical State 원칙을 유지하고, 선택 기능을 프로젝트 전체에서 일관되게 탈부착할 수 있도록 계약을 강화한다.

## 1. 기준 우선순위

충돌 시 다음 순서로 해석한다.

1. `00-governance/00-source-concept-original.md`의 제품 철학
2. `00-governance/00-normative-baseline.md`의 v0.2 비협상 기준
3. 승인된 ADR 및 `03-architecture-decision-baseline.md`
4. 도메인 문서의 불변조건과 Canonical Owner
5. Runtime·Interface·Engineering 문서
6. 구현 코드와 테스트

하위 문서는 상위 의미를 변경하지 않는다. 의미 변경이 필요하면 기준 문서, 영향 문서, Migration/Removal 판단, Acceptance, Risk를 같은 변경 세트로 갱신한다.

## 2. v0.2 핵심 강화점

### 2.1 기능을 5개 범주로 분류
모든 기능은 구현 전에 반드시 다음 중 하나로 분류한다.

- **Core Domain**: Bot, Identity, Brain, Canonical Memory, Goal, Task, Execution, Core Lease, Command, Domain Event 등 DXBOT의 제품 의미
- **Optional Capability**: 교체 가능한 기능의 안정 계약
- **Provider**: Capability의 구체 구현
- **Interface**: CLI/TUI/Web/API 등 Runtime 접근 표면
- **Plugin**: 안정된 외부 확장 계약을 통해 배포되는 독립 확장 패키지

분류되지 않은 기능은 설계 승인하지 않는다. Plugin과 Provider/Capability를 동일 개념으로 사용하지 않는다.

### 2.2 추가 가능성뿐 아니라 제거 가능성을 계약화
Optional Capability, Provider, Interface, Plugin은 `Deprecated → 신규 사용 차단 → Drain → Detach → Compatibility/Migration → Data/Cache 정리 → Dependency 제거 → Code 제거`의 Removal Contract를 가져야 한다.

### 2.3 Harness Provider 독립성 강화
DeepSeek는 DXBOT의 필수 구성요소가 아니라 초기 Harness Provider 후보 중 하나다. Bot/Task별 Provider 선택과 복수 Provider 등록을 허용하되 한 Execution 내부에서는 Provider를 고정한다.

### 2.4 Plugin을 독립 시스템으로 승격
`10-architecture/16-plugin-system.md`를 신설한다. Plugin은 내부 Rust trait ABI가 아니라 안정된 Manifest/Wire/SDK 계약, Permission, Lifecycle, Isolation, Version, Migration, Uninstall Data Semantics를 가진다.

### 2.5 Routine을 1급 도메인 계약으로 보강
Routine은 `Bot-owned persistent trigger/schedule definition`이며 별도 Agent가 아니다. Trigger가 일반 Task를 생성하며 restart/missed trigger/overlap/disable/archive semantics를 갖는다.

### 2.6 Waiting Task의 영속 Continuation
Task가 `Waiting`을 Commit하면 재개에 필요한 continuation/checkpoint, pending child/delegation, wait condition, deadline, correlation, revision을 같은 논리적 Commit에서 보존해야 한다.

### 2.7 Side Effect Write-Ahead Ledger
비멱등 외부 변경은 실행 전에 Intent/Key를 영속하고, 결과를 별도 Outcome으로 기록한다. 외부 동작 후 결과 기록 전에 crash한 경우 임의 retry하지 않고 `Unknown/Reconciliation Required`로 처리한다.

### 2.8 Long-term Memory 성장 정책
Runtime memory pressure나 cache eviction을 이유로 Canonical Memory를 암묵적으로 삭제하지 않는다. Retention, compaction, archival, explicit forget을 별도 정책으로 관리한다.

### 2.9 Repository Naming/Layout 규범
Repository path는 lowercase kebab-case를 기본으로 한다. 단, Rust source module 파일(`*.rs`)은 Rust toolchain/IDE 관례를 따르기 위해 `snake_case`를 허용한다. 이 예외를 제외한 `#[path]` 기반 우회는 기본 금지한다.

### 2.10 Cross-Layer Scenario Review와 2회 재검수
문서 검수는 파일 단위 일관성 검사로 끝내지 않는다. Bot persistence, Multi-Bot, Waiting recovery, Side Effect reconciliation, Routine restart, Provider replacement/removal/multi-selection, Plugin lifecycle을 Command→Domain→Persistence→Scheduler→Provider→Recovery→Projection→API 흐름으로 추적한다.

모든 문서 세트 변경은 완료 후 별도의 두 검수를 거친다.

1. **Structural / Consistency Review**: ID, 경로, depends_on, Canonical Owner, 용어, 링크, Acceptance, Risk, Naming
2. **Cross-Layer Executability Review**: 정상·실패·취소·Crash·Migration·Disable·Removal 시 상태 소유권과 재개 가능성

## 3. 권장 읽기 순서

| 순서 | 문서군 | 목적 |
|---:|---|---|
| 1 | `00-governance` | 최상위 의미, 분류, 변경/검수 규칙 |
| 2 | `10-architecture` | 계층·Capability/Provider/Plugin·저장 경계 |
| 3 | `20-domains` | Bot·Brain·Memory·Task·Core·Network·Control 상태 의미 |
| 4 | `30-runtime` | 동시성·자원·보안·복구·관측·구성 |
| 5 | `40-interfaces` | 동일 Runtime을 사용하는 Control/API/UI |
| 6 | `50-engineering` | Rust 구현·성능·테스트·Migration·Repository 규칙 |
| 7 | `60-delivery` | 구현 순서, Acceptance, Risk, Open Question |

## 4. 공통 설계 규칙

- Bot이 Identity·Memory·Persistent State를 소유한다. Session과 UI는 소유자가 아니다.
- 하나의 Bot에는 하나의 Brain이 있고 Core는 Brain이 취득하는 일시적 실행 Lease다.
- Canonical State와 Derived State를 명시하고 동일 사실의 복수 원본을 금지한다.
- Domain은 Provider/Plugin/UI 타입을 알지 않는다.
- Consumer는 concrete Provider가 아니라 안정 Capability Contract에 의존한다.
- 복수 Provider가 존재하면 명시적 Selector/Policy가 결정하며 등록 순서나 암묵 fallback을 금지한다.
- Provider/Plugin 제거가 Core Domain schema 변경을 요구하지 않아야 한다.
- 공유 쓰기는 명령 경로에서 직렬화하고 읽기는 immutable/versioned snapshot을 우선한다.
- 모든 큐·버퍼·캐시는 item과 byte 상한, 퇴출 정책, 관측 지표를 가진다.
- 공통화는 의미가 같은 계약에만 적용하며 `common`, `utils`, `helpers` 같은 무소유 저장소를 금지한다.
- 불필요한 복사·할당·상태·정책·재시도·캐시 중복을 금지한다.
- 모델·도구·네트워크 시간을 제외한 Runtime overhead는 측정 가능한 예산으로 관리한다.

## 5. 완료 정의

v0.2 문서 집합은 다음 조건을 만족해야 구현 기준으로 승인할 수 있다.

1. 모든 신규 기능이 5개 분류 중 하나와 Canonical/Lifecycle Owner를 가진다.
2. Provider replacement/removal/multi-selection Acceptance가 자동화 가능하다.
3. Routine, Waiting Continuation, Side Effect Ledger가 Storage/Recovery/API/Test까지 닫힌 경로를 가진다.
4. DeepSeek를 제거해도 Bot/Brain/Memory/Task/Core의 의미가 바뀌지 않는다.
5. Plugin은 Provider와 분리된 Manifest/Permission/Lifecycle/Version/Uninstall 계약을 가진다.
6. Repository Naming/Layout 규칙이 CI로 검사 가능하다.
7. Long-term Memory의 성장과 삭제가 cache pressure와 분리된다.
8. Structural Review와 Cross-Layer Executability Review를 모두 통과한다.
