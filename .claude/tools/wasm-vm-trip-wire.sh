#!/usr/bin/env bash
# Layering trip-wires for crates/script/elidex-js (VM-side WebAssembly host)
# (plan-memo m4-12-pr-d16-wasm-vm-plan.md §4.3).
#
# Verifies the engine-bridge boundary holds from the VM side:
#   #1  vm/host/wasm/*.rs (8 host files) hold zero wasmtime:: tokens —
#       all wasm execution goes through the elidex-wasm-runtime engine-
#       indep surface (WasmRuntime / WasmModule / WasmInstance / ...).
#   #2  vm/wasm_payload.rs holds zero wasmtime:: tokens — payload structs
#       only carry engine-indep handles.
#   #3  Crate-wide: anywhere in crates/script/elidex-js/src/ that any
#       wasmtime:: token appears is a layering violation (no allow-list
#       — the VM crate has no engine-bound concerns).  Mirrors trip-wire
#       #2 of wasm-runtime-trip-wire.sh on the boa side.
#
# Doc-comment mentions (//, //!, ///) describing the boundary are
# excluded since they don't cross it (same idiom as the runtime
# trip-wire).
#
# Run from the workspace root.  Exits non-zero on any violation.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VM="$ROOT/crates/script/elidex-js/src/vm"

# Strip grep output lines whose content (after the `path:line:` prefix
# the -n flag produces) starts with `//` — those are comments /
# docstrings and don't count for tier discipline.
strip_comments() { sed -E '/^[^:]*:[0-9]+:[[:space:]]*\/\//d'; }

red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }

fail=0

echo "trip-wire #1: vm/host/wasm/*.rs wasmtime:: usage"
host_hits=$(grep -rEn 'wasmtime::' "$VM/host/wasm" 2>/dev/null | strip_comments || true)
if [ -n "$host_hits" ]; then
  red "FAIL (vm/host/wasm/ must consume engine-indep API only)"
  echo "$host_hits"
  fail=1
else
  green "OK"
fi

echo "trip-wire #2: vm/wasm_payload.rs wasmtime:: usage"
# -H forces filename prefix even with a single-file argument so
# strip_comments (`path:line:content`) lines up.
payload_hits=$(grep -HEn 'wasmtime::' "$VM/wasm_payload.rs" 2>/dev/null | strip_comments || true)
if [ -n "$payload_hits" ]; then
  red "FAIL (payload structs must hold engine-indep handles only)"
  echo "$payload_hits"
  fail=1
else
  green "OK"
fi

echo "trip-wire #3: crate-wide elidex-js wasmtime:: usage"
crate_hits=$(grep -rEn 'wasmtime::' "$ROOT/crates/script/elidex-js/src" 2>/dev/null | strip_comments || true)
if [ -n "$crate_hits" ]; then
  red "FAIL (VM crate has no engine-bound concerns — no allow-list)"
  echo "$crate_hits"
  fail=1
else
  green "OK"
fi

if [ "$fail" -ne 0 ]; then
  red ""; red "elidex-js VM-side wasm layering trip-wires FAILED"
  exit 1
fi
green ""; green "elidex-js VM-side wasm layering trip-wires PASSED"
