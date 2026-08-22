---
title: "Live Control과 Safe-Point Directive"
document_id: "DXB-RUN-036"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-RUN-030", "DXB-RUN-032", "DXB-DOM-023"]
---
# Live Control과 Safe-Point Directive

Cancel/Suspend/Resume/Redirect는 typed Directive를 사용한다.

```text
Queued → AwaitingSafePoint → Applying
Applying → Applied | Rejected | Superseded | RecoveryRequired
```

Receipt `Committed`는 Directive creation commit을 뜻한다. `--wait applied`는 Directive를 생성하는 control operation에만 허용한다.

Bot/Project lifecycle command와 graceful Runtime shutdown은 별도 owner 상태를 Query/Status로 관찰하며 P0 `applied` wait를 사용하지 않는다. P0 `target-terminal` wait는 없다. long-running target 완료는 `task/process watch` 또는 show polling으로 관찰한다.

Report는 Query이며 별도의 미정의 report-generation Command를 가정하지 않는다.
