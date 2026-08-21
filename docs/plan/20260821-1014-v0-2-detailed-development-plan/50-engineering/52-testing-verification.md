---
title: "테스트·검증 전략"
document_id: "DXB-ENG-052"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-013", "DXB-ARC-016", "DXB-ENG-050"]
---

# 테스트·검증 전략

## 1. 목적

상태기계·동시성·복구·Provider 교체/제거·Plugin lifecycle을 deterministic한 증거로 검증하고 문서 관계도 두 단계로 재검수한다.

## 2. 계층

Unit, Property/Model, Concurrency Model, Component, Contract/Conformance, Integration, E2E, Fault/Recovery, Performance, Soak, Security, Architecture/Docs를 사용한다. 핵심 CI는 Fake Provider로 외부 모델 비결정성을 분리한다.

## 3. Deterministic Testkit

virtual Clock, deterministic ID/RNG, scripted Harness/Model/Tool/Sandbox, transactional temp store, event recorder, scheduler executor, fault transport, Artifact store, policy fixture, failpoint, invariant checker를 제공한다.

추가 fixture:
- fake Provider A/B/C
- fake Plugin package/host
- side-effect external status simulator
- Routine clock/DST occurrence generator
- Waiting child/delegation simulator

## 4. Provider Conformance

모든 Harness/외부 Provider는 single/streaming/tool/error/cancel/timeout/large-output/approval/checkpoint/unknown-event/shutdown scenario를 동일 contract로 실행한다.

추가:
- explicit selection
- multiple Provider registered
- drain/unload
- removed Provider config
- Provider-free build

## 5. Cross-Layer Scenario Suite

### Bot Persistence
`Create → Task → Memory → Session 종료 → Runtime 종료 → Restore`

### Multi-Bot
`Bot A → Delegate → Bot B → Result → Bot A Resume`

### Waiting Recovery
`Parent → A/B/C → A/B 완료 → Waiting Commit → kill → restore → C only → one resume`

### Side Effect
`Prepare Intent → mutate → crash → Unknown → reconcile → no duplicate`

### Routine
`Trigger → Task → restart → missed/next trigger → occurrence dedup`

### Provider Replacement
`Provider A → detach → Provider B → same Domain semantics`

### Provider Removal
`Provider crate/config removal → Domain/runtime build → DB restore → unrelated Task success → explicit unsupported for related request`

### Multi-Provider
`Bot A→Provider A, Bot B→Provider B, Task C→Provider C` and each Execution fixed.

### Plugin Lifecycle
`install→validate→enable→use→disable/drain→upgrade/rollback→uninstall/data policy`.

## 6. Fault / Recovery

process kill, DB busy/full, commit response loss, migration crash, Provider crash, Plugin crash, Continuation corruption, side-effect outcome loss, routine occurrence crash, outbox poison, projection/index rebuild를 failpoint로 검증한다.

## 7. Concurrency

cancel/complete, provider unload/start, Plugin disable/callback, deactivate/submit, memory update/delete, lease expiry/result, Waiting child duplicate, Routine duplicate trigger, shutdown/callback, permit leak를 sleep 없이 barrier/fake clock으로 검증한다.

## 8. Architecture / Removal Tests

- crate dependency allowlist
- Domain→Provider/Plugin 금지
- Provider A→Provider B 금지
- minimal/provider-free compile
- orphan Cargo dependency/feature/registry config
- `common/utils/helpers` uncontrolled module
- public API semver/schema
- naming/layout validator

## 9. 문서 재검수

### Review 1 Structural
file naming, document ID, depends_on, Canonical Owner, duplicate policy, Plugin/Provider terminology, Manifest, Acceptance, Risk, links.

### Review 2 Cross-Layer
Command→Application→Domain→Persistence→Scheduler→Provider→Recovery→Projection→API 흐름에서 정상/없음/교체/제거/복수/crash/cancel/timeout/partial/migration/disable를 추적한다.

두 Review는 같은 사람이 연속 체크한 표시가 아니라 **서로 다른 검사 목적과 증거**를 남긴다.

## 10. Release Gate

P0:
- format/lint/build
- naming/dependency/docs structural
- unit/property
- storage/Waiting/Side Effect recovery
- fake provider e2e
- provider replacement/removal/multi-select
- Routine restart
- security smoke
- migration
- Cross-Layer scenario suite

P1/P2에 Plugin full lifecycle, sandbox platform, actual Harness canary, soak, distributed chaos를 추가한다.

## 11. 검증 기준

- AT-MOD-001/002/003, AT-ROUTINE-001, AT-TASK-003, AT-SFX-001, AT-PLUGIN-001, AT-REPO-001이 suite에 연결된다.
- 핵심 race가 sleep 기반이 아니다.
- actual model outage가 core Runtime gate를 흔들지 않는다.
- Structural Review와 Cross-Layer Review가 release evidence에 각각 존재한다.
