---
title: "Dynamic Core Scheduler"
document_id: "DXB-DOM-024"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-023", "DXB-DOM-021", "DXB-ARC-012", "DXB-ARC-017"]
---

# Dynamic Core Scheduler

## 1. 목적

v0.4 Scheduler-owned Core Lease/fairness/admission을 유지하고 Channel participant activation도 동일 scheduling authority를 사용하게 한다.

## 2. 비협상 원칙

- Bot/Project/Channel schema에 fixed Core count를 Identity로 저장하지 않는다.
- Channel/Manager 전용 Core를 영구 배정하지 않는다.
- Coordinator/Manager/Control API가 Lease를 직접 mint/revoke하지 않는다.
- Provider selection/permit은 Provider Host가 소유한다.
- stale/expired Lease late result는 Canonical commit 권한이 없다.

## 3. Channel WorkItem

Channel routing 결과는 필요한 경우 `BotId + TaskId + ProjectId? + ChannelId? + ThreadId? + priority/class` provenance를 가진 일반 WorkItem으로 Scheduler에 제출된다.

Project/Channel ID는 observability/fairness input일 뿐 Core identity가 아니다.

## 4. Fairness

기존 per-Bot/Task fairness에 추가로 Channel burst가 전체 deployment를 독점하지 않도록 admission input을 제공할 수 있다. 실제 ceiling/default는 Resource Policy SSOT가 소유한다.

## 5. Rebalance / Control

Manager의 “Researcher를 더 붙여라”, “Core를 하나 더 써라” intent는 participant selection/parallelism/resource hint로 변환할 수 있으나 실제 Lease 수는 Scheduler가 결정한다.

## 6. 검증 기준

- 같은 Manager Bot의 연속 Channel 응답이 서로 다른 Core Lease를 사용해도 identity가 동일.
- participant fan-out이 Scheduler hard ceiling을 우회하지 않음.
- Channel archive/leave가 worker handle 직접 kill로 구현되지 않음.
- v0.4 deterministic fairness/lease fencing tests 유지.
