# Observability and Tracing Guide

## Applies When

log, trace, metric, audit event, diagnostic context를 추가·변경할 때 적용한다.

## Core Rules

telemetry는 canonical state가 아니며 correctness 결정을 log 존재 여부에 의존하지 않는다. trace/metric은 operation/task/execution/provider generation 같은 stable correlation을 사용하되 secret/private payload를 기본 수집하지 않는다.

Audit는 일반 diagnostic log와 구분한다. 보안·승인·고위험 action처럼 durable audit가 필요한 경우 해당 Owner의 atomicity/recovery contract를 따른다.

## Decision Rules

새 metric/log는 어떤 운영 질문을 답하는지와 cardinality/retention 비용을 설명할 수 있어야 한다. high-cardinality ID나 raw payload를 편의상 label/log에 넣지 않는다.

## Forbidden Patterns

secret/token/raw sensitive content logging, unbounded label cardinality, Provider별 독립 global telemetry system, log parsing을 protocol/retry/security source로 사용하는 것을 금지한다.

## Verification

redaction, correlation, bounded cardinality, audit crash atomicity, telemetry failure가 product flow를 불필요하게 중단하지 않는지 검사한다.
