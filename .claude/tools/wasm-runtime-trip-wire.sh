#!/usr/bin/env bash
# Layering trip-wires for crates/script/elidex-wasm-runtime
# (plan-memo m4-12-pr-wasm-runtime-engine-indep-completion-plan.md §4.2).
#
# Verifies the file-tier discipline holds:
#   #1  Public surface has no wasmtime:: leak (tier-E exception aside).
#   #2  elidex-js-boa has no direct wasmtime:: usage (boa migration done).
#   #3  host/ uses wasmtime:: (it MUST — it's the engine-bound internal).
#   #4  value.rs / imports.rs are wasmtime-free in code (tier A engine-
#       indep semantic — conversion glue lives in engine_conv.rs).
#       Doc-comment mentions (//!, ///) are excluded since they describe
#       the boundary rather than crossing it.
#
# Run from the workspace root.  Exits non-zero on any violation.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SRC="$ROOT/crates/script/elidex-wasm-runtime/src"
BOA="$ROOT/crates/script/elidex-js-boa/src"

# Strip grep output lines whose content (after the `path:line:` prefix
# the -n flag produces) starts with `//` — those are comments /
# docstrings and don't count for tier discipline.
strip_comments() { sed -E '/^[^:]*:[0-9]+:[[:space:]]*\/\//d'; }

red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }

fail=0

echo "trip-wire #1: public surface wasmtime:: leak"
# Allowed: error.rs documented exception (pub source: Option<wasmtime::Error>,
# pub fn source_err).  Reject anything else matching `^pub [^(].*wasmtime::`.
leak=$(grep -REn '^pub [^(].*wasmtime::' "$SRC" || true)
filtered=$(echo "$leak" | grep -v 'error\.rs:.*pub source: Option<wasmtime::Error>' \
                       | grep -v 'error\.rs:.*pub fn source_err' \
                       | grep -v '^$' || true)
if [ -n "$filtered" ]; then
  red "FAIL"; echo "$filtered"; fail=1
else
  green "OK"
fi

echo "trip-wire #2: elidex-js-boa direct wasmtime:: usage"
if [ -d "$BOA" ]; then
  hits=$(grep -rEn 'wasmtime::' "$BOA" | strip_comments || true)
  if [ -n "$hits" ]; then
    red "FAIL (boa must use engine-indep API only after Stage 10 migration)"
    echo "$hits" | head -20
    fail=1
  else
    green "OK"
  fi
else
  echo "  (elidex-js-boa not found — skipping)"
fi

echo "trip-wire #3: host/ uses wasmtime:: (must be >= 1)"
host_hits=$(grep -rEn 'wasmtime::' "$SRC/host" 2>/dev/null | strip_comments | wc -l | awk '{print $1}')
if [ "$host_hits" -lt 1 ]; then
  red "FAIL (host/ should contain wasmtime:: bindings)"
  fail=1
else
  green "OK ($host_hits hits)"
fi

echo "trip-wire #4: value.rs + imports.rs wasmtime-free (tier A)"
tier_a_hits=$(grep -En 'wasmtime::' "$SRC/value.rs" "$SRC/imports.rs" 2>/dev/null | strip_comments || true)
if [ -n "$tier_a_hits" ]; then
  red "FAIL"; echo "$tier_a_hits"; fail=1
else
  green "OK"
fi

if [ "$fail" -ne 0 ]; then
  red ""; red "elidex-wasm-runtime layering trip-wires FAILED"
  exit 1
fi
green ""; green "elidex-wasm-runtime layering trip-wires PASSED"
