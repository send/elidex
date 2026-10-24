#!/usr/bin/env bash
# THE CONTROL HARNESS for `webref-generic-core-trip-wire.sh` — sourced by that
# wire's controls file, never run on its own.
#
# WHY IT IS A SEPARATE FILE: CLAUDE.md's touch-time split, taken when the
# controls file was about to pass the line threshold. The seam is HOW a control
# runs versus WHICH controls exist: this file holds the scratch root the
# fixtures live in, the git helper that builds them, the shim-quoting helper,
# the FIFO capability probe and `_control` itself; the controls file holds each
# fixture and the assertion made over it.
# ⚠ SOURCED ONLY AFTER THE CONTROLS FILE HAS ASSERTED WHAT THE WIRE HANDS IT
# (`$SELF`, `$SCRATCH`, `_git`, …), which is what this file consumes. It sets
# `_HARNESS` immediately before sourcing, so an unset one means this file was
# reached some other way.
if [ -z "${_HARNESS:-}" ]; then
  echo "!! This file is the control HARNESS for \`webref-generic-core-trip-wire.sh\`. It is" >&2
  echo "   SOURCED by that wire's controls file and has no meaning alone." >&2
  echo "   Run the wire instead." >&2
  exit 2
fi
#
# ⚠ THE FIXTURES ARE BUILT WITH GIT, so whoever runs this must not be able to
# change what they contain (#501 R92). A global `core.excludesFile` of `*.py`
# made `git add -A` skip the fixtures' own files, and an `init.templateDir`
# could seed `info/exclude`. `_fgit` is what the fixtures are built with; the
# channels it closes are named at its definition below.
# ⚠ PER CALL, NOT `export` (#501 R94). Exported, it applied to the REAL scan
# too and took `safe.directory` with it — measured: a checkout owned by
# another UID, readable only because of a global `safe.directory` entry, went
# from working to "detected dubious ownership", so the required gate could not
# run in an otherwise valid environment. The fixtures need a clean config; the
# repository needs the user's. The inventory is made machine-independent by
# `--exclude-per-directory` instead (see `_scan`), which is why nothing has to
# be stripped for the real read.
# ⚠ AND IT SCRUBS THE ENVIRONMENT-PROVIDED CONFIG TOO, which `_git` deliberately
# does NOT. `_git` keeps `GIT_CONFIG*` because clearing it broke a checkout that
# was only readable through a caller's `safe.directory` (#501 R97) — right for
# the REAL scan, wrong for the fixtures, which must be built from a known
# configuration whatever the caller carries. Reproduced by the external
# reviewer and here: invoking the gate with
# `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.excludesFile
#  GIT_CONFIG_VALUE_0=<file containing *.py>` made the fixtures' `git add` skip
# their own `.py` inputs and produced **ten** `CONTROL NOT EXERCISED` failures
# on a clean checkout. `GIT_CONFIG_GLOBAL`/`_SYSTEM` alone do not reach that
# channel. The asymmetry is the point and it is why there are two helpers:
# PRESERVE for the repository, SCRUB for the fixtures.
# ⚠ In a subshell (`( … )`), so the unset cannot leak into the real scan.
_fgit() ( unset GIT_CONFIG GIT_CONFIG_PARAMETERS GIT_CONFIG_COUNT
          GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null _git "$@" )

# ONE FACT, RECORDED ONCE: DID THIS FIXTURE'S BUILD CHAIN SUCCEED?
# Every fixture below is built as `( cd … && … ) || _fixture_failed <name>`, and
# `_control` refuses to report on a fixture whose chain did not.
# ⚠ THIS REPLACES FIVE HAND-WRITTEN PRECONDITIONS, and the reason to prefer it
# is not brevity. Each fixture is built under `|| true`, so a step that fails
# leaves a TREE THAT LOOKS PLAUSIBLE — and for some fixtures that tree still
# satisfies the control's own assertion, which then passes having tested
# nothing. Five such sites were found one at a time, each fixed with a bespoke
# probe re-deriving the same fact from the fixture's end state (was the blob
# tracked? is there a replace ref? did `info/exclude` get written? is it a repo
# at all?), and the comment introducing the first two called the class closed at
# two. It was not: `cachedir`, `empty` and `lstreefail` followed, two of them
# added by the commit that wrote the sentence. The fact those probes
# reconstruct is available for free at the point of failure, for ALL of them —
# so the population is every fixture, not the five somebody noticed.
_FIX_FAILED=""
_fixture_failed() { _FIX_FAILED="$_FIX_FAILED $1"; }
# Distinguish an environment failure from a dead assertion: an empty scratch
# dir would exercise nothing and silently "pass". `mktemp -d` is checked, and
# the cleanup path is the physical absolute one the wire resolves it to (#501
# R55: an unchecked `mktemp` made an `rm -rf` expand to the repo root).
if ! CTL="$(mktemp -d "$SCRATCH/ctlXXXXXX")" || [ -z "$CTL" ] || [ ! -d "$CTL" ]; then
  echo "!! could not create a scratch dir for the controls (TMPDIR/disk?)," >&2
  echo "   so this run's assertions were never proved able to fire." >&2
  exit 2
