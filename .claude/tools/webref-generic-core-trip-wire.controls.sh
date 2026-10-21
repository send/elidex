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
# `$_CONTROLS` (this file's own path: the entry check, the run registry's site walk in
# the run primitives and the label census in the correspondence part read it; the
# defined-labels check does not), the five `CONTROL_*` sample strings. That list — and only that list — is checked
# at entry. (No part calls the wire's `_git`, so the entry check no longer asks
# for it: a tracer over a full run counted 0 calls.)
# ⚠ WHAT THIS FILE DEFINES IS NOT AN INTERFACE, and a previous revision said it
# was: it named `$CTL`, `_control`, a per-run env global, two summary-line globals
# and `ctl_ok` "for the wire to read", and the wire reads NONE of
# them (`grep -c '\$CTL\|ctl_ok\|\b_control\b'`
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
# a mutation record (the census in the correspondence part counts the `_*_lbl` variables
# the wire or a part SPELLS as well as the labels the run's calls registered).
# What each asserts is tabled in docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §4.
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
# (the census: the run registry, bijected with this file's call lines, and the
# expanded (orphans) or spelled (the ratchet) `_*_lbl` variables, against the records — in the correspondence part; the
# records whose needle this is pin it)
_cen_lbl="every label the run used is named by a mutation record"
# …the remaining labels, whose producer is not a `_control` — the relcwd and fsmonitor
# blocks, the defined-labels check, the standing checks (their producers are
# below), the table check (`$_capt_lbl`, `_cap_check`), the capability groups' and
# the limit's labels (`$_fifo_lbl`, `$_perm_lbl`, `$_pfl_lbl`: the table's group and
# `limit` rows hold them), the orderly-end check (in the wire) and the anchor rule's self-test
# (`$_af_lbl`, in the correspondence part) —, defined HERE so the check that follows can ask for every label at once:
_rel_lbl="a relative scratch dir is removed on exit"
_fsm_lbl="a caller's fsmonitor hook does not run"
_lbd_lbl="every label the parts use is defined"
_ord_lbl="the run ends only through a verdict"
_wm_lbl="only the watchdog's marker makes a signal status a timeout"
_cap_lbl="the gate declines exactly the members of a group whose flag is 0"
_capt_lbl="the capability table is well formed and names every absence it is asked about"
_fifo_lbl="this machine holds a FIFO, or its caller declared that it cannot"
_perm_lbl="this user cannot read a mode-000 file, or its caller declared that it can"
_pfl_lbl="this env lists its environment NUL-separated, or its caller declared that it cannot"
_ev_lbl="a previous run's channel evidence never answers for the next"
_og_lbl="a call's process groups end with it and hold none of its caller's streams"
_af_lbl="a record's anchor is fresh only if it changes one line, once, and keeps what it does not read"
# THE PARTS, SPELLED ONCE: every file beside the wire, under the name a copy
# derives from its own `$SELF` (`<wire>.<part>.sh`). Read from this list: the
# label check below, in the correspondence part the sibling guard, and in the
# mutation set its entry check, the per-run copies and `_mut_rm_copies` (the trap and the end of the run), and the
# derivation of `_MUT_TARGETS` (the prefix words), which `_mut_parse` and `_mut_restore_copies` read.
_MUT_PARTS="controls harness ctlrun mutations correspondence records fixtures mutgen"
# …AND THE PATH A PART HAS, spelled once too: `_mut_part_path <part>` sets `_mut_pp` to
# `<wire stem>.<part>.sh`, derived from the CALLER'S OWN `$SELF` (so a `.mutant.<pid>` copy finds
# its own copies), and `_mut_pp_pre` / `_mut_pp_suf` to the two halves around the part name: a glob
# over the parts is `"$_mut_pp_pre"*"$_mut_pp_suf"` (the stem stays quoted, so a metacharacter in the
# checkout's path is literal). An optional second argument is the STEM to use instead of the caller's
# own (`${SELF%<ext>}`): the mutation run names its per-run copies through it, so what the run writes
# and what a copy's own helper looks for are one rule (`_mut_pp_ext` is the wire's extension, which the
# stem strips and the run's copy of the wire carries). Every site goes through it; the wire alone
# spells the path itself (it finds this file before this helper exists).
_mut_part_path() { _mut_pp_ext=".sh"; _mut_pp_pre="${2:-${SELF%"$_mut_pp_ext"}}."; _mut_pp_suf=".sh"; _mut_pp="$_mut_pp_pre$1$_mut_pp_suf"; }
# THE HARNESS — how a control runs — LIVES BESIDE THIS FILE; this file is which
# controls exist. Its absence ends the run at "decided nothing", as this file's
# own absence does in the wire.
_mut_part_path harness; _HARNESS="$_mut_pp"
if [ ! -r "$_HARNESS" ]; then
  echo "!! the control harness beside these controls ($_HARNESS) is missing or" >&2
  echo "   unreadable, so no control here can run. This run decided nothing." >&2
  exit 2
fi
# ⚠ EVERY LABEL A PART REFERENCES IS DEFINED AND NON-EMPTY, asked before anything
# else runs. A label renamed here and left in a producer is reached only when
# that producer fires, so the rename can sit unseen behind a green run: the names
# are derived from the text, by the regex the harness uses to forward the
# postconditions' labels, not listed. The files read are the wire and every part of
# `_MUT_PARTS` (the wire spells `_ord_lbl`'s one expansion: the orphan check's population is
# a subset of what is asserted here, the ratchet's is this list; a label variable whose name is never
# spelled, built by `printf -v` or `eval`, is the one thing neither sees: none exists). A part that is not readable is skipped here and reported by its
# own existence check: controls - by the wire, before this file runs; harness -
# just above; fixtures - below, after the harness is sourced; ctlrun - at the
# end of the harness; mutations - at the end of this file; correspondence, records and
# mutgen - in the mutation set.
_lbd_files=("$SELF")
for _lbd_p in $_MUT_PARTS; do
  _mut_part_path "$_lbd_p"; _lbd_f="$_mut_pp"
  [ ! -r "$_lbd_f" ] || _lbd_files+=("$_lbd_f")
