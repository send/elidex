#!/usr/bin/env bash
# Re-derivation harness for the 2026-07 citation-hygiene program, as carried by
# slice A-iii (`2026-07-citation-hygiene-Aiii-suite-scheduler.md`).
#
# The memo carries no measured digits of its own: every quantity it relies on is
# printed by a function here, and the memo cites the function name. Run one:
#
#     bash docs/plans/2026-07-citation-hygiene-A-rederive.sh <name>
#     bash docs/plans/2026-07-citation-hygiene-A-rederive.sh all
#
# Rationale: five review rounds produced a stale-or-underived-coordinate finding
# four times. Prose descriptions of executable things rot and self-contradict;
# an executable does neither.
#
# THIS FILE IS THE DISPATCHER AND THE ONLY ENTRY POINT. A-iii lands BEFORE A-ii
# (A-iii §0), so this branch carries only the parts A-iii's blocks need:
#   -integrity  `$REPO_ROOT`, `_measure`/`_measured`, `selfcheck` -- the harness
#               machinery, belonging to no slice; sourced FIRST because every
#               other part reads it
#   -common     `suites`, `budget`, `lanes` -- blocks other memos cite too
#   -Aiii       `suiteset`, `filters`, `ruleset`, `touchset`
# The A-i / A-ii / B parts and their blocks return when PR #514 is reworked onto
# this harness after PR #501 lands; the part list below is the one place that
# grows, and `selfcheck` compares it against the parts on disk.
set -uo pipefail
_HARNESS_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
for _part in integrity common Aiii; do
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
# blocks are cited as exit criteria: a reader scrolling `all` has no reason
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
# THE BLOCK ROSTER, written once. `all` dispatches it positionally (below) and
# the entry point admits a name only from this list (+ `$AUTHOR_LOCAL`, which
# are blocks excluded from `all` by design, and `all` itself): a sourced helper
# such as `say`, `fixtures`, or `_measured` is a declared function too, and
# `declare -F` alone ran it and handed back a silent exit 0 as "the
# re-derivation" (Codex R32, measured: `… _measured` exited 0 with no output).
BLOCKS="selfcheck suites suiteset filters ruleset budget touchset"
all() { # shellcheck disable=SC2086  # word-splitting the roster IS the positional dispatch
        set -- $BLOCKS
        local failed="" rc
        while [ "$#" -gt 0 ]; do
          say "$1"; "$1"; rc=$?
          [ "$rc" -eq 0 ] || failed="$failed $1(exit $rc)"
          shift; done
        printf '\n(author-local, excluded from `all`: %s)\n' "$AUTHOR_LOCAL"
        if [ -n "$failed" ]; then printf 'FAILED BLOCKS:%s\n' "$failed"; return 1; fi
        printf 'ALL BLOCKS EXITED 0\n'; }

# Only a ROSTERED block is dispatchable: a shell builtin, an executable, a typo
# (Codex R26) or a sourced helper (Codex R32) would otherwise run and hand back
# its own status as "the re-derivation". The admitted set is `$BLOCKS` +
# `$AUTHOR_LOCAL` + `all`, and the usage line prints that same set.
_block=${1:-all}
case " $BLOCKS $AUTHOR_LOCAL all " in
  *" $_block "*) declare -F -- "$_block" >/dev/null || { printf 'rostered block not defined: %s\n' "$_block" >&2; exit 2; } ;;
  *) printf 'unknown block: %s\n' "$_block" >&2
     printf 'blocks: %s all\n' "$(printf '%s %s' "$BLOCKS" "$AUTHOR_LOCAL" | tr -s ' \n' ' ')" >&2
     exit 2 ;;
esac
"$_block" "$@"