fi
# CAN THIS FILESYSTEM HOLD A FIFO? Two controls need one — `odd` (an entry git
# cannot store) and `fifotracked` (a tracked path replaced by one) — and both
# used to create theirs under `|| true`, so on a filesystem without them each
# fell back to a state its own assertion could not distinguish: `odd` printed a
# note claiming "every other control ran" (false in exactly the runs where
# `fifotracked` had just failed for the same missing capability), and
# `fifotracked` reported a WIRE failure for a MACHINE limitation. Asked once,
# answered once, reported once — the shape `$_perm_line` already uses for the
# other capability this harness cannot assume.
_fifo_ok=0
if mkfifo "$CTL/.fifoprobe" 2>/dev/null; then _fifo_ok=1; command rm -f "$CTL/.fifoprobe"; fi

# ⚠ NO SECOND `trap ... EXIT` HERE. `trap` REPLACES; a second one silently
# discarded the scratch-root cleanup and left an empty directory behind on
# every successful run (#501 R94, reproduced). `CTL` is created UNDER
# `$SCRATCH`, so the trap at the top already removes it — one owner, one
# cleanup, nothing to compose.

# ⚠ A PATH EMBEDDED IN A GENERATED SCRIPT IS QUOTED, BY ONE HELPER. A shim that
# embeds a path — a real tool's, or a fixture's — writes it into `/bin/sh`
# source; spliced in bare, a path
# holding a space or a shell metacharacter split into words and every shim went
# invalid, so the controls using them exited for the wrong reason (PR519,
# reproduced by the external reviewer with git at `/tmp/tool space/git`). Same
# lesson as #501 R96 on `_ctl_env`: a path is data. Single quotes are the one
# `/bin/sh` quoting with no expansion inside; an embedded `'` is closed, escaped
# and reopened.
_shq() { printf "'%s'" "$(printf '%s' "$1" | sed "s/'/'\\\\''/g")"; }
_REAL_GIT="$(_shq "$(command -v git)")"
_REAL_GREP="$(_shq "$(command -v grep)")"
# ⚠ ASSERTED, NOT A `_control`: this is the harness's own part, not an arm of the
# wire, so the mutation set (which edits the wire) has nothing to aim at. It is
# checked by round-tripping a path that holds each thing that broke it.
_shq_probe="/tool dir/it's \$HOME \`x\`"
if [ "$(sh -c "printf %s $(_shq "$_shq_probe")")" != "$_shq_probe" ]; then
  echo "!! the shim-quoting helper does not round-trip a path through /bin/sh, so every" >&2
  echo "   shim built with it may exec the wrong thing. This run decided nothing." >&2
  exit 2
fi

