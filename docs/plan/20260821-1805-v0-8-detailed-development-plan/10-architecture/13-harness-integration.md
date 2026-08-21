---
title: "Harness 통합 경계"
document_id: "DXB-ARC-013"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-ARC-012"]
---

# Harness 통합 경계

## 1. 목적

DeepSeek-inspired Harness를 교체 가능한 execution substrate로 사용하되 DXBOT Bot/Brain/Memory/Task identity와 persistence를 외부 Harness가 소유하지 않게 한다.

## 2. Adapter 경계

```text
DXBOT Application/Runtime
→ Capability Contract
→ Provider Host
→ Harness Adapter
→ external Harness/Model/Tool/Sandbox
```

Harness Session·trajectory·subagent·plugin tree를 Bot, Thread, Core, Domain Event로 자동 매핑하지 않는다.

## 3. 입력과 결과

Execution snapshot과 bounded context를 adapter에 전달하고, result/tool output은 typed stable DTO와 resource accounting을 거쳐 proposal/evidence로 돌아온다. untrusted output은 Canonical Memory·Permission·Task state를 직접 변경하지 않는다.

## 4. Lifecycle와 recovery

Provider generation, activity, deadline, cancellation, side-effect unknown, restart/re-registration은 Common Provider Host가 조정한다. Harness process/session 손실은 Bot identity 손실이 아니다.

## 5. 검증 기준

- upstream type이 Domain/public Application schema에 누출되지 않음
- Provider Host 우회 production path 0
- deterministic Reference Provider로 Core correctness 검증 가능
- external upgrade가 compatibility/conformance 없이 merge되지 않음
