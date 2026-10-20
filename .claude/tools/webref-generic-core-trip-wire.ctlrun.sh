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
# from the controls file, `$_fw2_lbl`, `$_VFY` and `$_CAP_TABLE` (the capability
# table, whose ONLY expander is this file's reader), and the capability flags
# by the names the table's `group` rows give (`_fifo_ok`, `_perm_ok`: set by the
# harness's probes, read here by `_cap_flag_of` alone); from the wire,
# `$SELF`, `$BASH` and `$_CONTROLS` (the string the wire sourced the controls
# file by, which is what `BASH_SOURCE` holds for that file's frames: the
# registry's site walk looks for it).
# ⚠ ASSERTED AT SOURCE TIME, below, every name above but `$_VFY` (the controls
# file makes it after this file is sourced, so no guard here could see it).
# WHAT IT DEFINES: `_ctl_reg` (the registry's writer: the harness's window
# verdict calls it too — after this file is sourced, since the controls file
# calls the verdict after it sourced the harness), `_ctl_gate`, `_ctl_run`,
# `_control`, `_ctl_wd_s` (the watchdog's seconds), and `_ctl_wd_pg` (an out-global:
# the watchdog's process group after a `_ctl_run`, read by the controls file).
# THE TABLE'S READER: `_cap_rows <kind> <callback>` (outside readers: `_cap_check`,
# `_cap_lines` (controls), the census's membership check (correspondence), `_mut_run`'s excuse (mutations)), `_cap_group_of`
# (out-globals `_cap_g`, a label's group, `_cap_f`, its flag NAME), `_cap_flag_of`
# (out-global `_cap_v`, a flag's VALUE), the callbacks `_cap_group_cb`, `_cap_gflag_cb`,
# `_cap_flag_cb`, `_cap_guard_cb`, the guard's `_cap_missing` and `_cap_n`, and the
# function-locals a nested pass shadows: `_cr_k`, `_cr_cb`, `_cr_rest`, `_cr_row`,
# `_cr_rr`, `_cr_tab`, `_cr_nl`, `_cr_w`, `_cr_i`, the callback's row `_cr_f`, `_cr_nf`,
# `_cr_bad`, `_cr_stop`, and `_cg_l`, `_cf_g`, `_cf_n`, `_cap_gn`.
# ⚠ SOURCED ONLY AT THE END OF THE HARNESS, which sets `_CTLRUN` immediately
# before sourcing, so an unset one means this file was reached some other way.
if [ -z "${_CTLRUN:-}" ]; then
  echo "!! This file holds the CONTROL RUN PRIMITIVES for \`webref-generic-core-trip-wire.sh\`." >&2
  echo "   It is SOURCED by that wire's control harness and has no meaning alone." >&2
  echo "   Run the wire instead." >&2
  exit 2
