# Fixture and Test Layout Guide

## Applies When

test file, fixture, golden, reference provider, intentionally broken fixture를 추가·이동할 때 적용한다.

## Core Rules

fixture는 어떤 Contract/Owner를 증명하는지 명확해야 한다. versioned public/persistence contract fixture는 compatibility evidence로 유지한다. Common Conformance scenario는 Common/Testkit이 소유하고 Provider는 factory/config fixture만 제공하는 방식을 우선한다.

Golden/generated snapshot은 source-of-truth와 생성 명령을 갖고 수동 수정하지 않는다. false-positive 방지를 위해 의미 있는 negative/broken fixture를 둔다.

## Decision Rules

한 Provider에만 의미 있는 fixture는 Provider boundary에, 여러 Provider의 동일 contract를 검증하면 Common Testkit에 둔다. 큰 binary fixture는 필요성과 생성/보존 비용을 검토한다.

## Forbidden Patterns

unversioned compatibility fixture, 실제 credential 포함, provider마다 Conformance scenario 복사, generated output 직접 편집을 금지한다.

## Verification

fixture owner, naming/path, regeneration, old fixture load, negative fixture가 실제 실패를 유발하는지 검사한다.