done
_lbd_names="$(grep -ho '_[A-Za-z0-9_]*_lbl' "${_lbd_files[@]}" | sort -u)" || _lbd_names=""
_lbd_missing=""
for _lbd_n in $_lbd_names; do
  [ -n "${!_lbd_n:-}" ] || _lbd_missing="$_lbd_missing $_lbd_n"
done
if [ -z "$_lbd_names" ]; then
  echo "!! CONTROL FAILED ($_lbd_lbl): no label was found in the files read (${_lbd_files[*]}: the wire, then each readable part), so none was checked" >&2
  exit 2
elif [ -n "$_lbd_missing" ]; then
  echo "!! CONTROL FAILED ($_lbd_lbl): used but not defined, or empty:$_lbd_missing" >&2
  exit 2
fi
# THE CAPABILITY TABLE: which controls need a capability this machine may lack. Declared
# HERE, AFTER the label check above (it expands the group labels that check asked about)
# and BEFORE the harness is sourced (ctlrun's source-time guard reads it); read only
# through ctlrun's `_cap_rows` (plan §2.2). Rows, TAB-separated, no empty field: `group`
# (name, the harness probe's flag variable, label, why absent, the claim its controls
# support), `member` (group, a `_control` label spelled literally), `limit` (a P-x ID a
# probe answers, label, why), `fact` (a P-x ID whose limit is decided, why).
_CAP_TABLE="group	fifo	_fifo_ok	$_fifo_lbl	this filesystem holds no FIFO	an entry git cannot store neither hangs nor hides a verdict, and a tracked path replaced by one is not opened
member	fifo	a tracked path replaced by a FIFO is not opened
member	fifo	an unstorable entry neither hangs nor hides
group	perm	_perm_ok	$_perm_lbl	this user can read a mode-000 file	an unreadable file or an unsearchable directory fails closed, and a restrictive umask decides nothing
member	perm	an unreadable file fails closed
member	perm	an unsearchable dir fails closed
member	perm	a tracked file under an unsearchable dir is not absent
member	perm	an unresolvable self-test root decides nothing
member	perm	a restrictive umask decides nothing
limit	P-f	$_pfl_lbl	this env refuses -0
fact	P-b	keyed on git's documented 2.42 boundary (CR-S3), not a probe"
# shellcheck source=/dev/null
. "$_HARNESS"

# THE FIXTURE BUILD LIVES BESIDE THIS FILE: that one builds the trees, this one
# asserts over them. Its absence ends the run at "decided nothing", as the
# harness's does. It is NOT sourced here: the harness runs it in the fixture
# build window (`_fgit_window`), a child built from nothing, and checks the
# window's postconditions inside it.
_mut_part_path fixtures; _FIXTURES="$_mut_pp"
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
if ! _VFY="$(mktemp -d "$SCRATCH/verifyXXXXXX")" || [ -z "$_VFY" ] || [ ! -d "$_VFY" ] || ! mkdir "$_VFY/.reap"; then
  echo "!! could not create the verifier's directory (TMPDIR/disk?); decided nothing." >&2
  exit 2
fi
# The run's REAP SET (the wire's exit path, beside `_k2_reap`) is `$_VFY/.reap`: `_ctl_run` records every group it
# launches there. Its identity check reads `ps -o etime=`; a machine where that cannot read this run's own pid
# decides nothing here rather than red a check later.
_K2_REAP_D="$_VFY/.reap"
_k2_age $$ 0 || { echo "!! decided nothing: the reap's identity check needs \`ps -o etime=\` (read through awk), which this machine does not give" >&2; exit 2; }


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
# THE TABLE CHECK (not a standing check: it runs no `_ctl_run` row), asked once, before
# the first gated run, each red under `$_capt_lbl`: every row parses, group names and
# member labels are unique, every member's group is declared, every flag is 0 or 1,
# every word of WEBREF_WIRE_LACKS names a group or a `limit` row, and every ID in
# `$_fw_limits` has a `limit` or `fact` row. Two reds are under their own labels:
# an ABSENCE the wire learned from a probe and its caller did not declare (a group whose
# flag is 0, a `limit` row's ID in `$_fw_limits`; plan §2.3). Returns 1 on any red; its
# call line is the writer (`[ "$ctl_ok" -eq 0 ] || exit 1` follows, before the mutation
# set and the summary).
# ⚠ WEBREF_WIRE_LACKS (read here and nowhere else, in the parent; the window is built
# from nothing and `--selftest` children do not source this file): the caller's list of
# the absences it declares, space-separated group names (`fifo`: no FIFO on this
# filesystem; `perm`: this user reads a mode-000 file, i.e. root) and `limit` IDs (`P-f`:
# an `env` that refuses -0). THE COST: a root, FIFO-less or `-0`-less contributor sets it
# once (`WEBREF_WIRE_LACKS="fifo perm P-f"`); the red line says so. A no-FIFO contributor's
# declaration does not make the opt-in mutation run green: the records that plant a FIFO
# (needle `$_pg_lbl`) end at "decided nothing" there, WRONG REASON (BS-7). An undeclared absence
# is red, never a green with a `⚠`. The probe still decides what the gate does, so a
# declaration cannot make a capable machine skip anything (a stale one runs every
# control); it decides only whether a probe's 0 is acceptable. A `fact` row (P-b) needs
# none. Mutants inherit it (`_mut_trial` does not clear it), so a declaring machine's
# mutants see the same declaration. The table is read through ctlrun's readers only.
_cap_check() {
  local _cc_bad=0 _cc_nl=$'\n' _cc_tb=$'\t' _cc_gs _cc_ms _cc_lm _cc_fs _cc_lk _cc_r _cc_w _cc_l _cc_id _cc_t
  _cc_gs="$_cc_nl"; _cc_ms="$_cc_nl"; _cc_lm="$_cc_nl"; _cc_fs="$_cc_nl"
  _cc_lk="${WEBREF_WIRE_LACKS-}"; _cc_lk="${_cc_lk//$_cc_tb/ }"; _cc_lk=" ${_cc_lk//$_cc_nl/ } "
  _cap_rows - _cap_chk_row
  _cap_rows member _cap_chk_member
  _cc_r="$_cc_lk"
  while :; do
    while [ "${_cc_r# }" != "$_cc_r" ]; do _cc_r="${_cc_r# }"; done
    [ -n "$_cc_r" ] || break
    _cc_w="${_cc_r%% *}"; _cc_r="${_cc_r#"$_cc_w"}"
    case "$_cc_gs" in *"$_cc_nl$_cc_w$_cc_nl"*) continue ;; esac
    case "$_cc_lm" in *"$_cc_nl$_cc_w$_cc_tb"*) continue ;; esac
    _cap_chk_red "WEBREF_WIRE_LACKS names \"$_cc_w\", which is no group and no limit row of the capability table"
  done
  while IFS= read -r _cc_l; do
    [ -n "$_cc_l" ] || continue
    _cc_id="${_cc_l%% —*}"
    case "$_cc_lm" in
      *"$_cc_nl$_cc_id$_cc_tb"*)
        _cc_t="${_cc_lm#*"$_cc_nl$_cc_id$_cc_tb"}"; _cc_t="${_cc_t%%"$_cc_nl"*}"
        case "$_cc_lk" in *" $_cc_id "*) ;; *) _cap_chk_absent "${_cc_t%%"$_cc_tb"*}" "limit \"$_cc_id\"" "$_cc_id" "${_cc_t#*"$_cc_tb"}" ;; esac ;;
      *) case "$_cc_fs" in *"$_cc_nl$_cc_id$_cc_nl"*) ;; *) _cap_chk_red "the window reported a machine limit \"$_cc_id\", which no limit or fact row names" ;; esac ;;
    esac
  done <<EOF_LIM
