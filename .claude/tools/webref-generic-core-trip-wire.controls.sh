#!/usr/bin/env bash
# THE CONTROLS FOR `webref-generic-core-trip-wire.sh` — sourced by it, never run
# on its own.
#
# WHY IT IS A SEPARATE FILE: the wire crossed 1000 lines, and CLAUDE.md's
# touch-time split discipline says to split at a real cohesion seam when you
# touch such a file — not to defer it. This is that seam, and it was visible
# well before the line count: the wire ANSWERS about a tree, and everything here
# exists to show those answers are reachable. Not all of them are covered — the
# verdict sites with no control are the defer slot
# `#11-k2-wire-verdict-site-controls`, in
# 2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md §8. One
# green run of the scanner tells you nothing this file has not earned.
#
# THE INTERFACE, AND IT IS ASSERTED BELOW RATHER THAN DESCRIBED. What this file
# (and the harness it sources) CONSUMES from the wire: `$SELF` (re-invoked per
# control), `$SCRATCH` (the one scratch root, whose trap also cleans up `$CTL`),
# `$_CONTROLS` (this file's own path, which the defined-labels check reads, and
# the run registry's site walk in the run primitives compares frames against), the
# five `CONTROL_*` sample strings. That list — and only that list — is checked
# at entry. (No part calls the wire's `_git`, so the entry check no longer asks
# for it: a tracer over a full run counted 0 calls.)
# ⚠ WHAT THIS FILE DEFINES IS NOT AN INTERFACE, and a previous revision said it
# was: it named `$CTL`, `_control`, a per-run env global, `$_perm_line`,
# `$_fifo_line` and `ctl_ok` "for the wire to read", and the wire reads NONE of
# them (`grep -c '_perm_line\|_fifo_line\|\$CTL\|ctl_ok\|\b_control\b'`
# over the wire → **0**; the earlier spelling of this command left `_control`
# out, and it needs `\b` because a comment there names the sibling wire's
# `ban_control`). They belong to the controls — defined here or in the
# harness beside this file. The data flow is one-way, and saying otherwise
# invented a contract nobody could break.
# ⚠ AND THE CONSUMES LIST HAS BEEN WRONG TWICE. It first claimed `$ROOT` and
# `_phys`, which this file did not then mention; the revision that said so also
# added a mutation harness that consumes `$_CONTROLS`, which the same revision
# left off the list and out of the guard. A list maintained by hand beside the
# code it describes is the thing that drifts — the guard below is the only
# reason this one is true, and it is why the guard enumerates rather than the
# comment.
#
# WHY SOURCED AND NOT A SEPARATE PROGRAM. The alternative the plan-review
# offered — a real entry point taking explicit parameters — costs a second copy
# of the wire's locale pin and `GIT_NO_*` exports, i.e. a second statement of
# how this gate reads git. That is the decision-surface duplication this
# instrument spent four review rounds collapsing (#501 R76, R80, R81, R89), and
# it is what CLAUDE.md's "one issue, one way" forbids. The seam is real (answers
# vs. proof the answers are reachable); what was missing was the interface being
# enforced instead of narrated, and that is what this file now does.
#
# ⚠ ITS ABSENCE IS NOT A SKIPPED SELF-TEST. The wire refuses to run without it
# (exit 2, "decided nothing") rather than scanning with its controls silently
# gone — which is the one way a split like this could weaken the gate it is
# meant to keep legible.
# THE CONTRACT, ASSERTED AT ENTRY. Run on its own this file has none of the
# names above, and it used to say so by failing in `mktemp` with
# `mkdtemp failed on /ctlXXXXXX: Read-only file system` — a diagnostic about the
# wrong subject entirely, which is how a reader concludes the controls are
# broken rather than misinvoked.
# ⚠ AND THIS IS THE PIN FOR "IS THIS A WIRE?". The question has ONE decider —
# `scripts/trip-wires.sh`, which diffs its `*-trip-wire.sh` glob against
# `REQUIRED_WIRES` in BOTH directions — so a rename that brought this file INTO
# the convention would run it and fail twice over: here, with the message below,
# and there, with `trip-wire(s) ran but are not registered`. Nothing rests on
# the name being one token short of the glob.
_ctl_missing=
for _n in SELF SCRATCH _CONTROLS CONTROL_REMOVED CONTROL_K2 CONTROL_TOOLS CONTROL_BINARY CONTROL_CLEAN; do
  [ -n "${!_n:-}" ] || _ctl_missing="$_ctl_missing \$$_n"
done
if [ -n "$_ctl_missing" ]; then
  echo "!! This file is the CONTROLS for \`webref-generic-core-trip-wire.sh\`. It is" >&2
  echo "   SOURCED by that wire and has no meaning on its own; missing:$_ctl_missing" >&2
  echo "   Run the wire instead — it sources this file and refuses to run without it." >&2
  exit 2
fi
# DEFINED BEFORE THE HARNESS IS SOURCED: the harness uses `$_fws_lbl` while it loads.
# ONE LABEL PER PRODUCER, each the text its diagnostic prints, and each named by
# a mutation record (the census in the mutation set counts the `_*_lbl`
# variables as well as the labels the run's calls registered). What each
# asserts is tabled in docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §4.
_fw_lbl="the fixture build window completed"
_fw2_lbl="no control runs over an incomplete or untrusted fixture build window"
_fwd_lbl="the fixture build window ran without a shell diagnostic"
_pa_lbl="a window git whose inputs no fixtures-file command altered reads configuration only from its repo's config file"
_pal_lbl="this git reports a configuration origin outside the repo's file"
_pb_lbl="the fixture git has no system or global layer outside the void"
_pbl_lbl="this git names its system files through git var"
_pc_lbl="nothing is written into the fixture git's void"
_pd_lbl="the fixture git copies no template"
_pe_lbl="no exec-path override reaches the fixture git"
_pf_lbl="the fixture build window's environment holds only its allowlist"
_pg_lbl="every fixture repo persists only the configuration a plain git init writes"
_ph_lbl="the fixture build window reads in the wire's locale"
_pi_lbl="the fixture repos use the files ref format"
_pj_lbl="the fixture build window runs the pinned git first on its PATH"
_pk_lbl="no fixture repo reads objects from a store outside it"
_pkl_lbl="this git reports alternate object stores"
_fws_lbl="every mode restriction a fixture sealed was applied"
_mg_lbl="the boundary-mutant generator derives a non-empty set from the wire's regexes"
# (the census: the run registry and the `_*_lbl` variables, against the
# records — in the mutation set; the records whose needle this is pin it)
_cen_lbl="every label the run used is named by a mutation record"
# …the remaining labels, whose producer is not a `_control` — the relcwd and fsmonitor
# blocks, the defined-labels check, the three standing checks (their producers sit
# below) and the orderly-end check (in the wire) —, defined HERE so the check
# that follows can ask for every label at once:
_rel_lbl="a relative scratch dir is removed on exit"
_fsm_lbl="a caller's fsmonitor hook does not run"
_lbd_lbl="every label the parts use is defined"
_ord_lbl="the run ends only through a verdict"
_wm_lbl="only the watchdog's marker makes a signal status a timeout"
_cn_lbl="_ctl_need declines exactly what this machine cannot do"
_ev_lbl="a previous run's channel evidence never answers for the next"
# THE HARNESS — how a control runs — LIVES BESIDE THIS FILE; this file is which
# controls exist. Its absence ends the run at "decided nothing", as this file's
# own absence does in the wire.
_HARNESS="${SELF%.sh}.harness.sh"
if [ ! -r "$_HARNESS" ]; then
  echo "!! the control harness beside these controls ($_HARNESS) is missing or" >&2
  echo "   unreadable, so no control here can run. This run decided nothing." >&2
  exit 2
