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
# trust, decided there) and `_fixture_did_fail` (the membership predicate over `$_FIX_FAILED`);
# from the controls file, `$_fw2_lbl`, `$_VFY` and `_ctl_need` (a prefix
# assignment on a call); from the wire,
# `$SELF`, `$BASH` and `$_CONTROLS` (the string the wire sourced the controls
# file by, which is what `BASH_SOURCE` holds for that file's frames: the
# registry's site walk looks for it).
# WHAT IT DEFINES: `_ctl_reg` (the registry's writer; the harness's window
# verdict calls it too — after this file is sourced, since the controls file
# calls the verdict after it sourced the harness), `_ctl_gate`, `_ctl_run`,
# `_control`, `_ctl_wd_s` (the watchdog's seconds), and `_ctl_wd_pg` (an out-global:
# the watchdog's process group after a `_ctl_run`, read by the controls file).
# ⚠ SOURCED ONLY AT THE END OF THE HARNESS, which sets `_CTLRUN` immediately
# before sourcing, so an unset one means this file was reached some other way.
if [ -z "${_CTLRUN:-}" ]; then
  echo "!! This file holds the CONTROL RUN PRIMITIVES for \`webref-generic-core-trip-wire.sh\`." >&2
  echo "   It is SOURCED by that wire's control harness and has no meaning alone." >&2
  echo "   Run the wire instead." >&2
  exit 2
