---
title: "DXBOT v0.8.6 문서 매니페스트와 검수 기록"
document_id: "DXB-MANIFEST"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-BASE-000", "DXB-DEL-061", "DXB-DEL-062", "DXB-DEL-063", "DXB-ENG-052", "DXB-ENG-054"]
review_revision: 6
adversarial_review_rounds: 5
final_rechecks: 2
---
# DXBOT v0.8.6 문서 매니페스트와 검수 기록

<!-- manifest-machine:start -->
active_package_path: docs/plan/20260822-2027-v0-8-6-detailed-development-plan
plan_version: 0.8.6
review_revision: 6
adversarial_review_rounds: 5
final_rechecks: 2
markdown_count: 49
parent_plan_commit: d07f6041d54dd3a2602446336080257a1520b9ae
source_baseline_commit: 5b55b67fbaaf0f3192c126865e8b665ff26fc5aa
<!-- manifest-machine:end -->

## Resolution ledger

| Finding family | Document status | Executable status |
|---|---|---|
| self-contained exit/error/output contract | Resolved | metadata generator/golden Blocked |
| RequestDigest server-only input contradiction | Resolved | digest fixture Blocked |
| Receipt/Directive/Target/wait conflation | Resolved | state-model fixture Blocked |
| Approval/Authority/ActionGrant/SideEffect owner gap | Resolved | persistence/concurrency fixture Blocked |
| default policy semantic gap | Resolved | bootstrap fixture Blocked |
| natural-language-only CLI grammar | Resolved as typed snapshot | Rust SSOT generation Blocked |
| idempotency expiry ambiguity | Resolved | storage horizon fixture Blocked |
| review revision/evidence mismatch | Resolved | validator checks evidence counts |
| validator overclaim | Resolved | semantic review remains human/executable evidence |

`Resolved`는 문서 의미가 닫혔다는 뜻이다. Rust, Storage, Runtime, CLI 또는 Harness가 통과했다는 뜻이 아니다.

## Five adversarial review evidence

<!-- review-evidence:start -->
- `R1-GOV` | governance/self-contained/evidence truth | version, status, exit dependency 교정 | PASS
- `R2-OWNER` | Canonical Owner/state ownership | security, side-effect, membership, context owner 교정 | PASS
- `R3-CLI` | parse-to-output cross-layer contract | digest, wait, schema, operation mode 교정 | PASS
- `R4-FAIL` | crash/concurrency/resource/security | key horizon, orphan capacity, same-UID, audit 교정 | PASS
- `R5-FLOW` | implementation order/modularity/scope | M1A precedence, Core unit, Harness contract 교정 | PASS
<!-- review-evidence:end -->

## Final repository rechecks

<!-- final-recheck:start -->
- `RC1-STRUCTURAL` | IDs, dependency DAG, package version, owner/registry/status 관계 | PASS
- `RC2-CROSS-LAYER` | CLI→Contract→Domain→Storage→Runtime→Provider→Recovery→Output 추적 | PASS
<!-- final-recheck:end -->

## Evidence boundary

이 package에서 실행 가능한 것은 문서 validator와 negative fixture다. Rust build, M1A storage spike, generated contract source, CLI/Runtime E2E, real Harness canary, security/performance soak는 `Specified` 또는 `Blocked`이며 `Passed`로 표시하지 않는다.