fi
# ⚠ EVERY LABEL A PART REFERENCES IS DEFINED AND NON-EMPTY, asked before anything
# else runs. A label renamed here and left in a producer is reached only when
# that producer fires, so the rename can sit unseen behind a green run: the names
# are derived from the parts' own text, by the regex the harness uses to forward
# the postconditions' labels, not listed. The parts read are every part of
# `_MUT_PARTS`. A part that is not readable is skipped here and reported by its
# own existence check: controls - by the wire, before this file runs; harness -
# just above; fixtures - below, after the harness is sourced; ctlrun - at the
# end of the harness; mutations - at the end of this file; records and mutgen -
# in the mutation set.
# THE PARTS, SPELLED ONCE: every file beside the wire, under the name a copy
# derives from its own `$SELF` (`<wire>.<part>.sh`). Read from this list: the
# label check below, and in the mutation set its entry check, the sibling guard,
# the per-run copies and `_mut_rm_copies` (the trap and the end of the run).
_MUT_PARTS="controls harness ctlrun mutations records fixtures mutgen"
_lbd_files=()
for _lbd_p in $_MUT_PARTS; do
  _lbd_f="${SELF%.sh}.$_lbd_p.sh"
  [ ! -r "$_lbd_f" ] || _lbd_files+=("$_lbd_f")
done
# grep with no file operand would read the wire's stdin, so an empty list skips it
_lbd_names=""
[ "${#_lbd_files[@]}" -eq 0 ] || _lbd_names="$(grep -ho '_[A-Za-z0-9_]*_lbl' "${_lbd_files[@]}" | sort -u)" || _lbd_names=""
_lbd_missing=""
for _lbd_n in $_lbd_names; do
  [ -n "${!_lbd_n:-}" ] || _lbd_missing="$_lbd_missing $_lbd_n"
done
if [ -z "$_lbd_names" ]; then
  echo "!! CONTROL FAILED ($_lbd_lbl): no label was found in the parts, so none was checked" >&2
  exit 2
elif [ -n "$_lbd_missing" ]; then
  echo "!! CONTROL FAILED ($_lbd_lbl): used but not defined, or empty:$_lbd_missing" >&2
  exit 2
fi
# shellcheck source=/dev/null
. "$_HARNESS"

# THE FIXTURE BUILD LIVES BESIDE THIS FILE: that one builds the trees, this one
# asserts over them. Its absence ends the run at "decided nothing", as the
# harness's does. It is NOT sourced here: the harness runs it in the fixture
# build window (`_fgit_window`), a child built from nothing, and checks the
# window's postconditions inside it.
_FIXTURES="${SELF%.sh}.fixtures.sh"
if [ ! -r "$_FIXTURES" ]; then
  echo "!! the fixture build beside these controls ($_FIXTURES) is missing or" >&2
  echo "   unreadable, so no control here has a tree to run over. This run decided nothing." >&2
  exit 2
fi
# The labels `_fgit_postconditions` uses reach the window by name, through its prelude.
_fgit_window "$_FIXTURES"
# THE PARENT'S VERIFIER DIRECTORY. Every file the parent writes and reads
# back after the build (the fsmonitor hook and its mark, the run primitives'
# evidence directory, where the PATH-prefix shims leave their marks, the controls'
# output, the mutation runner's lists) lives here, made now, after the
# build, by `mktemp -d` beside `$CTL` and `$_FW_DIR`, never under either:
# `$CTL` is the fixtures' tree, and the verifier writes nothing there
# (docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-census.md §4, "The parent's directory").
# Out of the fixtures' reach except through the declared class (c) of
# docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §0.3 (an executable
# written into a caller-`PATH` directory, `staleshim`).
if ! _VFY="$(mktemp -d "$SCRATCH/verifyXXXXXX")" || [ -z "$_VFY" ] || [ ! -d "$_VFY" ]; then
  echo "!! could not create the verifier's directory (TMPDIR/disk?); decided nothing." >&2
  exit 2
fi


