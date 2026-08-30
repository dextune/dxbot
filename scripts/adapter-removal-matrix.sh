#!/usr/bin/env bash
#
# DXB-DEL-068 H11 — adapter-removal build/check matrix.
#
# Runs the exact production-adapter removal variants across the feature-sliced
# crates (provider-host, runtime-host, cli) and prints stable, greppable
# variant/factory evidence. It NEVER mutates source: it only invokes cargo
# build/check/tree and emits a deterministic report.
#
# Required variants (mirrors the plan §12 "Adapter Removal Build"):
#   1. normal   — every production adapter, no testkit in the built CLI
#   2. dsh-only  — only the DSH ACP slice (`--no-default-features --features dsh-acp`)
#   3. minimax-only — only the direct MiniMax slice
#   4. deepseek-only — only the direct DeepSeek slice
#   5. all-production — explicit all three production slices
#
# For each variant it records:
#   * cargo check/build success (fail closed on any error)
#   * the resolved provider-host feature set (which slices are compiled in)
#   * a `cargo tree` proof that `testkit` is ABSENT from the normal built CLI
#   * a `cargo tree` proof that a removed slice's transport dep is absent
#
# Usage:
#   scripts/adapter-removal-matrix.sh                 # run the full matrix
#   scripts/adapter-removal-matrix.sh --check-only    # cargo check (faster)
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

MODE="build"
if [[ "${1:-}" == "--check-only" ]]; then
  MODE="check"
fi

PASS=0
FAIL=0
declare -a FAILURES=()

hr() { printf '%.0s-' {1..72}; printf '\n'; }

section() {
  hr
  printf '## %s\n' "$1"
  hr
}

# Run one cargo build/check for a crate + feature selection. Prints a stable
# EVIDENCE line. Fails the matrix (records a failure) on any cargo error.
run_variant() {
  local label="$1"; shift
  local crate="$1"; shift
  # remaining args are the cargo feature flags
  printf 'VARIANT %-22s crate=%-14s flags="%s"\n' "$label" "$crate" "$*"
  if cargo "$MODE" -p "$crate" "$@" >/dev/null 2>"/tmp/dxb-matrix-$$.err"; then
    printf 'RESULT  %-22s crate=%-14s status=PASS\n' "$label" "$crate"
    PASS=$((PASS + 1))
  else
    printf 'RESULT  %-22s crate=%-14s status=FAIL\n' "$label" "$crate"
    sed 's/^/    cargo> /' "/tmp/dxb-matrix-$$.err" | head -20
    FAIL=$((FAIL + 1))
    FAILURES+=("$label ($crate)")
  fi
  rm -f "/tmp/dxb-matrix-$$.err"
}

# Print the resolved provider-host feature set for a cli feature selection.
provider_host_features() {
  local label="$1"; shift
  local features
  features="$(cargo tree -p cli "$@" -e features -i provider-host 2>/dev/null \
    | grep -oE 'provider-host feature "[a-z0-9-]+"' \
    | sed -E 's/provider-host feature "([a-z0-9-]+)"/\1/' \
    | sort -u | tr '\n' ',' | sed 's/,$//')"
  printf 'FACTORY-SLICES %-18s provider-host-features=[%s]\n' "$label" "$features"
}

# Assert a token is ABSENT from a cli variant's dependency tree.
assert_absent() {
  local label="$1"; shift
  local token="$1"; shift
  if cargo tree -p cli "$@" 2>/dev/null | grep -q "$token"; then
    printf 'PROOF   %-22s %s=PRESENT (UNEXPECTED)\n' "$label" "$token"
    FAIL=$((FAIL + 1))
    FAILURES+=("$label expected $token absent")
  else
    printf 'PROOF   %-22s %s=ABSENT (ok)\n' "$label" "$token"
    PASS=$((PASS + 1))
  fi
}

# Assert a token is PRESENT in a cli variant's dependency tree.
assert_present() {
  local label="$1"; shift
  local token="$1"; shift
  if cargo tree -p cli "$@" 2>/dev/null | grep -q "$token"; then
    printf 'PROOF   %-22s %s=PRESENT (ok)\n' "$label" "$token"
    PASS=$((PASS + 1))
  else
    printf 'PROOF   %-22s %s=ABSENT (UNEXPECTED)\n' "$label" "$token"
    FAIL=$((FAIL + 1))
    FAILURES+=("$label expected $token present")
  fi
}

# Assert a provider-host slice feature is NOT enabled for a cli variant. This is
# the authoritative removal proof: a removed slice contributes no feature, no
# module, no factory row (unlike shared deps such as `nix`, which control-server
# also uses for its socket layer and are therefore not an ACP-removal signal).
assert_slice_absent() {
  local label="$1"; shift
  local slice="$1"; shift
  local features
  features="$(cargo tree -p cli "$@" -e features -i provider-host 2>/dev/null \
    | grep -oE 'provider-host feature "[a-z0-9-]+"' \
    | sed -E 's/provider-host feature "([a-z0-9-]+)"/\1/' | sort -u)"
  if grep -qx "$slice" <<<"$features"; then
    printf 'PROOF   %-22s provider-host/%s=ENABLED (UNEXPECTED)\n' "$label" "$slice"
    FAIL=$((FAIL + 1))
    FAILURES+=("$label expected slice $slice removed")
  else
    printf 'PROOF   %-22s provider-host/%s=REMOVED (ok)\n' "$label" "$slice"
    PASS=$((PASS + 1))
  fi
}

