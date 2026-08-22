---
title: "용어집과 도메인 모델"
document_id: "DXB-GOV-002"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-BASE-000"]
---
# 용어집과 도메인 모델

## 1. 제품 용어

| 용어 | 정의 | Canonical Owner |
|---|---|---|
| Bot | Session과 독립된 Persistent Identity·Memory·State 엔티티 | `DXB-DOM-020` |
| Brain | 한 Bot의 Identity·Memory·Goal·실행 정책을 연결하는 logical semantics | `DXB-DOM-021` |
| Core Lease | Scheduler가 하나의 Execution에 임시 부여하는 bounded 실행 권한 | `DXB-DOM-024` |
| Task | durable work intent | `DXB-DOM-023` |
| Execution | immutable Task revision을 수행하는 하나의 attempt | `DXB-DOM-023` |
| Side Effect Record | 외부 변경 intent와 uncertain outcome을 추적하는 durable ledger row | `DXB-DOM-023` |
| Conversation / Thread | persistent message parent와 그 lineage branch | `DXB-DOM-027` |
| Project / Channel Membership | scope별 Bot 참여 관계와 CAS generation | `DXB-DOM-028/029` |

## 2. Application Contract 용어

| 용어 | 정의 | Owner |
|---|---|---|
| RequestDigest | client가 전송 전 계산 가능한 operation/raw typed selector/payload/semantic option의 canonical digest | `DXB-IFC-040` |
| ResolvedBindingDigest | server가 Principal, resolved target/revision, authorization와 effective policy generations를 RequestDigest에 결박한 digest | `DXB-IFC-040` |
| CommandId | logical mutation identity | `DXB-IFC-040` |
| IdempotencyKey | issuance epoch와 random component를 가진 principal-local mutation key | `DXB-IFC-040` |
| Operation Receipt | request binding과 Application operation commit/recovery 상태 | `DXB-IFC-040` |
| Directive | cancel/suspend/redirect처럼 target에 적용될 control intent와 safe-point 상태 | `DXB-RUN-036` |
| Wait Predicate | receipt, directive 또는 target의 명시된 상태를 관찰하는 operation별 predicate | `DXB-IFC-040` |
| Query Snapshot | stable ordering/watermark에 결박된 bounded read view | `DXB-IFC-040` |
| Terminal Stream Record | complete/partial/gap/error와 last safe cursor를 나타내는 JSONL 마지막 record | `DXB-IFC-040` |

## 3. Security 용어

| 용어 | 정의 | Owner |
|---|---|---|
| LocalPrincipalRef | InstanceId와 authenticated OS UID로 server가 파생한 principal | `DXB-RUN-032` |
| AuthorityBinding | scope/action set과 subject를 immutable generation으로 결박한 security state | `DXB-RUN-032` |
| Approval | 특정 action digest에 대한 요청과 승인/거부/만료 결정 | `DXB-RUN-032` |
| ActionGrant | 승인·authority decision에서 발급된 제한된 실행 권한 | `DXB-RUN-032` |
| RoleRef | 사람이 이해하는 역할 분류이며 그 자체로 Authority가 아님 | `DXB-DOM-028/029` |

## 4. 반드시 구분할 상태

```text
Local Journal State ≠ Operation Receipt State
Operation Receipt Committed ≠ Directive Applied
Directive Applied ≠ Target Lifecycle Terminal
CLI Timeout/SIGINT ≠ Target Cancellation
RequestDigest ≠ ResolvedBindingDigest
Role ≠ AuthorityBinding ≠ Authorization Decision ≠ ActionGrant
History ≠ Memory
Provider Session ≠ Bot/Thread/Execution/Core Lease
Host Action ≠ Application Command
```

## 5. 상태 축

### Local Journal

`PreparedUnsent → SentUnknown → Observed → Terminal` 또는 local-only `Abandoned`.

### Operation Receipt

`Accepted → Committed | Rejected | Superseded | RecoveryRequired`; `RecoveryRequired`는 explicit reconcile 후 terminal decision으로 전이할 수 있다.

### Directive

`Queued → AwaitingSafePoint → Applying → Applied | Rejected | Superseded | RecoveryRequired`.

`Committed`는 operation 자체의 canonical write가 완료됐다는 뜻이다. Task 완료, Directive 적용, Runtime process 종료와 동의어가 아니다.
