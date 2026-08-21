---
title: "v0.8 결정 기록과 후속 Open Question"
document_id: "DXB-DEL-063"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-GOV-003", "DXB-DEL-060", "DXB-IFC-040", "DXB-IFC-041"]
---

# v0.8 결정 기록과 후속 Open Question

## 1. 목적

고도화 플랜의 OQ-080~089를 P0 구현 전에 모두 닫고, 결정 근거·owner·재검토 trigger를 기록한다. `Open Question`이라는 제목을 유지하지만 P0 표의 상태는 전부 `Resolved`다.

## 2. P0 Resolved Decisions

| ID | 상태 | 결정 | Canonical Owner / 근거 | 재검토 Trigger |
|---|---|---|---|---|
| OQ-080 | Resolved | P0 reference는 Linux user-scoped Runtime, data root/InstanceId당 active instance 1개 | RUN-035; local-first와 single writer | Windows/macOS/system-wide/remote 지원 승인 |
| OQ-081 | Resolved | XDG config/data/state/runtime, UDS `control.sock`, descriptor+lock, user service manager, HostGeneration fencing | RUN-035/RUN-032 | platform adapter 또는 storage writer 모델 변경 |
| OQ-082 | Resolved | durable Operation Receipt; key는 principal/action/target/request digest/schema에 결박; nonterminal TTL-GC 금지, terminal 최소 30일; CLI journal은 bounded refs만 | IFC-040/RUN-033 | audit/volume benchmark가 retention 변경 요구 |
| OQ-083 | Resolved | Canonical ID 최종; exact name/alias는 explicit scope에서 unique할 때만; fuzzy/last-used mutation 금지; destructive action ID+revision | IFC-040/041 | 별도 interactive discovery UI 승인 |
| OQ-084 | Resolved | P0 list/history/search/export는 snapshot-consistent, stable sort+ID tie-breaker; cursor가 query/filter/scope/principal/snapshot/schema에 bind | IFC-040 | storage가 snapshot 불가함을 evidence로 증명 |
| OQ-085 | Resolved | machine stream은 JSONL header/item|event/terminal; terminal에 Complete/Partial/CancelledLocally/Gap/Failed와 last safe cursor | IFC-040/041 | 다른 incremental media type 도입 |
| OQ-086 | Resolved | wait=`accepted|committed|terminal`; timeout은 local wait; exit 0은 requested condition 충족 시만, partial=13, timeout=12 | IFC-041 | compatibility major 변경 |
| OQ-087 | Resolved | Rust Application Contract logical module/crate가 schema SSOT; generated JSON schema/golden/operation/help/exit; 동일 major current+2 schema minor, CLI current/previous runtime minor fixture | IFC-040/ENG-053 | second-language/remote transport가 실제 필요 |
| OQ-088 | Resolved | terminal control 기본 sanitize, raw P0 미지원; export no-overwrite/no-follow/exclusive temp/mode 0600/atomic rename/cleanup | RUN-032/IFC-041 | explicit raw/admin export feature 승인 |
| OQ-089 | Resolved | P0 command 범위는 IFC-041 matrix; Goal/Routine/Core detail/Provider lifecycle/Plugin/Capability test/compact/index/trace/repair는 P1/P2 | IFC-041/DEL-060 | Bot-only/Project-Channel slices가 추가 owner operation 필요 증명 |

## 3. Decision Constraints

위 결정이 의미하는 금지사항:
- platform 미결정을 이유로 generic multi-transport framework 생성 금지
- receipt retention 이후 blind retry 금지
- fuzzy search 결과를 mutation target으로 자동 사용 금지
- live page를 snapshot처럼 표시 금지
- terminal record 없는 JSONL complete success 금지
- local timeout을 Runtime rollback으로 처리 금지
- Domain/Persistence type을 public schema source로 사용 금지
- `--raw`, `--force-overwrite`를 P0 shortcut으로 추가 금지
- P1/P2 command를 P0 Release blocker로 되살리지 않음

## 4. 후속 P1/P2 Open Questions

P0를 차단하지 않는다.
- Windows named-pipe/macOS launchd adapter와 equivalent peer credential
- system-wide/multi-user/remote Runtime IAM
- receipt retention을 30일 이상 확장하는 archive tier
- snapshot storage implementation/compaction 최적화
- second-language SDK/IDL 필요성
- Plugin/provider lifecycle CLI 전체
- raw forensic export의 별도 privileged workflow
- distributed/HA Runtime과 cross-instance Bot placement

각 항목은 P0 semantic을 완화하지 않는 별도 Tier A ADR/plan을 요구한다.

## 5. 검증 기준

- OQ-080~089 상태가 모두 Resolved.
- 각 결정이 Canonical Owner와 Acceptance/Risk에 연결.
- unresolved P1/P2 질문이 P0에 unsafe default/generic fallback을 만들지 않음.
