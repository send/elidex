#!/usr/bin/env bash
# Layering trip-wire for the generic webref core (`.claude/tools/_webref/`
# plus the `.claude/tools/webref` entry script) — plan-memo
# 2026-07-citation-hygiene-Ai-spec-label-map.md §2 invariant K2, which that
# memo's §12(3) names as an exit criterion.
#
# `_webref/DESIGN.md` says the package "should stay generic enough to move to
# a standalone repository later" and closes with "keep new generic behavior
# free of elidex-specific file paths AND PUT ELIDEX POLICY IN ADAPTER COMMANDS
# OR DOCUMENTATION".
#
# ⚠ Those are TWO axes, and this wire reaches only the first.  Nothing
# mechanical covers the policy half — it is a judgement about wording, and the
# instrument for it is diff review.  #501 found the policy half violated in a
# commit set this wire was green on, five rounds running, so do not read a
# green here as `DESIGN.md` compliance.
#
# TWO CHECKS, BOTH ABSOLUTE.  Each is closed and decidable; neither is a
# heuristic, and this wire makes no un-asserted report.
#
# (A) THE PIN.  A-i removed exactly one host path from this tree, at two
#     sites: `_webref/cli.py` and the `webref` entry script each named
#     `.claude/skills/elidex-review/axes.md` (measured at base `44cd165d`:
#     1 each; at HEAD: 0).  This wire FAILS if it comes back.  Same shape as
#     its four siblings, which pin the re-introduction of a named deleted
#     construct rather than recognising an open category.
#
# (B) THE K2 PREDICATE, verbatim from the memo's §2: no
#     `.claude/(skills|tools)/` plus two further path segments, anywhere in
#     this tree.  NARROW — two fixed roots, a fixed segment count — so it
#     enumerates its own population and is not a seed.  Measured 0 here.
#
#     ⚠ It was asserted once and the assertion was lost to two fixes that each
#     made sense alone.  `3aaad3cb` (R55) deleted
#     `TestSliceBoundary.test_no_elidex_file_path_in_this_package` as "a second
#     copy of a scan this wire already makes over a strictly larger range";
#     `b0912a6c` (R62) then demoted that scan to report-only, voiding the
#     deletion's ground, and nobody restored it.  Between those two commits a
#     file carrying `.claude/skills/elidex-plan-review/preflight.py` passed
#     this wire GREEN.
#
# WHAT THIS WIRE DELIBERATELY DOES NOT DO.  An earlier revision also ran a
# wide `<any top-level entry>/<something>` SEED — printed, never asserted.  It
# was widened four times, each fix opening the next hole (syntactic
# over-reach; resolving-on-disk under-reach; a string-keyed exemption that
# over-suppressed; a basename-keyed one that collided), and interpolation
# `docs/${x}/y.md` was the fifth.  This repo's recorded rule is that when a
# predicate cannot return its population it is a SEED and widening the regex
# is the wrong repair.  A seed that reports and asserts nothing is also a
# print with no consumer, which `CLAUDE.md` calls dead code.  So it is GONE,
# not demoted: what covers that open class is the diff — every line entering
# this tree passes review, and `git diff origin/main...HEAD -- .claude/` is
# finite.  Two classes it could never see either way: bare top-level names
# with no separator (`"docs"`, `"crates"`, `"CLAUDE.md"` — live at
# `cli.py`'s `--paths` default and `refresh.py`'s usage string, both
# pre-existing) and interpolation.
#
# RUNTIME: shell + grep only, bash 3.2 compatible, no toolchain.  That is a
# contract, not a coincidence: `.github/workflows/ci.yml` runs this driver
# with no setup step ("the wires are grep-only") and `CLAUDE.md` rests the
# ungated-job decision on it.  An earlier revision of this wire used `python3`
# and broke that premise for all five wires (#501 R69).  Anything needing more
# than grep belongs in a test, not here.
#
# Run from anywhere.  Exits non-zero if (A) or (B) fails.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SCOPE_DIR="$ROOT/.claude/tools/_webref"
SCOPE_FILE="$ROOT/.claude/tools/webref"
for p in "$SCOPE_DIR" "$SCOPE_FILE"; do
  [ -e "$p" ] || { echo "!! $p does not exist — this wire would pass over a tree it never read" >&2; exit 2; }
done

# The host path A-i removed, spelled out, closed. Fixed string, `grep -F`.
PIN='.claude/skills/elidex-review/axes.md'
# §2's K2 predicate. Fixed ERE, `grep -E`.
K2RE='\.claude/(skills|tools)/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+'

# A filesystem walk, not `git grep`: an untracked file under the package is
# exactly where a violation lands during authoring, and `git grep` reads the
# index. (Measured on this repo: a plant in an unstaged file read GREEN.)
# `-I` skips binaries; `__pycache__` is pruned.
_files() { # $1 = dir root, $2 = extra file (may be empty)
  find "$1" -name __pycache__ -prune -o -type f -print
  [ -n "${2:-}" ] && printf '%s\n' "$2"
  return 0
}