${_fw_limits:-}
EOF_LIM
  [ "$_cc_bad" -eq 0 ]
}
_cap_chk_red() { echo "!! CONTROL FAILED ($_capt_lbl): $1" >&2; _cc_bad=1; }
# $1 = the row's label, $2 = what is absent, $3 = the name that declares it, $4 = why it is absent
_cap_chk_absent() {
  echo "!! CONTROL FAILED ($1): $2 is absent on this machine ($4) and its caller did not declare it. A contributor on such a machine declares it once: WEBREF_WIRE_LACKS=\"$3\" (several names: space-separated)." >&2
  _cc_bad=1
}
_cap_chk_row() {
  if [ -n "$_cr_bad" ]; then _cap_chk_red "row \"$_cr_row\": $_cr_bad"; return 0; fi
  case "${_cr_f[0]}" in
    group)
      case "$_cc_gs" in *"$_cc_nl${_cr_f[1]}$_cc_nl"*) _cap_chk_red "group \"${_cr_f[1]}\" is declared twice" ;; esac
      _cc_gs="$_cc_gs${_cr_f[1]}$_cc_nl"
      _cap_flag_of "${_cr_f[1]}"
      case "$_cap_v" in 0|1) ;; *) _cap_chk_red "group \"${_cr_f[1]}\": its flag ${_cr_f[2]} holds \"$_cap_v\", not 0 or 1" ;; esac
      [ "$_cap_v" != 0 ] || case "$_cc_lk" in *" ${_cr_f[1]} "*) ;; *) _cap_chk_absent "${_cr_f[3]}" "group \"${_cr_f[1]}\"" "${_cr_f[1]}" "${_cr_f[4]}" ;; esac ;;
    limit) _cc_lm="$_cc_lm${_cr_f[1]}$_cc_tb${_cr_f[2]}$_cc_tb${_cr_f[3]}$_cc_nl" ;;
    fact) _cc_fs="$_cc_fs${_cr_f[1]}$_cc_nl" ;;
    member)
      case "$_cc_ms" in *"$_cc_nl${_cr_f[2]}$_cc_nl"*) _cap_chk_red "member label \"${_cr_f[2]}\" is listed twice" ;; esac
      _cc_ms="$_cc_ms${_cr_f[2]}$_cc_nl" ;;
  esac
}
_cap_chk_member() {
  case "$_cc_gs" in *"$_cc_nl${_cr_f[1]}$_cc_nl"*) ;; *) _cap_chk_red "member \"${_cr_f[2]}\" names group \"${_cr_f[1]}\", which no group row declares" ;; esac
}
_cap_check || ctl_ok=1
# THE STANDING CHECKS BELOW run `_ctl_run` over `$_VFY` (a root that
# always exists: their stubs read no tree, so they ask no fixture question) in
# a subshell. Each opens by reading `_fw_trusted` — untrusted is red
# (`ctl_ok=1`) with no line of its own: W2 is the gate's report, printed once
# by the first gated run, and these four reads decide nothing about any run.
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
# which must be dead after the call (the child's group stop: a bounded poll,
# `_wm_gone`, 20 × 0.05 s, as a killed process can stay visible to `kill -0` until
# its reparenting reaper collects it), and after each row the call's end must have
# confirmed the watchdog's group (`_ctl_wd_pg`) gone: `_ctl_left` does not name it.
# The hung run is sent TERM before its KILL (the stop procedure), so its own exit
# path runs: the hang script traps TERM and leaves `<script>.term`, asserted when
# a TERM can reach a child of this run (`_wm_tg`: a `sh` that signals itself exits
# 0 only when TERM was ignored at the run's entry, which its children inherit). A row
# with no arm below is red, as `_cap`'s is. Two rows, ONE call line: the
# registry's site is the line, and one label has one site. The subshell writes
# `$_VFY/.wm_end` once it has its verdict (green or red); the `then` branch reds under the
# label when it is absent, as an abort inside the subshell would otherwise red the run unlabelled.
printf '%s\n' 'kill -KILL $$' > "$_VFY/.wm_abort"
printf '%s\n' 'trap '\'': > "$0.term"; exit 143'\'' TERM; sleep 5 & echo $! > "$0.pid"; wait' > "$_VFY/.wm_hang"
command rm -f "$_VFY/.wm_end"
{ sh -c 'kill -TERM $$; exit 0'; } 2>/dev/null && _wm_tg=0 || _wm_tg=1
if [ "$_fw_trusted" -ne 1 ]; then ctl_ok=1
elif ! ( _wm_bad=""
         _wm_gone() { _wm_i=0; while kill -0 "$1" 2>/dev/null; do
             [ "$_wm_i" -lt 20 ] || return 1; _wm_i=$((_wm_i + 1)); sleep 0.05; done; }
         for _wm_row in abort hang; do
           SELF="$_VFY/.wm_$_wm_row"; [ "$_wm_row" != hang ] || _ctl_wd_s=1
           _wm_r=0; _ctl_run "$_wm_lbl" "$_VFY" > /dev/null 2> "$_VFY/.wm_err" || _wm_r=$?
           _wm_msg="$(cat "$_VFY/.wm_err")"
           case " ${_ctl_left-} " in *" $_ctl_wd_pg "*) _wm_bad="$_wm_bad; $_wm_row: the watchdog's process group outlived the call" ;; esac
           case "$_wm_row" in
             abort)
               [ "$_wm_r" -eq 0 ] || _wm_bad="$_wm_bad; the run returned $_wm_r, not 0"
               [ "${_rc:-0}" -ge 128 ] || _wm_bad="$_wm_bad; _rc is ${_rc:-unset}, not a signal status"
               case "$_wm_msg" in *"killed after "*) _wm_bad="$_wm_bad; a signal at once was reported as a timeout" ;; esac ;;
             hang)
               [ "$_wm_r" -eq 1 ] || _wm_bad="$_wm_bad; a run the watchdog killed returned $_wm_r, not 1"
               [ -e "$_VFY/.control_timeout" ] || _wm_bad="$_wm_bad; the watchdog wrote no marker"
               [ "$_wm_tg" -eq 0 ] || [ -e "$_VFY/.wm_hang.term" ] || _wm_bad="$_wm_bad; the hung run was sent no TERM before its KILL, so its own exit path could not run"
               case "$_wm_msg" in *"killed after ${_ctl_wd_s}s"*) ;; *) _wm_bad="$_wm_bad; the kill was not reported as a ${_ctl_wd_s} s timeout" ;; esac
               _wm_d="$(cat "$_VFY/.wm_hang.pid" 2>/dev/null)" || _wm_d=""
               case "$_wm_d" in ''|*[!0-9]*) _wm_bad="$_wm_bad; the hung run recorded no descendant pid" ;;
                 *) _wm_gone "$_wm_d" || _wm_bad="$_wm_bad; the hung run's descendant ($_wm_d) outlived the kill" ;; esac ;;
             *) _wm_bad="$_wm_bad; no arm for row \"$_wm_row\"" ;;
           esac
         done
         : > "$_VFY/.wm_end"
         [ -z "$_wm_bad" ] || { echo "!! CONTROL FAILED ($_wm_lbl)$_wm_bad" >&2; exit 1; } ); then
  [ -e "$_VFY/.wm_end" ] || echo "!! CONTROL FAILED ($_wm_lbl): the check ended before its verdict" >&2
  ctl_ok=1