# ⚠ Every _control call is an operand of `||`: `set -e` is suspended only
# inside such a command, so a bare call would abort the script on the first
# non-zero and the remaining arms would never run.
ctl_ok=0
# THE WINDOW'S VERDICT, WRITTEN AFTER `ctl_ok=0`. An incomplete window built
# nothing, so it ends the run here with W alone ("decided nothing"); a
# complete but UNTRUSTED window (a red postcondition, or W3) ends it here too,
# after its own reports — a control over it asserts nothing. `_control` itself
# refuses over a build that is not complete and trusted too (W2), wherever
# this line sits (it does so through `_ctl_run`, which asks `_ctl_gate`).
_fgit_window_verdict_exit "$_fw_lbl"
# THE THREE STANDING CHECKS BELOW run `_ctl_run` over `$_VFY` (a root that
# always exists: their stubs read no tree, so they ask no fixture question) in
# a subshell. Each opens by reading `_fw_trusted` — untrusted is red
# (`ctl_ok=1`) with no line of its own: W2 is the gate's report, printed once
# by the first gated run, and these three reads decide nothing about any run.
# STANDING CHECK of `_ctl_run`'s timeout clause, both ways. A status >= 128 is a
# timeout ONLY beside the watchdog's own marker: `SELF` is pointed (in a
# subshell) at a script that signals itself at once (KILL: no core dump, and no
# disposition a caller hands down can ignore it): `_ctl_run` must return 0
# with `_rc` >= 128 and report no kill — that status is the caller's "expected N,
# got M", not "killed after …s". The record that deletes the marker clause (in
# the record table) is killed through this check's label. And a wire that hangs
# IS a timeout: with the watchdog at 1 s (`_ctl_wd_s`, set in the subshell) and
# `SELF` a script that sleeps past it, `_ctl_run` must return 1, having written
# its marker and reported the kill as "killed after 1s" — the marker write and
# the message are pinned here, not by a record. The reaping is pinned here too:
# the hang script leaves a background descendant (its pid in `<script>.pid`),
# which must be dead after the call (the child's group kill), and after each row
# the watchdog's group (`_ctl_wd_pg`) must be gone (the watchdog's group kill).
# Both are asked with a bounded poll (`_wm_gone`, 20 × 0.05 s): a killed process
# can stay visible to `kill -0` until its reparenting reaper collects it. A row
# with no arm below is red, as `_cn`'s is. Two rows, ONE call line: the
# registry's site is the line, and one label has one site.
printf '%s\n' 'kill -KILL $$' > "$_VFY/.wm_abort"
printf '%s\n' 'sleep 5 & echo $! > "$0.pid"; wait' > "$_VFY/.wm_hang"
if [ "$_fw_trusted" -ne 1 ]; then ctl_ok=1
elif ! ( _wm_bad=""
         _wm_gone() { _wm_i=0; while kill -0 "$1" 2>/dev/null; do
             [ "$_wm_i" -lt 20 ] || return 1; _wm_i=$((_wm_i + 1)); sleep 0.05; done; }
         for _wm_row in abort hang; do
           SELF="$_VFY/.wm_$_wm_row"; [ "$_wm_row" != hang ] || _ctl_wd_s=1
           _wm_r=0; _ctl_run "$_wm_lbl" "$_VFY" > /dev/null 2> "$_VFY/.wm_err" || _wm_r=$?
           _wm_msg="$(cat "$_VFY/.wm_err")"
           _wm_gone "-$_ctl_wd_pg" || _wm_bad="$_wm_bad; $_wm_row: the watchdog's process group outlived the call"
           case "$_wm_row" in
             abort)
               [ "$_wm_r" -eq 0 ] || _wm_bad="$_wm_bad; _ctl_run returned $_wm_r, not 0"
               [ "${_rc:-0}" -ge 128 ] || _wm_bad="$_wm_bad; _rc is ${_rc:-unset}, not a signal status"
               case "$_wm_msg" in *"killed after "*) _wm_bad="$_wm_bad; a signal at once was reported as a timeout" ;; esac ;;
             hang)
               [ "$_wm_r" -eq 1 ] || _wm_bad="$_wm_bad; a run the watchdog killed returned $_wm_r, not 1"
               [ -e "$_VFY/.control_timeout" ] || _wm_bad="$_wm_bad; the watchdog wrote no marker"
               case "$_wm_msg" in *"killed after ${_ctl_wd_s}s"*) ;; *) _wm_bad="$_wm_bad; the kill was not reported as a ${_ctl_wd_s} s timeout" ;; esac
               _wm_d="$(cat "$_VFY/.wm_hang.pid" 2>/dev/null)" || _wm_d=""
               case "$_wm_d" in ''|*[!0-9]*) _wm_bad="$_wm_bad; the hung run recorded no descendant pid" ;;
                 *) _wm_gone "$_wm_d" || _wm_bad="$_wm_bad; the hung run's descendant ($_wm_d) outlived the kill" ;; esac ;;
             *) _wm_bad="$_wm_bad; no arm for row \"$_wm_row\"" ;;
           esac
         done
         [ -z "$_wm_bad" ] || { echo "!! CONTROL FAILED ($_wm_lbl)$_wm_bad" >&2; exit 1; } ); then
  ctl_ok=1
fi
command rm -f "$_VFY/.wm_abort" "$_VFY/.wm_hang" "$_VFY/.wm_hang.pid" "$_VFY/.wm_err" "$_VFY/.control_timeout"
# STANDING CHECK of the evidence directory's freshness (`_ctl_run` makes it
# afresh, `rm -rf` then `mkdir`, for every call that delivers a channel): a
# `path` a previous run left must not answer for this one. Two rows, ONE call
# line, over `$_VFY` with `SELF` a stub that calls `k2evprobe` and exits 0.
# `ran` delivers the PATH prefix `$_VFY/.ev_ran`, whose `k2evprobe` shim writes
# `path`: `_ctl_run` returns 0 and `path` is there. `stale` then delivers
# `$_VFY/.ev_stale`, which holds no `k2evprobe`, so the run calls no shim: it
# must return 1 naming "PATH prefix never reached the run" — a 0 is the first
# row's `path` answering. A row with no arm below is red, as `_cn`'s is.
command mkdir "$_VFY/.ev_ran" "$_VFY/.ev_stale"
printf '#!/bin/sh\n: > "${K2_CTL_REACHED:?}/path"\n' > "$_VFY/.ev_ran/k2evprobe"
chmod +x "$_VFY/.ev_ran/k2evprobe"
printf '%s\n' 'k2evprobe 2>/dev/null; exit 0' > "$_VFY/.ev_run"
if [ "$_fw_trusted" -ne 1 ]; then ctl_ok=1
elif ! ( SELF="$_VFY/.ev_run"; _ev_bad=""
         for _ev_row in ran stale; do
           _ev_r=0; _ctl_run "$_ev_lbl" "$_VFY" "" "" "$_VFY/.ev_$_ev_row" > /dev/null 2> "$_VFY/.ev_err" || _ev_r=$?
           case "$_ev_row" in
             ran)
               [ "$_ev_r" -eq 0 ] || _ev_bad="$_ev_bad; ran: _ctl_run returned $_ev_r, not 0"
               [ -f "$_VFY/.reached/path" ] || _ev_bad="$_ev_bad; ran: the shim left no path" ;;
             stale)
               [ "$_ev_r" -eq 1 ] || _ev_bad="$_ev_bad; stale: a prefix the run never called returned $_ev_r, not 1 (the previous run's path answered)"
               case "$(cat "$_VFY/.ev_err")" in *"PATH prefix never reached the run"*) ;; *) _ev_bad="$_ev_bad; stale: the red did not name the PATH prefix" ;; esac ;;
             *) _ev_bad="$_ev_bad; no arm for row \"$_ev_row\"" ;;
           esac
         done
         [ -z "$_ev_bad" ] || { echo "!! CONTROL FAILED ($_ev_lbl)$_ev_bad" >&2; exit 1; } ); then
  ctl_ok=1