printf 'DXB-DEL-068 H11 adapter-removal matrix (mode=%s)\n' "$MODE"
printf 'repo=%s\n' "$REPO_ROOT"
printf 'toolchain=%s\n' "$(rustc --version 2>/dev/null || echo unknown)"

section "Variant 1 — normal (all production adapters, no testkit)"
run_variant "normal" provider-host
run_variant "normal" runtime-host
run_variant "normal" cli
provider_host_features "normal"
# The built CLI must carry NO synthetic canary/reference: testkit is absent.
assert_absent "normal-cli" "testkit" -e features
# Direct HTTP adapters present => reqwest present; ACP present => nix present.
assert_present "normal-cli" "reqwest"
assert_present "normal-cli" "nix"

section "Variant 2 — dsh-only (--no-default-features --features dsh-acp)"
run_variant "dsh-only" provider-host --no-default-features --features dsh-acp
run_variant "dsh-only" runtime-host --no-default-features --features dsh-acp
run_variant "dsh-only" cli --no-default-features --features dsh-acp
provider_host_features "dsh-only" --no-default-features --features dsh-acp
# Direct HTTP adapters removed => no reqwest HTTP transport in the built CLI.
assert_absent "dsh-only-cli" "reqwest" --no-default-features --features dsh-acp
assert_absent "dsh-only-cli" "testkit" --no-default-features --features dsh-acp -e features
assert_slice_absent "dsh-only-cli" "direct-deepseek" --no-default-features --features dsh-acp
assert_slice_absent "dsh-only-cli" "direct-minimax" --no-default-features --features dsh-acp
assert_slice_absent "dsh-only-cli" "http-transport" --no-default-features --features dsh-acp

section "Variant 3 — direct-minimax-only"
run_variant "minimax-only" provider-host --no-default-features --features direct-minimax
run_variant "minimax-only" runtime-host --no-default-features --features direct-minimax
run_variant "minimax-only" cli --no-default-features --features direct-minimax
provider_host_features "minimax-only" --no-default-features --features direct-minimax
# ACP slice removed => the `dsh-acp` provider-host feature is not enabled.
assert_slice_absent "minimax-only-cli" "dsh-acp" --no-default-features --features direct-minimax
assert_present "minimax-only-cli" "reqwest" --no-default-features --features direct-minimax

section "Variant 4 — direct-deepseek-only"
run_variant "deepseek-only" provider-host --no-default-features --features direct-deepseek
run_variant "deepseek-only" runtime-host --no-default-features --features direct-deepseek
run_variant "deepseek-only" cli --no-default-features --features direct-deepseek
provider_host_features "deepseek-only" --no-default-features --features direct-deepseek
assert_slice_absent "deepseek-only-cli" "dsh-acp" --no-default-features --features direct-deepseek
assert_slice_absent "deepseek-only-cli" "direct-minimax" --no-default-features --features direct-deepseek

section "Variant 5 — all-production (explicit)"
run_variant "all-production" cli --no-default-features --features "direct-deepseek direct-minimax dsh-acp"
provider_host_features "all-production" --no-default-features --features "direct-deepseek direct-minimax dsh-acp"

section "Unknown/removed factory keys fail closed"
# Run exact test names under genuinely isolated feature sets. The provider-host
# self dev-dependency has default-features=false, so these tests cannot be
# accidentally compiled with every production slice re-enabled.
run_removed_key_test() {
  local label="$1"; shift
  local features="$1"; shift
  local test_name="$1"; shift
  printf 'VARIANT %-22s crate=%-14s test=%s features="%s"\n' "$label" "provider-host" "$test_name" "$features"
  local output
  if output="$(cargo test -p provider-host --no-default-features --features "$features" \
      --lib "factory::tests::$test_name" -- --exact 2>"/tmp/dxb-matrix-$$.err")" \
      && grep -q "test factory::tests::$test_name .* ok" <<<"$output" \
      && grep -q "1 passed; 0 failed" <<<"$output"; then
    printf 'RESULT  %-22s status=PASS test=%s\n' "$label" "$test_name"
    PASS=$((PASS + 1))
  else
    printf 'RESULT  %-22s status=FAIL test=%s\n' "$label" "$test_name"
    printf '%s\n' "$output" | sed 's/^/    test> /' | head -20
    sed 's/^/    cargo> /' "/tmp/dxb-matrix-$$.err" | head -20
    FAIL=$((FAIL + 1))
    FAILURES+=("$label $test_name")
  fi
  rm -f "/tmp/dxb-matrix-$$.err"
}

run_removed_key_test "dsh-no-deepseek" "dsh-acp testkit" "removed_deepseek_key_fails_closed"
run_removed_key_test "dsh-no-minimax" "dsh-acp testkit" "removed_minimax_key_fails_closed"
run_removed_key_test "minimax-no-acp" "direct-minimax testkit" "removed_acp_key_fails_closed"

section "SUMMARY"
printf 'PASS=%d FAIL=%d\n' "$PASS" "$FAIL"
if (( FAIL > 0 )); then
  printf 'FAILURES:\n'
  for f in "${FAILURES[@]}"; do printf '  - %s\n' "$f"; done
  printf 'MATRIX=FAIL\n'
  exit 1
fi
printf 'MATRIX=PASS\n'
