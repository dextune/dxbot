---
title: "보안·권한·승인·Sandbox"
document_id: "DXB-RUN-032"
version: "0.1.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-012", "DXB-DOM-025", "DXB-RUN-031"]
---


# 보안·권한·승인·Sandbox

## 1. 목적

지속형 Bot과 다중 Core/Bot이 장기간 권한을 보유하는 위험을 통제하고, Tool·Memory·Workspace·Network·Bot 간 접근을 최소 권한과 감사 가능한 정책으로 실행한다.

## 2. 책임 범위

- principal, capability, permission, policy
- human/machine approval
- sandbox와 process/filesystem/network
- secret handling
- untrusted content와 prompt injection
- Bot-to-Bot trust
- 감사와 break-glass

본 문서는 일반 보안 설계 기준이며 특정 규제 준수 인증을 주장하지 않는다.

## 3. Threat Model

주요 위협:
- 악성/오염된 Web·파일·Tool output이 모델을 조작
- Bot 또는 Core의 권한 과다
- secret가 prompt/log/subprocess env로 유출
- workspace 탈출, symlink/junction, path traversal
- 임의 process/network 실행
- Bot 간 권한 confused deputy
- stale Core/Lease의 늦은 write
- plugin/provider 공급망
- Control Plane 계정 탈취
- Memory poisoning와 민감 정보 장기 보존
- audit/log 위조 또는 누락
- denial of service와 비용 폭주

## 4. 권한 모델

### Principal
- User/Operator
- Bot
- Core Execution
- Service/Plugin
- Remote Worker
- Break-glass Operator

### Resource
- Bot/Memory/Task/Artifact
- Workspace path
- Tool/Skill
- Model/provider
- Network destination
- Secret handle
- Control operation

### Decision
`allow`, `deny`, `ask`, `allow_with_constraints`

Decision 입력:
- principal chain
- resource/action
- Bot/Task/Execution context
- policy version
- trust/sensitivity
- requested scope
- deadline/resource budget
- prior approval/delegation
- environment/deployment mode

기본은 deny다. deny 이유는 모델에 민감 세부를 노출하지 않으면서 운영자 audit에는 충분히 기록한다.

## 5. Capability Grant

Core는 시작 시 최소 grant를 받는다.
- allowed tools/actions
- workspace roots와 read/write mode
- network allowlist
- model/provider
- secret handles
- max subprocess/resource
- expiry
- lease/fencing
- delegation allowed 여부

Grant는 immutable이고 Core Identity와 결합한다. 추가 권한은 approval로 새 grant revision을 발급한다.

## 6. Tool 실행 보안 흐름

1. Tool call schema 검증
2. untrusted arguments normalize
3. permission decision
4. `ask`이면 structured approval
5. resource admission
6. sandbox policy resolve
7. secret handle 최소 범위 주입
8. execute
9. output size/type/trust label 적용
10. side effect/audit record
11. result normalization

보안 판단을 Tool별 임의 코드로 복제하지 않고 공통 pipeline에 둔다.

## 7. Sandbox

Sandbox policy 차원:
- filesystem read/write roots
- network off/allowlist/full
- process namespace/child tree
- environment allowlist
- CPU/memory/time/file size
- device access
- syscall/container/OS provider
- temp directory
- workspace ownership
- cleanup/forensics

원칙:
- 작업 경로는 canonicalize와 root containment를 모두 확인
- link-shaped path를 따라 재귀 삭제하지 않음
- temp dir는 private permissions와 예측 불가 이름
- process env는 allowlist 방식, `*KEY*/*SECRET*/*TOKEN*/*PASSWORD*`류 제거
- stdout/stderr는 bounded collect 또는 spill
- kill 후 child tree exit를 기다림
- network와 filesystem sandbox를 동일 개념으로 착각하지 않음

## 8. Secret 관리

- Domain/Memory/Event에 secret 원문 저장 금지
- `SecretRef`/opaque handle만 전달
- provider가 실제 호출 직전에 resolve
- prompt/system context에 secret를 포함하지 않음
- subprocess env에 필요한 항목만 일회성 주입
- 출력 redaction과 canary test
- rotation 후 새 resolve가 적용되며 진행 중 실행과 관계를 기록
- secret 접근은 purpose와 principal audit
- Bot 간 Message로 secret 전달 금지

