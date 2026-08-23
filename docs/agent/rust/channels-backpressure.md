# Channels and Backpressure Guide

## Applies When

queue, async channel, stream buffer, output sink를 생성하거나 용량 정책을 바꿀 때 적용한다.

## Core Rules

모든 queue/channel에는 owner, max items, 필요 시 max bytes, overflow/backpressure, cancellation, shutdown/drain, metrics가 있어야 한다. unbounded channel과 무기한 background queue를 금지한다.

Provider 내부 transport queue도 Common admission을 우회해 semantic work를 축적할 수 없다.

## Decision Rules

overflow는 `reject | block/backpressure | drop derived observation | spill to durable owner` 중 의미를 명시한다. Canonical work를 조용히 drop하지 않는다. slow consumer가 producer와 전체 Runtime을 무한정 붙잡지 않도록 bounded policy를 둔다.

## Forbidden Patterns

item count만 있고 byte cap이 필요한 large payload queue, hidden retry queue, shutdown 시 sender/receiver ownership 불명확, cache를 queue처럼 사용하는 것을 금지한다.

## Verification

full queue, slow consumer, cancellation, producer crash, shutdown/drain, byte ceiling, leak을 검증한다.
