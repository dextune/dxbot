---
title: "통합 구현 로드맵과 단계별 Gate v0.8.7"
document_id: "DXB-DEL-060"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-BASE-000", "DXB-ARC-010", "DXB-ARC-018", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042", "DXB-ENG-052"]
---
# 통합 구현 로드맵과 단계별 Gate v0.8.7

## M0A — Semantic Closure
Exit: `AT-BASE-001`, `AT-PLAN-002`, `AT-DOC-CONTRACT-001`.

## M1A — Storage Proof
submission/idempotency binding, atomic Receipt/outbox/audit, snapshot/disk pressure를 증명한다. Exit: `AT-STORAGE-001`, `AT-IDEMP-001`.

## M1B — Kernel/Domain/Contract Source
schema generator, operation lifecycle, journal types, Reference Provider test policy를 구현한다. Exit: `AT-CONTRACT-001`, `AT-SUBMIT-001`, `AT-JOURNAL-001`.

## M2 — Instance/Security
bootstrap, UID Principal, default policy, Approval/Authority/ActionGrant/audit를 구현한다. **M2 security command subset만 freeze**한다. Exit: `AT-HOST-001`, `AT-BOOT-001`, `AT-SECSTATE-001`, `AT-AUDIT-001`.

## M3 — Bot-only CLI
runtime/bot/conversation/thread/task submit-show-result/operation show를 구현한다. watch는 M5까지 show polling으로 대체한다. explicit Reference Provider test policy는 deterministic E2E에서만 사용한다. **M3 subset만 freeze**한다. Exit: `AT-CLI-CORE-001`, `AT-SUBMIT-001`, `AT-APP-005`.

## M4 — Multi-Bot/Project/Channel
project/channel/membership/delegation을 구현하고 **M4 subset만 freeze**한다. Exit: `AT-MEMBER-001`, `AT-DELEGATE-001`.

## M5 — Recovery/Streaming/Harness
watch/subscription, export, side-effect reconcile, provider observation/real Harness canary를 구현하고 M5 subset을 freeze한다. Exit: `AT-APP-006`, `AT-APP-007`, `AT-EXPORT-001`, `AT-HARNESS-001`.

## M6 — Full Freeze
compatibility/security/performance/packaging evidence 후 전체 63 operation generated schema를 freeze한다. 그 전 full freeze는 금지한다.