fi
# NO NAME IS SET PER CALL: what a control needs from the machine is read from the
# capability table by its own label (`_ctl_gate`), so no caller's environment can
# decline one. The watchdog's seconds are set here for the same reason. Every other
# name is assigned before the first call by the wire, the harness or the controls
# file, or by the primitive itself; the guard below asserts those that exist by now.
# A run's ENVIRONMENT is no name: it travels as `_ctl_run`'s trailing arguments.
_ctl_wd_s=30
# THE CAPABILITY TABLE'S READER, the only code that expands `$_CAP_TABLE` or a flag
# (plan §2.2). The callback runs once per row — kind `group`, `member`, `limit` or
# `fact`: each WELL-FORMED row of it (any other kind is a programming error: exit 2, below); `-`: every row, `_cr_bad` set on a malformed one
# (unknown kind, wrong field count, an empty field, a `group` flag field that is no
# variable name: `${!x-}` on `a-b` aborts bash 5.3, is empty on 3.2) — sees it through
# dynamic scope (`_cr_row`, `_cr_f[0…]`, `_cr_nf`, `_cr_bad`), returns 0, and stops the
# pass by `_cr_stop=1`, not a status (`cb || …` runs with errexit off). Cursor and fields
# are function-local, so a nested pass leaves the outer one intact; fields are split by
# parameter expansion (`read` collapses an empty one: tab is IFS whitespace).
_cap_rows() { # $1 = row kind (group, member, limit, fact, or `-` for every row), $2 = the callback
  local _cr_k="$1" _cr_cb="$2" _cr_tab=$'\t' _cr_nl=$'\n' _cr_row _cr_rr _cr_nf _cr_bad _cr_stop=0 _cr_w _cr_i
  local _cr_rest="$_CAP_TABLE$_cr_nl"
  case "$_cr_k" in group|member|limit|fact|-) ;; *) echo "!! _cap_rows: \"$_cr_k\" is no row kind to visit; decided nothing." >&2; exit 2 ;; esac
  local -a _cr_f
  while [ -n "$_cr_rest" ] && [ "$_cr_stop" -eq 0 ]; do
    _cr_row="${_cr_rest%%"$_cr_nl"*}"; _cr_rest="${_cr_rest#*"$_cr_nl"}"
    _cr_f=(); _cr_nf=0; _cr_rr="$_cr_row$_cr_tab"
    while [ -n "$_cr_rr" ]; do
      _cr_f[_cr_nf]="${_cr_rr%%"$_cr_tab"*}"; _cr_rr="${_cr_rr#*"$_cr_tab"}"; _cr_nf=$((_cr_nf + 1))
    done
    case "${_cr_f[0]}" in group) _cr_w=6 ;; member|fact) _cr_w=3 ;; limit) _cr_w=4 ;; *) _cr_w=0 ;; esac
    _cr_bad=""
    if [ "$_cr_w" -eq 0 ]; then _cr_bad="\"${_cr_f[0]}\" is no row kind"
    elif [ "$_cr_nf" -ne "$_cr_w" ]; then _cr_bad="a ${_cr_f[0]} row holds $_cr_nf field(s), not $_cr_w"
    else
      for ((_cr_i = 0; _cr_i < _cr_nf; _cr_i++)); do
        [ -n "${_cr_f[_cr_i]}" ] || { _cr_bad="field $((_cr_i + 1)) is empty"; break; }
      done
      [ "${_cr_f[0]}" != group ] || [ -n "$_cr_bad" ] || case "${_cr_f[2]}" in
        [0-9]*|*[!A-Za-z0-9_]*) _cr_bad="the flag field \"${_cr_f[2]}\" is no variable name" ;;
      esac
    fi
    if [ "$_cr_k" = - ] || { [ -z "$_cr_bad" ] && [ "${_cr_f[0]}" = "$_cr_k" ]; }; then "$_cr_cb"; fi
  done
  return 0
}
# THE LOOKUP (exact comparison, no fork): `_cap_g` = the label's group, `_cap_f` = that
# group's flag NAME; both empty for a non-member, `_cap_f` alone for an undeclared group.
_cap_group_of() { # $1 = a label
  local _cg_l="$1"; _cap_g=""; _cap_f=""
  _cap_rows member _cap_group_cb; [ -z "$_cap_g" ] || _cap_rows group _cap_gflag_cb
}
_cap_group_cb() { [ "${_cr_f[2]}" != "$_cg_l" ] || { _cap_g="${_cr_f[1]}"; _cr_stop=1; }; }
_cap_gflag_cb() { [ "${_cr_f[1]}" != "$_cap_g" ] || { _cap_f="${_cr_f[2]}"; _cr_stop=1; }; }
# A GROUP'S FLAG VALUE (`_cap_v`; empty for no such group): the one indirect expansion.
_cap_flag_of() { local _cf_g="$1" _cf_n; _cap_v=""; _cap_rows group _cap_flag_cb; } # $1 = a group name
_cap_flag_cb() { [ "${_cr_f[1]}" != "$_cf_g" ] || { _cf_n="${_cr_f[2]}"; _cap_v="${!_cf_n-}"; _cr_stop=1; }; }
# THE SOURCE-TIME GUARD, in the form of the controls file's and the mutation set's:
# a rename in the file that declares a name is a named exit 2 here, not an unbound
# abort at the first read. The flags are asserted by the table's own names.
_cap_missing=
for _cap_n in _fw_trusted _fw2_said _fw2_lbl SELF BASH _CONTROLS _CAP_TABLE; do
  [ -n "${!_cap_n:-}" ] || _cap_missing="$_cap_missing \$$_cap_n"
