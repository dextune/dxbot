---
title: "근거 소스 맵과 차용 추적표"
document_id: "DXB-ADP-090"
version: "0.1.0"
status: "Reference Snapshot"
normative: false
priority: "P0"
last_updated: "2026-08-25"
depends_on: ["DXB-ADP-000"]
target_owners: ["docs/plan"]
package_path: "docs/plan/20260825-0930-grok-architecture-adoption-plan"
source_baseline:
  dxbot_commit: "e43739614631c95752482c2ad2ec53cb7ebd251f"
  grok_reconstructed_commit: "a9f633e09d49a85829b8236331b9e21f7e612634"
---
# 근거 소스 맵과 차용 추적표

## 1. Baseline lock

| 저장소 | Commit | 기준일 | 용도 |
|---|---|---|---|
| `dextune/dxbot` | `e43739614631c95752482c2ad2ec53cb7ebd251f` | 2026-08-24 commit | 분석 시 code/docs baseline |
| `b-nnett/grok-bot-0.18-reconstructed` | `a9f633e09d49a85829b8236331b9e21f7e612634` | 2026-08-23 commit | 외부 참고 구현 |

Grok 저장소는 공식 Anysphere 원본 monorepo가 아니라 공개 앱의 비공식 재구성본이다. 모듈명과 경계는 원본과 다를 수 있다.

## 2. DXBOT 기준 소스

| 경로 | 확인 목적 |
|---|---|
| `AGENTS.md` | correctness, owner, resource, failure, 재검수 규약 |
| `Cargo.toml` | workspace crate와 current responsibility split |
| `docs/agent/guide-index.md` | 작업별 guide routing |
| `docs/agent/documentation-rules.md` | canonical owner, front matter, package 관계 규칙 |
| `docs/agent/architecture/common-extension.md` | Common/Extension lifecycle와 책임 |
| `docs/agent/runtime/lifecycle-shutdown.md` | created→ready→draining→stopped와 reverse cleanup |
| `docs/agent/quality/documentation-consistency.md` | 문서 간 owner/contract 검증 |
| `docs/agent/quality/review-completion.md` | 두 차례 재검수 |
| `docs/plan/20260823-1310-v0-8-10-detailed-development-plan/readme.md` | 활성 v0.8.10 scope/gate |
| `.../10-architecture/10-system-architecture.md` | Shell→Control→Application→Domain→Runtime→Provider DAG |
| `.../10-architecture/12-shared-contracts-extension-model.md` | Application/Provider/Plugin owner 방화벽 |
| `.../20-domains/21-brain-context.md` | Brain과 execution-local state 분리 |
| `.../20-domains/22-memory-system.md` | scope/revision/provenance 기반 Memory |
| `.../20-domains/24-dynamic-core-scheduler.md` | Core Lease와 admission/fencing |
| `.../20-domains/25-multi-bot-network.md` | durable Delegation 의미 |
| `.../20-domains/27-conversation-thread-model.md` | Conversation/Thread durable lineage |
| `.../20-domains/29-channel-collaboration-model.md` | Channel/membership/fan-out owner |
| `crates/runtime-host/src/host.rs` | 현재 host lifecycle 구현 범위 |
| `crates/provider-host/src/*` | 현재 provider protocol/harness/adapter 범위 |
| `crates/dxbot-core/src/types.rs` | IDs, selector, command, operation, journal types |
| `crates/cli/src/*` | durable journal/recovery/safe writer/current CLI surface |

## 3. Grok 참고 소스

아래 링크는 baseline commit에 고정한다.