fi
command rm -f "$_VFY/.wm_end" "$_VFY/.wm_abort" "$_VFY/.wm_hang" "$_VFY/.wm_hang.pid" "$_VFY/.wm_hang.term" "$_VFY/.wm_err" "$_VFY/.control_timeout"
# STANDING CHECK of the evidence directory's freshness (`_ctl_run` makes it
# afresh, `rm -rf` then `mkdir`, for every call that delivers a channel): a
# `path` a previous run left must not answer for this one. Two rows, ONE call
# line, over `$_VFY` with `SELF` a stub that calls `k2evprobe` and exits 0.
# `ran` delivers the PATH prefix `$_VFY/.ev_ran`, whose `k2evprobe` shim writes
# `path`: `_ctl_run` returns 0 and `path` is there. `stale` then delivers
# `$_VFY/.ev_stale`, which holds no `k2evprobe`, so the run calls no shim: it
# must return 1 naming "PATH prefix never reached the run" — a 0 is the first
# row's `path` answering. A row with no arm below is red, as `_cap`'s is. The subshell writes
# `$_VFY/.ev_end` once it has its verdict (green or red); the `then` branch reds under the
# label when it is absent, as an abort inside the subshell would otherwise red the run unlabelled.
command mkdir "$_VFY/.ev_ran" "$_VFY/.ev_stale"
printf '#!/bin/sh\n: > "${K2_CTL_REACHED:?}/path"\n' > "$_VFY/.ev_ran/k2evprobe"
chmod +x "$_VFY/.ev_ran/k2evprobe"
printf '%s\n' 'k2evprobe 2>/dev/null; exit 0' > "$_VFY/.ev_run"
command rm -f "$_VFY/.ev_end"
if [ "$_fw_trusted" -ne 1 ]; then ctl_ok=1
elif ! ( SELF="$_VFY/.ev_run"; _ev_bad=""
         for _ev_row in ran stale; do
           _ev_r=0; _ctl_run "$_ev_lbl" "$_VFY" "" "" "$_VFY/.ev_$_ev_row" > /dev/null 2> "$_VFY/.ev_err" || _ev_r=$?
           case "$_ev_row" in
             ran)
               [ "$_ev_r" -eq 0 ] || _ev_bad="$_ev_bad; ran: the run returned $_ev_r, not 0"
               [ -f "$_VFY/.reached/path" ] || _ev_bad="$_ev_bad; ran: the shim left no path" ;;
             stale)
               [ "$_ev_r" -eq 1 ] || _ev_bad="$_ev_bad; stale: a prefix the run never called returned $_ev_r, not 1 (the previous run's path answered)"
               case "$(cat "$_VFY/.ev_err")" in *"PATH prefix never reached the run"*) ;; *) _ev_bad="$_ev_bad; stale: the red did not name the PATH prefix" ;; esac ;;
             *) _ev_bad="$_ev_bad; no arm for row \"$_ev_row\"" ;;
           esac
         done
         : > "$_VFY/.ev_end"
         [ -z "$_ev_bad" ] || { echo "!! CONTROL FAILED ($_ev_lbl)$_ev_bad" >&2; exit 1; } ); then
  [ -e "$_VFY/.ev_end" ] || echo "!! CONTROL FAILED ($_ev_lbl): the check ended before its verdict" >&2
  ctl_ok=1
