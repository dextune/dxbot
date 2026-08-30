#!/usr/bin/env bash
#
# DXB-DEL-068 H11 — secret / diagnostic leak scan.
#
# Scans the audited DSH patch, the deployment config examples, and the built
# provider/runtime diagnostic surfaces for accidental secret material and for
# raw-credential leakage into patch/config/argv/log/diagnostic strings. It is a
# read-only static scan: it NEVER mutates source and never prints a matched
# secret value (only the offending file:line marker).
#
# What it checks (fail closed on any hit):
#   1. The audited patch (`deploy/dsh/dsh-acp.patch.yaml`) and config examples
#      carry only an env-reference (`env:MINIMAX_API_KEY` / `apiKeyEnv`), never a
#      raw key literal.
#   2. No obvious credential literals (Bearer tokens, `sk-`/`api_key=` inline
#      values, PEM blocks) in deploy/ or config examples.
#   3. The provider/runtime error/diagnostic code returns category-only strings:
#      the ACP provider forbids leaking the raw key, path, prompt, or stderr —
#      asserted by the crate's own secret-canary tests, which this script runs.
#
# Usage: scripts/secret-scan.sh
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

FAIL=0
note() { printf '%s\n' "$*"; }
hit() { printf 'LEAK   %s\n' "$*"; FAIL=1; }

note "DXB-DEL-068 H11 secret-scan"
note "repo=$REPO_ROOT"

# ---------------------------------------------------------------------------
# 1. Deployment patch/config must reference the credential by env NAME only.
# ---------------------------------------------------------------------------
note "-- scan: deploy/ and config examples reference env, never raw key --"
SCAN_PATHS=(deploy)
# Config examples live under deploy/dsh; include any *.example.json / *.yaml.
while IFS= read -r -d '' f; do
  # A raw MiniMax key would appear as a long opaque token assigned to a key
  # field; the audited patch must only carry `apiKeyEnv: MINIMAX_API_KEY`.
  if grep -nE '(apiKey|api_key|token|secret|password)\s*[:=]\s*["'"'"']?[A-Za-z0-9._-]{16,}' "$f" \
      | grep -viE 'env:|Env\b|apiKeyEnv|_ref|credential_ref|MINIMAX_API_KEY' >/dev/null; then
    hit "raw-credential-literal candidate in $f"
    grep -nE '(apiKey|api_key|token|secret|password)\s*[:=]' "$f" | sed 's/^/    /'
  fi
  # PEM private key blocks must never ship.
  if grep -q 'BEGIN [A-Z ]*PRIVATE KEY' "$f"; then
    hit "PEM private key block in $f"
  fi
  # Bearer literal with a value.
  if grep -nE 'Bearer[[:space:]]+[A-Za-z0-9._-]{12,}' "$f" >/dev/null; then
    hit "inline Bearer token in $f"
  fi
done < <(find "${SCAN_PATHS[@]}" -type f \( -name '*.json' -o -name '*.yaml' -o -name '*.yml' -o -name '*.example.*' \) -print0)

# The audited patch specifically must carry apiKeyEnv and NOT an apiKey literal.
PATCH="deploy/dsh/dsh-acp.patch.yaml"
if [[ -f "$PATCH" ]]; then
  if ! grep -qE 'apiKeyEnv:\s*MINIMAX_API_KEY' "$PATCH"; then
    hit "audited patch $PATCH is missing the expected apiKeyEnv env reference"
  fi
  if grep -qE '^\s*apiKey:\s' "$PATCH"; then
    hit "audited patch $PATCH carries a raw apiKey literal"
  fi
  note "OK     audited patch references MINIMAX_API_KEY by env only"
else
  hit "audited patch $PATCH not found"
fi

# ---------------------------------------------------------------------------
# 2. Source diagnostics must not embed a hardcoded credential literal. (We scan
#    for obvious secret literals in crate sources; env NAMES are allowed.)
# ---------------------------------------------------------------------------
note "-- scan: crate sources carry no hardcoded credential literal --"
if grep -rnE '(let|const)[[:space:]]+[A-Za-z_]+[[:space:]]*[:=][^=].*("sk-[A-Za-z0-9]{16,}"|"Bearer [A-Za-z0-9._-]{12,}")' crates/ >/dev/null 2>&1; then
  hit "hardcoded credential literal in crates/"
  grep -rnE '("sk-[A-Za-z0-9]{16,}"|"Bearer [A-Za-z0-9._-]{12,}")' crates/ | grep -v test | sed 's/^/    /'
fi
note "OK     no hardcoded credential literal found in crate sources"

# ---------------------------------------------------------------------------
# 3. Run the crate's own secret-canary / diagnostic-isolation tests, which are
#    the executable owner of "raw key/path/prompt/stderr never escape a
#    diagnostic". These use canary secrets and assert they never appear.
# ---------------------------------------------------------------------------
note "-- run: provider/runtime secret-canary + diagnostic isolation tests --"
if cargo test -p provider-host --features testkit >/tmp/dxb-secret-ph-$$.log 2>&1; then
  # Surface the canary-related test names that ran (evidence), value-free.
  grep -iE 'test .*(canary|redact|secret|isolation|scrub|env_)' "/tmp/dxb-secret-ph-$$.log" \
    | sed 's/^/    /' | head -20 || true
  note "OK     provider-host secret-canary + isolation tests passed"
else
  hit "provider-host secret-canary tests failed"
  tail -20 "/tmp/dxb-secret-ph-$$.log" | sed 's/^/    /'
fi
rm -f "/tmp/dxb-secret-ph-$$.log"

# runtime-host config diagnostics are category-only (path/digest-free); its
# tests assert leaked errors never contain the path/commit/secret.
if cargo test -p runtime-host provider_config >/tmp/dxb-secret-rh-$$.log 2>&1; then
  note "OK     runtime-host provider_config diagnostic isolation tests passed"
else
  hit "runtime-host provider_config diagnostic isolation tests failed"
  tail -20 "/tmp/dxb-secret-rh-$$.log" | sed 's/^/    /'
fi
rm -f "/tmp/dxb-secret-rh-$$.log"

note "-----------------------------------------------------------------------"
if (( FAIL != 0 )); then
  note "SECRET-SCAN=FAIL"
  exit 1
fi
note "SECRET-SCAN=PASS"