fi
command rm -rf "$_VFY/.ev_ran" "$_VFY/.ev_stale" "$_VFY/.ev_run" "$_VFY/.ev_err" "$_VFY/.reached"
# STANDING CHECK of the gate's `_ctl_need` arms (`_ctl_gate`, in the run
# primitives), through `_ctl_run` in a subshell with `SELF` pointed at a script
# that exits at once, so "ran" is `_rc` set, and `_VFY` pointed at a directory
# of its own (`$_VFY/.cn`): every file the run primitives touch sits under
# `$_VFY` (the registry, `.declined`, the run's output and the watchdog's
# marker), so the rows reach none of the run's own — the shared `.declined` is
# read by the asks below from its first line, and this check's rows do not
# register in the run's registry (its label reaches the census as a `_*_lbl`
# value). The check leaving the run's `.declined` — the file the asks below
# read — as it found it is pinned: it is noted before the check (absent, or its
# line count and checksum) and compared after it, red under this label when it
# differs, in whatever order the rows run.
# Table-driven: the value given (`-` = none, `''` = empty) → the return,
# whether a run was made, `dir` = `.declined` made a directory first (or `chan` =
# the call passes the malformed channel argument `X`), and
# `untrusted` = the window's trust forced off for that row alone (`_fw_trusted=0`,
# `_fw2_said=1`, both restored after it: the rows below run trusted); after
# each, `_ctl_need` is unset again (in posix mode bash 3.2 keeps a prefix
# assignment, so the gate must clear it). Under a 0 the gate notes the label in
# `.declined`; a note it could not make, and a value that is not 0 or 1, are its
# own red (captured: the expected outcome there), and the red must name the
# value, or the failed note. A malformed channel under a 0 is red (1), naming
# the malformed argument: `_ctl_run` validates its channels whatever the gate
# answered, so a decline cannot hide a broken call. Under a 0 with trust off the gate returns 1 before
# the need arms: no run, no note — the trust arm's place before the need arms,
# which nothing on a capable machine would otherwise tell from the reverse.
# A row the loop cannot parse — a 4th column that is not `-`/`dir`/`chan`, a 5th that
# is not `-`/`untrusted`, a column missing — is red naming the row and is not
# run; a parsed row whose key (return, 4th, 5th) no arm below asserts is red too
# (`0 - -` is the one key the return and the run alone assert): a row never
# loses its arm in silence.
_cn_state() { # the run's `.declined`: absent, not a regular file, or its line count and checksum
  if [ ! -e "$_VFY/.declined" ]; then echo absent
  elif [ ! -f "$_VFY/.declined" ]; then echo "not a regular file"
  else echo "$(wc -l < "$_VFY/.declined" | tr -d '[:space:]') line(s), cksum $(cksum < "$_VFY/.declined")"; fi
}
_cn_before="$(_cn_state)"
command mkdir "$_VFY/.cn"
printf '%s\n' 'exit 0' > "$_VFY/.cn/.cn_run"
if [ "$_fw_trusted" -ne 1 ]; then ctl_ok=1
elif ! ( SELF="$_VFY/.cn/.cn_run"; _VFY="$_VFY/.cn"; _cn_bad=""
       while read -r _cn_v _cn_want _cn_ran _cn_decl _cn_trust; do
         _cn_line="$_cn_v $_cn_want $_cn_ran $_cn_decl $_cn_trust"
         case "$_cn_decl" in -|dir|chan) ;; *) _cn_bad="$_cn_bad; row \"$_cn_line\" did not parse: column 4 is \"$_cn_decl\", not -, dir or chan"; continue ;; esac
         case "$_cn_trust" in -|untrusted) ;; *) _cn_bad="$_cn_bad; row \"$_cn_line\" did not parse: column 5 is \"$_cn_trust\", not - or untrusted"; continue ;; esac
         case "$_cn_v" in -) unset _ctl_need ;; "''") _cn_val=""; _ctl_need="" ;; *) _cn_val="$_cn_v"; _ctl_need="$_cn_v" ;; esac
         [ "$_cn_ran" = ran ] || _cn_ran=""
         _cn_row="$_cn_v"; [ "$_cn_decl" = - ] || _cn_row="$_cn_v $_cn_decl"; [ "$_cn_trust" = - ] || _cn_row="$_cn_row $_cn_trust"
         command rm -rf "$_VFY/.declined"; [ "$_cn_decl" != dir ] || command mkdir "$_VFY/.declined"
         _cn_tr="$_fw_trusted"; _cn_sd="$_fw2_said"; [ "$_cn_trust" != untrusted ] || { _fw_trusted=0; _fw2_said=1; }
         _cn_ch=""; [ "$_cn_decl" != chan ] || _cn_ch=X
         unset _rc; _cn_got=0; _ctl_run "$_cn_lbl" "$_VFY" "" "" "" "" "" ${_cn_ch:+"$_cn_ch"} > /dev/null 2> "$_VFY/.cn_err" || _cn_got=$?
         _fw_trusted="$_cn_tr"; _fw2_said="$_cn_sd"
         [ "$_cn_got" -eq "$_cn_want" ] || _cn_bad="$_cn_bad; _ctl_need $_cn_row: returned $_cn_got, not $_cn_want"
         _cn_was="${_rc+made}"
         [ "${_rc+ran}" = "$_cn_ran" ] || _cn_bad="$_cn_bad; _ctl_need $_cn_row: a run was ${_cn_was:-not made}"
         [ -z "${_ctl_need+set}" ] || _cn_bad="$_cn_bad; _ctl_need $_cn_row: left set after the call"
         case "$_cn_want$_cn_decl$_cn_trust" in
           0--) ;; # a run that returned 0: the return and the run above are its whole assertion
           2--) [ "$(tail -n 1 "$_VFY/.declined" 2>/dev/null)" = "$_cn_lbl" ] || _cn_bad="$_cn_bad; _ctl_need $_cn_row: the decline was not noted" ;;
           1--) case "$(cat "$_VFY/.cn_err")" in *"holds \"$_cn_val\", not 0 or 1"*) ;; *) _cn_bad="$_cn_bad; _ctl_need $_cn_row: the red did not name the value" ;; esac ;;
           1dir-) case "$(cat "$_VFY/.cn_err")" in *"the decline could not be noted"*) ;; *) _cn_bad="$_cn_bad; _ctl_need $_cn_row: the red did not name the failed note" ;; esac ;;
           1-untrusted) [ ! -e "$_VFY/.declined" ] || _cn_bad="$_cn_bad; _ctl_need $_cn_row: a decline was noted over an untrusted window" ;;
           1chan-) case "$(cat "$_VFY/.cn_err")" in *"malformed channel argument"*) ;; *) _cn_bad="$_cn_bad; _ctl_need $_cn_row: the red did not name the malformed channel" ;; esac ;;
           *) _cn_bad="$_cn_bad; _ctl_need $_cn_row: no arm for row \"$_cn_line\"" ;;
         esac
       done <<EOF_CN
1 0 ran - -
x 1 - - -
'' 1 - - -
- 0 ran - -
0 1 - - untrusted
0 1 - dir -
0 1 - chan -
0 2 - - -
EOF_CN
       [ -z "$_cn_bad" ] || { echo "!! CONTROL FAILED ($_cn_lbl)$_cn_bad" >&2; exit 1; } ); then
  ctl_ok=1
