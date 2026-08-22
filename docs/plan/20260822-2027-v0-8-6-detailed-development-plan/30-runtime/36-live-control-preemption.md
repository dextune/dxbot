---
title: "Live Control과 Safe-Point Directive"
document_id: "DXB-RUN-036"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-RUN-030", "DXB-RUN-032", "DXB-DOM-023"]
---
# Live Control과 Safe-Point Directive

cancel, suspend, resume, redirect는 typed Directive를 사용한다.

```text
DirectiveId / Revision
Action: Redirect | Suspend | Resume | Cancel
TargetRef / ExpectedRevision / ExecutionGeneration
Principal / AuthorityDecision / ActionGrant refs
Payload/ArtifactRef?
State / OutcomeRef
```

## State

```text
Queued → AwaitingSafePoint → Applying
Applying → Applied | Rejected | Superseded | RecoveryRequired
RecoveryRequired → Applied | Rejected | Superseded
```

Operation Receipt `Committed`는 Directive record 생성 commit을 뜻한다. Directive `Applied` 또는 target terminal은 별도 wait predicate다.

## Action semantics

- Redirect: running Execution을 in-place rewrite하지 않고 safe point 후 새 TaskRevision/Execution을 만든다.
- Suspend: request와 실제 quiesced/checkpoint를 구분하고 transient lease/resource를 release한다.
- Resume: current authorization/config/provider를 재검증하고 새 generation을 사용한다.
- Cancel: unknown side effect가 있으면 false `Cancelled` 대신 RecoveryRequired/Reconciliation을 사용한다.
- Report: 기본은 Query다. durable report artifact 생성이 필요하면 별도 typed report-generation Command를 사용하며 generic process control이 아니다.

CLI local timeout/SIGINT는 Directive를 rollback하거나 target state를 변경하지 않는다.
