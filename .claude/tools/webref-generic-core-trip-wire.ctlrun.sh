#!/usr/bin/env bash
# THE CONTROL RUN PRIMITIVES for `webref-generic-core-trip-wire.sh` — sourced by
# that wire's control harness (`…trip-wire.harness.sh`), never run on its own.
#
# WHY IT IS A SEPARATE FILE: CLAUDE.md's touch-time split, taken when the
# harness was about to pass the line threshold. The seam is the one the harness
# already drew in time. Everything before it is HOW A FIXTURE BUILD WINDOW
# WORKS — the scratch root, the window built from nothing, the postconditions
# checked inside it, the seal, the verdict, the fixture-failure fact the window
# records — and runs in the window or is forwarded into it by its prelude
# (`declare -f`), with the harness's own load-time asserts beside it. Everything
# here runs AFTER the window, in the parent, over the trees it built: the gate
# that refuses a control over an untrusted build or an unbuilt fixture, the one
# run primitive (the wire in self-test mode, as its own process group under a
# watchdog), and `_control` itself. Nothing here reaches the window: the
# prelude forwards none of these names, and the window calls none of them.
# WHAT IT CONSUMES: from the harness, `_fw_trusted` and `_fw2_said` (the window's
# trust, decided there) and `_fixture_did_fail` (the one reader of
# `$_FIX_FAILED`); from the controls file, `$_fw2_lbl`, `$_VFY` and `_ctl_env`
# (an ARRAY the caller sets per run); from the wire, `$SELF` and `$BASH`.
# WHAT IT DEFINES: `_ctl_gate`, `_ctl_run`, `_control`.
# ⚠ SOURCED ONLY AT THE END OF THE HARNESS, which sets `_CTLRUN` immediately
# before sourcing, so an unset one means this file was reached some other way.
if [ -z "${_CTLRUN:-}" ]; then
  echo "!! This file holds the CONTROL RUN PRIMITIVES for \`webref-generic-core-trip-wire.sh\`." >&2
  echo "   It is SOURCED by that wire's control harness and has no meaning alone." >&2
  echo "   Run the wire instead." >&2
  exit 2