fi
_cn_after="$(_cn_state)"
[ "$_cn_after" = "$_cn_before" ] || { echo "!! CONTROL FAILED ($_cn_lbl): the standing check changed the run's \$_VFY/.declined (before: $_cn_before; after: $_cn_after)" >&2; ctl_ok=1; }
command rm -rf "$_VFY/.cn"
# THE SUMMARY IS DECIDED FROM THE GATE'S DECISION: asked after a capability
# group's calls, this answers whether `$_VFY/.declined` grew since the previous
# ask (the first ask counts from the file's first line) — and reds when it did
# while the group's flag is 1: then this machine can pose the question, and a
# decline is the value carried wrong or the gate declining what it should run.
# Nothing between two asks can decline but a call that carries `_ctl_need`. The
# LAST ASK, before this file's exit on `ctl_ok`, reds any decline between the
# last group's ask and the census unconditionally: no summary line was decided
# from it (one after the census is a late registration, red at `_mut_census_end`
# under `$_cen_lbl`).
_cn_seen=0
_cn_declined() { # $1 = the group's capability flag value; $2 = the red's reason, if not the flag
  local _cd_n=0 _cd_new _cd_why="${2:-}"
  [ -n "$_cd_why" ] || _cd_why="while this machine's capability flag is 1"
  [ ! -e "$_VFY/.declined" ] || _cd_n="$(wc -l < "$_VFY/.declined" | tr -d '[:space:]')"
  _cd_new=$((_cd_n - _cn_seen)); _cn_seen=$_cd_n
  [ "$_cd_new" -gt 0 ] || return 1
  if [ "$1" -eq 1 ]; then
    echo "!! CONTROL FAILED ($_cn_lbl): $_cd_new control(s) declined $_cd_why:" >&2
    tail -n "$_cd_new" "$_VFY/.declined" | sed 's/^/     /' >&2
    ctl_ok=1
  fi
  return 0
}
# A mode a fixture sealed and the window could not apply (or refused) is red
# here, with its own label: the controls gated on that mode having taken effect
# would otherwise be skipped as if this machine could not enforce it.
if [ -n "$_fw_seal_bad" ]; then
  echo "!! CONTROL FAILED ($_fws_lbl): $(printf '%s' "$_fw_seal_bad" | tr '\n' ';')" >&2
  ctl_ok=1
fi
# A postcondition this machine cannot evaluate (P-b: `git var` before git 2.42;
# P-f: an `env` that refuses -0) is a machine limitation, reported below and not
# a red. The window writes one line per limitation, naming the postcondition by
# its ID only: a record's needle is the postcondition's LABEL, a full label the
# run must print in a diagnostic's label slot (`CONTROL FAILED (<needle>)` /
# `CONTROL NOT EXERCISED (<needle>)`), so a label printed on a green line would
# let any unrelated red "kill" that record for the right-looking reason.
_limit_lines=""
[ -z "$_fw_limits" ] || _limit_lines="$(printf '%s\n' "$_fw_limits" | sed 's/^/            ⚠ NOT EXERCISED on this machine: /')"
_control "$CTL/clean" 0 "PASSED"                  "green is reachable"   || ctl_ok=1
_control "$CTL/pin"   1 "K2: a"  "K2 fires on the path A-i removed" || ctl_ok=1
_control "$CTL/k2"    1 "K2: a"  "K2 fires on a path never here"    || ctl_ok=1
_control "$CTL/tools" 1 "K2: a"  "K2 fires under the tools root too" || ctl_ok=1
# ⚠ THE NEEDLE IS THE RECORD, NOT THE HEADLINE. `"K2: a"` passes with `-a`
# removed from `_content`'s grep too: this grep answers `Binary file … matches`
# on stdout and exits 0, so the run still reds. Measured as a surviving mutant.
# ⚠ BOTH ARMS STILL EMIT A RECORD in that state — the staged one names a TEMP
# BLOB PATH that no longer exists, the worktree one names the real entry — so
# "it names a temp path" is only half the reason, and not the half that
# discriminates. What discriminates is `:1:`, the LINE NUMBER, which only a
# read produces; `(worktree)` is chosen over `(staged)` because its path half is
# stable. Getting this reason wrong is how the needle drifts back.
_control "$CTL/binary" 1 "control.dat (worktree):1:" "K2 fires inside binary content, and the content is READ" || ctl_ok=1
_control "$CTL/link"   1 "K2: a" "K2 fires on a symlink's target"    || ctl_ok=1
_control "$CTL/nl"     1 "K2: a" "a newline in a filename is not a split" || ctl_ok=1
_control "$CTL/seg"    1 "K2: a" "K2 covers @ and non-ASCII segments"     || ctl_ok=1
_control "$CTL/cache"  1 "K2: a" "a regular file named __pycache__ is read" || ctl_ok=1
_control "$CTL/cachedir" 1 "K2: a" "a file UNDER a cache directory is read"  || ctl_ok=1
_control "$CTL/name"   1 "entry NAME" "an entry's own NAME is the hierarchy"  || ctl_ok=1
_control "$CTL/emptyname" 1 "entry NAME" "an EMPTY entry's name is the hierarchy" || ctl_ok=1
_control "$CTL/quotename" 1 "entry NAME" "a quote inside a name segment"          || ctl_ok=1
_control "$CTL/nlname"  1 "entry NAME" "a NEWLINE inside a name segment"          || ctl_ok=1
_control "$CTL/rawbyte" 1 "K2: a"      "a byte no UTF-8 locale can bracket"       || ctl_ok=1
_control "$CTL/forge"  0 "PASSED" "a name cannot forge a verdict record"      || ctl_ok=1
_control "$CTL/linkname" 1 "entry NAME" "a SYMLINK's own name is the hierarchy" || ctl_ok=1
_control "$CTL/ignored" 0 "PASSED" "an IGNORED generated artefact does not fire" || ctl_ok=1
_control "$CTL/lsfail" 1 "the tracked inventory exited" "a failed TRACKED inventory fails closed" "" "" "$CTL/fakegit" || ctl_ok=1
_control "$CTL/wtlsfail" 1 "the worktree inventory exited" "a failed WORKTREE inventory fails closed" "" "" "$CTL/fakegitwt" || ctl_ok=1
# Not a `_control`: the question is what the run leaves BEHIND, which its exit
# status and output cannot say. Run from a directory of its own, so a leaked
# relative scratch lands where this block can see it.
# A `mktemp` that answers with a RELATIVE path, as GNU `mktemp -d` does under a
# relative `TMPDIR` (macOS's ignores a relative `TMPDIR`, so the real tool
# cannot pose this question here). Built by the fixtures file, bare, beside the
# other PATH-prefix shims. It marks `path` whenever it runs, and `_ctl_run` demands that mark
# back for the prefix it was given — a run that never reached the shim had no
# relative scratch to leave behind, so its clean cwd is no evidence (the
# shim's path was dropped from the prefix once, and the block stayed green).
# Deleting the prefix ARGUMENT at the call site drops that demand too - the
# opt-in `SCRATCH=…` record's class; the always-on `path` demand catches a
# prefix the delivery fails to deliver, and a run that never calls the shim.
# ⚠ Run through `_ctl_run`, like every `_control`: gated, watchdogged, one report.
if ! _ctl_run "$_rel_lbl" "$CTL/clean" "$CTL/relcwd" "" "$CTL/fakerelmktemp"; then ctl_ok=1
elif [ "$_rc" -eq 0 ]; then
  if [ -n "$(ls -A "$CTL/relcwd")" ]; then
    echo "!! CONTROL FAILED ($_rel_lbl): left behind in $CTL/relcwd:" >&2
    ls -A "$CTL/relcwd" | sed 's/^/     /' >&2
    ctl_ok=1
  fi
else
  echo "!! CONTROL FAILED ($_rel_lbl): the run itself did not pass (exit $_rc)" >&2
  printf '%s\n' "$_out" | sed 's/^/     /' >&2
  ctl_ok=1