_control() { # $1 = root, $2 = expected exit, $3 = expected message, $4 = label,
             # $5 = optional scope subdir (relative), $6 = optional extra entry (relative),
             # $7 = optional PATH prefix (to shadow a tool the wire calls),
             # and `_ctl_env` (an ARRAY, set by the caller) for environment
             # assignments. ⚠ NOT a string: `env ${8:-}` word-split them, so a
             # `TMPDIR` holding a space broke the routed control with
             # `env: 'dir/.../.git': No such file or directory` and the whole
             # gate exited 1 for a contributor whose temp path has a space
             # (#501 R96). A path is data here too.
  # ⚠ WITH A WATCHDOG, because a hang is a verdict this harness could not
  # otherwise report (#501 R92). The FIFO finding's whole harm was that the
  # local gate BLOCKED instead of failing closed — and a control for it, run
  # plainly, would block too: it would never return to say so. Reverting that
  # fix now produces a reported control failure rather than a wire that waits
  # forever. Backgrounded and `wait`ed rather than polled, so a healthy
  # control costs nothing; the killer is itself killed when the child returns.
  # ⚠ THE FIXTURE'S BUILD STATUS, ASKED BEFORE ANYTHING ELSE. Every fixture is
  # built under `|| _fixture_failed <name>`, so "did the tree this control is
  # about actually get built?" is one lookup rather than a probe re-derived per
  # site from the fixture's end state. Without it a broken build leaves a
  # plausible-looking tree, and for several fixtures that tree still satisfies
  # the assertion below — a control green over a question never posed.
  case " $_FIX_FAILED " in
    *" ${1##*/} "*)
      echo "!! CONTROL NOT EXERCISED ($4): its fixture did not build, so this control" >&2
      echo "   would be asserting over a tree that never posed the question." >&2
      return 1 ;;
  esac
  _out_f="$CTL/.control_out"
  # ⚠ SELF-TEST MODE IS AN ARGUMENT (see the wire, beside `_SELFTEST`): it is
  # the one channel a shell cannot leave behind for a later ordinary run.
  # ⚠ `set -m` SO THE CHILD IS ITS OWN PROCESS GROUP. Without it the watchdog's
  # `kill -9 "$_cpid"` reaches only the wire's own bash; a command substitution
  # or a `grep` BLOCKED beneath it — precisely the FIFO hang this watchdog
  # exists to catch — is reparented and stays blocked after the gate reports
  # the timeout, so repeated local runs accumulate permanent orphans. Measured
  # on bash 5.3 and 3.2: group kill reaps the descendant, top-PID kill does not.
  set -m
  PATH="${7:+$7:}$PATH" \
    env ${_ctl_env[@]+"${_ctl_env[@]}"} "$SELF" --selftest "$1" "${5:-}" "${6:-}" \
    > "$_out_f" 2>&1 & _cpid=$!
  set +m
  # ⚠ THE TIMER IS A SEPARATE PROCESS FROM THE SHELL THAT FORKED IT. `$!` is
  # the subshell; killing only that reparents the `sleep` to PID 1, where it
  # runs out its 30 s — one orphan per control, dozens per local gate run
  # (#501 R93, observed). The subshell traps TERM and takes its own children
  # with it, so the pair is reaped as a unit with shell builtins only.
  ( trap 'kill $(jobs -p) 2>/dev/null; exit 0' TERM
    sleep 30 & wait
    kill -9 -"$_cpid" 2>/dev/null || kill -9 "$_cpid" 2>/dev/null ) & _wpid=$!
  wait "$_cpid"; _rc=$?
  kill -TERM "$_wpid" 2>/dev/null || true; wait "$_wpid" 2>/dev/null || true
  # ⚠ CLEARED HERE, NOT BY THE CALLER. `_ctl_env` is file-scope (bash has no
  # array parameter), and it used to be reset by a `_ctl_env=()` line after each
  # control that set one — a positional convention, so a control inserted
  # between a set and its clear would silently inherit the previous control's
  # environment. One owner: whoever sets it gets exactly the next control.
  _ctl_env=()
  _out="$(cat "$_out_f" 2>/dev/null)"
  if [ "$_rc" -ge 128 ]; then
    echo "!! CONTROL FAILED ($4): the wire did not finish — killed after 30s (signal" >&2
    echo "   status $_rc). A gate that blocks is worse than one that reds: it never" >&2
    echo "   reaches a verdict at all. Something on this path opened what it must not." >&2
    return 1
  fi
  if [ "$_rc" -ne "$2" ]; then
    echo "!! CONTROL FAILED ($4): expected exit $2, got $_rc. A green below would" >&2
    echo "   mean nothing — this wire was not shown able to reach that verdict." >&2
    printf '%s\n' "$_out" | sed 's/^/     /' >&2
    return 1
  fi
  case "$_out" in *"$3"*) : ;; *)
    echo "!! CONTROL FAILED ($4): exit $2 as expected, but for the wrong reason —" >&2
    echo "   the output does not contain \"$3\"." >&2
    printf '%s\n' "$_out" | sed 's/^/     /' >&2
    return 1 ;;
  esac
  return 0
}