| 경로 | 관찰 패턴 | 차용 여부 |
|---|---|---|
| [`README.md`](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/a9f633e09d49a85829b8236331b9e21f7e612634/README.md) | hybrid Electron/host/coordinator/router, local Docker, packaging | 배경만 참고 |
| [`docs/ARCHITECTURE.md`](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/a9f633e09d49a85829b8236331b9e21f7e612634/docs/ARCHITECTURE.md) | editable source roots와 pinned upstream input | provenance 참고 |
| [`source/internal/host-extensions.ts`](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/a9f633e09d49a85829b8236331b9e21f7e612634/source/internal/host-extensions.ts) | dependency graph, cycle check, rollback, reverse teardown | Adopt semantics |
| [`source/host/extensions/registry.ts`](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/a9f633e09d49a85829b8236331b9e21f7e612634/source/host/extensions/registry.ts) | explicit build registry와 ID 검증 | 제한적 참고 |
| [`source/electron-main/box/local-docker-host-connector.ts`](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/a9f633e09d49a85829b8236331b9e21f7e612634/source/electron-main/box/local-docker-host-connector.ts) | ownership, loopback, digest, read-only mount, readiness, replace | Adopt invariant, Docker 비종속화 |
| [`source/host/extensions/state-backstop/state-backstop-service.ts`](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/a9f633e09d49a85829b8236331b9e21f7e612634/source/host/extensions/state-backstop/state-backstop-service.ts) | bounded store snapshot, byte cap, debounce, explicit status/dispose | recovery snapshot invariant만 차용 |
| [`source/host/extensions/session/session-diagnostics.ts`](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/a9f633e09d49a85829b8236331b9e21f7e612634/source/host/extensions/session/session-diagnostics.ts) | dedicated session diagnostic reporter | typed/bounded envelope로 강화해 차용 |
| [`source/host/extensions/memory/memory-service.ts`](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/a9f633e09d49a85829b8236331b9e21f7e612634/source/host/extensions/memory/memory-service.ts) | inspectable files, atomic write, tombstone, synthesis fingerprint | projection만 차용 |
| [`source/host/agents/agent-messaging.ts`](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/a9f633e09d49a85829b8236331b9e21f7e612634/source/host/agents/agent-messaging.ts) | async send, later fresh wake, no polling, fan-out caution | Message semantics 참고 |
| [`source/node-agent-coordinator/inference-router.ts`](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/a9f633e09d49a85829b8236331b9e21f7e612634/source/node-agent-coordinator/inference-router.ts) | provider routing, per-agent ordering, atomic transcript | 일부 참고, scheduler로는 Reject |
| [`source/host/extensions/inference/provider-session.ts`](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/a9f633e09d49a85829b8236331b9e21f7e612634/source/host/extensions/inference/provider-session.ts) | HTTP/CLI provider 이질성, streaming/tool/usage | Conformance 입력 |
| [`source/host/agents/agent-clone.ts`](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/a9f633e09d49a85829b8236331b9e21f7e612634/source/host/agents/agent-clone.ts) | DB/profile/settings copy clone | Reject |
| [`scripts/bootstrap-runtime.mjs`](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/a9f633e09d49a85829b8236331b9e21f7e612634/scripts/bootstrap-runtime.mjs) | pinned artifact bootstrap | provenance 참고 |
| [`scripts/verify.mjs`](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/a9f633e09d49a85829b8236331b9e21f7e612634/scripts/verify.mjs) | packaged artifact verification | publication check 참고 |
| [`scripts/verify-publication-tree.mjs`](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/a9f633e09d49a85829b8236331b9e21f7e612634/scripts/verify-publication-tree.mjs) | clean publication tree 검사 | 제한적 참고 |
| [`PROVENANCE.md`](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/a9f633e09d49a85829b8236331b9e21f7e612634/PROVENANCE.md) | reconstructed/upstream 경계 기록 | Adopt documentation practice |

## 4. 제안 추적표

| 제안 문서 | 주 근거 | DXBOT owner | 상태 |
|---|---|---|---|
| `DXB-ADP-020` | host-extensions, registry | runtime-host | Close |
| `DXB-ADP-021` | local-docker-host-connector | runtime-security/runtime-host | Observe→Close |
| `DXB-ADP-022` | state-backstop, diagnostics | runtime-host/runtime-audit | Adopt |
| `DXB-ADP-023` | memory-service/session files | application/CLI | Adopt as derived only |
| `DXB-ADP-024` | agent-messaging | application | Close |
| `DXB-ADP-025` | inference-router/provider-session | provider-host | Close |
| `DXB-ADP-026` | bootstrap/verify/provenance | runtime-bootstrap/scripts | Conditional Adopt |

## 5. 해석 제한

- source path 존재는 해당 패턴의 품질 또는 원본 설계 의도를 보증하지 않는다.
- Grok의 try/catch fallback, fixed timer, direct credential handling은 그대로 차용하지 않는다.
- DXBOT baseline 이후 변경이 생기면 gap assessment를 다시 수행해야 한다.
- 각 제안의 최종 truth는 DXBOT Canonical Owner 문서와 code/test evidence가 소유한다.
