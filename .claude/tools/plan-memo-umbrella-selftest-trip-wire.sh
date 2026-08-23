#!/usr/bin/env bash
# Trip-wire: the plan-memo umbrella checker's self-test and mutation proof
# (umbrella plan docs/plans/2026-08-plan-memo-umbrella-checker.md §1 "Interim
# connection on main").
#
# Runs `plan-memo-umbrella-check.py --self-test --mutants`: every control in
# `plan_memo_selftest_cases.py` (POSITIVE / POSITIVE-NOVEL / NEGATIVE /
# KNOWN-MISS) against the checker's ONE pipeline (`check()`), then every row of
# `plan_memo_selftest_mutants.py` -- a source edit that removes one lexing
# clause or gating stage, exec'd into a fresh module set, whose named control
# must turn red.  A mutant whose substring no longer applies is a FAIL, as is
# one that survives.
#
# Memo-INDEPENDENT: the fixtures are built in `tempfile` directories, so this
# wire does not read any plan memo and cannot red a PR that edits one.  The
# memo run itself (`plan-memo-umbrella-check.py <memo>`) is NOT a trip-wire:
# `SCHEMAS` matches one document family's header rows and exits 2 on any other
# memo; it is invoked by hand when that memo is edited (see CLAUDE.md
# Development Rules).
#
# ⚠ Unlike the other wires this one needs `python3` (3.9+; stdlib only).  An
# absent interpreter is a FAIL, not a skip -- a gate that silently covers less
# than the docs claim is the failure `scripts/trip-wires.sh` exists to catch.
#
# Run from anywhere.  Exits non-zero on any failing control or surviving mutant.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CHECKER="$ROOT/.claude/tools/plan-memo-umbrella-check.py"

red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }

if ! command -v python3 >/dev/null 2>&1; then
  red "FAIL: python3 not found -- the plan-memo checker self-test did not run."
  red "      This wire needs an interpreter; install python3 rather than skipping it."
  exit 1
fi

echo "trip-wire: plan-memo-umbrella-check --self-test --mutants"
if out="$(python3 "$CHECKER" --self-test --mutants 2>&1)"; then
  # the summary lines only; the per-control listing is for a failing run
  printf '%s\n' "$out" | grep -E 'control\(s\):|mutant\(s\),|all controls' || true
  green "OK (every control behaved as declared; every mutant was killed)"
  exit 0
fi
printf '%s\n' "$out"
red "FAIL: plan-memo-umbrella-check self-test -- see the FAIL lines above"
exit 1