## 9. Untrusted Content와 Prompt Injection

- Web/file/message/tool output은 기본 untrusted label
- instruction과 data channel을 구조적으로 구분
- untrusted content가 policy/system instruction을 변경할 수 없음
- sensitive Tool 전에 policy/approval를 재평가
- retrieved Memory도 provenance/trust를 유지
- HTML/Unicode/hidden content normalize와 위험 signal
- 모델의 "이미 승인됨" 주장으로 approval를 대체하지 않음
- 결과 검증과 action 실행을 분리
- 고위험 작업은 deterministic guard 또는 human approval
- 모델-visible context와 실제 권한을 분리하여 prompt가 권한 자체를 만들지 못하게 함

## 10. Bot-to-Bot 보안

- target Bot은 source Bot Identity를 인증
- delegation token은 audience/scope/expiry/task에 묶음
- source 권한 자동 상속 금지
- received Memory/Artifact는 target policy로 재검증
- 재위임 가능 여부와 최대 depth
- trust level과 reputation은 authorization의 단독 근거가 아님
- cross-tenant는 기본 금지
- Message payload size/rate/schema 제한

## 11. Approval

Approval Request:
- action summary
- exact scope/resource
- risk and side effect
- requested duration
- Bot/Task/Core
- proposed constraints
- alternatives
- request digest

Approval Decision:
- approver principal
- allow/deny/constraints
- expiry/single-use
- reason
- bound request digest
- audit ID

Action arguments가 바뀌면 기존 approval를 재사용할 수 없다. UI가 없으면 fail-closed 또는 명시적 machine policy를 사용한다.

## 12. Plugin/Dependency 보안

- dependency/source/license review
- signed or pinned artifact digest
- plugin manifest capability declaration
- load-time schema validation
- runtime privilege isolation
- update canary와 rollback
- SBOM과 vulnerability scanning
- `unsafe`/native code 별도 검토
- plugin의 Control Plane 접근 금지 또는 최소 Port
- self-modification capability는 P3 이전 기본 비활성

## 13. 예외상황

- audit sink unavailable: 민감 write fail-closed 또는 durable local spool
- sandbox provider unavailable: unsandboxed fallback 금지
- approval client disconnected: pending 유지/expiry
- secret resolver failure: retry 분류 후 실행 중지
- path race: open-at/handle 기반 enforcement 또는 재검증
- revoked permission during execution: 신규 call 차단, high-risk 진행 call cancel
- compromised Bot 의심: quarantine, credentials revoke, Core stop, forensic preserve
- corrupt policy: startup fail-closed
- break-glass: 제한 시간, 2인 승인 가능, 모든 action 강화 audit

## 14. 확장성

원격 Worker에서도 capability grant와 policy snapshot을 서명/검증한다. 중앙 정책과 node enforcement를 이중화하되 결정 충돌 시 더 제한적인 정책을 적용한다. Tenant/RBAC/ABAC 확장은 Resource/Principal model에 추가하고 Domain 계약을 바꾸지 않는다.

## 15. 구현 우선순위

- **P0:** default deny, Tool pipeline guard, workspace/path, secret ref, audit, basic approval
- **P1:** network/process sandbox, Bot delegation token, policy versions, incident quarantine
- **P2:** remote signed grants, tenant isolation, plugin signing
- **P3:** 고급 content taint propagation과 policy analytics

## 16. 검증 기준

- approval 없이 고위험 Tool이 실행되지 않는다.
- prompt/log/event/subprocess 기본 env에서 secret canary가 검출되지 않는다.
- path traversal/symlink/junction test가 workspace 밖 변경을 막는다.
- stale/revoked Core가 write하지 못한다.
- source Bot 권한이 target Bot에 자동 전파되지 않는다.
- sandbox 미가용 시 host 실행으로 fallback하지 않는다.
- 모든 민감 action이 request digest와 approver를 가진 audit record로 추적된다.
