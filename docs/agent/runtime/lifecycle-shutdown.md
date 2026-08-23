# Lifecycle and Shutdown Guide

## Applies When

Runtime/Provider/Plugin/process/task의 start/ready/drain/stop/restart나 supervisor 구조를 바꿀 때 적용한다.

## Core Rules

lifecycle state와 health observation을 구분한다. Ready 전 신규 selection/admission, Draining 후 신규 activity, stop 완료 전 child/process/channel/resource 잔존을 허용하지 않는다. stale generation의 late callback/write를 fencing한다.

shutdown은 cancellation 요청이 아니라 child join, channel close/drain, permit/activity 반환, process cleanup이 확인된 상태다.

## Decision Rules

lifecycle Owner는 global 사실을 하나만 소유하고 Extension이 별도 global lifecycle state를 복제하지 않는다. cross-component shutdown은 bounded deadline과 명시적 partial/recovery 의미를 가져야 한다.

## Forbidden Patterns

fire-and-forget child, health=ready 동일시, drain 중 신규 selection, cleanup 실패를 success stop으로 기록, endpoint failure를 강제 kill로 자동 escalation하는 것을 금지한다.

## Verification

start failure, degrade, drain/new-call race, shutdown-under-load, crash/restart, stale callback, leak을 검사한다.
