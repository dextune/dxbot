---
title: "자원·비용·Runtime Memory Admission 관리"
document_id: "DXB-RUN-031"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-RUN-030", "DXB-DOM-024"]
---
# 자원·비용·Runtime Memory Admission 관리

Runtime Resource Governor는 process-wide memory/cost/concurrency와 recovery headroom을 소유한다. CLI local journal retention/capacity 정책의 Canonical Owner는 CLI(`DXB-IFC-041`)이며 Runtime Resource Governor가 이를 중복 정의하지 않는다.

Runtime queue/cache/page/stream/provider activity/receipt metadata에는 item+byte cap이 있다. Critical/Emergency에서는 신규 semantic work를 commit 전에 거부하고 control/recovery headroom을 보호한다.

nonterminal Receipt/Directive/SideEffect/Process는 TTL로 삭제하지 않는다. recovery admission과 operator reconcile을 우선한다.

`--all`과 subscription/export는 incremental bounded processing을 사용한다.