_scan() { # $1 = dir root, $2 = extra file; prints "pin\t…" / "k2\t…" lines
  _files "$1" "${2:-}" | while IFS= read -r f; do
    grep -IFn -- "$PIN" "$f" 2>/dev/null | while IFS= read -r hit; do
      printf 'pin\t%s:%s\n' "${f#$ROOT/}" "$hit"
    done
    grep -IEno -- "$K2RE" "$f" 2>/dev/null | while IFS= read -r hit; do
      printf 'k2\t%s:%s\n' "${f#$ROOT/}" "$hit"
    done
  done
  return 0
}

# The classifier the VERDICT reads. Factored out so the controls below exercise
# the same path the exit status comes from -- #501 R69 measured the earlier
# shape and a mutation to the k2 line survived: the controls proved `_scan`,
# not the thing that decides.
_verdict() { # $1 = _scan output; sets PIN_HITS / K2_HITS
  PIN_HITS="$(printf '%s\n' "$1" | grep '^pin	' || true)"
  K2_HITS="$(printf '%s\n' "$1" | grep '^k2	' || true)"
}

# ---- CONTROLS, run BEFORE the real tree ------------------------------------
# The samples are spelled out a SECOND time on purpose. They are the
# independent subject each check is tested against, exactly as
# `layout-box-reader-trip-wire.sh`'s `ban_control` passes a hand-written
# sample line beside the pattern. If someone edits `PIN` or `K2RE` to a
# different spelling, the two stop agreeing and THAT is the signal — the
# failure a pattern-derived fixture cannot see.
CONTROL_HIT='.claude/skills/elidex-review/axes.md'
CONTROL_MISS='.claude/skills/elidex-review/workflow.md'

# Distinguish an environment failure from a dead assertion: an empty scratch
# dir would exercise nothing and silently "pass". `mktemp -d` is checked, and
# the cleanup path is the absolute one it returned (#501 R55: an unchecked
# `mktemp` made an `rm -rf` expand to the repo root).
if ! CTL="$(mktemp -d)" || [ -z "$CTL" ] || [ ! -d "$CTL" ]; then
  echo "!! could not create a scratch dir for the controls (TMPDIR/disk?)," >&2
  echo "   so this run's assertions were never proved able to fire." >&2
  exit 2
fi
trap 'case "$CTL" in /*/*) rm -rf "$CTL";; esac' EXIT

mkdir -p "$CTL/hit" "$CTL/miss"
printf 'AXES = "%s"  # planted\n' "$CONTROL_HIT"  > "$CTL/hit/control.py"
printf 'OTHER = "%s"  # planted\n' "$CONTROL_MISS" > "$CTL/miss/control.py"

ctl_hit="$(ROOT="$CTL" _scan "$CTL/hit" "" || true)"
ctl_miss="$(ROOT="$CTL" _scan "$CTL/miss" "" || true)"

_verdict "$ctl_hit"
if [ -z "$PIN_HITS" ]; then
  echo "!! POSITIVE CONTROL FAILED: a planted \`$CONTROL_HIT\` did NOT reach the pin" >&2
  echo "   verdict, so a green below would mean nothing. \$PIN is empty or misspelled," >&2
  echo "   or the classifier dropped it." >&2
  exit 1
fi
_verdict "$ctl_miss"
if [ -n "$PIN_HITS" ]; then
  echo "!! NEGATIVE CONTROL FAILED: \`$CONTROL_MISS\` reached the pin verdict, so the" >&2
  echo "   pin is matching more than the path it names." >&2
  exit 1
fi
if [ -z "$K2_HITS" ]; then
  echo "!! K2 CONTROL FAILED: a planted \`$CONTROL_MISS\` did NOT reach the K2 verdict," >&2
  echo "   so a green below would mean nothing. \$K2RE is broken, or the classifier" >&2
  echo "   dropped it." >&2
  exit 1
fi
echo "  controls: the pin fires on a planted \`$CONTROL_HIT\` and not on a sibling;"
echo "            the K2 predicate fires on \`$CONTROL_MISS\`"

# ---- THE REAL TREE ----------------------------------------------------------
scanned="$(_files "$SCOPE_DIR" "$SCOPE_FILE" | wc -l | tr -d ' ')"
if [ "$scanned" -eq 0 ]; then
  echo "!! scanned 0 files; this wire would report no violation for a reason that is not 'there are none'" >&2
  exit 2
fi
echo "  scanned $scanned file(s) under the generic core"

_verdict "$(_scan "$SCOPE_DIR" "$SCOPE_FILE" || true)"
failed=0

if [ -n "$PIN_HITS" ]; then
  echo "!! a host path A-i REMOVED is back in the generic core:"
  printf '%s\n' "$PIN_HITS" | sed 's/^pin	/     /'
  failed=1
else
  echo "  pin: the removed host path is not present -- ABSOLUTE"
fi

if [ -n "$K2_HITS" ]; then
  echo "!! K2: a \`.claude/(skills|tools)/<a>/<b>\` host path is named in the generic core:"
  printf '%s\n' "$K2_HITS" | sed 's/^k2	/     /'
  failed=1
else
  echo "  K2: 0 \`.claude/(skills|tools)/<a>/<b>\` paths named here -- ABSOLUTE"
fi

[ "$failed" -eq 0 ] || exit 1
echo "webref generic-core layering trip-wire PASSED"