done
declare -F _fixture_did_fail > /dev/null || _cap_missing="$_cap_missing _fixture_did_fail()"
_cap_guard_cb() { local _cap_gn="${_cr_f[2]}"; [ -n "${!_cap_gn:-}" ] || _cap_missing="$_cap_missing \$$_cap_gn"; }
[ -z "${_CAP_TABLE:-}" ] || _cap_rows group _cap_guard_cb
if [ -n "$_cap_missing" ]; then
  echo "!! This file holds the CONTROL RUN PRIMITIVES for \`webref-generic-core-trip-wire.sh\`. It is" >&2
  echo "   SOURCED by that wire's control harness and has no meaning alone; missing:$_cap_missing" >&2
  echo "   Run the wire instead." >&2; exit 2
fi
# No control may run over a build that is not COMPLETE AND TRUSTED, WHEREVER
# the verdict exit sits (docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §3),
# nor over a fixture that did not build. ONE GATE, asked by ONE run primitive:
# `_ctl_run` below asks `_ctl_gate <label> <fixture>…` about the fixture trees the
# run is aimed at, derived from its own arguments (root and cwd) — not a name
# picked per caller. The two blocks that are not a `_control` (relcwd, fsmonitor)
# run through it as well, so a hang or a failed build is reported the same way in
# all three paths. The one run made outside it (the fsmonitor block's plain `git
# ls-files` follow-up, after `_ctl_run` returned) sits inside the branch that `_ctl_run`'s own ask opened.
# W2 is asked by `_fw_trusted` alone — the trust flag `_fgit_window` sets at its end — and
# reported ONCE per run (`_fw2_said`), under `$_fw2_lbl` (defined in the controls
# file), not under each refused label: the labels refused after the first are
# reported nowhere. Every fixture's `( cd … && … )` build chain runs under
# `|| _fixture_failed <name>` (what the fixtures file builds bare — every PATH-prefix
# shim dir, the routed control's decoy — fails by errexit, which ends the window: W,
# before any control), so "did the tree this control is about actually get built?" is one
# lookup rather than a probe re-derived per site: without it a broken build leaves
# a plausible-looking tree that can still satisfy the assertion — a control green
# over a question never posed. The controls file's three standing checks run over
# `$_VFY` (a root no fixture build makes, so no failed build can be named for
# it) and ask no fixture question: their stubs read no tree. Each reads
# `_fw_trusted` itself before its rows and, untrusted, reds in silence (W2 is this
# gate's report, printed once by the first gated run), so `_ctl_gate` is W2's
# one producer and W2's records tell "the gate refused" from "a control ran".
# THE RUN REGISTRY (`$_VFY/.registry`): each label passed to `_control`,
# `_ctl_run`, `_ctl_gate` or `_fgit_window_verdict_exit` is appended when the
# call runs, as `<file>:<line>	<kind>	<label>` (exactly three TAB fields: the census
# reds any other row) — under the `$_VFY` of the call: the standing checks' rows run under
# the run's own, so they register here as `direct` calls, `_cap`'s at its one `_ctl_run` line
# (a label only an `echo` produces reaches the census as a `_*_lbl` value). THE SITE RULE IS A
# BIJECTION WITH THE TEXT, checked by the census (`_mut_correspondence`, in the correspondence part): the
# registered sites are exactly the call lines of the controls file (`$_CONTROLS`: a non-comment line
# naming `_control`, `_ctl_run`, `_ctl_gate` or `_fgit_window_verdict_exit` as a word), kinds
# included. A call that never runs, a call registered at another line, and a call the text hides
# (`eval`, a split word, a helper defined on another line) are each red under the census's label.
# THE MECHANISM: THE SITE IS THE OUTERMOST FRAME WHOSE `BASH_SOURCE` IS THE CONTROLS FILE
# (`$_CONTROLS`): the top-level controls line that led here, walked from the top of the call stack
# down; the frames between (this file, the harness: `_control` and `_ctl_run` both end in `_ctl_gate`,
# whose first statement this is, as it is the window verdict's) are not the site while a controls
# frame is on the stack, which the wire guarantees (the primitives are reached only through the
# controls file; the `-gt 1` bound only keeps the index at a frame). THE KIND is `control` when
# `_control` is on the call stack (`FUNCNAME`), else `direct` (a primitive the controls file calls
# itself: `_ctl_run`, `_ctl_gate`, the verdict); the census requires a `control` label to be literal.
# A file, not a variable, so a call made in a subshell (`x="$(_control …)"`, a `( … )` block) is
# recorded too. The line is the one bash reports for that frame: the call's line for a plain call
# and a continuation line's first line (measured, both shells); a multi-line `$(…)`, an `eval`
# string and a `trap` body report what bash does there (an `eval` string and a `trap` body FOLD
# lines), and the bijection is what compares it with the text. The census reads the file once —
# ONE LABEL, ONE SITE: a label registered from two lines is red there — and notes its length;
# the controls file's last statement reds if it grew after that (`_mut_census_end`), whatever
# shell made the late call.
_ctl_reg() { # $1 = the label
  local _rg_i=$((${#BASH_SOURCE[@]} - 1)) _rg_k=direct
  case " ${FUNCNAME[*]} " in *" _control "*) _rg_k=control ;; esac
  while [ "$_rg_i" -gt 1 ] && [ "${BASH_SOURCE[$_rg_i]}" != "$_CONTROLS" ]; do _rg_i=$((_rg_i - 1)); done
  printf '%s:%s\t%s\t%s\n' "${BASH_SOURCE[$_rg_i]##*/}" "${BASH_LINENO[$((_rg_i - 1))]}" "$_rg_k" "$1" >> "$_VFY/.registry"
}
# THE CAPABILITY ARM is a lookup in the table by the control's own label
# (`_cap_group_of`): a label no `member` row holds runs; a member of a group no
# well-formed `group` row declares is red under the label, decided BEFORE any indirect
# expansion (`${!f-}` on an empty name aborts bash 5.3, expands empty on 3.2); a member
# whose group's flag is 0 returns 2 (`_control` maps it to 0); any other value runs it,
# the fail-safe direction, and `_cap_check` (controls file) reds the value. The label is
# registered on every machine. The gate is the registration, the trust arm (W2), the
# capability arm, then the fixture half; trust sits BEFORE capability (an untrusted
# window is red, never declined), and each arm is a row of the `_cap` standing check.
_ctl_gate() { # $1 = the label to report under, $2… = fixture names (basenames)
  _ctl_reg "$1"
  local _g_for="$1" _g_n
  shift
  if [ "$_fw_trusted" -ne 1 ]; then
    [ "$_fw2_said" -eq 1 ] || echo "!! CONTROL FAILED ($_fw2_lbl): a control was reached over a build that was not complete and trusted" >&2
    _fw2_said=1; return 1
  fi
  _cap_group_of "$_g_for"
  if [ -n "$_cap_g" ]; then
    if [ -z "$_cap_f" ]; then
      echo "!! CONTROL FAILED ($_g_for): its capability group \"$_cap_g\" is declared by no well-formed group row" >&2
      return 1
    fi
    _cap_flag_of "$_cap_g"
    [ "$_cap_v" != 0 ] || return 2
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
# once per run under `$_fw2_lbl`, by the first refusal; a member of a group the
# table does not declare, a failed fixture: each under the label), a channel argument is malformed (not NAME=VALUE) or reserved (below), the evidence directory
# could not be made, a delivered channel is missing from the evidence (`CONTROL NOT
# EXERCISED`: it never reached the run), or the WATCHDOG killed the run (under the
# label); 2 ONLY when the gate
# declined the run for a capability this machine lacks (the label is a member of a
# group whose flag is 0, above: nothing ran, nothing was reported); every other outcome
# — a status >= 128 the wire came back with at once (a 134 abort, a 141 SIGPIPE)
# included — returns 0 with `_rc` and `_out` set (the run's status and its
# stdout+stderr), and the caller's own comparison is the verdict on it. Every
# run that launched sets `_ctl_wd_pg` too: the watchdog's process group, which
# the `_wm` standing check asserts gone after the call.
# EVIDENCE THAT A DELIVERED CHANNEL REACHED THE RUN (every env channel, the
# umask and the PATH prefix the call delivers is demanded, derived from the
# delivery, not judged per caller; the prefix's evidence is `path`, written by a
# shim in the prefix when the run calls it): for a call that delivers any of the
# three, the evidence directory is made afresh (`rm -rf` then `mkdir`: a `path` a
# previous run left must not answer for this one) and the child is `exec env
# <channels> K2_CTL_REACHED=<dir> "$BASH" -c '<recorder>' ctlrec <dir> <names…> --
# "$BASH" "$SELF" …`; a call that delivers none demands nothing back, so it makes
# no directory and runs no recorder (`_r_pre` stays empty). The recorder is an
# inline `"$BASH" -c` string, bash builtins only, run under `$BASH` — the wire's own
# interpreter, never /bin/sh (macOS's turns `POSIXLY_CORRECT=1` into `y`). It
# writes `env.<NAME>` for each delivered name that is set, and `umask`, then
# execs the wire (the files are written under a 022 umask, which it restores
# first: a delivered 777 would leave them unreadable to the check). Its own names
# are `_k2rec_`-prefixed. REFUSED before delivery, as a `CONTROL FAILED` naming
# the argument: the recorder's own names (that prefix) and the names the delivery
# itself sets (`K2_CTL_REACHED`, `SHLVL`, `PWD`, `OLDPWD`, `_`). What the refusal
# buys is an early, named red, and the close of a false green: a recorder name
# delivered as itself (`_k2rec_n=_k2rec_n Y=1`) is recorded as delivered while the
# wire sees another value (as `n=n X=1` was, measured on both shells, when the
# names were unprefixed). It does NOT cover every name bash maintains, and need not: any other
# (`IFS`, `PPID`, `RANDOM`, `LINENO`, `SHELLOPTS`, … — not enumerated) reaches
# the recorder as bash's value, which disagrees with the delivery, so it falls
# to NOT EXERCISED — red, never green, the fail-safe direction (measured, both
# shells: of the names tried, 14 on 3.2 and 20 on 5.3 reach the recorder rewritten). That
# red is a true one: with the refusal removed, each refused name was recorded as
# the value the wire itself then saw (measured, both shells).
# The boundary between the harness's delivery and the wire's
# first statement is the one point where every channel is observable (the wire
# unsets `GREP_OPTIONS` before its first external command). After the run, every
# delivered `NAME=VALUE` must have `env.<NAME>` holding exactly VALUE, a
# delivered umask must equal the recorded one (both as 4 octal digits), and a
# delivered PATH prefix (`$5`) must have `path` present; else
# `CONTROL NOT EXERCISED` under the caller's label (with the run's status and
# output, as `_control` reports a wrong verdict), return 1. cwd needs none: a
# failed `cd` exits 97 at delivery. A channel argument that is not `NAME=…`, or is
# reserved, is a `CONTROL FAILED` and no run (checked after the gate's
# registration, before delivery, whatever the gate answered: a malformed call
# under a decline is red, not a silent 2 — row X of the `_cap` standing check).
_ctl_run() {
  local _r_for="$1" _r_root="$2" _r_cwd="${3:-}" _r_umask="${4:-}" _r_pfx="${5:-}"
  local _out_f _r_wmark _cpid _wpid _r_reach="$_VFY/.reached" _r_a _r_n _r_v _r_miss="" _r_w _r_g _r_gs=0
  local -a _r_ch=("${@:8}") _r_names=() _r_pre=()
  _ctl_gate "$_r_for" "${_r_root##*/}" ${_r_cwd:+"${_r_cwd##*/}"} || _r_gs=$?
  for _r_a in ${_r_ch[@]+"${_r_ch[@]}"}; do
    _r_n="${_r_a%%=*}"
    case "$_r_a" in *=*) : ;; *) _r_n="" ;; esac
    case "$_r_n" in ''|[0-9]*|*[!A-Za-z0-9_]*)
      echo "!! CONTROL FAILED ($_r_for): malformed channel argument \"$_r_a\" (want NAME=VALUE)" >&2
      return 1 ;;
    _k2rec_*|K2_CTL_REACHED|SHLVL|PWD|OLDPWD|_)
      echo "!! CONTROL FAILED ($_r_for): channel argument \"$_r_a\" names a variable the delivery sets or bash rewrites" >&2
      return 1 ;;
    esac
    _r_names+=("$_r_n")
  done
  [ "$_r_gs" -eq 0 ] || return "$_r_gs"
  if [ ${#_r_names[@]} -gt 0 ] || [ -n "$_r_umask$_r_pfx" ]; then
    command rm -rf "$_r_reach" && mkdir "$_r_reach" || {
      echo "!! CONTROL FAILED ($_r_for): the evidence directory could not be made" >&2; return 1; }
    _r_pre=(env ${_r_ch[@]+"${_r_ch[@]}"} K2_CTL_REACHED="$_r_reach" "$BASH" -c \
      '_k2rec_d=$1; shift; _k2rec_u=$(umask); umask 022
       while [ "$1" != -- ]; do _k2rec_n=$1; shift; [ -z "${!_k2rec_n+x}" ] || printf %s "${!_k2rec_n}" > "$_k2rec_d/env.$_k2rec_n"; done
       shift; printf %s "$_k2rec_u" > "$_k2rec_d/umask"; umask "$_k2rec_u"; exec "$@"' \
      ctlrec "$_r_reach" ${_r_names[@]+"${_r_names[@]}"} --)
  fi
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
    exec ${_r_pre[@]+"${_r_pre[@]}"} "$BASH" "$SELF" --selftest "$_r_root" "${6:-}" "${7:-}" ) \
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
    echo "   of a control whose injected channel was not delivered says nothing (the run: exit $_rc)." >&2
    printf '%s\n' "$_out" | sed 's/^/     /' >&2
    return 1
  fi
  return 0
}
_control() { # $1 = root, $2 = expected exit, $3 = expected message, $4 = label,
             # $5 = optional scope subdir (relative), $6 = optional extra entry (relative),
             # $7 = optional PATH prefix, $8 = optional umask to run under,
             # $9… = the run's environment, each a `NAME=VALUE` argument; the
             # run itself is `_ctl_run`'s.
             # A 2 from it (the table declined it: its group's flag is 0 here) is
             # green and the label registered; on a declaring machine that group's
             # summary line (`_cap_lines`, controls file) says what was not exercised,
             # and with no declaration `_cap_check` has already redded the run.
  local _cr=0
  _ctl_run "$4" "$1" "" "${8:-}" "${7:-}" "${5:-}" "${6:-}" "${@:9}" || _cr=$?
  # Not pinned by a record: on a capable machine nothing declines, so this line is
  # never reached (measured: removed, the wire PASSED on both shells); on an
  # incapable one, removed, the 2 falls to the `return 1` below and the caller's
  # `|| ctl_ok=1` reds the run (measured, the fifo flag forced to 0: rc 1).
  [ "$_cr" -ne 2 ] || return 0
  [ "$_cr" -eq 0 ] || return 1
  # `! … -eq`, NOT `-ne`: a non-numeric expected exit makes `[` exit 2, which
  # `-ne` reads as "not different" and passes; `!` reds it. Pinned by the record
  # that swaps the `pin` control's exit and message arguments.
  if ! [ "$_rc" -eq "$2" ]; then
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
