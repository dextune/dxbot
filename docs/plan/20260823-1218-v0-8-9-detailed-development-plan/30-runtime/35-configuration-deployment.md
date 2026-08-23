---
title: "구성·Bootstrap·Default Policy·Runtime Instance·배포"
document_id: "DXB-RUN-035"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-ARC-011", "DXB-RUN-032"]
---
# 구성·Bootstrap·Default Policy·Runtime Instance·배포

first init은 InstanceId, HostGeneration, LocalPrincipal, default Brain/Permission/Resource/Provider policy generations, owner AuthorityBinding, DataSchemaVersion을 원자적으로 생성한다.

Bot creation은 Provider readiness와 분리한다. configured Ready production Provider가 없으면 Bot은 Inactive로 생성될 수 있고 Activate/Task admission에서 typed `provider-unavailable`을 반환한다. Reference Provider는 explicit test profile/policy에서만 선택한다.

`runtime stop` graceful command는 HostGeneration을 CLI argv로 요구하지 않는다. Control client가 authenticated endpoint descriptor의 InstanceId/HostGeneration을 local discovery하여 request binding에 포함하고 server가 current generation을 fence한다.

explicit `--host-stop --host-generation`만 host-level escalation이며 endpoint failure가 자동 escalation으로 바뀌지 않는다.
