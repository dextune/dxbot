---
title: "상세 설계 문서 표준 템플릿"
document_id: "DXB-GOV-004"
version: "0.8.0"
status: "Accepted Template"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-GOV-001", "DXB-GOV-002", "DXB-GOV-003"]
---

# 상세 설계 문서 표준 템플릿

새 Domain/Runtime/Application Contract/Interface/Capability/Provider/Plugin 문서는 다음 항목을 적용 가능한 범위에서 작성한다.

## 1. 필수 머리말

- 목적과 active scope
- Classification / Quality Tier / Priority
- Canonical Owner / Lifecycle Owner / Persistent State Owner
- 명시적 비범위와 금지 dependency
- P0/P1/P2 구분

## 2. 상태와 상호작용

- input/output/event와 typed error
- 상태기계·revision·race winner
- ResourceSelector와 Resolved ResourceRef
- Operation Receipt/idempotency/request digest
- cancellation/deadline/wait/timeout
- page snapshot/order/cursor 또는 stream event/terminal/resume
- authorization/approval/information flow

## 3. Runtime·자원·보안

- owner가 있는 task/channel/cache와 item+byte cap
- allocation/copy/serialization boundary
- crash/restart/reconcile
- local endpoint/terminal/file attack surface
- secret/redaction/audit
- compatibility/migration/removal

## 4. Public Contract 문서 추가 항목

- public DTO ≠ Domain/Persistence type
- schema source와 generated artifact
- machine output/partial status
- stable error/exit class
- capability/version negotiation
- deterministic fixture와 golden snapshot

## 5. Interface 문서 추가 항목

- command path와 단일 Application operation mapping
- target selector와 expected revision
- wait/follow/timeout
- human output와 machine schema
- stdout/stderr/exit
- input source(argv/stdin/file)와 secret 금지
- terminal sanitization과 file export policy

## 6. 검증과 변경

- Given/When/Then Acceptance
- Risk/OQ/Milestone 연결
- Review 1 Structural/Traceability 범위
- Review 2 Fault/Compatibility 경로
- Review 3 Scope/Overengineering 질문
- 구현되지 않은 검증을 PASS로 주장하지 않음

## 7. 변경 체크리스트

1. active package 밖의 문장을 요구하지 않는가?
2. Canonical Owner가 하나인가?
3. selector/receipt/page/stream/machine/local-security 의미가 필요한가?
4. Domain/Application/Provider/Interface shortcut이 없는가?
5. bounded resource와 cleanup이 있는가?
6. P1/P2가 P0 Gate로 유입되지 않았는가?
7. code/schema/test/docs/manifest가 같은 change set인가?
