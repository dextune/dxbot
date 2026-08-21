---
title: "DXBOT v0.8 상세 개발기획 문서 집합"
document_id: "DXB-INDEX"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-DEL-060", "DXB-DEL-061", "DXB-DEL-062", "DXB-DEL-063"]
---

# DXBOT 상세 개발기획 문서 집합 v0.8

본 디렉터리는 **Resolved Implementation Baseline + CLI Contract Closure**를 주제로 하는 self-contained active 개발기획 package다. 과거 package는 변경 역사와 비교 근거이며 현재 구현자는 v0.8 active Canonical Owner만으로 P0 계약을 판정한다.

> **Persistent DXBOT Runtime → Headless Application Contract → CLI Reference Interface**

## 1. 목표

- Persistent Bot/Single Brain/Dynamic Core/Memory/Task/Process/Security/Resource 의미 보존
- hidden normative inheritance 제거
- Backend부터 CLI까지 통합 dependency DAG
- Resource Selector와 Operation Receipt 완결
- Runtime Instance/endpoint/lock/readiness/concurrent start 완결
- snapshot pagination과 partial/terminal subscription 완결
- JSON/JSONL/wait/timeout/exit/file/terminal automation contract 완결
- public schema SSOT와 실제 CI Gate 정의
- Bot-only → Project/Channel → Recovery/Streaming vertical slice

## 2. Active Scope

```text
Linux user-scoped Runtime Instance
→ Application Layer
→ typed Command / Query / Subscription
→ authenticated Unix-domain Control Endpoint / Client
→ dxb CLI
```

TUI, Web Control Center, BFF, frontend state, generic remote/IDL/workflow framework, distributed HA, Plugin ecosystem 전체 CLI는 active P0 scope가 아니다.

## 3. 핵심 결정

| 주제 | v0.8 결정 |
|---|---|
| Runtime | data root/InstanceId당 active HostGeneration 1개 |
| Platform | Linux user service + XDG paths + Unix-domain socket |
| Target | Canonical ID 최종, scoped exact name/alias only |
| Mutation recovery | durable Operation Receipt + RequestDigest binding |
| Receipt retention | nonterminal TTL-GC 금지, terminal/key 최소 30일 |
| Query | snapshot-consistent default, stable sort+ID tie-breaker |
| Subscription | at-least-once, explicit gap/resync, JSONL terminal record |
| Wait | accepted/committed/target terminal과 local timeout 분리 |
| Machine | stdout=result, stderr=diagnostic, stable exit 0/2~18 |
| Security | endpoint peer/path, terminal sanitize, no-overwrite atomic export |
| Schema | Rust Application Contract source → deterministic generated artifacts |
| Development | vertical slice first, no speculative crate/framework |

## 4. Canonical Owner Map

| 책임 | Owner |
|---|---|
| 제품 불변조건 / active scope | [DXB-BASE-000](00-governance/00-normative-baseline.md) |
| 문서 상태 / Effective Baseline | [DXB-GOV-001](00-governance/01-document-governance.md) |
| 용어 | [DXB-GOV-002](00-governance/02-glossary-domain-model.md) |
| ADR 결정 | [DXB-GOV-003](00-governance/03-architecture-decision-baseline.md) |
| 전체 dependency / Runtime boundary | [DXB-ARC-010](10-architecture/10-system-architecture.md) |
| Canonical event/state/receipt | [DXB-ARC-014](10-architecture/14-event-state-model.md) |
| Runtime resource | [DXB-RUN-031](30-runtime/31-resource-governance.md) |
| Authorization/local threat model | [DXB-RUN-032](30-runtime/32-security-permissions-sandbox.md) |
| Runtime Instance/deployment | [DXB-RUN-035](30-runtime/35-configuration-deployment.md) |
| public selector/receipt/page/stream/schema | [DXB-IFC-040](40-interfaces/40-api-protocols.md) |
| P0 command/output/exit/file/terminal | [DXB-IFC-041](40-interfaces/41-cli.md) |
| tests/3 reviews | [DXB-ENG-052](50-engineering/52-testing-verification.md) |
| roadmap | [DXB-DEL-060](60-delivery/60-implementation-roadmap.md) |
| Acceptance | [DXB-DEL-061](60-delivery/61-acceptance-traceability.md) |
| Risk | [DXB-DEL-062](60-delivery/62-risk-register.md) |
| resolved decisions | [DXB-DEL-063](60-delivery/63-open-questions.md) |
| package inventory/evidence | [DXB-MANIFEST](manifest.md) |

## 5. P0 CLI Surface

- `runtime start|status|stop|doctor`, `version`
- `bot create|list|show|activate|deactivate|archive|restore`
- `conversation show|send|history`
- `thread create|list|show|send|history|branch`
- `task submit|list|show|watch|cancel|suspend|resume|redirect|result`
- `memory get|search|history|propose|promote`
- `project create|list|show|archive|restore`
- `channel create|list|show|join|leave|members|send|history`
- `process show|watch`
- `operation show|reconcile`
- `approval list|show|approve|deny`
- `provider list|show`
- `reconcile operation|side-effect`

정확한 typed mapping, wait, selector, idempotency, output, exit/security/resource는 `DXB-IFC-041`의 P0 Command Matrix가 소유한다.

## 6. 구현 순서

```text
M0 Effective Baseline / Scope Freeze
→ M1 Workspace + Domain/Persistence + Contract Kernel
→ M2 Runtime Instance / Host / Local Security
→ M3 Bot-only CLI Vertical Slice
→ M4 Project / Channel Vertical Slice
→ M5 Recovery / Streaming / Automation
→ M6 Freeze / Actual CI / Release Planning
```

## 7. Inventory

| Group | Count |
|---|---:|
| Governance | 6 |
| Architecture | 8 |
| Domains | 10 |
| Runtime | 9 |
| Interfaces | 2 |
| Engineering | 5 |
| Delivery | 5 |
| Plan documents | 45 |
| readme + manifest | 2 |
| **Total Markdown** | **47** |

## 8. Document Tree

```text
00-governance/   baseline, source, governance, glossary, ADR, template
10-architecture/ system, workspace, contracts, harness, event, storage, plugin, extension
20-domains/      bot, brain, memory, task, scheduler, multi-bot, control, conversation, project, channel
30-runtime/      concurrency, resource, security, recovery, observability, instance, control, collaboration, process
40-interfaces/   public contract, CLI command matrix
50-engineering/  Rust, performance, testing, compatibility, CI/release
60-delivery/     roadmap, acceptance, risk, decisions, external snapshot
```

## 9. Acceptance 상태

v0.8 문서는 계약과 deterministic fixture를 `Specified`한다. 이 package 생성 시 실행 가능한 문서 구조 validator와 세 차례 문서 Review 결과는 `manifest.md`에 기록한다. 실제 Rust build, Runtime/CLI E2E, security/performance tests는 구현 후 `Executable/Passed` evidence를 생성해야 한다.

## 10. 완료 정의

1. active package가 self-contained다.
2. previous hidden inheritance 0.
3. P0 Command Matrix와 P1/P2 분리가 완결된다.
4. selector/receipt/Runtime instance/page/stream/machine/local security/schema가 닫힌다.
5. Backend→CLI roadmap과 vertical slices가 실행 가능하다.
6. actual CI가 inventory/schema/dependency/matrix를 검증한다.
7. TUI/Web active artifact/milestone 0.
8. Review 1/2/3이 전체 package에서 수정 후 재실행 PASS다.
