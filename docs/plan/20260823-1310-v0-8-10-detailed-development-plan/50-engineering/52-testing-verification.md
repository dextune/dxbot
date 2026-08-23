---
title: "테스트·검증·사용자 관점 5회 Review와 2회 재검수 전략 v0.8.10"
document_id: "DXB-ENG-052"
version: "0.8.10"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-ENG-050", "DXB-ENG-051", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042", "DXB-IFC-043", "DXB-ARC-018", "DXB-RUN-033"]
---
# 테스트·검증·사용자 관점 5회 Review와 2회 재검수 전략 v0.8.10

## 필수 deterministic fixture

### Discovery/First Run

- Runtime 없이 top/group/command help와 local client version
- no endpoint `runtime start` default bootstrap 성공/중간 crash/profile write 순서
- 0/1/2 verified endpoint 선택과 no silent fallback
- stale profile, symlink, wrong owner, stale HostGeneration fail closed
- Runtime이 필요한 command가 auto-start하지 않음
- provider-unavailable에서 `runtime doctor --section provider` action 제공

### Selector/Preflight/Mutation

- Bot/Project/Channel/Task selector 0/1/N result
- `conversation send <bot-selector>` → 정확한 Main Conversation
- `channel send <channel-selector>` → 정확한 Channel Conversation
- Thread selector가 redundant parent ID 없이 parent scope를 보존
- optional `if-*` 생략 시 required canonical ID/revision/generation materialization
- explicit canonical ID+CAS path와 auto-preflight path의 동일 CommandPayload semantic
- preflight 뒤 revision race → one conflict, auto retry 0
- ambiguity/fuzzy suggestion이 mutation execution으로 승격되지 않음

### Rendering/Automation

- human/json/jsonl golden snapshot
- nonzero machine error도 complete JSON 1개
- stdout schema와 stderr diagnostic/progress 분리
- TTY/non-TTY/color/pager/locale에 따른 machine shape drift 0
- non-TTY prompt 0
- content/secret/hidden target existence echo 0
- typed next action의 stable code/key/args와 human shell rendering 분리
- `--all` local ceiling에서 partial count+resume cursor+exit 14
- safe output no-follow/no-replace와 stdout summary 유지

### Submission/Recovery

- `Prepared → Dispatching` fsync-before-send crash matrix
- matching Prepared semantic reuse와 mismatching digest non-reuse
- retention-eligible Prepared만 prune, Dispatching 이상 prune 0
- command별 single writer, concurrent observer, writer-exit takeover
- Dispatching crash 후 two-binding lookup, both absent same-ID replay
- SIGINT, broken pipe, local timeout 뒤 implicit Runtime cancel 0
- recovery result에 OperationRef와 `operation_may_continue`
- corrupt/unknown journal에서 auto replay/delete 0, read-only/help/version 유지

### State/Security/Delivery

- PreAcceptError vs Receipt Rejected vs Task/Delegation Rejected/Deferred
- Approval approve/deny/revoke 후 original operation continuation/recovery
- Membership + AuthorityBinding + AuditIntent atomicity
- CreateBot committed payload에 BotRef/MainConversationRef
- Acceptance 하나당 최초 milestone 하나와 cumulative Gate
- user journey `UJ-001~010`의 input/result/failure/recovery/automation trace

## 5회 Review 목적

1. 새 사용자 onboarding/first useful result
2. 반복적인 interactive mutation과 selector/CAS burden
3. shell/CI machine automation과 stable output
4. failure, approval, provider, crash/recovery 재진입
5. multi-instance, bounded output, security/accessibility safety

## 변경 후 별도 재검수

- Structural: package path, frontmatter, ID/DAG, owner, command/input/journey/Acceptance registry, naming, manifest
- Cross-Layer Executability: `User Selector → Preflight → CommandPayload → Journal → Binding → Receipt/Domain → Recovery → Human/Machine Projection`

문서상의 journey GO는 실제 CLI fixture PASS를 대체하지 않는다.