fi
_control "$CTL/lstreefail" 1 "the HEAD inventory exited" "a failed HEAD inventory fails closed" "" "" "$CTL/fakegitls" || ctl_ok=1
_control "$CTL/headprobe" 1 "that ref EXISTS, so this HEAD is not unborn" "a failed HEAD PROBE is not an unborn HEAD" "" "" "$CTL/headprobe" || ctl_ok=1
_control "$CTL/catfail" 1 "symlink blob (staged) could not be read" "a failed staged-blob read is not a clean target" "" "" "$CTL/catfail" || ctl_ok=1
_control "$CTL/phantom" 1 "the inventory listed it but it is gone" "an inventoried path that vanished is not silently skipped" "" "" "$CTL/fakegitphantom" || ctl_ok=1
_control "$CTL/globspec" 1 "the inventory listed it but it is gone" "the vanished-path question is asked of a LITERAL path" "" "" "$CTL/fakegitglob" || ctl_ok=1
_control "$CTL/badref" 1 "does not name a branch" "a malformed HEAD ref is not an unborn repository" || ctl_ok=1
_control "$CTL/punctslash" 1 "K2: a" "punctuation BEFORE a slash is part of the path" || ctl_ok=1
_control "$CTL/atclaude" 0 "PASSED" "a PATH character before .claude is not a prose boundary" || ctl_ok=1
_control "$CTL/linestart" 1 "K2: a" "a reference at the start of a line fires" || ctl_ok=1
_control "$CTL/textgreen" 0 "PASSED" "running text that only looks like a two-segment reference stays green" || ctl_ok=1
_control "$CTL/slashname" 1 "entry NAME" "a stored path naming .claude after a slash, under tools, fires" || ctl_ok=1
_control "$CTL/pathgreen" 0 "PASSED" "a stored path that only looks like one stays green" || ctl_ok=1
_control "$CTL/slashtext" 1 "K2: a" "a reference written after a slash fires" || ctl_ok=1
_control "$CTL/finalone" 1 "K2: a" "a ONE-character final segment fires" || ctl_ok=1
_control "$CTL/interone" 1 "K2: a" "a ONE-character intermediate segment fires" || ctl_ok=1
_control "$CTL/pathfirstone" 1 "entry NAME" "a ONE-character first segment in a STORED path fires" || ctl_ok=1
_control "$CTL/midclass" 0 "PASSED" "the final segment's middle stops where its last character does" || ctl_ok=1
_control "$CTL/bnd" 1 "K2: a" "the leading boundary covers =, --opt=, :, a backtick, ** and ," || ctl_ok=1
_control "$CTL/orphan" 0 "PASSED" "an orphan branch is an unborn HEAD, not a read failure" || ctl_ok=1
_control "$CTL/ancestorlink" 1 "ancestor component is a symlink" "the worktree read does not traverse an ancestor symlink" || ctl_ok=1
# ⚠ GREEN-DIRECTION CONTROLS — they prove the wire does NOT red on a legitimate
# tree, which is the failure mode that gets a required gate switched off rather
# than fixed. ⚠ An earlier version of this comment said "TWO" and added
# "every other control here proves the wire can RED"; both halves were false
# when written, and the second one still is — derive the count instead of
# reading one:
#     awk -F'"' '/^(if ! )?_control /{gsub(/ /,"",$3); print $3}' <this file> | sort | uniq -c
# (`$3` is the EXPECTED-EXIT argument: 0 = green-direction, 1 = red, 2 = the
# wire refusing to decide; the calls under a `_ctl_need=` prefix are not counted:
# they begin with the prefix, which the `^(if ! )?_control ` anchor does not
# match — `grep -c '^_ctl_need=.*_control ' <this file>` counts those. ⚠ The first spelling of this command
# printed `$4`, the expected MESSAGE, which counts nothing — a derivation offered
# in place of a claim has to be run once before it is written down.)
_control "$CTL/punct" 0 "PASSED" "closing punctuation is not a path segment" || ctl_ok=1
_control "$CTL/suffixpath" 0 "PASSED" "a segment merely ENDING in .claude is not the host path" || ctl_ok=1
# …and the self-test escape hatch, which must not be reachable from the
# environment. This is PR519 R8's reproduction, exactly: both old names
# exported, the companion set to the PID that IS the wire's parent here, and
# pointing at the CLEAN fixture. The argument names a violating one; an
# environment channel would scan the clean tree and exit 0.
_control "$CTL/committed" 1 "(in HEAD)" "an exported SELFTEST cannot redirect the scan" "" "" "" "" \
  "WEBREF_WIRE_SELFTEST=$CTL/clean" "WEBREF_WIRE_SELFTEST_PPID=$$" || ctl_ok=1
_control "$CTL/no-such-fixture" 2 "--selftest needs a fixture root" "a missing self-test root decides nothing" || ctl_ok=1
_control "$CTL/grepfail" 1 "the entry NAME went unchecked" "a failed NAME matcher fails closed" "" "" "$CTL/fakegrep" || ctl_ok=1
_control "$CTL/grepfaillink" 1 "the symlink TARGET went unchecked" "a failed TARGET matcher fails closed" "" "" "$CTL/fakegrep" || ctl_ok=1
_control "$CTL/nltarget" 1 "K2: a" "a NEWLINE-terminated symlink target is not truncated" || ctl_ok=1
_control "$CTL/linkslash" 0 "PASSED" "readlink's own newline is not read as stored content" || ctl_ok=1
_control "$CTL/staged" 1 "(staged)" "a STAGED violation reverted in the worktree still fires" || ctl_ok=1
# ⚠ THE TWO FIFO CONTROLS SHARE ONE PRECONDITION AND ONE REPORT LINE, decided
# after the second of them from what the gate did (the `$_perm_line` shape, and
# for the same reason: a summary written apart from its decision can disagree
# with it). A machine that cannot hold a FIFO is a capability limit, not a wire
# defect — but it is also not something this run may pass over in silence.
# `_ctl_need="$_fifo_ok"` carries the limit's VALUE with the call (no `if` around
# it): the call runs on every machine and registers its label for the census, and
# where the value is 0 the gate declines it without a run (`_ctl_gate`, in the run primitives).
_fifo_line="            an entry git cannot store neither hangs nor hides a verdict, and a
            tracked path replaced by one is not opened"
