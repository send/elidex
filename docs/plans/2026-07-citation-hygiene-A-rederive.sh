#!/usr/bin/env bash
# Re-derivation harness for the 2026-07 citation-hygiene slices (A-i / A-ii /
# A-iii / B).
#
# The memos carry no measured digits of their own: every quantity they rely on is
# printed by a function here, and the memo cites the function name. Run one:
#
#     bash docs/plans/2026-07-citation-hygiene-A-rederive.sh <name>
#     bash docs/plans/2026-07-citation-hygiene-A-rederive.sh all
#
# Rationale: five review rounds produced a stale-or-underived-coordinate finding
# four times. Prose descriptions of executable things rot and self-contradict;
# an executable does neither. This harness is also where the §6 fixture bodies
# live, so the fixtures a reviewer measures are byte-identical to the ones A ships.
#
# THIS FILE IS THE DISPATCHER AND THE ONLY ENTRY POINT. The blocks live in the
# sourced parts below, one per slice plus a common part and an integrity part;
# six memos cite block names by THIS path, so the invocation surface above is
# fixed and every block name resolves here regardless of which part defines it.
#
# The slice seam, MEASURED (`grep -nE 'rederive [a-z ]*<block>' …-citation-hygiene-*.md`,
# plus each memo's §15 block list, which is the authoritative enumeration):
#
#   A-i   §15: citations keysets readers regions couplings budget
#   A-ii  §15: citations column carvecolumn instruments remedies reloadstale
#              armmatrix budget couplings marker lanes
#   A-iii §15: suites filters suiteset ruleset budget couplings lanes
#   umbrella : suites, lanes
#
# Cited by more than one memo -> `-common.sh`: citations, couplings, budget, lanes.
# Cited by exactly one -> that slice's file. Uncited blocks are routed by the
# quantity they derive, not by guess:
#   partition, offline -> B  (B §4.1.2/§4.1.8 embed these two scripts verbatim:
#                             the 203/948 round-trip census and the SystemExit escape)
#   anchors            -> A-ii (its 7 preflight symbols: A-ii 26 hits, A-i 1)
#   timing             -> A-ii (the CLI-subprocess axis §4.2.1/§4.5 instrument)
#   bmemo, staleclaims -> B  (they derive the edit classes B's memo needs)
#   selfcheck          -> integrity (it derives a property of the harness AS A
#                                 WHOLE -- that every block below states its own
#                                 exit status -- so it belongs to no slice)
# Helpers are placed with their callers: `_runner` (4 A-ii blocks) -> A-ii;
# `fixtures` and `_proto` have callers in two files -> common.
#
# `-integrity.sh` is the ONE PART THAT IS NOT ON THE SLICE SEAM, and it is
# sourced FIRST because everything else reads it: `$REPO_ROOT`, settled at SOURCE
# time and required by the `cd` below, plus `_measure`/`_measured` and
# `selfcheck`. Call sites, MEASURED (`grep -cE '(^|[^_A-Za-z])_measure(d)? '`):
# `-common` 15, `-Aiii` 7, `-Aii` 1, `-B` 1 -- every part but `-Ai`, which has
# none. A-i §8 names this seam and is the only site carrying the layout figures.
set -uo pipefail
_HARNESS_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
for _part in integrity common Ai Aii Aiii B; do
  # shellcheck source=/dev/null
  . "$_HARNESS_DIR/2026-07-citation-hygiene-A-rederive-$_part.sh"
done
unset _part
# `-integrity.sh` resolves $REPO_ROOT from ITS OWN path and exits 2 if it cannot, so
# by here it is settled. Blocks still run with cwd at the root -- the `git grep`
# baselines and the `git ls-files` census read it -- but the root is now the repo
# THIS SCRIPT lives in rather than wherever the caller stood, and a failure is
# loud. The old line was `cd "$(git rev-parse --show-toplevel)"`, which NO-OPS
# when the substitution fails; see `-integrity.sh`'s measurements.
cd "$REPO_ROOT" || { printf 'FATAL: cannot cd to %s\n' "$REPO_ROOT" >&2; exit 2; }

# `all` ROLLS UP AND PROPAGATES. A block's verdict line is one line in 300+, and
# `couplings` is §12(3)'s exit criterion: a reader scrolling `all` has no reason
# to find a RED buried mid-stream, and a caller had no status to check at all. So
# the run ends with an anchored roster and exits non-zero if any block failed.
#
# THE ROSTER NAMES BLOCKS BY POSITIONAL PARAMETER, and that is load-bearing. The
# roll-up shipped as `for f in …; do "$f" || failed="$failed $f(exit $?)"; done`,
# which reads `$f` AFTER the block has run -- and the shell has no lexical
# scoping, so a block that assigns a plain `f` renames its own roster entry.
# MEASURED, in a checkout where `origin/main` does not resolve (which is what
# makes enough blocks fail to see it): `budget`'s
# `for f in docs/plans/…-A-rederive*.sh` left the glob's LAST match, so it
# reported itself as `docs/plans/2026-07-citation-hygiene-A-rederive.sh(exit 1)`
# -- the dispatcher's own path, not a block -- and `couplings`'s
# `while IFS= read -r f` left `f` empty at EOF, so it reported as `(exit 1)`.
# Two of eight entries named nothing a reader could run, in the roster whose
# whole job is to name what to run. A function's positional parameters are its
# own: a callee cannot reach `$1`, so the name in the roster is the name that was
# dispatched, by construction rather than by every block's good behaviour. Same
# rule as `_measure`: make the wrong report unrepresentable.
#
# `selfcheck` runs FIRST and reads this very roster: it is the check that no block
# named below hands back an accidental exit status, and `all` is the only place
# the block set is written down, so deriving its scope from `$@` here is what
# keeps the check from needing a second list to drift against.
all() { set -- selfcheck citations partition keysets column carvecolumn instruments remedies \
                reloadstale armmatrix suites anchors regions offline couplings suiteset marker \
                budget filters ruleset timing bmemo
        local failed="" rc
        while [ "$#" -gt 0 ]; do
          say "$1"; "$1"; rc=$?
          [ "$rc" -eq 0 ] || failed="$failed $1(exit $rc)"
          shift; done
        printf '\n(author-local, excluded from `all`: %s)\n' "$AUTHOR_LOCAL"
        if [ -n "$failed" ]; then printf 'FAILED BLOCKS:%s\n' "$failed"; return 1; fi
        printf 'ALL BLOCKS EXITED 0\n'; }

"${1:-all}" "$@"
