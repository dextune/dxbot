---
title: "위험 등록부"
document_id: "DXB-DEL-062"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DEL-060", "DXB-RUN-032", "DXB-RUN-031", "DXB-IFC-040", "DXB-IFC-041"]
---

# 위험 등록부

## 1. 목적

v0.8 구현 기준선의 Critical/High 위험을 owner, 선행 신호, Acceptance, milestone blocker에 연결한다. 기존 Persistent Identity/Memory/Task/Provider/Security/Resource 위험은 각 active Canonical 문서의 검증 기준으로 계속 관리한다.

## 2. v0.8 Risk

| ID | 위험 | 영향 | Owner / 핵심 완화 | Acceptance | Blocker |
|---|---|---|---|---|---|
| R-090 | 숨은 inheritance로 구현자별 계약 분기 | Critical | GOV-001, active package materialization | AT-BASE-001 | M0 |
| R-091 | P0 command 범위 불명확으로 scope 폭증 | High | IFC-041 matrix/P1-P2 freeze | AT-CLI-008 | M0/M3 |
| R-092 | uncertain mutation receipt 복구 불가 | Critical | IFC-040 durable receipt/key binding | AT-APP-005 | M1/M3 |
| R-093 | ambiguous selector가 wrong resource 변경 | Critical | IFC-040 exact scoped resolution | AT-CLI-009 | M1/M3 |
| R-094 | duplicate Runtime instance/endpoint/data split-brain | Critical | RUN-035 lock/fencing/InstanceId | AT-HOST-001 | M2 |
| R-095 | pagination 누락·중복을 complete snapshot으로 오판 | High | IFC-040 stable sort/snapshot/cursor | AT-APP-006 | M1/M5 |
| R-096 | partial JSONL/timeout을 success로 오판 | Critical | IFC-040/041 terminal/wait/exit | AT-APP-007, AT-CLI-010 | M3/M5 |
| R-097 | local endpoint/terminal/export 공격 | Critical | RUN-032 + CLI safe renderer/writer | AT-SEC-005 | M2/M5 |
| R-098 | client/server public schema drift | Critical | Rust schema SSOT/generated check | AT-SCHEMA-001 | M1/M6 |
| R-099 | 선언 Gate가 실제 CI에서 미실행 | Critical | ENG-054 active validator/broken fixtures | AT-CI-001 | M0/M6 |
| R-100 | Contract-all-at-once가 미사용 추상화 폭증 | High | vertical slice, logical boundary first | Review 3 | M1~M6 |

## 3. 기존 Critical Cluster 비회귀

계속 직접 Gate로 다룬다.
- Session/Provider/Core와 Bot/Thread/Task identity 혼용
- stale revision/generation/authorization write
- Side Effect duplicate/unknown blind retry
- Project/Channel Brain화와 Shared Memory 복제
- Provider Host bypass/Dependency Firewall 침투
- unbounded queue/cache/stream/context
- Durable Process child state 복제/replay 오류
- Private→Shared leak/ActionGrant replay
- Runtime memory OOM/leak/recovery storm

## 4. 선행 신호

- prior plan path/“상속” 문구가 active normative requirement로 등장
- matrix 없는 CLI subcommand 또는 one operation의 여러 mutation 구현
- receipt 없이 mutation transport retry
- name/alias mutation이 ID/revision 없이 commit
- same data root에 socket/lock/writer가 둘 이상
- cursor에 filter/scope/schema binding 없음
- JSONL terminal record 없는 성공 exit
- Runtime timeout을 rollback으로 설명
- world-writable endpoint, raw ANSI/OSC, overwrite-capable export
- public DTO를 Domain/Persistence derive로 공유
- workflow가 v0.1 hash만 검사
- P0 use-case 없는 generic RPC/IDL/crate 추가

## 5. Risk 처리 규칙

- Critical Risk는 owner, deterministic fixture, Acceptance, milestone exit evidence 없이 close하지 않는다.
- `Accepted` 문서 상태는 Risk mitigation 실행 완료가 아니다.
- residual risk는 likelihood/impact/monitor/rollback과 승인자를 release evidence에 남긴다.
- 안전하지 않은 fallback으로 milestone을 통과하지 않는다.

## 6. 검증 기준

- R-090~100 모두 owner/Acceptance/Review/milestone 중 하나 이상 연결.
- Critical Risk orphan 0.
- P0 Release 전에 R-090~099 executable evidence.
- R-100 Review 3에서 unused abstraction 0으로 검증.