_ctl_need="$_fifo_ok" _control "$CTL/fifotracked" 1 "NOT opened" "a tracked path replaced by a FIFO is not opened" || ctl_ok=1
_control "$CTL/notcommitted" 1 "K2: a" "per-clone info/exclude cannot hide an entry" || ctl_ok=1
_control "$CTL/committed" 1 "(in HEAD)" "a COMMITTED violation fixed only in the index still fires" || ctl_ok=1
# ⚠ A FIXTURE THAT DID NOT BUILD MUST NOT PASS, and `_fixture_failed` is where
# that is now decided — for every control, not for the five somebody noticed.
# The history is worth keeping because it is what the general mechanism costs
# to NOT have: a failed `git replace` left this control green while testing
# nothing (#501 R96, reproduced with a `git` wrapper failing only `replace`);
# the fix was a bespoke `replace -l` probe here; the comment introducing it
# called the class closed at two sites; `cachedir`, `empty` and `lstreefail`
# then turned up in the same state, two of them added by the commit that wrote
# that sentence. Five hand-written probes, each re-deriving from the fixture's
# END STATE a fact the BUILD already knew.
# ⚠ AND ONE OF THOSE PROBES HAD TO BE SUBTLE, which is the other argument for
# not writing them: this run exports `GIT_NO_REPLACE_OBJECTS=1`, so reading the
# blob returns the violating bytes whether or not the replacement took — the
# obvious probe would have passed vacuously for the very reason the control
# exists, and only `replace -l` worked. A build status has no such trap.
_control "$CTL/replaced" 1 "(staged)" "a replace ref cannot substitute the staged blob" || ctl_ok=1
# A `git` that answers `ls-files` with nothing unless the caller's
# `GIT_CONFIG_COUNT` reached it. The fixture holds a violation, so a run that
# KEEPS the configuration reds on K2 and a run that strips it reads zero and
# exits 2 — the two verdicts differ in status AND message, which is what makes
# this control discriminate rather than merely fire.
# ⚠ Shimmed because the real case — a checkout owned by another UID, reachable
# only via `safe.directory` — cannot be built here. The variable's SURVIVAL is
# the property under test, and that is constructible; the ownership is not.
# Built by the fixtures file, bare, beside the other PATH-prefix shims, through
# the same builder as every exec-through shim. It marks `path` whenever the `ls-files` arm is
# reached, BEFORE the configuration test, and `_ctl_run` demands that mark back
# for the prefix it was given — a run that never reached the shim had no
# configuration to keep or strip, so its K2 red is no evidence (the shim's
# path was dropped from the prefix once, and the control stayed green).
# Deleting the prefix ARGUMENT at the call site drops that demand too - the
# opt-in `GIT_CONFIG*) unset` record's class; the always-on `path` demand
# catches a prefix the delivery fails to deliver, and a run that never calls
# the shim.
if ! _control "$CTL/cfgkept" 1 "K2: a" "the caller's git CONFIGURATION survives the routing purge" "" "" "$CTL/fakegitcfg" "" \
  "GIT_CONFIG_COUNT=1" "GIT_CONFIG_KEY_0=safe.directory" "GIT_CONFIG_VALUE_0=*"; then ctl_ok=1
fi
_control "$CTL/routed" 1 "K2: a" "exported GIT_DIR cannot redirect the scan" "" "" "" "" \
  "GIT_DIR=$CTL/routeddecoy/.git" "GIT_WORK_TREE=$CTL/routeddecoy" || ctl_ok=1
_control "$CTL/glob[1]" 0 "PASSED" "a glob character in the checkout path does not widen the scope" "scope" || ctl_ok=1
_control "$CTL/globextra*[e]" 1 "K2: a" "a pattern character in the checkout path does not misplace the entry script" "scope" "side/entry" || ctl_ok=1
_control "$CTL/nulblob" 1 "holds a NUL" "a NUL-bearing staged symlink blob is not a path" || ctl_ok=1
_control "$CTL/stagedlink" 1 "(staged) ->" "a STAGED symlink target is a stored path" || ctl_ok=1
_control "$CTL/headlink" 1 "(in HEAD) ->" "a COMMITTED symlink target is a stored path" || ctl_ok=1
_control "$CTL/inscope" 2 "INSIDE the tree" "scratch inside the scanned tree decides nothing" "" "" "$CTL/fakemktemp" || ctl_ok=1
_control "$CTL/extra"  1 "K2: a" "a symlinked EXTRA entry is scanned" "sub" "entry" || ctl_ok=1
# …the second of the pair `$_fifo_line` above reports on, and the decision.
_ctl_need="$_fifo_ok" _control "$CTL/odd" 0 "PASSED" "an unstorable entry neither hangs nor hides" || ctl_ok=1
if _cn_declined "$_fifo_ok"; then
  _fifo_line="            ⚠ NOT EXERCISED on this machine: BOTH FIFO controls (this
            filesystem holds no fifo), so this run carries no evidence that an
            entry git cannot store neither hangs nor hides a verdict, NOR that a
            tracked path replaced by one is not opened — the second being the
            control whose original finding was a local gate that HUNG"
