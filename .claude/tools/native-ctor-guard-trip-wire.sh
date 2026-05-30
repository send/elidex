#!/usr/bin/env bash
# Layering trip-wire for crates/script/elidex-js (native ctor `new`-only
# discipline) — plan-memo m4-12-pr-vm-native-constructor-only-flag-plan.md
# §6 trip-wire bullet (Stage 3c deliverable).
#
# Post-PR the 66 per-ctor `if !ctx.is_construct() { ... }` guards (and the
# 27 callers of the now-deleted `vm/host/events.rs::check_construct`
# helper) collapse to a single dispatch-side gate at
# `vm/interpreter.rs::call_dispatch` driven by the
# `CallShape::ConstructorOnly` discriminant.  This wire catches any
# regression that re-introduces either historic form:
#
#   #1  `if !ctx.is_construct` literal-guard reintroduction (whole-crate,
#       NOT vm/host/-scoped — Promise lives at vm/natives_promise.rs in
#       core VM per §5 F23 IMP 2026-05-30).
#   #2  `check_construct(ctx` helper call regression (helper itself was
#       deleted in Stage 3b).
#
# Doc-comment mentions (//, ///, //!) describing the historic pattern in
# `vm/interpreter.rs:318` (dispatch-gate doc), `vm/shape_ops.rs:291`
# (`create_constructor_only_function` doc), and `vm/host/wasm/module.rs:47`
# (D-16 site narrative) are excluded — they describe what the gate
# replaced, they don't re-introduce it.
#
# Run from the workspace root.  Exits non-zero on any violation.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SRC="$ROOT/crates/script/elidex-js/src"

# Strip grep output lines whose content (after the `path:line:` prefix
# the -n flag produces) starts with `//` — those are comments /
# docstrings and don't count as a guard re-introduction.  Identical
# idiom to the sibling wasm-runtime / wasm-vm trip-wires.
strip_comments() { sed -E '/^[^:]*:[0-9]+:[[:space:]]*\/\//d'; }

red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }

fail=0

echo "trip-wire #1: per-ctor 'if !ctx.is_construct' literal-guard reintroduction"
literal_hits=$(grep -rEn 'if !ctx\.is_construct' "$SRC" 2>/dev/null | strip_comments || true)
if [ -n "$literal_hits" ]; then
  red "FAIL (per-ctor literal guard returned — should be unified at vm/interpreter.rs::call_dispatch via CallShape::ConstructorOnly)"
  echo "$literal_hits"
  fail=1
else
  green "OK"
fi

echo "trip-wire #2: 'check_construct(ctx' helper re-introduction"
helper_hits=$(grep -rEn 'check_construct\(ctx' "$SRC" 2>/dev/null | strip_comments || true)
if [ -n "$helper_hits" ]; then
  red "FAIL (check_construct helper re-introduced — was deleted in Stage 3b; route through CallShape::ConstructorOnly instead)"
  echo "$helper_hits"
  fail=1
else
  green "OK"
fi

if [ "$fail" -ne 0 ]; then
  red ""; red "elidex-js native-ctor-guard discipline trip-wires FAILED"
  exit 1
fi
green ""; green "✓ all clear"
