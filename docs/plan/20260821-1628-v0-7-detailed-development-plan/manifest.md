---
title: "DXBOT v0.7 문서 매니페스트와 검수 기록"
document_id: "DXB-MANIFEST"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-DEL-061", "DXB-DEL-062", "DXB-DEL-063", "DXB-ENG-052"]
---

# DXBOT v0.7 문서 매니페스트와 검수 기록

## 1. Baseline

- Source baseline: `docs/plan/20260821-1544-v0-6-detailed-development-plan`
- Target: `docs/plan/20260821-1628-v0-7-detailed-development-plan`
- Upgrade theme: **Headless Application Contract + CLI Reference Interface**
- Active product Interface: **CLI only**
- Quality Tier: **Tier A — Common Contract / Runtime Boundary**
- Compatibility: **v0.6 Backend semantic preservation + explicit Interface scope amendment**
- Change type: **development-plan/documentation only**
- Code changes in this package: **none**

## 2. Inventory

| Group | Count |
|---|---:|
| Governance | 6 |
| Architecture | 8 |
| Domains | 10 |
| Runtime | 9 |
| Interfaces | 2 |
| Engineering | 5 |
| Delivery | 5 |
| **Plan documents** | **45** |
| readme + manifest | 2 |
| **Total Markdown** | **47** |

Active Interface inventory:

```text
40-interfaces/40-api-protocols.md   DXB-IFC-040
40-interfaces/41-cli.md             DXB-IFC-041
```

`40-interfaces/42-tui.md` / `DXB-IFC-042`와 `40-interfaces/43-web-control-center.md` / `DXB-IFC-043`은 active package에 존재하지 않는다.

## 3. Change Classification

- New Canonical Documents: **0**
- v0.7 Upgraded Existing Plan Documents: **30**
- Explicitly Superseded/Removed from Active Package: **2 (`DXB-IFC-042`, `DXB-IFC-043`)**
- Baseline-Reused Active Plan Documents: **15**

### 3.1 Upgraded Documents

- Governance: `DXB-BASE-000`, `DXB-GOV-001`, `DXB-GOV-002`, `DXB-GOV-003`, `DXB-GOV-004`
- Architecture: `DXB-ARC-010`, `011`, `012`, `014`, `015`
- Domains: `DXB-DOM-026`, `027`
- Runtime: `DXB-RUN-030`, `031`, `032`, `033`, `034`, `035`, `038`
- Interfaces: `DXB-IFC-040`, `041`
- Engineering: `DXB-ENG-050`, `051`, `052`, `053`, `054`
- Delivery: `DXB-DEL-060`, `061`, `062`, `063`

### 3.2 Baseline-Reused Active Documents

- Governance: `DXB-SOURCE-000`
- Architecture: `DXB-ARC-013`, `016`, `017`
- Domains: `DXB-DOM-020`~`025`, `028`, `029`
- Runtime: `DXB-RUN-036`, `037`
- Delivery: `DXB-DEL-064`

## 4. Supersession / Residual Reference Policy

`DXB-IFC-042/043`은 omission이 아니라 `DXB-BASE-000`에서 explicit supersession 처리했다.

v0.7 문서에서 TUI/Web 문자열이 허용되는 경우는 다음뿐이다.
1. supersession/removal/non-goal을 설명하는 문맥
2. v0.7 안정화 뒤 별도 후속 버전에서의 future re-entry 조건
3. byte-for-byte 의미 보존이 필요한 `DXB-SOURCE-000` 원문 보존본의 과거 제품 방향

위 residual은 active implementation milestone/release gate가 아니다. `DXB-SOURCE-000`은 원문 보존 목적 때문에 수정하지 않았으며, v0.7 active scope는 `DXB-BASE-000`이 명시적으로 amendment한다.

## 5. Review 1 — Structural / Consistency

**Result: PASS after fixes**

검사 범위:
- file/path lowercase kebab-case와 inventory/count
- document ID/depends_on/owner 관계
- active Interface file/dependency/milestone/Acceptance/Risk/OQ
- v0.6 inheritance와 explicit supersession
- Contract/Traceability/Removal 의미
- Canonical Owner duplication
- 과거 Review policy drift

발견 및 수정:

### F1 — Runtime config의 stale Web 표현
`DXB-RUN-035`의 과거 검증 문구에 `CLI/Web` 병렬 hardcode가 남아 있었다.

수정:
- `DXB-RUN-035`를 v0.7로 승격
- Project/Channel/Resource limit SSOT를 `Application Contract/CLI` 기준으로 변경
- Runtime Host/Control config와 CLI local profile/secret boundary 추가

재검수:
- Web/TUI active configuration requirement 0
- Interface-neutral config semantics 보존