fi
command rm -rf "$_VFY/.ev_end" "$_VFY/.ev_ran" "$_VFY/.ev_stale" "$_VFY/.ev_run" "$_VFY/.ev_err" "$_VFY/.reached"
# STANDING CHECK of the gate's capability arm (`_ctl_gate`, over the table `_cap_rows`
# reads), through `_ctl_run` over `$_VFY` with `SELF` a stub that exits at once, so
# "ran" is `_rc` set. ONE call line (the registry's site is the line); each row replaces
# `_CAP_TABLE` and the synthetic flags `_zz_ok` / `_yy_ok` in the subshell (the subject is
# the gate's mechanism, not the real table), does NOT redirect `_VFY` (the label registers
# in the run's registry), and sends its stderr to `$_VFY/.cap_err`: a row's EXPECTED red
# must never reach the run's output, where it would let any unrelated red "kill" every
# `$_cap_lbl` record. `$_cap_lbl` is a member of: M `zz`; N neither (`zz`'s members a proper
# prefix and a proper superstring of it); Y `yy` (`zz` declared first); U `xx`, which no
# group row declares; F1, F2 `zz`, whose flag field is `a-b`, `1a`. Columns: table, the two
# flags, trust (`no` = `_fw_trusted=0`, `_fw2_said=1` for that row alone), channel (`X` =
# malformed), the return wanted, whether a run was made. A row the loop cannot parse, or
# that no arm below asserts, is red naming the row. The subshell writes `$_VFY/.cap_end` once it
# has its verdict (green or red); the `then` branch reds under the label when it is absent, as an
# abort inside the subshell (a flag name the reader's validation let through, `${!x-}` on bash 5.3)
# would otherwise red the run unlabelled.
printf '%s\n' 'exit 0' > "$_VFY/.cap_run"
command rm -f "$_VFY/.cap_end"
if [ "$_fw_trusted" -ne 1 ]; then ctl_ok=1
elif ! ( SELF="$_VFY/.cap_run"; _ca_bad=""; _ca_tb=$'\t'; _ca_nl=$'\n'
         _ca_gz="group${_ca_tb}zz${_ca_tb}_zz_ok${_ca_tb}zz label${_ca_tb}no zz${_ca_tb}zz claim"
         _ca_gy="group${_ca_tb}yy${_ca_tb}_yy_ok${_ca_tb}yy label${_ca_tb}no yy${_ca_tb}yy claim"
         while read -r _ca_t _ca_zz _ca_yy _ca_trust _ca_chan _ca_want _ca_ran; do
           _ca_line="$_ca_t $_ca_zz $_ca_yy $_ca_trust $_ca_chan $_ca_want $_ca_ran"
           case "$_ca_t:$_ca_zz$_ca_yy:$_ca_trust:$_ca_chan:$_ca_want:$_ca_ran" in
             [MNYU]:[01][01]:yes:-:[012]:ran|[MNYU]:[01][01]:yes:-:[012]:-|[MNYU]:[01][01]:no:-:[012]:-|M:[01][01]:yes:X:[012]:-|F[12]:[01][01]:yes:-:[012]:-) ;;
             *) _ca_bad="$_ca_bad; row \"$_ca_line\" did not parse"; continue ;;
           esac
           case "$_ca_t" in
             M) _CAP_TABLE="$_ca_gz${_ca_nl}member${_ca_tb}zz${_ca_tb}$_cap_lbl${_ca_nl}$_ca_gy" ;;
             N) _CAP_TABLE="$_ca_gz${_ca_nl}member${_ca_tb}zz${_ca_tb}${_cap_lbl%?}${_ca_nl}member${_ca_tb}zz${_ca_tb}${_cap_lbl}x${_ca_nl}$_ca_gy" ;;
             Y) _CAP_TABLE="$_ca_gz${_ca_nl}$_ca_gy${_ca_nl}member${_ca_tb}yy${_ca_tb}$_cap_lbl" ;;
             U) _CAP_TABLE="$_ca_gz${_ca_nl}member${_ca_tb}xx${_ca_tb}$_cap_lbl" ;;
             F1) _CAP_TABLE="group${_ca_tb}zz${_ca_tb}a-b${_ca_tb}zz label${_ca_tb}no zz${_ca_tb}zz claim${_ca_nl}member${_ca_tb}zz${_ca_tb}$_cap_lbl" ;;
             F2) _CAP_TABLE="group${_ca_tb}zz${_ca_tb}1a${_ca_tb}zz label${_ca_tb}no zz${_ca_tb}zz claim${_ca_nl}member${_ca_tb}zz${_ca_tb}$_cap_lbl" ;;
           esac
           _zz_ok="$_ca_zz"; _yy_ok="$_ca_yy"
           _ca_tr="$_fw_trusted"; _ca_sd="$_fw2_said"; [ "$_ca_trust" != no ] || { _fw_trusted=0; _fw2_said=1; }
           _ca_ch=""; [ "$_ca_chan" = - ] || _ca_ch="$_ca_chan"
           unset _rc; _ca_got=0
           _ctl_run "$_cap_lbl" "$_VFY" "" "" "" "" "" ${_ca_ch:+"$_ca_ch"} > /dev/null 2> "$_VFY/.cap_err" || _ca_got=$?
           _fw_trusted="$_ca_tr"; _fw2_said="$_ca_sd"
           _ca_err="$(cat "$_VFY/.cap_err")"
           [ "$_ca_got" -eq "$_ca_want" ] || _ca_bad="$_ca_bad; row \"$_ca_line\": returned $_ca_got, not $_ca_want"
           _ca_exp=""; [ "$_ca_ran" != ran ] || _ca_exp=ran
           [ "${_rc+ran}" = "$_ca_exp" ] || _ca_bad="$_ca_bad; row \"$_ca_line\": a run was$([ -n "${_rc+x}" ] || echo " not") made"
           case "$_ca_t:$_ca_trust:$_ca_chan" in
             [MNY]:yes:-) ;; # the return and the run above are the whole assertion
             M:no:-) [ -z "$_ca_err" ] || _ca_bad="$_ca_bad; row \"$_ca_line\": an untrusted window reported more than W2's one line" ;;
             U:yes:-) case "$_ca_err" in *'group "xx"'*) ;; *) _ca_bad="$_ca_bad; row \"$_ca_line\": the red did not name xx" ;; esac ;;
             F[12]:yes:-) case "$_ca_err" in *'group "zz"'*) ;; *) _ca_bad="$_ca_bad; row \"$_ca_line\": the red did not name zz" ;; esac ;;
             M:yes:X) case "$_ca_err" in *"malformed channel argument"*) ;; *) _ca_bad="$_ca_bad; row \"$_ca_line\": the red did not name the malformed channel" ;; esac ;;
             *) _ca_bad="$_ca_bad; row \"$_ca_line\": no arm" ;;
           esac
         done <<EOF_CAP