fi
# THE ONE NAME A CALLER SETS PER CALL is cleared here, at source time, before
# any primitive reads it: the wire's environment is the caller's, and an
# exported `_ctl_need=0` would decline the first control in silence. The
# watchdog's seconds are set here for the same reason (a caller's value would not
# be this file's). Every other name the primitives read is assigned by the wire,
# the harness or the controls file before the first call, or by the primitive
# itself before it is read. A run's ENVIRONMENT is no name at all: it travels as
# the call's own trailing arguments (`_ctl_run`, below), so nothing file-scope
# can hand one control's environment to the next.
unset _ctl_need
_ctl_wd_s=30
# No control may run over a build that is not COMPLETE AND TRUSTED, WHEREVER
# the verdict exit sits (docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §3),
# nor over a fixture that did not build. ONE GATE, asked by ONE run primitive:
# `_ctl_run` below asks `_ctl_gate <label> <fixture>…` about the fixture trees the
# run is aimed at, derived from its own arguments (root and cwd) — not a name
# picked per caller. The two blocks that are not a `_control` (relcwd, fsmonitor)
# run through it as well, so a hang or a failed build is reported the same way in
# all three paths. The one run made outside it (the fsmonitor block's plain `git
# ls-files` preflight) sits inside the branch that `_ctl_run`'s own ask opened.
# W2 is asked by `_fw_trusted` alone — the trust flag `_fgit_window` sets at its end — and
# reported ONCE per run (`_fw2_said`), under `$_fw2_lbl` (defined in the controls
# file), not under each refused label: the labels refused after the first are
# reported nowhere. Every fixture is built under `|| _fixture_failed <name>` (the
# PATH-prefix shim dirs and the routed control's decoy the fixtures file builds are
# bare-built, so a failure there is errexit, which the window reports; the two the controls file builds after the
# window under `$_VFY` — relcwd's `mktemp`, cfgkept's `git` — are bare-built in the
# parent, where errexit ends the wire at once, unlabelled, before this gate is
# asked), so "did the tree this control is about actually get built?" is one
# lookup rather than a probe re-derived per site: without it a broken build leaves
# a plausible-looking tree that can still satisfy the assertion — a control green
# over a question never posed. The controls file's two standing checks run over
# `$_VFY` (a root no fixture build makes, so no failed build can be named for
# it) and ask no fixture question: their stubs read no tree. Each reads
# `_fw_trusted` itself before its rows and, untrusted, reds in silence (W2 is this
# gate's report, printed once by the first gated run), so `_ctl_gate` is W2's
# one producer and W2's records tell "the gate refused" from "a control ran".
# THE RUN REGISTRY (`$_VFY/.registry`): each label passed to `_control`,
# `_ctl_run`, `_ctl_gate` or `_fgit_window_verdict_exit` is appended when the
# call runs, as `<file>:<line>	<kind>	<label>` — under the `$_VFY` of the call:
# the controls file's `_ctl_need` standing check runs its rows under a `_VFY`
# of its own, so its rows reach that registry, not this one (its label reaches
# the census as a `_*_lbl` value). THE SITE IS THE OUTERMOST FRAME
# WHOSE `BASH_SOURCE` IS THE CONTROLS FILE (`$_CONTROLS`): the top-level controls
# line that led here, walked from the top of the call stack down — so a helper
# function defined in the controls file registers at each line that calls it,
# not at its body, and the frames between (this file, the harness: `_control`
# and `_ctl_run` both end in `_ctl_gate`, whose first statement this is, as it
# is the window verdict's) are not the site while a controls frame is on the
# stack, which the wire guarantees (the primitives are reached only through the
# controls file; the `-gt 1` bound only keeps the index at a frame). THE KIND
# is `control` when `_control` is on the call stack (`FUNCNAME`), else `direct`
# (a primitive the controls file calls itself: `_ctl_run`, `_ctl_gate`, the
# verdict); the census requires a `control` label to be literal. A file, not a
# variable, so a call made in a subshell (`x="$(_control …)"`, a `( … )` block)
# is recorded too, at the same site. The line is the one bash reports for that
# frame: for a plain call, a `$(…)`, a `( … )`, a continuation line and a
# controls-file helper it is the call's line (both shells); a multi-line `eval`
# string and a `trap` body FOLD lines (measured: bash 5.3 reports the string's
# own line, and line 1 of the trap body for every `trap` on that line; bash 3.2
# the line after the `eval` plus the offset, and the firing line for a trap), so
# two calls there can be one site, as two calls on one line are — the controls
# file has no `eval` or `trap` call. The census (`_mut_correspondence`, in the
# mutation set) reads the file once — ONE LABEL, ONE SITE: a label registered
# from two lines is red there — and notes its length; the controls file's last
# statement reds if it grew after that (`_mut_census_end`), whatever shell made
# the late call.
_ctl_reg() { # $1 = the label
  local _rg_i=$((${#BASH_SOURCE[@]} - 1)) _rg_k=direct
  case " ${FUNCNAME[*]} " in *" _control "*) _rg_k=control ;; esac
  while [ "$_rg_i" -gt 1 ] && [ "${BASH_SOURCE[$_rg_i]}" != "$_CONTROLS" ]; do _rg_i=$((_rg_i - 1)); done
  printf '%s:%s\t%s\t%s\n' "${BASH_SOURCE[$_rg_i]##*/}" "${BASH_LINENO[$((_rg_i - 1))]}" "$_rg_k" "$1" >> "$_VFY/.registry"
}
# `_ctl_need="$<flag>"` (a prefix assignment on the call) carries the VALUE of
# the capability flag a control needs — the value, not the flag's name: nothing
# is expanded indirectly here, so there is no identifier to get wrong (a flag
# that is unset is a `set -u` abort of the wire at the caller's line: red,
# unlabelled). The gate takes it into a local and UNSETS it first — in posix
# mode bash 3.2 keeps such an assignment past the call, which would decline
# every later control in silence. The label is registered on every machine;
# a 1 runs, a 0 returns 2 — not run, not red — after appending the label to
# `$_VFY/.declined`, which the controls file's asks read (`_cn_declined` there:
# a group's summary line is decided from it, and a decline between the last
# group's ask and the census is red at its last ask — one after the census is a
# late registration, red at `_mut_census_end`; `_control` maps the 2 to 0) — an append that fails is red
# under the label, not a silent decline; and anything else is red under the
# label. Whether a value was given at all is `${_ctl_need+set}`: an empty value
# is "given", and red. Each arm — 1, 0, the failed note, `x`, empty, unset — is
# a row of the controls file's standing check (`$_cn_lbl`). The gate is the
# registration, the trust arm (W2), these arms, then the fixture half. The trust
# arm sits BEFORE the need arms: an untrusted window is red, never declined
# (a 0 under it would turn W2 into a "decline?" ask on an incapable machine);
# that order is a row of the same standing check (`untrusted`).
_ctl_gate() { # $1 = the label to report under, $2… = fixture names (basenames)
  _ctl_reg "$1"
  local _g_for="$1" _g_need="${_ctl_need-}" _g_given="${_ctl_need+set}" _g_n
  unset _ctl_need
  shift
  if [ "$_fw_trusted" -ne 1 ]; then
    [ "$_fw2_said" -eq 1 ] || echo "!! CONTROL FAILED ($_fw2_lbl): a control was reached over a build that was not complete and trusted" >&2
    _fw2_said=1; return 1
  fi
  [ -z "$_g_given" ] || case "$_g_need" in 1) : ;;
    0) printf '%s\n' "$_g_for" >> "$_VFY/.declined" || {
         echo "!! CONTROL FAILED ($_g_for): the decline could not be noted in \$_VFY/.declined" >&2; return 1; }
       return 2 ;;
    *) echo "!! CONTROL FAILED ($_g_for): _ctl_need holds \"$_g_need\", not 0 or 1" >&2; return 1 ;;
  esac
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
# process group under a watchdog of `_ctl_wd_s` seconds (30, set at source time
# above; the `_wm` standing check in the controls file sets 1 in its subshell and
# runs a wire that sleeps past it). Every self-test run over a fixture is built
# on it — `_control` and the two blocks that are not one; the controls file spawns
# the wire nowhere else — so a hang there is reported ("killed after 30s") on
# every such path, and only the ASSERTION is the caller's. The opt-in mutation
# trials (`_mut_trial` in the mutations file, `WEBREF_WIRE_MUTANTS=1`) run the
# wire unbounded: slot `#11-trip-wire-liveness-bound`. Arguments (empty =
# none): $1 label, $2 root, $3 cwd to run from, $4 umask to run under, $5 PATH
# prefix (to shadow a tool the wire calls), $6 optional scope subdir (relative),
# $7 optional extra entry (relative); then, from $8 on, the run's environment
# assignments, each its own `NAME=VALUE` argument (validated: see the body).
# ⚠ EACH ITS OWN ARGUMENT, NOT ONE STRING: an earlier shape took the
# assignments as one string, expanded unquoted into `env`, which word-split
# it, so a `TMPDIR` holding a space broke the routed control with
# `env: 'dir/.../.git': No such file or directory` and the whole gate exited 1 for a
# contributor whose temp path has a space (#501 R96). A path is data here too.
# RETURN CONTRACT: 1 ONLY when the gate refused (an untrusted window: reported
# once per run under `$_fw2_lbl`, by the first refusal; a `_ctl_need` value that
# is not 0 or 1, a decline it could not note, a failed fixture: each under the
# label), a channel argument is malformed (not NAME=VALUE), the evidence directory
# could not be made, a delivered channel is missing from the evidence (`CONTROL NOT
# EXERCISED`: it never reached the run), or the WATCHDOG killed the run (under the
# label); 2 ONLY when the gate
# declined the run for a capability this machine lacks (`_ctl_need`, above: nothing
# ran, nothing was reported, the label noted in `.declined`); every other outcome
# — a status >= 128 the wire came back with at once (a 134 abort, a 141 SIGPIPE)
# included — returns 0 with `_rc` and `_out` set (the run's status and its
# stdout+stderr), and the caller's own comparison is the verdict on it. Every
# run that launched sets `_ctl_wd_pg` too: the watchdog's process group, which
# the `_wm` standing check asserts gone after the call.
# EVIDENCE THAT A DELIVERED CHANNEL REACHED THE RUN (every env channel, the
# umask and the PATH prefix the call delivers is demanded, derived from the
# delivery, not judged per caller; the prefix's evidence is `path`, written by a
# shim in the prefix when the run calls it): the
# child is `exec env <channels> K2_CTL_REACHED=<dir> "$BASH" -c '<recorder>'
# ctlrec <dir> <names…> -- "$BASH" "$SELF" …`. The recorder is INLINE (a file
# under `$_VFY` can be missing under `_cn`'s redirected `_VFY`, and the `ran` rows
# would not notice), bash builtins only, and runs under `$BASH` — the wire's own
# interpreter, never /bin/sh (macOS's turns `POSIXLY_CORRECT=1` into `y`). It
# writes `env.<NAME>` for each delivered name that is set, and `umask`, then
# execs the wire (the files are written under a 022 umask, which it restores
# first: a delivered 777 would leave them unreadable to the check): the boundary between the harness's delivery and the wire's
# first statement is the one point where every channel is observable (the wire
# unsets `GREP_OPTIONS` before its first external command). After the run, every
# delivered `NAME=VALUE` must have `env.<NAME>` holding exactly VALUE, a
# delivered umask must equal the recorded one (both as 4 octal digits), and a
# delivered PATH prefix (`$5`) must have `path` present; else
# `CONTROL NOT EXERCISED` under the caller's label, return 1. cwd needs none: a
# failed `cd` exits 97 at delivery. A channel argument that is not `NAME=…` is a
# `CONTROL FAILED` and no run (checked after the gate's registration, before
# delivery).
_ctl_run() {
  local _r_for="$1" _r_root="$2" _r_cwd="${3:-}" _r_umask="${4:-}" _r_pfx="${5:-}"
  local _out_f _r_wmark _cpid _wpid _r_reach _r_a _r_n _r_v _r_miss="" _r_w _r_g
  local -a _r_ch=("${@:8}") _r_names=()
  _ctl_gate "$_r_for" "${_r_root##*/}" ${_r_cwd:+"${_r_cwd##*/}"} || return $?
  for _r_a in ${_r_ch[@]+"${_r_ch[@]}"}; do
    _r_n="${_r_a%%=*}"
    case "$_r_a" in *=*) : ;; *) _r_n="" ;; esac
    case "$_r_n" in ''|[0-9]*|*[!A-Za-z0-9_]*)
      echo "!! CONTROL FAILED ($_r_for): malformed channel argument \"$_r_a\" (want NAME=VALUE)" >&2
      return 1 ;;
    esac
    _r_names+=("$_r_n")
  done
  _r_reach="$_VFY/.reached"
  command rm -rf "$_r_reach" && mkdir "$_r_reach" || {
    echo "!! CONTROL FAILED ($_r_for): the evidence directory could not be made" >&2; return 1; }
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
  # ⚠ `set -m` SO THE CHILD AND THE WATCHDOG ARE EACH THEIR OWN PROCESS GROUP.
  # Without it the watchdog's kill reaches only the wire's own bash; a command
  # substitution or a `grep` BLOCKED beneath it — precisely the FIFO hang this
  # watchdog exists to catch — is reparented and stays blocked after the gate
  # reports the timeout, so repeated local runs accumulate permanent orphans
  # (#501 R92); and a watchdog stopped by its pid alone leaves its `sleep` to
  # PID 1 for the rest of its seconds, one orphan per control (#501 R93). Both
  # are killed as a group, with SIGKILL, which no inherited disposition can
  # ignore (a TERM-based stop waited out every watchdog under a caller's
  # `trap "" TERM`). Pinned by the `_wm` standing check: its hang row's
  # descendant is dead after the call, and the watchdog's group (`_ctl_wd_pg`)
  # is gone after both rows.
  # ⚠ THE LAUNCHES ARE SILENCED AND READ NO TERMINAL: `{ … & } 2>/dev/null` and
  # `< /dev/null`. bash's setpgid races the forked child on macOS, and the loser
  # prints `child setpgid (N to N): Operation not permitted` on the wire's
  # stderr before the child's own redirect; under a tty with `stty tostop` that
  # write stops the child (SIGTTOU), a false red that prints the previous
  # control's output. Not pinned: not an equivalent mutant but an unpinnable
  # race, so no record can kill its removal on demand. Measured instead, under
  # `stty tostop` in a pty (`script -q /dev/null`), 20 whole-wire runs per cell
  # beside 10 busy loops, false reds (status 150) and stray `setpgid` lines, the
  # unsilenced launch (14d0b35d) against this one: bash 5.3 base 0 and 0 of 20 (two
  # series; an earlier series had 4 of 20, which did not reproduce), head 0 of 20;
  # bash 3.2 base 0 of 20, head 0 of 20; no stray line in any series. The whole-wire
  # counts therefore do not separate the two launches. The evidence for the
  # silencing is a launch-shape probe, not whole-wire runs: 0 of 2500 launches
  # failed under tostop with the silenced shape.
  _r_wmark="$_VFY/.control_timeout"; command rm -f "$_r_wmark"
  set -m
  { ( [ -z "$_r_cwd" ] || cd "$_r_cwd" || exit 97
    [ -z "$_r_umask" ] || umask "$_r_umask"
    export PATH="${_r_pfx:+$_r_pfx:}$PATH"
    exec env ${_r_ch[@]+"${_r_ch[@]}"} K2_CTL_REACHED="$_r_reach" "$BASH" -c \
      'd=$1; shift; u=$(umask); umask 022
       while [ "$1" != -- ]; do n=$1; shift; [ -z "${!n+x}" ] || printf %s "${!n}" > "$d/env.$n"; done
       shift; printf %s "$u" > "$d/umask"; umask "$u"; exec "$@"' \
      ctlrec "$_r_reach" ${_r_names[@]+"${_r_names[@]}"} -- \
      "$BASH" "$SELF" --selftest "$_r_root" "${6:-}" "${7:-}" ) \
    < /dev/null > "$_out_f" 2>&1 & _cpid=$!; } 2>/dev/null
  # THE MARKER IS WRITTEN BEFORE THE KILL, and it is the timeout fact. The
  # watchdog's own exit status cannot be: the child dies the moment the kill
  # lands, so the parent's group kill below can reach a watchdog that fired and
  # has not yet exited, and its 137 then reads as "stopped in its sleep".
  # Measured with a replica under load (2 × ncpu busy loops): the status form lost a
  # fired watchdog 3/500 and 3/1000 times on bash 5.3, 0/500 and 1/1000 on 3.2; the
  # marker-before-kill form 0/1000 on each. `true`, not `:`: a failed redirection on a
  # POSIX special builtin is fatal in bash 5's posix mode, which would end the
  # watchdog before its kill; on `true` it is not. The `|| true` keeps the kill
  # for a caller whose errexit reaches this subshell; every caller today is an
  # `||` or `if !` operand, under which errexit is off here, so it has no record.
  # `${_ctl_wd_s-}`, not `$_ctl_wd_s`: FAIL-CLOSED ON AN UNSET BOUND. Under set -u the
  # bare expansion aborts this subshell, so the run would go unwatched and still
  # pass; the empty word fails the sleep, the marker and kill follow, and the
  # first control's timeout report (its bare `${_ctl_wd_s}`) ends the wire red
  # under set -u (case R8nowd).
  { ( sleep "${_ctl_wd_s-}"
      true > "$_r_wmark" || true
      kill -9 -"$_cpid" ) < /dev/null > /dev/null 2>&1 & _wpid=$!; } 2>/dev/null
  set +m
  _ctl_wd_pg="$_wpid"
  wait "$_cpid" 2>/dev/null; _rc=$?
  kill -9 -"$_wpid" 2>/dev/null || true; wait "$_wpid" 2>/dev/null || true
  _out="$(cat "$_out_f" 2>/dev/null)"
  # Only the watchdog's own marker makes a status >= 128 a timeout; a 134 (abort)
  # or 141 (SIGPIPE) that came back at once is the caller's "expected N, got M".
  # Pinned by the standing check in the controls file (`$_wm_lbl`) and its record.
  if [ "$_rc" -ge 128 ] && [ -e "$_r_wmark" ]; then
    echo "!! CONTROL FAILED ($_r_for): the wire did not finish — killed after ${_ctl_wd_s}s (signal" >&2
    echo "   status $_rc). A gate that blocks is worse than one that reds: it never" >&2
    echo "   reaches a verdict at all. Something on this path opened what it must not." >&2
    return 1
  fi
  for _r_a in ${_r_ch[@]+"${_r_ch[@]}"}; do
    _r_n="${_r_a%%=*}"; _r_v="${_r_a#*=}"
    { [ -f "$_r_reach/env.$_r_n" ] && [ "$(cat "$_r_reach/env.$_r_n"; printf x)" = "${_r_v}x" ]; } \
      || _r_miss="$_r_miss $_r_n"
  done
  if [ -n "$_r_umask" ]; then
    _r_w="$(printf '%04o' $((8#$_r_umask)) 2>/dev/null)" || _r_w="?"
    _r_g="$(cat "$_r_reach/umask" 2>/dev/null)" && _r_g="$(printf '%04o' $((8#$_r_g)) 2>/dev/null)" || _r_g="!"
    [ "$_r_w" = "$_r_g" ] || _r_miss="$_r_miss umask"
  fi
  [ -z "$_r_pfx" ] || [ -f "$_r_reach/path" ] || _r_miss="$_r_miss PATH prefix"
  if [ -n "$_r_miss" ]; then
    echo "!! CONTROL NOT EXERCISED ($_r_for):${_r_miss} never reached the run — the verdict" >&2
    echo "   of a control whose injected channel was not delivered says nothing." >&2
    return 1
  fi
  return 0
}
_control() { # $1 = root, $2 = expected exit, $3 = expected message, $4 = label,
             # $5 = optional scope subdir (relative), $6 = optional extra entry (relative),
             # $7 = optional PATH prefix, $8 = optional umask to run under,
             # $9… = the run's environment, each a `NAME=VALUE` argument; the
             # run itself is `_ctl_run`'s.
             # A 2 from it (`_ctl_need` carried a 0: the capability is missing
             # here) is green: the label was registered, the gate noted the
             # decline, and the controls file's next ask decides a summary line
             # from it or reds it (`_cn_declined` there).
  local _cr=0
  _ctl_run "$4" "$1" "" "${8:-}" "${7:-}" "${5:-}" "${6:-}" "${@:9}" || _cr=$?
  # Not pinned by a record: on a capable machine nothing declines, so this line is
  # never reached (measured: removed, the wire PASSED on both shells); on an
  # incapable one, removed, the 2 falls to the `return 1` below and the caller's
  # `|| ctl_ok=1` reds the run (measured, the fifo flag forced to 0: rc 1).
  [ "$_cr" -ne 2 ] || return 0
  [ "$_cr" -eq 0 ] || return 1
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
