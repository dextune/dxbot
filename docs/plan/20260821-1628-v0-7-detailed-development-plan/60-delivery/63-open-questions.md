---
title: "미결정 사항과 권장 기본값"
document_id: "DXB-DEL-063"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-GOV-003", "DXB-ARC-017", "DXB-DOM-022", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-037", "DXB-RUN-038", "DXB-IFC-040", "DXB-IFC-041", "DXB-DEL-060"]
---

# 미결정 사항과 권장 기본값

## 1. 목적

v0.7의 Canonical semantic은 고정하되 local IPC, daemon lifecycle, protocol compatibility 범위, CLI naming/exit number/credential backend처럼 증거가 필요한 세부를 ADR/benchmark 전에 임의 확정하지 않는다.

## 2. 기존 Open Question 유지

v0.6의 OQ-001~067, OQ-101~124 및 P2 질문은 명시적으로 닫히지 않은 한 유효하다. 단, `DXB-IFC-042/043` 구현만을 위한 Interface-specific OQ는 v0.7 active Gate에서 제외한다.

## 3. v0.7 Open Questions

| ID | 질문 | 권장 기본 방향 | Gate |
|---|---|---|---|
| **OQ-068** | Runtime Host와 CLI local transport | semantic 먼저 고정, platform IPC/loopback은 benchmark/security 후 ADR | M2 |
| **OQ-069** | Runtime daemon/service lifecycle | CLI exit independence 필수, exact service manager는 platform ADR | M2 |
| **OQ-070** | protocol/schema versioning 단위 | protocol/schema/runtime/client/data schema 분리 | M1 |
| **OQ-071** | CLI command naming/alias 안정성 | P0 command tree 최소화 후 freeze | M3 |
| **OQ-072** | JSON/JSONL schema compatibility | automation 안정성 우선, human output과 분리 | M3/M4 |
| **OQ-073** | exit code registry | stable error class와 1:1 의미 연결, 숫자는 한 ADR/registry에서 freeze | M3 |
| **OQ-074** | event cursor retention/resync window | bounded retention + explicit full resync | M1/M4 |
| **OQ-075** | interactive confirmation/non-interactive UX | TTY-aware + explicit `--yes`/non-interactive policy | M3/M4 |
| **OQ-076** | local credential/profile 저장 방식 | argv/plain secret 기본 금지, OS/platform evidence 기반 ADR | M2/M5 |
| **OQ-077** | Runtime/CLI packaging 방식 | single installer보다 process independence/upgrade compatibility 우선 | M5 |
| **OQ-078** | shell completion/man page 범위 | P1, core correctness 후 결정 | M5 |
| **OQ-079** | embedded/in-process CLI test adapter | production shortcut 금지, deterministic test 필요성에 한해 제한 | M1/M2 |

## 4. v0.7에서 이미 고정된 항목

Open Question이 아니다.
- active product Interface = Application Contract + CLI
- `DXB-IFC-042/043` active baseline superseded
- Runtime Host lifetime ≠ CLI process lifetime
- CLI SIGINT/disconnect ≠ target Task/Process cancel
- CLI direct Domain/Store/Scheduler/Provider access 금지
- Command/Query/Subscription 분리
- mutation idempotency/lost-response recovery 필요
- subscription cursor/gap/resync 필요
- incompatible protocol pair silent fallback 금지
- machine output과 human rendering 분리
- large output/watch bounded paging/streaming
- Common Authorization/Resource policy client-side duplicate 금지
- TUI/Web/BFF/frontend speculative 구현 금지

## 5. 결정 증거

각 OQ는 invariants, security/privacy, resource/RSS/allocation/cache, concurrency, fault/recovery, compatibility/migration, Provider independence, deterministic fixture, benchmark/prototype, owner, ADR link를 사용한다.

## 6. 검증 기준

- OQ-068~079가 해당 Gate 전에 ADR/실험/benchmark로 닫힌다.
- 질문이 남아 있다는 이유로 direct Backend shortcut, unbounded output, plaintext secret, silent version fallback을 구현하지 않는다.
- superseded Interface를 위한 OQ를 v0.7 blocker로 되살리지 않는다.