M 1 1 yes - 0 ran
M 0 1 yes - 2 -
N 0 0 yes - 0 ran
Y 0 1 yes - 0 ran
Y 1 0 yes - 2 -
U 0 0 yes - 1 -
M 0 1 no - 1 -
M 0 1 yes X 1 -
F1 0 0 yes - 1 -
F2 0 0 yes - 1 -
EOF_CAP
         : > "$_VFY/.cap_end"
         [ -z "$_ca_bad" ] || { echo "!! CONTROL FAILED ($_cap_lbl)$_ca_bad" >&2; exit 1; } ); then
  [ -e "$_VFY/.cap_end" ] || echo "!! CONTROL FAILED ($_cap_lbl): the check ended before its verdict" >&2
  ctl_ok=1
fi
command rm -f "$_VFY/.cap_end" "$_VFY/.cap_run" "$_VFY/.cap_err"
# STANDING CHECK of a call's process groups against its caller (the wire's exit path: the reap set, the stop, the
# watchdog's `exec`). Four rows, ONE call line, over `$_VFY`, each inside `$( ( … ) )`: the call runs in an inner
# `( … ) 9>&-` with a 5 s watchdog and its own reap set, `$_K2_REAP_D/og` (a directory inside the run's, which the
# run's end reads too), and sends fd 9 into the substitution (`9>&1`, before its `> /dev/null`), whose output the
# rows read. The inner starts with fd 9 closed: bash 3.2 saves a redirected fd 9 at fd 10 without close-on-exec.
# kill: the stub waits until both groups' entries hold their content, then SIGKILLs the shell running the call;
# the substitution must end at once (under 3 s), not when the watchdog's sleep ends (a watchdog that kept fd 9 or
# a saved stream holds it), and the reap must stop what the killed call left. norec: the reap set is a missing
# directory, so the first record fails: the call must end 2, name the group on fd 9, and leave nothing in it.
# nodel: the stub turns its own entry into a directory, which `rm -f` cannot remove, root included: the call must
# end 2 and name the group. sig: an outer shell holds the wire's traps (`_og_trap`) and the run's reap set; the stub
# TERMs it, then the inner, which dies by the default action, so the outer's exit path must read the nested set:
# 143 with its line, its scratch dir gone, both groups gone. It stands down when TERM cannot reach a child (`_wm_tg`).
# Each row starts, and the check ends, with `rm -rf` of the nested set, which removes a live entry only after a red
# row: each row confirms its groups gone first (kill and sig by `_og_gone`, norec its one group, nodel by the call's end).
printf '%s\n' 'd="$2/.reap/og"; until n=0; for f in "$d"/*; do [ ! -s "$f" ] || n=$((n + 1)); done; [ "$n" -ge 2 ]; do sleep 0.05; done; kill -KILL $PPID' > "$_VFY/.og_kill"
printf '%s\n' 'sleep 5 & wait' > "$_VFY/.og_norec"
printf '%s\n' 'e="$2/.reap/og/$$"; while [ ! -f "$e" ]; do sleep 0.05; done; rm -f "$e"; mkdir "$e"' > "$_VFY/.og_nodel"
printf '%s\n' 'd="$2/.reap/og"; until n=0; for f in "$d"/*; do [ ! -s "$f" ] || n=$((n + 1)); done; [ "$n" -ge 2 ]; do sleep 0.05; done; kill -TERM "$(cat "$2/.og_outer")"; kill -TERM $PPID; sleep 5' > "$_VFY/.og_sig"
command rm -rf "$_VFY/.og_end" "$_K2_REAP_D/og" "$_VFY/.og_scr"
if [ "$_fw_trusted" -ne 1 ]; then ctl_ok=1
elif ! ( _og_bad=""
         _og_gone() { _og_i=0; while kill -0 "$1" 2>/dev/null; do
             [ "$_og_i" -lt 40 ] || return 1; _og_i=$((_og_i + 1)); sleep 0.05; done; }
         _og_trap() { SCRATCH="$_VFY/.og_scr"; mkdir -p "$SCRATCH"; exec 9>&1; sh -c 'echo $PPID' > "$_VFY/.og_outer"; _k2_traps; }
         for _og_row in kill norec nodel sig; do
           [ "$_og_row" != sig ] || [ "$_wm_tg" -eq 1 ] || continue
           command rm -rf "$_K2_REAP_D/og"; mkdir "$_K2_REAP_D/og"; SELF="$_VFY/.og_$_og_row"; _og_t0=$SECONDS
           _og_rd="$_K2_REAP_D/og"; [ "$_og_row" != norec ] || _og_rd="$_VFY/.og_none"
           _og_st="$( ( _ctl_wd_s=5; [ "$_og_row" != sig ] || _og_trap; ( _K2_REAP_D="$_og_rd"; _ctl_run "$_og_lbl" "$_VFY" 9>&1 > /dev/null 2> "$_VFY/.og_err"; echo "returned $?" ) 9>&- ) 2>/dev/null; echo "ended $?" )"
           _og_dt=$((SECONDS - _og_t0))
           case "$_og_row" in
             kill)
               [ "$_og_dt" -lt 3 ] || _og_bad="$_og_bad; kill: a group of the call held its killed caller's stream for $_og_dt s"
               ( _K2_REAP_D="$_og_rd"; _k2_reap )
               for _og_f in "$_og_rd"/*; do _og_p="${_og_f##*/}"; case "$_og_p" in ''|*[!0-9]*) continue ;; esac
                 _og_gone "-$_og_p" || _og_bad="$_og_bad; kill: the reap left group $_og_p running"; done ;;
             norec)
               case "$_og_st" in *"ended 2") ;; *) _og_bad="$_og_bad; norec: the call did not end 2 ($_og_st)" ;; esac
               case "$_og_st" in *"could not record process group "*) _og_p="${_og_st##*process group }"; _og_p="${_og_p%%,*}" ;; *) _og_p="" ;; esac
               case "$_og_p" in ''|*[!0-9]*) _og_bad="$_og_bad; norec: no line named the unrecorded group" ;;
                 *) _og_gone "-$_og_p" || _og_bad="$_og_bad; norec: the unrecorded group $_og_p was left running" ;; esac ;;
             nodel)
               case "$_og_st" in *"ended 2") ;; *) _og_bad="$_og_bad; nodel: the call did not end 2 ($_og_st)" ;; esac
               case "$_og_st" in *"could not drop process group "*) ;; *) _og_bad="$_og_bad; nodel: no line named the undropped group" ;; esac ;;
             sig)
               case "$_og_st" in *"stopped by SIGTERM"*"ended 143") ;; *) _og_bad="$_og_bad; sig: the signalled call did not end 143 with its line ($_og_st)" ;; esac
               [ ! -e "$_VFY/.og_scr" ] || _og_bad="$_og_bad; sig: the signalled call's exit path left its scratch dir"
               _og_n=0; for _og_f in "$_og_rd"/*; do _og_p="${_og_f##*/}"; case "$_og_p" in ''|*[!0-9]*) continue ;; esac; _og_n=$((_og_n + 1))
                 _og_gone "-$_og_p" || _og_bad="$_og_bad; sig: the exit path left group $_og_p running"; done
               [ "$_og_n" -eq 2 ] || _og_bad="$_og_bad; sig: the reap set held $_og_n groups, not 2" ;;
             *) _og_bad="$_og_bad; no arm for row \"$_og_row\"" ;;
           esac
         done
         command rm -rf "$_K2_REAP_D/og"
         : > "$_VFY/.og_end"
         [ -z "$_og_bad" ] || { echo "!! CONTROL FAILED ($_og_lbl)$_og_bad" >&2; exit 1; } ); then
  [ -e "$_VFY/.og_end" ] || echo "!! CONTROL FAILED ($_og_lbl): the check ended before its verdict" >&2
  ctl_ok=1
