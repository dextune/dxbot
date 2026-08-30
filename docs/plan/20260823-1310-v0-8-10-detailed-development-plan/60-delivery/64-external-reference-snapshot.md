---
title: "외부 참조 스냅샷"
document_id: "DXB-DEL-064"
version: "0.8.10"
status: "Reference Snapshot"
normative: false
priority: "P1"
last_updated: "2026-08-30"
depends_on: ["DXB-ARC-013"]
---

# 외부 참조 스냅샷

## 1. 목적

Harness와 외부 구현 자료는 DXBOT 제품 의미를 결정하지 않고 Adapter/Provider 설계의 참고·위험 탐색·compatibility evidence로만 사용한다. 본 문서는 normative하지 않다. 공식 DeepSeek Harness dependency의 immutable coordinate는 이 문서 한 곳에서만 소유한다.

## 2. Official DeepSeek Harness Snapshot

- 확인 기준일: 2026-08-30
- repository: `https://github.com/deepseek-ai/deepseek-harness`
- branch snapshot: `master`
- immutable commit: `cd5ef8148158c3a752a658978873241fdf8e2bbc`
- matching tag: `dsh-v0.1.2-alpha.1`
- root package version: `0.1.2-alpha.1`
- license: MIT
- Node requirement: `^22.19.0 || >=24.0.0`
- package manager: `pnpm@11.7.0`
- upstream status: developer preview; compatibility-breaking changes 예정, security audit 미완료, production-ready로 간주 금지

공식 integration surface:

- `pnpm dsh --profile acp`: automation-only ACP v1 newline-delimited JSON-RPC stdio server
- `@deepseek-ai/dsh-acp`: initialize, new/list/resume/close, prompt/cancel, semantic update와 permission request
- `@deepseek-ai/dsh-llm-pi-ai`: installed catalog 또는 hand-declared provider route; `anthropic-messages` protocol과 `apiKeyEnv` credential reference 지원
- `@deepseek-ai/dsh-subagent-acp`: fresh process per run, credential-scrubbed environment, EOF→signal teardown pattern
- stdout은 protocol frame 전용이며 ACP client는 trusted controller를 전제한다. ACP `authenticate`는 즉시 성공하므로 보안 인증 경계가 아니다.

## 3. 채택과 비채택

- 채택: exact-pinned ACP subprocess boundary, capability/version handshake, configuration composition, append-only execution trajectory, semantic tool lifecycle, cancellation/drain/teardown 방어 원칙
- 채택: official `dsh-llm-pi-ai` hand-declared `anthropic-messages` route를 통한 `MiniMax-M3` 구성
- 비채택: Cordis/Node in-process embedding, upstream Session/Agent/Subagent/Plugin/Storage type의 DXBOT Bot/Core/Memory/Domain public API 사용
- 비채택: DSH sandbox/approval을 DXBOT authority 또는 유일한 security control로 신뢰
- 비채택: floating branch, unpinned `npx latest`, runtime package install, ambient user DSH home/credential discovery

## 4. DXBOT 재해석

| 외부 개념 | DXBOT 적용 |
|---|---|
| Session/event trajectory | Provider/Harness execution evidence; Bot Domain Journal과 분리 |
| Agent/Subagent | Bot/Core/Task identity로 자동 매핑하지 않음 |
| Plugin/profile/patch tree | DXBOT-owned audited subprocess deployment composition |
| ACP stdio | public Contract를 대체하지 않는 compatibility wire |
| LLM provider/model | DXBOT Provider binding 아래 고정된 `minimax` / `MiniMax-M3` external route |
| permission request | DXBOT Security decision을 요청하는 one-shot signal; 자체 권한 아님 |
| storage/session resume | external evidence/recovery input; canonical Process state 아님 |
| sandbox/tool execution | DXBOT permission/resource/side-effect/OS isolation 아래 defense in depth |

## 5. Supply-chain와 Upgrade 규칙

- production dependency는 위 tag/commit에 대응하는 exact package version, package integrity, executable digest, transitive lockfile, SBOM과 MIT notice를 기록한다.
- release build는 network 없이 pinned artifact를 재현해야 하며 startup-time install을 금지한다.
- upstream type/event를 Domain/Application public schema로 re-export하지 않는다.
- upgrade는 snapshot 갱신, ACP compatibility, shared Conformance, fault/security/resource, license/SBOM delta evidence를 요구한다.
- developer-preview 상태에서는 자동 update와 silent latest 추종을 금지한다.
- 외부 자료와 DXBOT Canonical Owner가 충돌하면 DXBOT owner를 따른다.

## 6. 재검토 Trigger

- stable release/API major/license/security advisory
- ACP version/capability/session settlement 변화
- `dsh-llm-pi-ai` protocol/model/credential route 변화
- sandbox/permission/tool pipeline 변화
- Adapter Conformance 또는 MiniMax-M3 live canary 실패
- process isolation으로 완화할 수 없는 security finding

## 7. 검증 기준

- release artifact가 external dependency version/digest를 출력한다.
- external upgrade PR에 snapshot/compatibility/conformance/fault/SBOM/license evidence가 있다.
- upstream type이 Domain/Application public schema를 오염시키지 않는다.
- raw credential이 profile/argv/log/evidence에 없다.
- 외부 자료가 제품 원칙을 암묵 변경하지 않는다.

실제 채택·모듈화 단계와 gate는 [DXB-DEL-068](68-modular-harness-adoption-plan.md)을 따른다.
