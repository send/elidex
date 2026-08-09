# The re-derivation harness's INTEGRITY MACHINERY — sourced FIRST by
# `2026-07-citation-hygiene-A-rederive.sh`, which is the only entry point.
# Not executable on its own: it defines no dispatch and sets no shell options.
# It does resolve `$REPO_ROOT` at source time (below), because every other part
# and the dispatcher itself depend on that being settled before anything runs.
#
# What lives here is THE MEASUREMENT PRIMITIVE AND NOTHING ELSE: the function that
# makes a failed measurement UNREPRESENTABLE AS A PASS (`_measure` / `_measured`)
# and the repo root every scan and every `git show` resolves against
# (`$REPO_ROOT`). Both are needed at SOURCE time by every other part, which is
# why this file is sourced first and why it is the one part with no consumers of
# its own.
#
# ⚠ The three checks that range over the harness AS A WHOLE moved to
# `-audit.sh` -- they are CONSUMERS of this file, not part of it, and keeping
# them here put it in the 700-800 authoring band. That file's header states the
# seam. Do not restate its contents here: this comment named "the two checks"
# through the commit that made them three.
#
# ⚠ `inventory` MEASURES this file's claim to be kernel and does not confirm it.
# Do not restate the answer here -- an earlier revision of this comment named A-i,
# which the umbrella-first ranking landed in the SAME commit had already made wrong.
# Run `rederive inventory` and read the rows whose `part` is `integrity`.

# THE REPO THIS HARNESS LIVES IN, derived from THIS FILE's own path -- never from
# cwd. `_wtscan`'s roots are relative (`.claude/tools/`), so before this they
# resolved against whatever directory the caller happened to be standing in: the
# dispatcher used to `cd "$(git rev-parse --show-toplevel)"`, and that `cd`
# NO-OPS when the substitution fails. Measured, with a violation planted on the
# branch: invoked with cwd `/` the `cd` printed `fatal: not a git repository`,
# did nothing, both counts came back 0 and `couplings` printed VERDICT: GREEN;
# invoked from a SIBLING worktree it audited that worktree instead of this one.
# Both are the drift `memory/feedback_worktree-cwd-drift.md` records. Failing to
# resolve the root is now fatal rather than silent.
REPO_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && git rev-parse --show-toplevel) || {
  printf 'FATAL: cannot resolve the repo root from %s\n' "${BASH_SOURCE[0]}" >&2
  printf '       -- run the harness from a checkout of the repo it lives in.\n' >&2
  exit 2
}

# --- THE MEASUREMENT PRIMITIVE ------------------------------------------------
# EVERY quantity this harness reports goes through here. The umbrella's `:91`
# constraint is *"Counts are commands. No slice memo carries a quantity it did not
# derive"*, so the one thing this harness must not be able to print is a count
# whose command NEVER RAN. The idiom it replaces -- `n=$(cmd | wc -l)` -- could
# not tell that apart from a real zero on EITHER axis:
#
#   STATUS  `$(...)` discards the pipeline's status and `wc -l` of nothing is
#           `0`, which is the PASS condition at every call site here. Measured,
#           in a checkout with no remote-tracking ref: `git grep … origin/main`
#           died `fatal: unable to resolve revision: origin/main`, the count read
#           `0`, and `couplings` printed `VERDICT: GREEN` and exited 0 -- it
#           certified a derivation that did not occur, which is the negation of
#           this harness's charter. `37c7eb02` fixed exactly this for `_wtscan`
#           and left the two `git grep` baselines on the adjacent lines.
#   EMPTY   `printf '%s\n' "$var" | wc -l` is **1** for an empty `$var`, because
#           printf emits the newline unconditionally. A match count that reads 1
#           when there are no matches is the same class from the other side. The
#           `if [ -n … ]` guards that used to hold this off are not needed here:
#           the count is taken from the command's own output, not from a re-print.
#
#   _measure [--nomatch <status>] <var> <cmd> [arg...]
#
# runs <cmd> ONCE, and:
#   * captures its stdout in `$_MEASURE_OUT`, so a caller that must also PRINT
#     the hits reads that instead of running the command a second time (running
#     it twice is how the pre-`37c7eb02` sites threw the status away twice, and
#     it lets the listing and the count disagree),
#   * on success sets <var> to `wc -l` of that output -- 0 for no output,
#   * on failure sets <var> to `!FAILED(rc=N)`, prints a loud diagnostic, and
#     returns 1.
#
# The sentinel is what makes "the command did not run" UNREPRESENTABLE AS A PASS
# rather than merely detected at the sites someone remembered: every gate in this
# harness is `= 0`, `!FAILED(rc=N)` is not `0` and is not a number, so no failed
# measurement can satisfy any of them -- including at a call site written later by
# someone who has not read this comment. Callers should still propagate the
# return status so the block's exit code carries it too; a caller that forgets
# still cannot print GREEN.
#
# `--nomatch <status>` is the status a SEARCH returns when it ran and matched
# nothing (`git grep` -> 1). It is REQUIRED on the baselines and forbidden
# elsewhere: without it "no matches" -- the expected, correct result -- would be
# indistinguishable from a broken ref, which is the confusion this function
# exists to end. Every OTHER status stays a failure, so git's 128 for an
# unresolvable revision and 127 for a missing interpreter are still fatal.
_MEASURE_OUT=""
_measure() {
  local __nomatch=""
  [ "${1:-}" = "--nomatch" ] && { __nomatch=$2; shift 2; }
  local __var=$1; shift
  local __raw __rc=0
  # The `\034` sentinel is not decoration: `$(...)` strips ALL trailing newlines,
  # so output ending in a blank line would count one line short. Appending a
  # non-newline byte and stripping it back makes the count byte-identical to
  # `cmd | wc -l`.
  __raw=$( { "$@"; __r=$?; printf '\034'; exit "$__r"; } ) || __rc=$?
  __raw=${__raw%$'\034'}
  if [ "$__rc" -ne 0 ] && [ "$__rc" != "$__nomatch" ]; then
    _MEASURE_OUT=""
    printf '!! MEASUREMENT FAILED (rc=%s): %s\n' "$__rc" "$*" >&2
    printf '!!   no count is reported -- a command that did not run measured nothing.\n' >&2
    printf -v "$__var" '!FAILED(rc=%s)' "$__rc"
    return 1
  fi
  _MEASURE_OUT=$__raw
  printf -v "$__var" '%s' "$(printf '%s' "$__raw" | wc -l | tr -d ' ')"
  return 0
}

# Print what the last `_measure` captured, without the extra blank line a
# `printf '%s\n'` on an already-newline-terminated capture would add.
_measured() { [ -n "$_MEASURE_OUT" ] && printf '%s' "$_MEASURE_OUT"; return 0; }