fi
command rm -rf "$_VFY/.og_end" "$_VFY/.og_kill" "$_VFY/.og_norec" "$_VFY/.og_nodel" "$_VFY/.og_sig" "$_VFY/.og_scr" "$_VFY/.og_outer" "$_VFY/.og_err" "$_K2_REAP_D/og" "$_VFY/.control_timeout"
# A mode a fixture sealed and the window could not apply (or refused) is red
# here, with its own label: the controls gated on that mode having taken effect
# would otherwise be skipped as if this machine could not enforce it.
if [ -n "$_fw_seal_bad" ]; then
  echo "!! CONTROL FAILED ($_fws_lbl): $(printf '%s' "$_fw_seal_bad" | tr '\n' ';')" >&2
  ctl_ok=1
fi
# A postcondition this machine cannot evaluate is a machine limitation: P-b's
# (`git var` before git 2.42) is a FACT, keyed on git's documented boundary, so it is
# reported below and green; P-f's (an `env` that refuses -0) is a probe's answer, so
# it is green only where its caller declared it (WEBREF_WIRE_LACKS, `_cap_check`) and
# red otherwise. The window writes one line per limitation, naming the postcondition
# by its ID only. THE SLOT RULE: a record's needle is the postcondition's LABEL, which
# only a diagnostic prints, in its label slot (`CONTROL FAILED (<needle>)` /
# `CONTROL NOT EXERCISED (<needle>)`); the limit line below is green and carries the
# ID alone, since a label on it would let any unrelated red "kill" that record for
# the right-looking reason.
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
# wire refusing to decide. ⚠ The first spelling of this command
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
# THE TWO FIFO CONTROLS (`fifotracked`, `odd`) are the table's `fifo` group (`_CAP_TABLE`):
# the calls run on every machine and register their labels for the census, and where
# the flag is 0 the gate declines them without a run; a declaring machine's summary
# line (`_cap_lines`) reports it, and undeclared, `_cap_check` has already redded it.
_control "$CTL/fifotracked" 1 "NOT opened" "a tracked path replaced by a FIFO is not opened" || ctl_ok=1
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
_control "$CTL/odd" 0 "PASSED" "an unstorable entry neither hangs nor hides" || ctl_ok=1
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
# not (2026-09-citation-hygiene-Ai-wire-k2-trip-wire-review-rounds.md §11.1; the note's own
# measurement is BSD only), so the real tool cannot pose this question on every
# machine. The control does not ask the machine's grep: `goptgrep` (the fixtures
# file, beside `fakegrep`) answers "no match" whenever a `GREP_OPTIONS` reached it
# and execs the real grep otherwise. It models the property under test — did the
# caller's variable reach grep? — not the value's meaning to one grep, as
# `fakegitcfg` does for `GIT_CONFIG_COUNT` (`fakerelmktemp` is not that kind: it
# returns a relative path unconditionally); so the question is posed on every
# machine and declined on none. The shim marks `path` before the test, and
# `_ctl_run` demands that mark back. ⚠ Deleting the prefix ARGUMENT below drops
# that demand with it (as relcwd's comment says): the shipped run stays green,
# and only the opt-in record that pins k2 (`unset GREP_OPTIONS`), run on a grep
# that ignores the variable (the GNU-like profile), reds it.
_control "$CTL/k2" 1 "K2: a" "a caller's GREP_OPTIONS cannot hide a file" "" "" "$CTL/goptgrep" "" "GREP_OPTIONS=--exclude=*" || ctl_ok=1
# A SIGNAL ENDS THE RUN 128+n AND SAYS SO, on the stderr the run started with. Each `sig<n>` shim (the fixtures
# file) marks `path` and signals its own group on the wire's HEAD probe; the child is `_ctl_run`'s, launched under
# `set -m`, so its SIGINT is not ignored. The row expects 0 and PASSED instead when a `sh` that signals itself
# exits 0: the signal was ignored at this run's entry, its children inherit that, and none can be stopped by it
# (a record of these rows then SURVIVES: red, never green; the opt-in starts the wire with the three at default).
# The group redirection silences bash's report of the probe's death.
for _sg in HUP:129 INT:130 TERM:143; do _sg_n="${_sg%:*}"; _sg_e="${_sg#*:}"; _sg_m="stopped by SIG$_sg_n"
  case "$_sg_n" in TERM) [ "$_wm_tg" -eq 0 ] ;; *) { sh -c "kill -$_sg_n \$\$; exit 0"; } 2>/dev/null ;; esac && { _sg_e=0; _sg_m=PASSED; }
  _control "$CTL/clean" "$_sg_e" "$_sg_m" "a signal ends the run red and says so" "" "" "$CTL/sig$_sg_n" || ctl_ok=1
