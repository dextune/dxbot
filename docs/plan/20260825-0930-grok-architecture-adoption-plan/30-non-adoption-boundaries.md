---
title: "명시적 비차용 경계"
document_id: "DXB-ADP-030"
version: "0.1.0"
status: "Reference Snapshot"
normative: false
priority: "P0"
last_updated: "2026-08-25"
depends_on: ["DXB-ADP-010"]
target_owners: ["docs/plan", "all"]
package_path: "docs/plan/20260825-0930-grok-architecture-adoption-plan"
source_baseline:
  dxbot_commit: "e43739614631c95752482c2ad2ec53cb7ebd251f"
  grok_reconstructed_commit: "a9f633e09d49a85829b8236331b9e21f7e612634"
---
# 명시적 비차용 경계

## 1. 목적

외부 프로젝트를 참고하는 과정에서 DXBOT의 핵심 방향이 제품 특수 구현으로 오염되는 것을 방지한다. 아래 항목은 추후 별도 Tier A 의사결정 없이는 도입하지 않는다.

## 2. Canonical state 관련

### 2.1 Agent directory를 Bot aggregate root로 사용 금지

Grok의 agent folder는 profile, DB, memory, automation을 한 사용자 단위로 묶는다. DXBOT은 Bot, Conversation, Task, Memory, Project, Channel, Provider, Execution의 owner를 분리하므로 이 구조를 canonical root로 채택하지 않는다.

### 2.2 DB/file copy 기반 Bot clone 금지

- `store.db` 복사 후 identity rewrite 방식 금지
- Conversation/Memory/Automation을 암묵 복제 금지
- clone이 필요하면 명시적 copy policy, provenance, 새 canonical IDs, authorization, receipt를 요구

### 2.3 사람이 편집하는 profile/memory 파일을 원본으로 사용 금지

파일은 projection 또는 explicit import source일 뿐이다. 편집 결과는 proposal/command 경로로 검증한다.

## 3. Runtime·동시성 관련

### 3.1 Agent별 Promise chain을 Scheduler로 사용 금지

동일 key ordering constraint는 참고할 수 있으나, admission, fairness, resource reservation, deadline, recovery를 Promise queue에 맡기지 않는다. Dynamic Core Scheduler와 Execution identity를 유지한다.

### 3.2 고정 sleep과 주기 pulse로 authoritative state 유지 금지

- UI 표시를 위해 일정 시간 sleep
- remote refresh를 덮기 위한 interval pulse
- readiness를 임의 지연으로 보장

이런 방식은 projection generation, event ordering, lease로 대체한다.

### 3.3 static extension 순서를 canonical dependency로 사용 금지

명시적 registry는 build composition에 유용하지만, dependency 의미는 descriptor graph가 소유한다. 배열 순서가 hidden dependency를 만들지 않게 한다.

## 4. Provider 관련

### 4.1 Application/Coordinator provider 분기 금지

`if provider == codex/claude/...`가 Application, CLI, Domain에 들어가지 않는다. 분기는 Provider Adapter 내부에 한정한다.

### 4.2 사용자 credential 파일 직접 rewrite 금지

기존 CLI 로그인 파일의 refresh/update를 adapter가 임의 수행하지 않는다. credential broker와 explicit owner 없이 원본을 수정하지 않는다.

### 4.3 전체 credential directory mount 금지

편의를 위해 `.codex`, `.claude` 등 전체 디렉터리를 sandbox에 read-only mount하는 것도 기본 금지한다. 필요한 최소 credential을 execution-scoped grant로 materialize한다.

## 5. Error·diagnostics 관련

### 5.1 오류 삼키기 금지

- catch 후 empty/default 반환
- malformed record silent skip을 성공 처리
- provider 오류를 assistant transcript text로 변환하여 정상 turn처럼 저장
- diagnostic 실패를 canonical success/failure와 혼합

부분 허용 시에도 `partial`, `degraded`, `unknown`을 명시하고 safe diagnostic을 남긴다.

### 5.2 Backstop 자동 rewrite 금지

Backstop은 관찰·격리·복구 안내를 제공하며 canonical schema나 state를 추정하여 수정하지 않는다.

## 6. Product/UI 관련

- Electron Main 중심 ownership
- preload bridge를 Application Contract 대체물로 사용
- shipped renderer checksum patching
- desktop updater/signing pipeline
- Router settings UI

이 항목은 DXBOT의 CLI-first headless runtime 범위에 속하지 않는다.

## 7. Architecture 확장 관련

이번 플랜으로 다음을 만들지 않는다.

- generic workflow engine
- dynamic plugin ABI
- service locator
- generalized event-sourcing platform
- cross-machine distributed scheduler
- multi-runtime orchestration platform
- crate-per-extension 구조

## 8. 예외 절차

비차용 항목이 필요해지면 다음을 갖춘 별도 Tier A 제안이 필요하다.

1. 현재 owner로 해결할 수 없는 이유
2. 최소 vertical slice
3. public compatibility 영향
4. failure/recovery/security model
5. 제거·migration 계획
6. adversarial review와 두 차례 최종 재검수