fi
# No control may run over a build that is not COMPLETE AND TRUSTED, WHEREVER
# the verdict exit sits (docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §3),
# nor over a fixture that did not build. ONE GATE, asked by ONE run primitive:
# `_ctl_run` below asks `_ctl_gate <label> <fixture>…` about the fixture trees the
# run is aimed at, derived from its own arguments (root and cwd) — not a name
# picked per caller. The three blocks that are not a `_control` run through it as
# well, so a hang or a failed build is reported the same way in all four paths.
# The one run made outside it (the fsmonitor block's plain `git ls-files`
# preflight) has its own gate, in the controls file.
# W2 is asked by `_fw_trusted` alone — the one flag `_fgit_window` writes;
# `$_fw2_lbl` is defined in the controls file. Every fixture is built under
# `|| _fixture_failed <name>` (the PATH-prefix shim dirs are bare-built, so a failure
# there is errexit, which the window reports), so "did the tree this control is
# about actually get built?" is one lookup rather than a probe re-derived per site: without it a
# broken build leaves a plausible-looking tree that can still satisfy the
# assertion — a control green over a question never posed.
_ctl_gate() { # $1 = the label to report under, $2… = fixture names (basenames)
  local _g_for="$1" _g_n
  shift
  if [ "$_fw_trusted" -ne 1 ]; then
    [ "$_fw2_said" -eq 1 ] || echo "!! CONTROL FAILED ($_fw2_lbl): a control was reached over a build that was not complete and trusted" >&2
    _fw2_said=1; return 1
  fi
  for _g_n in "$@"; do
    if _fixture_did_fail "$_g_n"; then
      echo "!! CONTROL NOT EXERCISED ($_g_for): its fixture ($_g_n) did not build, so this control" >&2
      echo "   would be asserting over a tree that never posed the question." >&2
      return 1
    fi
  done
  return 0
}
# THE ONE RUN PRIMITIVE: the wire in self-test mode over a fixture root, as its own
# process group under a 30 s watchdog. Everything that runs the wire over a fixture
# is built on it — `_control` and the three blocks that are not one — so a hang is
# reported ("killed after 30s") whichever path hit it, and only the ASSERTION is the
# caller's. Arguments (empty = none): $1 label, $2 root, $3 cwd to run from,
# $4 umask to run under, $5 PATH prefix (to shadow a tool the wire calls), $6 optional
# scope subdir (relative), $7 optional extra entry (relative); plus `_ctl_env` (an
# ARRAY, set by the caller) for environment assignments. ⚠ NOT a string: `env ${8:-}`
# word-split them, so a `TMPDIR` holding a space broke the routed control with
# `env: 'dir/.../.git': No such file or directory` and the whole gate exited 1 for a
# contributor whose temp path has a space (#501 R96). A path is data here too.
# RETURN CONTRACT: 1 ONLY when the gate refused or the WATCHDOG killed the run
# (both already reported under the label); every other outcome — a status >= 128
# the wire came back with at once (a 134 abort, a 141 SIGPIPE) included — returns
# 0 with `_rc` and `_out` set (the run's status and its stdout+stderr), and the
# caller's own comparison is the verdict on it.
# ⚠ ONE OWNER FOR `_ctl_env`: taken into a local copy and cleared as the FIRST
# statement, so every return path (the gate's included) leaves the global empty.
# It is file-scope (bash has no array parameter); a clear placed after the gate
# let a refused control hand its environment to the NEXT one. Whoever sets it
# gets exactly the next run.
_ctl_run() {
  local -a _ce=(${_ctl_env[@]+"${_ctl_env[@]}"})
  _ctl_env=()
  local _r_for="$1" _r_root="$2" _r_cwd="${3:-}" _r_umask="${4:-}" _r_pfx="${5:-}"
  _ctl_gate "$_r_for" "${_r_root##*/}" ${_r_cwd:+"${_r_cwd##*/}"} || return 1
  _out_f="$_VFY/.control_out"
  # ⚠ WITH A WATCHDOG, because a hang is a verdict this harness could not
  # otherwise report (#501 R92). The FIFO finding's whole harm was that the
  # local gate BLOCKED instead of failing closed — and a control for it, run
  # plainly, would block too: it would never return to say so. Reverting that
  # fix now produces a reported control failure rather than a wire that waits
  # forever. Backgrounded and `wait`ed rather than polled, so a healthy
  # run costs nothing; the killer is itself killed when the child returns.
  # ⚠ SELF-TEST MODE IS AN ARGUMENT (see the wire, beside `_SELFTEST`): it is
  # the one channel a shell cannot leave behind for a later ordinary run.
  # ⚠ `set -m` SO THE CHILD IS ITS OWN PROCESS GROUP. Without it the watchdog's
  # `kill -9 "$_cpid"` reaches only the wire's own bash; a command substitution
  # or a `grep` BLOCKED beneath it — precisely the FIFO hang this watchdog
  # exists to catch — is reparented and stays blocked after the gate reports
  # the timeout, so repeated local runs accumulate permanent orphans. Measured
  # on bash 5.3 and 3.2: group kill reaps the descendant, top-PID kill does not.
  _r_wmark="$_VFY/.control_timeout"; command rm -f "$_r_wmark"
  set -m
  ( [ -z "$_r_cwd" ] || cd "$_r_cwd" || exit 97
    [ -z "$_r_umask" ] || umask "$_r_umask"
    export PATH="${_r_pfx:+$_r_pfx:}$PATH"
    exec env ${_ce[@]+"${_ce[@]}"} "$BASH" "$SELF" --selftest "$_r_root" "${6:-}" "${7:-}" ) \
    > "$_out_f" 2>&1 & _cpid=$!
  set +m
  # ⚠ THE TIMER IS A SEPARATE PROCESS FROM THE SHELL THAT FORKED IT. `$!` is
  # the subshell; killing only that reparents the `sleep` to PID 1, where it
  # runs out its 30 s — one orphan per control, dozens per local gate run
  # (#501 R93, observed). The subshell traps TERM and takes its own children
  # with it, so the pair is reaped as a unit with shell builtins only.
  ( trap 'kill $(jobs -p) 2>/dev/null; exit 0' TERM
    sleep 30 & wait
    # The marker names the cause; the kill is the job. A write that fails (the
    # scratch dir gone) must not stop the kill behind it, in any caller's context.
    # No record: every caller today is an `||` or `if !` operand, under which
    # errexit is off inside this subshell too, so the form is unobservable here;
    # it holds for a caller that is not.
    { : > "$_r_wmark"; } 2>/dev/null || :
    kill -9 -"$_cpid" 2>/dev/null || kill -9 "$_cpid" 2>/dev/null ) & _wpid=$!
  wait "$_cpid"; _rc=$?
  kill -TERM "$_wpid" 2>/dev/null || true; wait "$_wpid" 2>/dev/null || true
  _out="$(cat "$_out_f" 2>/dev/null)"
  # Only the watchdog's own marker makes a status >= 128 a timeout; a 134 (abort)
  # or 141 (SIGPIPE) that came back at once is the caller's "expected N, got M".
  # Pinned by the standing check in the controls file (`$_wm_lbl`) and its record.
  if [ "$_rc" -ge 128 ] && [ -e "$_r_wmark" ]; then
    echo "!! CONTROL FAILED ($_r_for): the wire did not finish — killed after 30s (signal" >&2
    echo "   status $_rc). A gate that blocks is worse than one that reds: it never" >&2
    echo "   reaches a verdict at all. Something on this path opened what it must not." >&2
    return 1
  fi
  return 0
}
_control() { # $1 = root, $2 = expected exit, $3 = expected message, $4 = label,
             # $5 = optional scope subdir (relative), $6 = optional extra entry (relative),
             # $7 = optional PATH prefix; the run itself is `_ctl_run`'s
  _ctl_run "$4" "$1" "" "" "${7:-}" "${5:-}" "${6:-}" || return 1
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