done
# A SCRATCH DIR THAT CANNOT BE REMOVED is reported and the status stands: the `failrm` prefix's `rm` fails exactly
# `rm -rf <dir>` (the exit path's removal, after removing it), its `chmod` fails `chmod -R` (the exit path's only one,
# which errexit, off in the exit path, would otherwise turn into status 1).
_control "$CTL/clean" 0 "could not remove the scratch dir" "a scratch dir that cannot be removed does not change the verdict" "" "" "$CTL/failrm" || ctl_ok=1
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
# The plain call purges git's own list, `$_GIT_LOCAL_VARS` (a hand-named subset let a
# caller's GIT_OBJECT_DIRECTORY or GIT_COMMON_DIR break the call); `env` gives the one
# configuration it asks about back.
elif ( for _v in $_GIT_LOCAL_VARS; do unset "$_v"; done; cd "$_fsm_root" \
       && env "${_fsm_cfg[@]}" git ls-files >/dev/null 2>&1 ) || true
     [ ! -e "$_fsm_mark" ]; then
  echo "!! CONTROL NOT EXERCISED ($_fsm_lbl): the hook did not run under a plain" >&2
  echo "   git call either, so its not running under the wire proved nothing." >&2
  ctl_ok=1
fi
# THE FIVE PERMISSION CONTROLS are the table's `perm` group, as the FIFO pair is the
# `fifo` group. The fifth runs the clean fixture under umask 777 (`_control`'s
# `$8`): a umask that leaves the scratch dir unsearchable makes it unresolvable
# to a physical path, which the wire refuses to decide over.
_control "$CTL/err"  1 "could not be read" "an unreadable file fails closed" || ctl_ok=1
_control "$CTL/walk" 1 "could not be read" "an unsearchable dir fails closed" || ctl_ok=1
_control "$CTL/d2red" 1 "not provably absent" "a tracked file under an unsearchable dir is not absent" || ctl_ok=1
_control "$CTL/d5root" 2 "or the root to a physical path" "an unresolvable self-test root decides nothing" || ctl_ok=1
_control "$CTL/clean" 2 "could not resolve the scratch dir" "a restrictive umask decides nothing" "" "" "" 777 || ctl_ok=1
# ---- THE MUTATION SET, WHICH LIVES BESIDE THIS FILE -------------------------
# Two subjects, two files: this one runs the wire over a fixture and asserts the
# exit status; that one proves each control is about the arm it names. Its
# absence is not a skipped check — the correspondence between the two lists is
# what keeps either list honest, so the run ends at "decided nothing" rather
# than without it.
_mut_part_path mutations; _MUTATIONS="$_mut_pp"
if [ ! -r "$_MUTATIONS" ]; then
  echo "!! the mutation set beside these controls ($_MUTATIONS) is missing or" >&2
  echo "   unreadable, so nothing here was shown to be about the arm it names." >&2
  echo "   This run decided nothing." >&2
  exit 2
fi
# shellcheck source=/dev/null
. "$_MUTATIONS"
_mut_correspondence || ctl_ok=1

# (No check after the controls: every `_control` and every block that is not one
# runs through `_ctl_run` (which asks `_ctl_gate`), so none runs over an unbuilt tree;
# the fsmonitor block's plain `git ls-files` follow-up, made after `_ctl_run` returns,
# sits inside the branch that its `_ctl_run`'s ask opened.)
[ "$ctl_ok" -eq 0 ] || exit 1

_mut_run

# THE SUMMARY, one line per group in table order: any flag value but 0 states the claim,
# 0 what this run did NOT exercise (the member count from the table). `!= 0` is the
# gate's own predicate (`_ctl_gate`), so the summary and the gate cannot disagree; a
# value that is not 0 or 1 is `_cap_check`'s red. One site, one truth: the gate's decision is a function of (table,
# flag) through one lookup, which `_cap` pins, and every member reached the gate (the census).
_cap_lines() { _cap_rows group _cap_line_cb; }
_cap_line_cb() {
  local _cl_g="${_cr_f[1]}" _cl_why="${_cr_f[4]}" _cl_claim="${_cr_f[5]}" _cl_n=0
  _cap_rows member _cap_count_cb
  _cap_flag_of "$_cl_g"
  if [ "$_cap_v" != 0 ]; then printf '            %s\n' "$_cl_claim"
  else printf '            ⚠ NOT EXERCISED on this machine: %s control(s) of the %s group (%s), so this run carries no evidence that %s\n' \
    "$_cl_n" "$_cl_g" "$_cl_why" "$_cl_claim"; fi
}
_cap_count_cb() { [ "${_cr_f[1]}" != "$_cl_g" ] || _cl_n=$((_cl_n + 1)); }
echo "  controls: green reachable; K2 fires under both roots, on the removed path,"
echo "            inside binary content, on a symlink's stored target, on an entry's"
echo "            own NAME, on a symlinked entry script beside the scope, and under a"
echo "            caller's GREP_OPTIONS; an empty scope fails closed"
_cap_lines
[ -z "$_limit_lines" ] || printf '%s\n' "$_limit_lines"
echo "            (each asserted on this script's own exit status, over a fixture tree)"
# …and no label-taking call ran after the label census (`_mut_census_end`, in the
# correspondence part). This file's last statement: the wire makes no label-taking call
# after this file returns, so a late call is one made here, after the census.
_mut_census_end || exit 1