fi
# An empty scope must be an ERROR, not a pass: "no violations" and "nothing
# read" are different answers and only one of them is green.
# ⚠ THIS ONE IS WHY THE BUILD STATUS IS RECORDED RATHER THAN PROBED. A `git
# init` that never ran produces a BYTE-IDENTICAL verdict here: the
# `SCANNED -eq 0` guard short-circuits before the `ERR_HITS` block, so a
# non-repository and an empty repository both print
# `!! read 0 stored objects or files` and exit 2. No end-state probe over the
# fixture can tell them apart cheaply; the build's own exit status can, and
# does, for this control and every other.
_control "$CTL/empty" 2 "read 0 stored objects" "an empty scope fails loudly" || ctl_ok=1
_control "$CTL/unread" 2 "read 0 stored objects" "an inventoried entry that was never read is not counted as scanned" || ctl_ok=1
# A WALK KILLED MID-SCAN is not a verdict: `POSIXLY_CORRECT` plus the failing
# `cat` ends the `_scan` subshell after it has emitted `a.py`'s record.
_control "$CTL/catkill" 2 "did not complete" "a walk killed mid-scan is not a verdict" "" "" "$CTL/catfail" "" POSIXLY_CORRECT=1 || ctl_ok=1
_control "$CTL/d2green" 0 "PASSED" "a tracked directory deleted wholesale stays green" || ctl_ok=1
_control "$CTL/d2file" 0 "PASSED" "a tracked directory replaced by a file stays green" || ctl_ok=1
_control "$CTL/d3f1" 1 "K2: a" "a ] inside a first segment" || ctl_ok=1
_control "$CTL/d3f2" 1 "K2: a" "a double quote inside a first segment" || ctl_ok=1
_control "$CTL/d3f3" 1 "K2: a" "a single quote inside a first segment" || ctl_ok=1
_control "$CTL/d3f4" 1 "K2: a" "a backtick inside a first segment" || ctl_ok=1
_control "$CTL/d3m1" 1 "K2: a" "a ] inside an intermediate segment" || ctl_ok=1
_control "$CTL/d3m2" 1 "K2: a" "a double quote inside an intermediate segment" || ctl_ok=1
_control "$CTL/d3m3" 1 "K2: a" "a single quote inside an intermediate segment" || ctl_ok=1
_control "$CTL/d3m4" 1 "K2: a" "a backtick inside an intermediate segment" || ctl_ok=1
_control "$CTL/d3nb" 1 "K2: a" "a quoted one-segment reference before a slash token fails safe" || ctl_ok=1
# k2: a caller's `GREP_OPTIONS` over a violating file. BSD grep honours it, so
# without the wire's `unset` the file reads as "no match"; GNU grep — CI's — does
# not (2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md §11.1; the note's own
# measurement is BSD only), so the real tool cannot pose this question on every
# machine. The control does not ask the machine's grep: `goptgrep` (the fixtures
# file, beside `fakegrep`) answers "no match" whenever a `GREP_OPTIONS` reached it
# and execs the real grep otherwise. It models the property under test — did the
# caller's variable reach grep? — not the value's meaning to one grep, as
# `fakegitcfg` does for `GIT_CONFIG_COUNT` and `fakerelmktemp` for a relative
# `TMPDIR`; so the question is posed on every machine and declined on none. The
# shim marks `path` before the test, and `_ctl_run` demands that mark back.
_control "$CTL/k2" 1 "K2: a" "a caller's GREP_OPTIONS cannot hide a file" "" "" "$CTL/goptgrep" "" "GREP_OPTIONS=--exclude=*" || ctl_ok=1
# A `core.fsmonitor` hook injected through the caller's `GIT_CONFIG*`, which the
# wire keeps. Not a `_control`: the question is whether a command RAN, which the
# verdict cannot say. ⚠ The same hook is then shown to run under a plain git
# call over the same fixture — inside the branch `_ctl_run`'s ask opened, so
# this block asks the gate ONCE, under its one label — or its not running under
# the wire would prove nothing. The hook's path goes through `_shq`.
_fsm_root="$CTL/fsmon"
_fsm_mark="$_VFY/.fsmonitor_ran"
printf '#!/bin/sh\n: > %s\nexit 1\n' "$(_shq "$_fsm_mark")" > "$_VFY/fsmhook"
chmod +x "$_VFY/fsmhook"
_fsm_cfg=("GIT_CONFIG_COUNT=1" "GIT_CONFIG_KEY_0=core.fsmonitor" "GIT_CONFIG_VALUE_0=$(_shq "$_VFY/fsmhook")")
command rm -f "$_fsm_mark"
if ! _ctl_run "$_fsm_lbl" "$_fsm_root" "" "" "" "" "" "${_fsm_cfg[@]}"; then ctl_ok=1
elif [ "$_rc" -ne 0 ] || [ -e "$_fsm_mark" ]; then
  echo "!! CONTROL FAILED ($_fsm_lbl): exit $_rc; the hook ran: $([ -e "$_fsm_mark" ] && echo yes || echo no)" >&2
  printf '%s\n' "$_out" | sed 's/^/     /' >&2
  ctl_ok=1
elif ( unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE; cd "$_fsm_root" \
       && env "${_fsm_cfg[@]}" git ls-files >/dev/null 2>&1 ) || true
     [ ! -e "$_fsm_mark" ]; then
  echo "!! CONTROL NOT EXERCISED ($_fsm_lbl): the hook did not run under a plain" >&2
  echo "   git call either, so its not running under the wire proved nothing." >&2
  ctl_ok=1
fi
# ⚠ The line is built HERE, beside the decision that produces it. An earlier
# shape decided here and described it in the summary below, so the two could
# disagree — and an unconditional summary claimed exit-status evidence the run
# had not obtained (#501 R78). One site, one truth: the line is decided from what
# the gate did (`_cn_declined`), not from the flag. `_ctl_need="$_perm_ok"` on the
# five calls, as on the FIFO pair: registered on every machine, run where the
# value is 1. The fifth runs the clean fixture under umask 777 (`_control`'s
# `$8`): a umask that leaves the scratch dir unsearchable makes it unresolvable
# to a physical path, which the wire refuses to decide over.
_perm_line="            ⚠ NOT EXERCISED on this machine: the controls that need file
          permissions enforced (this user can read a mode-000 file), so this
          run carries no evidence for them"
_ctl_need="$_perm_ok" _control "$CTL/err"  1 "could not be read" "an unreadable file fails closed" || ctl_ok=1
_ctl_need="$_perm_ok" _control "$CTL/walk" 1 "could not be read" "an unsearchable dir fails closed" || ctl_ok=1
_ctl_need="$_perm_ok" _control "$CTL/d2red" 1 "not provably absent" "a tracked file under an unsearchable dir is not absent" || ctl_ok=1
_ctl_need="$_perm_ok" _control "$CTL/d5root" 2 "or the root to a physical path" "an unresolvable self-test root decides nothing" || ctl_ok=1
_ctl_need="$_perm_ok" _control "$CTL/clean" 2 "could not resolve the scratch dir" "a restrictive umask decides nothing" "" "" "" 777 || ctl_ok=1
if ! _cn_declined "$_perm_ok"; then
  _perm_line="            the controls that need file permissions enforced ran too"
fi
# ---- THE MUTATION SET, WHICH LIVES BESIDE THIS FILE -------------------------
# Two subjects, two files: this one runs the wire over a fixture and asserts the
# exit status; that one proves each control is about the arm it names. Its
# absence is not a skipped check — the correspondence between the two lists is
# what keeps either list honest, so the run ends at "decided nothing" rather
# than without it.
_MUTATIONS="${SELF%.sh}.mutations.sh"
if [ ! -r "$_MUTATIONS" ]; then
  echo "!! the mutation set beside these controls ($_MUTATIONS) is missing or" >&2
  echo "   unreadable, so nothing here was shown to be about the arm it names." >&2
  echo "   This run decided nothing." >&2
  exit 2
fi
# shellcheck source=/dev/null
. "$_MUTATIONS"
_mut_correspondence || ctl_ok=1
# THE LAST ASK (`_cn_declined`, above): a decline after the perm group's ask has
# no summary line, so it is red whatever this machine's flags are. (One after the
# census is a late registration as well, red at `_mut_census_end`.)
_cn_declined 1 "after the last capability group's ask, so no summary line was decided from them" || :

# (No check after the controls: every `_control` and every block that is not one
# runs through `_ctl_run` (which asks `_ctl_gate`), so none runs over an unbuilt tree;
# the fsmonitor block's plain `git ls-files` follow-up, made after `_ctl_run` returns,
# sits inside the branch that its `_ctl_run`'s ask opened.)
[ "$ctl_ok" -eq 0 ] || exit 1

_mut_run

echo "  controls: green reachable; K2 fires under both roots, on the removed path,"
echo "            inside binary content, on a symlink's stored target, on an entry's"
echo "            own NAME, on a symlinked entry script beside the scope, and under a"
echo "            caller's GREP_OPTIONS; an empty scope fails closed"
printf '%s\n' "$_fifo_line"
printf '%s\n' "$_perm_line"
[ -z "$_limit_lines" ] || printf '%s\n' "$_limit_lines"
echo "            (each asserted on this script's own exit status, over a fixture tree)"
# …and no label-taking call ran after the label census (`_mut_census_end`, in the
# mutation set). This file's last statement: the wire makes no label-taking call
# after this file returns, so a late call is one made here, after the census.
_mut_census_end || exit 1