### F2 — 과거 3-review governance drift
`DXB-GOV-001/004`가 과거 3단계 Review를 신규 변경의 필수 절차처럼 유지해 현재 프로젝트의 2회 독립 재검수 규칙 및 v0.6 이후 문서와 충돌했다.

수정:
- `DXB-GOV-001`에 2회 독립 Review SSOT 고정
- 과거 Structural + Contract/Traceability 검사항목을 Review 1에 통합
- Development Flow를 Review 2 Cross-Layer Executability/Compatibility에 통합
- `DXB-GOV-004` 템플릿 동기화
- 과거 3-review evidence는 역사 기록으로만 보존

재검수:
- 검사항목 축소 0
- Review 절차 SSOT 충돌 0

최종 Structural 결과:
- active Interface file = IFC-040/041 only
- IFC-042/043 active file 0
- IFC-042/043 active dependency/release blocker 0
- TUI/Web active implementation milestone 0
- Canonical Owner duplication 0
- v0.6 Backend inheritance regression 0

## 6. Review 2 — Cross-Layer Executability / Compatibility

**Result: PASS after fixes**

추적 경로:

```text
Shell
→ dxb CLI
→ Host Lifecycle Client (bootstrap only) / Control Client
→ Control Endpoint
→ Application Contract
→ Application Command/Query
→ Domain/Runtime
→ Persistence/Scheduler/Provider Host
→ Event/Result/Projection
→ Control Protocol
→ CLI Output
```

fault/compatibility 대입:
- Runtime absent/startup failure/readiness timeout
- stale revision
- permission deny/approval required
- command commit 후 response loss
- duplicate retry
- CLI crash/SIGINT/disconnect
- Runtime restart/endpoint recreation
- Provider loss
- event cursor gap/expiry
- slow stdout/broken pipe
- large history/export
- Runtime memory pressure
- protocol/schema mismatch

발견 및 수정:

### F3 — `dxb runtime start` bootstrap owner gap
초기 설계는 모든 CLI 동작을 Control Endpoint로 표현했지만 Runtime이 미기동이면 Endpoint 자체가 존재하지 않아 `runtime start` 경로가 완결되지 않았다.

수정:
- `DXB-ARC-010/011`, `DXB-GOV-002/003`, `DXB-IFC-040/041`, `DXB-ENG-050/052/054`, `DXB-DEL-060`에 narrow Host Lifecycle Adapter/Client 추가
- 허용 범위를 process/service start/status/stop/readiness로 제한
- Domain/Store/Task/Memory/Scheduler/Provider state 접근·mutation 금지
- Runtime Ready 이후 모든 Domain use-case는 Application Contract로 강제
- ADR-0083, bootstrap fixture, architecture CI gate 추가

재검수:
- Runtime absent→start→readiness→Control connect 경로 완결
- Host Lifecycle Client Domain shortcut 0
- CLI exit/SIGINT와 Runtime/Task/Process lifecycle 분리
- response-loss duplicate semantic effect 경로 0
- cursor gap silent loss 경로 0
- large output/slow consumer unbounded queue 경로 0
- protocol mismatch silent fallback 경로 0
- client-side authorization/resource policy duplication 경로 0

## 7. v0.6 Compatibility Result

v0.7은 Interface boundary를 강화하지만 다음 owner/identity를 변경하지 않는다.
- Bot/Conversation/Thread/Project/Channel/Task/Execution/Memory identity/revision
- Persistent Brain semantics / Dynamic Core Scheduler ownership
- Durable Process replay semantics
- Common Authorization / Information Flow / ActionGrant
- Side Effect Ledger / Live Control
- Provider Host / Capability Contract / Provider Session independence
- Runtime Memory envelope/reservation/pressure/recovery

Backend Canonical data migration은 **0이 기본**이며 protocol/schema compatibility와 별도로 관리한다.

## 8. Review Evidence Boundary

본 변경은 개발기획 문서 고도화이므로 실제 Rust binary/build/test/benchmark는 실행하지 않았다. 본 Review의 PASS는 **문서 구조·계약·의존성·fault-path 관계성 검수**에 대한 PASS다.

실제 구현 단계의 compile/conformance/E2E/performance/security gate는 `DXB-ENG-052/054`와 `DXB-DEL-060/061`이 요구한다.

## 9. Final Result

- Review 1 Structural / Consistency: **PASS**
- Review 2 Cross-Layer Executability / Compatibility: **PASS**
- Active Interface scope: **Headless Application Contract + CLI only**
- TUI/Web active implementation: **removed/superseded**
- Future TUI/Web: **v0.7 전체 안정화 이후 별도 버전에서 재계획**
- Backend v0.6 semantic regression found after final re-review: **0**
