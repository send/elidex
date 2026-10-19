#!/usr/bin/env bash
# THE MUTATION SET for `webref-generic-core-trip-wire.sh` — sourced by its
# controls, never run on its own.
#
# WHY IT IS A SEPARATE FILE. The controls answer one question — *can the wire
# reach every verdict it claims?* — by running the wire over a fixture and
# asserting its exit status. This file answers a different one: *is each control
# about the arm it names?* Those are two subjects, and the second is the layer
# that watches the first, so it is where the correspondence between the two
# lists is enforced rather than assumed.
#
# ⚠ THE SPLIT IS THE RULE'S OWN SHAPE: CLAUDE.md's touch-time discipline says a
# >1000-line file gets a "standalone prereq split" as its own PR or its own
# commit, not a defer slot to be done after the PR lands. This file is that
# standalone commit. The two lists must still be edited together — the
# correspondence check below ENFORCES that: it reads the labels the run
# registered (the run registry, and the `_*_lbl` variables) and this
# file's records, and reds when they drift.
#
# WHAT IT CONSUMES: `$_VFY` (from the controls file), the run registry
# (`$_VFY/.registry`: what the run primitives' `_ctl_reg` appended), `$_cen_lbl` (read by the
# census), `$_lbd_names` (from the controls file: the `_*_lbl` names the parts
# spell, each asserted defined and non-empty there; the census reads their
# values, and `_mut_run` the `_p<x>_lbl` and `_p<x>l_lbl` among them with the harness's
# `$_fw_limits`: the P-x half of the excuse, for a machine-limited record), `$_MUT_PARTS` (from the controls
# file, which spells the list of parts once), the capability table's READER
# (`_cap_rows`, `_cap_group_of`, `_cap_flag_of`, from the run primitives: functions
# only, asserted below by `declare -F`; neither the table nor a flag is named here, the functions answer,
# for the census's membership check and `_mut_run`'s capability excuse), and `$SELF`, `$SCRATCH`, `$K2RE`
# and `$K2RE_PATH` (from the wire); every other part it names is derived from
# `$SELF` (`${SELF%.sh}.<part>.sh`); the per-run copies iterate `_MUT_PARTS`.
# `_mut_run` copies every part in that list — the controls, the harness, its run
# primitives, this file, the record table, the fixture build and the generated
# half — beside each mutant.
# ⚠ THE TWO REGEXES ARE CONSUMED AS VALUES, not just named in a `for` list:
# `_mut_gen_run` compares what the wire's assignment LINE reads back as against
# what the RUNNING wire HOLDS, and that second half is these variables. They were
# left off this list and out of the guard by the commit that added the generator
# — the same drift the guard exists to catch, in the edit that widened what there
# is to drift.
# WHAT IT DEFINES: `_MUT_UNRECORDED_MAX`, `_MUT_RECORDS_MIN`, `_MUTGEN`,
# `_MUTRECORDS`, `_MUT_TARGETS`, `_mut_correspondence`,
# `_mut_census_end` (the controls file's last statement),
# `_mut_cap_member` (the census's membership callback, with its list `_mc_ctl` and the list's delimiter `_mc_nl`),
# `_mut_skip_glabel` (the capability excuse's callback: a needle that is a group's own label),
# `_mut_wrong_reason` (the wrong-reason diagnostic, printed by `_mut_trial` or its caller),
# `_mut_src_for`, `_mut_target`, `_mut_parse`, `_mut_apply`,
# `_mut_restore_copies`, `_mut_rm_copies`, `_mut_trial`, `_mut_run`. The
# generated half — `_mut_equivalent`, `_mut_assign_value`, `_mut_regex_mutants`,
# `_mut_splice`, `_mut_gen_floor`, `_mut_gen_run` — is in `…trip-wire.mutgen.sh`,
# and the record table — `_mutants` — in `…trip-wire.records.sh`; both are
# sourced below.
#
# TWO POPULATIONS, AND THE BOUNDARY IS WHAT EACH ONE'S UNIT IS.
#   * `_mutants` (in `…trip-wire.records.sh`) — HAND-WRITTEN, and its unit is a
#     LABEL: a label the run registered (what a `_control`, `_ctl_run`,
#     `_ctl_gate` or `_fgit_window_verdict_exit` call was passed) or the value of
#     a `_*_lbl` variable (the blocks that are not `_control`s, and the fixture
#     build window's producers). The number of labels no record names is
#     ratcheted (`_MUT_UNRECORDED_MAX` below), and the record says which edit
#     its label is about. Its claim: *THAT* label is about *THAT* edit. It is a
#     floor and cannot be a census of the wire's arms — a record exists only for
#     something somebody thought of.
#   * `_mut_gen_run` (in `…trip-wire.mutgen.sh`) — GENERATED from `$K2RE` and
#     `$K2RE_PATH` themselves, and its unit is a RULE of those two regexes: one
#     mutant per
#     bracket-expression member, one per quantifier, one per alternation
#     branch, one per escape. Its claim: *every* such rule is caught by SOME
#     control, or is argued equivalent. It cannot say which control, and a
#     direction needing a payload it would have to invent — widening a branch
#     list, say — is not in its population and stays with the hand records.
# OVERLAP IS PERMITTED, and where they overlap that is not two copies of one
# claim: a regex rule whose fixture needs a NEW CONTROL gets a record as well,
# because the control has to be named by one. ⚠ This used to say neither set is
# derivable from the other. At set level that holds; per record it does not —
# four hand records pin a group-drop or an unescape the generator now derives
# too — and the ambiguity is the kind this file exists to remove.
# ⚠ THOSE FOUR ARE ALSO EVIDENCE OF A GAP IN THE GENERATOR, and it has no
# control: a hand record existed for a group-drop and for an unescape before
# the scanner could derive either, so the data to notice that the scanner was
# walking `(`, `|` and `\` past as ordinary characters was already in this
# file, unread, for four review rounds.
# ⚠ WHAT DOES NOT BELONG HERE is a record for a regex rule that needed no new
# control — that is the generator's population, and hand-listing it would be
# the second spelling of a class this file keeps collapsing. The lines added to
# `textgreen` and `pathgreen` for the intermediate-segment and stored-path
# widenings are exactly that case: a fixture, no record, the generator the only
# thing asserting them.
# ⚠ AND IT ASSIGNS NO VARIABLE THE CALLER OWNS. `_mut_correspondence` used to set
# `ctl_ok` — a variable owned by the controls file — so a rename there would
# have left this file assigning an unused global while the caller's status
# stayed green: the exact cross-file drift the entry guard exists to prevent,
# reintroduced by the split that added the guard. It RETURNS a status now and
# the caller decides, which is a contract a rename cannot silently break.
# ⚠ Asserted at entry, for the same reason the controls file asserts its own:
# a stated interface nobody checks drifts like any other unexecuted claim.
_mut_missing=
for _n in _VFY _lbd_names _MUT_PARTS SELF SCRATCH K2RE K2RE_PATH; do
  [ -n "${!_n:-}" ] || _mut_missing="$_mut_missing \$$_n"
done
for _n in _cap_rows _cap_group_of _cap_flag_of; do
  declare -F "$_n" > /dev/null || _mut_missing="$_mut_missing $_n()"
done
if [ -n "$_mut_missing" ]; then
  echo "!! This file is the MUTATION SET for \`webref-generic-core-trip-wire.sh\`. It is" >&2
  echo "   SOURCED by that wire's controls and has no meaning alone; missing:$_mut_missing" >&2
  echo "   Run the wire instead." >&2
  exit 2
fi
# …and the generated half, which lives beside this file. Its absence is a
# missing sibling, reported as the other parts report theirs.
_MUTGEN="${SELF%.sh}.mutgen.sh"
if [ ! -r "$_MUTGEN" ]; then
  echo "!! the generated mutation half beside this mutation set ($_MUTGEN) is missing or" >&2
  echo "   unreadable, so no rule of \$K2RE or \$K2RE_PATH can be tested. This run decided nothing." >&2
  exit 2
fi
# THE GENERATED HALF — `_mut_gen_run` and what it needs — LIVES BESIDE THIS FILE,
# at the boundary this header draws between the two populations.
# shellcheck source=/dev/null
. "$_MUTGEN"
# THE RECORD TABLE — `_mutants` — LIVES BESIDE THIS FILE TOO: a flat case table
# with no seam inside it, split out when its growth would have taken this file
# past 1000 lines (CLAUDE.md touch-time split). This file keeps the READERS of
# the format; that one holds the data. Its absence is a missing sibling.
_MUTRECORDS="${SELF%.sh}.records.sh"
if [ ! -r "$_MUTRECORDS" ]; then
  echo "!! the record table beside this mutation set ($_MUTRECORDS) is missing or" >&2
  echo "   unreadable, so no record can be checked or run. This run decided nothing." >&2
  exit 2
fi
# shellcheck source=/dev/null
. "$_MUTRECORDS"

# ---- THE MUTATION SET ------------------------------------------------------
# WHY IT IS HERE AND NOT IN A COMMIT LOG. Every control above proves a verdict
# is REACHABLE. What it does not prove is that the verdict is PRODUCED BY THE
# CODE THE CONTROL IS ABOUT — a control can pass over an arm someone deleted, if
# another arm happens to red the same fixture (measured twice: #501 R70, where
# killing the verdict's error arm left a control green, and this slice's own
# `cachedir`, green with its force-add removed entirely).
#
# The answer is a mutation set, and the only question was where it lives. The
# population used to be PROSE — "each fix this wire's history names", scattered
# across review commits on #501, which that PR's squash merge ERASES. A
# criterion whose population disappears when the parent lands is not a
# criterion. So the set lives HERE, machine-readably, in the file that ships
# with the thing it is about:
#
#   one record per line, TAB-separated:
#       <sed expression 1> [TAB <sed expression 2> …] TAB <label the run must print in a diagnostic's label slot>
#
# THE NEEDLE IS THE LAST TAB FIELD; every field before it is ONE sed expression,
# and the run applies them together as `sed -e <expr 1> -e <expr 2> …`. A record
# that needs two edits (a range-scoped one and an unscoped one, say) is therefore
# two fields, never `expr1;expr2` in one — because the always-on anchor check
# (below) applies EACH expression alone and requires it to change EXACTLY ONE
# line of its target. A `;`-joined pair would be checked as one expression, and
# one half going stale (a renamed function, a respelled line) would leave the
# other half "fresh". The `controls:` / `harness:` / `ctlrun:` / `fixtures:` / `mutgen:` / `mutations:` prefix (below) is
# written on the FIRST expression and aims the whole record.
# ⚠ SIZE RULE, DISCHARGED: this file's growth was the record here-document, a
# flat case table with no seam inside it, and the rule here read "the edit that
# takes this file past 1000 lines FIRST splits the record table into its own
# part". That split is done: the table is `…trip-wire.records.sh` (sourced
# above), and what stays here is the format's readers, which have no reason to
# grow with the record count.
#
# The expressions are applied to a COPY of the file the record names — the wire,
# unless the record says otherwise (below); the copy must (a) differ from the
# original — a stale anchor that matches nothing is a FAILED entry, not a passing
# one — (b) exit non-zero, and (c) print the named control's own diagnostic, so
# an entry that reds for an unrelated reason is caught too.
#
# ⚠ AND THE FILE IT EDITS IS PART OF THE RECORD. An expression may carry the
# prefix `controls:`, `harness:`, `ctlrun:`, `fixtures:`, `mutgen:` or `mutations:`,
# which aims it at the CONTROLS, the CONTROL HARNESS, the CONTROL RUN PRIMITIVES,
# the FIXTURE BUILD, the MUTANT GENERATOR or THIS FILE (the census and the
# ratchet are its arms) instead of the wire. The run copies every shipped part
# beside each mutant, so a record says only which one to `sed`: the prefixes are
# the words of `_MUT_TARGETS`, and `_mut_target` derives each one's pair.
#
# RUN IT — it is NOT part of the gate (it costs one full control pass per entry,
# minutes rather than seconds), and a required check nobody can afford to run is
# how gates get switched off:
#
#     WEBREF_WIRE_MUTANTS=1 bash .claude/tools/webref-generic-core-trip-wire.sh
#
# ⚠ ADDING A LABEL MEANS ADDING A RECORD — a `_control` or an `_lbl="…"`
# definition, whatever it is about — the ratchet below is what makes that true.
# What does NOT belong here is a record for a rule of `$K2RE` or `$K2RE_PATH`
# that needed no new control: `_mut_gen_run` derives that population from the
# assignments themselves, so a hand-written copy would be a second spelling of
# the same class. The boundary is stated in full at the head of this file and
# again at that function.
# ⚠ Nothing here can detect an arm that never had one — this set is a floor, not
# a census. ⚠ AND A DECLARED FLOOR IS NOT A DETECTOR: this file used to say
# exactly the sentence above and assert nothing, so deleting a record shrank the
# set in silence and the run stayed green. Three things now hold it up, and each
# is checked below rather than described:
#   * a CORRESPONDENCE, IN BOTH DIRECTIONS AND ALWAYS-ON, against the labels
#     the run USED (the census below: the run registry — runtime, so a call
#     the text hides (`eval`, a split word, a command substitution) still
#     runs, and so is still registered — and the values of the `_*_lbl` names
#     the parts spell, `$_lbd_names`: a text derivation, each asserted
#     defined). A record's needle that EQUALS no such
#     label is an orphan (red; a stale anchor would otherwise report "wrong
#     reason" forever). AND the number of LABELS with NO record is ratcheted:
#     `_MUT_UNRECORDED_MAX` is EXACT: more bare labels is red, and so are fewer.
#     ⚠ THE SECOND DIRECTION IS THE ONE THAT CATCHES ANYTHING. The first has
#     never had a violation; the second is where both real gaps lived — the
#     `ls-tree` arm with no record, and a control whose fixture made its
#     mutation inert. A count-only floor was tried first and is gone: it could
#     not distinguish "one deleted, one added" from "unchanged", and it said
#     nothing about WHICH control had been left bare. The gap is the quantity
#     with meaning, so the gap is what is ratcheted.
#   * a STANDING NEGATIVE CONTROL — one record whose expected outcome is
#     SURVIVAL (`!survive`), so a harness that cannot distinguish a real kill
#     from an environment failure reds. That distinction is not hypothetical:
#     an inert entry run ONCE is what caught this harness reporting 18/18 when
#     every mutant was in fact exiting 126 on a permission bit, and a control
#     that is not standing cannot catch it twice.
#     ⚠ ITS EDIT IS A COMMENT, AND THAT IS A REQUIREMENT, not a convenience.
#     The first version appended a space to an `rm -f` argument list — inert,
#     but only ACCIDENTALLY so, and that particular `rm` is itself dead (the
#     scratch root's trap removes the whole directory). A negative control whose
#     inertness depends on the current behaviour of the code it edits stops
#     being a negative control the moment that code changes, silently. Comment
#     text cannot alter a verdict by construction.
#   * A RECORD FLOOR AND A STANDING-CONTROL CHECK, because the two directions
#     above are both blind to a DELETED record: several needles are named by
#     more than one record, so removing one leaves its control covered, and the
#     `!survive` entry names no control at all — nothing looked for it.
#     ⚠ WHAT IT DOES NOT SEE: a deletion and an addition in one edit. The set
#     shrinking is what is caught; the added record still has to kill.
# WHICH FILES A RECORD MAY EDIT, BESIDES THE WIRE: the prefix parts, spelled
# once. A record with no prefix edits the wire. `wire` is not a prefix: POSIX `w`
# takes a wfile operand, so `wire:p` is a VALID write to a file called `ire:p`
# (measured: BSD sed exits 0), and a record beginning with it must not be
# re-aimed. A prefix that is not listed here reaches `sed` with the prefix
# attached, where it fails to parse (measured on BSD sed: `sed 'harness:p'` is
# "extra characters at the end of h command", `sed 'fixtures:p'` is "invalid
# command code f"; GNU sed is not measured here).
_MUT_TARGETS="controls harness ctlrun fixtures mutgen mutations"
# EXACT, both ways (below); the "more" arm lists the bare labels.
# 18 -> 21 at CR-B2a (the group and limit labels `$_fifo_lbl`, `$_perm_lbl`, `$_pfl_lbl` were held by the capability
# table without a record) and back to 18 at CR-B2b: their probe-flip records (the absence check they pin) landed.
_MUT_UNRECORDED_MAX=18
# The record floor; current totals: 2026-09-citation-hygiene-k2-fgit-hermetic-corpus.md §6.1.
# Records this PR removed: `db2b0af1` the `_pg_declared` record (the declared-dir list went);
# `523ce31f` P-j's relative-PATH-entry record (no per-entry clause left to pin); `15d3e704` P-g's two
# line-`grep` failure records (the record comparison decides); `96eea58c` the pin's re-entry-limit
# record (it went with the counter); `2b56c33c` P-k's `grep` exit>1 record (the builtin
# `_fgit_alternates` has no such arm); `116a9b8b` the `fakerelmktemp` fixture-failure record (the gate
# no longer reads PATH-prefix dirs); `8c952d6d` the `_ctl_env` clear record
# (`ctlrun:/^  _ctl_env=()$/d`; the global went); `fd90b4b1` the two drop-prefix records (relcwd, cfgkept: a
# call-site drop is the opt-in records' class); the CR-S3 slice the four P-b definition-test records (stdout, hangup
# line, the control failing, git's top-level usage: the test went with the version-keyed limit).
# The CR-B1 slice removed k2's prefix record (k2 is posed on every machine, so the prefix went); the
# CR-B2a slice the two records of the prefix that followed (the FIFO pair's, the last ask's): the table replaced it. CR-B2a's review fixes added six (the table check's flag, member and group clauses, and the three
# standing checks' verdict markers): 231 -> 237. CR-B2b added three (the FIFO and permission probes' flips, P-f's
# definition test forced to the limit branch): 237 -> 240.
_MUT_RECORDS_MIN=240

# The resolver: $1 = `wire` or a word of `$_MUT_TARGETS`; out = `$_mut_src` (the
# shipped file) and `$_mut_tgt` (the mutable copy `sed` writes and `_mut_trial`
# compares) — `_mut_run`'s per-run names, derived rather than listed per part.
_mut_target() {
  _mut_src_for "$1"
  case "$1" in
    wire) _mut_tgt="$_mut_wire" ;;
    *)    _mut_tgt="$_mut_base.$1.sh" ;;
  esac
}
# …its first half, usable without a per-run `$_mut_base` (the always-on anchor
# check runs on every run, a mutation run's copies only on the opt-in one): the
# SHIPPED file a name stands for. `_mut_target` and the anchor check (on the
# `_pr_which` `_mut_parse` sets) both go through it, so there is one place a
# name becomes a path.
_mut_src_for() {
  case "$1" in
    wire) _mut_src="$SELF" ;;
    *)    _mut_src="${SELF%.sh}.$1.sh" ;;
  esac
}

# THE ONE PARSER OF A RECORD — the always-on checks and `_mut_run` both read a
# record through it, so the format has one reader. $1 = one line of `_mutants`.
# Sets `_pr_n` (how many expressions), `_pr_expr[0.._pr_n-1]` (with the target
# prefix stripped from the first), `_pr_needle` (the last TAB field), `_pr_which`
# (`wire` or a word of `$_MUT_TARGETS`) and `_pr_show` (the expressions as
# written, for a diagnostic). It sets nothing else: the file a record edits is
# `_mut_src_for "$_pr_which"`, asked by the caller. A line with no TAB has
# `_pr_n` = 0 and is not a record.
# ⚠ ANCHORED ON THE KNOWN NAMES, NEVER A GENERIC `<word>:` PARSE:
# `sed 'y/a:/b;/'` is a valid expression whose `${x%%:*}` is `y/a`, so a parse
# would re-aim a record at a target that does not exist.
_mut_parse() {
  _pr_rest="$1"; _pr_n=0; _pr_expr=(); _pr_needle=""; _pr_which=wire; _pr_tab=$'\t'
  while :; do
    case "$_pr_rest" in
      *"$_pr_tab"*) _pr_expr[_pr_n]="${_pr_rest%%"$_pr_tab"*}"; _pr_n=$((_pr_n + 1))
                    _pr_rest="${_pr_rest#*"$_pr_tab"}" ;;
      *)            _pr_needle="$_pr_rest"; break ;;
    esac
  done
  _pr_show=""
  if [ "$_pr_n" -gt 0 ]; then
    _pr_show="${1%"$_pr_tab$_pr_needle"}"
    for _pr_t in $_MUT_TARGETS; do
      case "${_pr_expr[0]}" in
        "$_pr_t":*) _pr_which="$_pr_t"; _pr_expr[0]="${_pr_expr[0]#"$_pr_t":}"; break ;;
      esac
    done
  fi
}

# The parsed record applied to a file: every expression, as one sed script.
# $1 = the file to read, $2 = the file to write. Needs `_mut_parse` to have run.
_mut_apply() {
  _ap_in="$1"; _ap_out="$2"; _ap_i=0; set --
  while [ "$_ap_i" -lt "$_pr_n" ]; do
    set -- "$@" -e "${_pr_expr[$_ap_i]}"; _ap_i=$((_ap_i + 1))
  done
  sed "$@" "$_ap_in" > "$_ap_out" 2>/dev/null
}

# EVERY MUTABLE COPY BACK TO THE SHIPPED BYTES, before an entry is applied, so no
# entry runs against another entry's edit. Both loops call this.
# ⚠ IT OWNS THE REPORT: a `cp` that fails on a disk or a permission bit is said
# to be that, not counted as a mutant that was not killed.
# ⚠ A SUBSHELL, so restoring does not overwrite the pair its caller resolves.
_mut_restore_copies() (
  for _mut_t in wire $_MUT_TARGETS; do
    _mut_target "$_mut_t"
    cp "$_mut_src" "$_mut_tgt" || {
      echo "!! the shipped $_mut_t could not be re-copied to \"$_mut_tgt\", so the entry about" >&2
      echo "   to run would have run against another entry's edit. This is a disk," >&2
      echo "   permission or path failure, NOT a gap in the mutation population." >&2
      return 1; }
  done
)

# Every per-run copy, removed: by the trap on an early exit, and at the end.
_mut_rm_copies() {
  command rm -f "$_mut_wire"
  for _mp in $_MUT_PARTS; do command rm -f "$_mut_base.$_mp.sh"; done
}

# ONE TRIAL, SHARED BY BOTH POPULATIONS. The mutated copy is already at
# `$_mut_tgt` and the file it came from at `$_mut_src` — whichever target the
# caller resolved, which for an entry with a `_MUT_TARGETS` prefix is NOT the
# wire; it is the wire copy that gets EXECUTED either way.
# $1 = how to name it in a diagnostic, $2 = what is required:
# a NEEDLE (a full label) the output must hold in a diagnostic's label slot,
# `CONTROL FAILED (<needle>)` or `CONTROL NOT EXERCISED (<needle>)`, `!survive`,
# or `!kill` (red, with `CONTROL FAILED (<label>)` for the label of some
# `_control`, none named).
# Returns 0 = as required, 1 = SURVIVED (the caller decides what that means),
# 2 = failed some other way, already reported, 3 = killed for the WRONG REASON by
# a mutant that exited 1 (NOT yet reported: the caller excuses it, for a record
# whose needle is a member of an absent group or that group's own label, or reports it through
# `_mut_wrong_reason`). A mutant that exited 2 ended at "decided nothing", which a
# harness break also reports, so that case is a 2 and never excused.
# The `!kill` and `!survive` forms return 0, 1 or 2 only.
_mut_wrong_reason() { # $1 = how to name it, $2 = the needle; reads the mutant's `$_mt_rc`
  echo "!! MUTANT $1 killed for the WRONG REASON (exit $_mt_rc): the output" >&2
  echo "   holds no \"CONTROL FAILED ($2)\" or \"CONTROL NOT EXERCISED ($2)\" (the" >&2
  echo "   needle in a diagnostic's label slot), so another check masked the one under test." >&2
}
_mut_trial() {
  # ⚠ THE MUTANT RUNS UNDER THE DRIVER'S OWN BASH (`$BASH`), as every control
  # child does, so a run driven by bash 3.2 exercises 3.2 end to end rather than
  # whatever `bash` `PATH` names. Nothing is executed through its mode bit.
  # ⚠ THE PAIR COMPARED IS THE FILE THIS ENTRY EDITS, not always the wire. With
  # `$SELF` hard-coded here, a `harness:` entry that matched nothing compared two
  # files that are never equal, so "MATCHED NOTHING" could not fire for it — the
  # one check that catches a stale anchor, blind on the target it was added for.
  if cmp -s "$_mut_tgt" "$_mut_src"; then
    echo "!! MUTANT $1: the edit MATCHED NOTHING, so this entry tested a copy" >&2
    echo "   identical to the file it edits." >&2
    return 2
  fi
  _mt_rc=0
  # (The mutant inherits every other variable, WEBREF_WIRE_LACKS among them: a declaring
  # machine's mutants see the same declaration, so a probe-flip mutant SURVIVES there (an absence the caller
  # declared is accepted, and `_mut_run` excuses that record: by the table's flag for the FIFO and permission flips, by the P-x half for
  # P-f's); it reds under its group's (or the limit's) label only if the inheritance is cut.)
  _mt_out="$(env -u WEBREF_WIRE_MUTANTS "$BASH" "$_mut_wire" 2>&1)" || _mt_rc=$?
  if [ "$2" = '!survive' ]; then
    # THE STANDING NEGATIVE CONTROL. Its edit is to COMMENT TEXT, so it
    # cannot change any verdict by construction and the wire MUST still be
    # green. If it reds, the harness is failing mutants for a reason that has
    # nothing to do with the mutation — a missing execute bit, a clobbered
    # copy, a full disk — and every "killed" above is unearned.
    [ "$_mt_rc" -ne 0 ] || return 0
    echo "!! THE NEGATIVE CONTROL DIED (exit $_mt_rc). An edit that changes no verdict" >&2
    echo "   reddened the wire, so this run cannot tell a real kill from a broken" >&2
    echo "   harness, and every kill reported above is unearned. Output:" >&2
    printf '%s\n' "$_mt_out" | sed 's/^/     /' >&2
    return 2
  fi
  [ "$_mt_rc" -eq 0 ] && return 1
  if [ "$2" = '!kill' ]; then
    # ⚠ AN ALLOWLIST, NOT AN EXCLUSION: a generated mutant is killed only if the
    # output holds `CONTROL FAILED (<label>)` for a label of `$_VFY/.ctllabels`
    # — the census's: the labels THIS run registered under `_control` (the
    # registry's `control` kind, literal by the census's rule; a mutant edits `$K2RE` /
    # `$K2RE_PATH` in the wire, so its `_control` set is this run's; its own
    # registry lives in its own `$_VFY`). The always-on floor runs inside the
    # mutant too (the `mutgen:` record depends on it), and a mutant of `$K2RE` or
    # `$K2RE_PATH` can make the generator refuse the mutated value — so a
    # `CONTROL FAILED` raised by the floor (`$_mg_lbl`), by a postcondition or by
    # the window is the mutation itself or the real tree reddening the run, and
    # says nothing about whether any fixture poses the question. The labels that
    # are not a `_control`'s are therefore not listed, whoever they belong to.
    [ -s "$_VFY/.ctllabels" ] || {
      echo "!! MUTANT $1: the list of _control labels is missing, so no kill can be told from a" >&2
      echo "   red raised by something else. This run decided nothing." >&2
      return 2; }
    while IFS= read -r _mt_l; do
      case "$_mt_out" in *"CONTROL FAILED ($_mt_l)"*) return 0 ;; esac
    done < "$_VFY/.ctllabels"
    echo "!! MUTANT $1 was killed BY THE WRONG SUBJECT (exit $_mt_rc): no _control failed," >&2
    echo "   so what reddened the run was the mutated predicate itself, the real tree," >&2
    echo "   the generator floor or a refusal, not a fixture posing the question this" >&2
    echo "   rule is about." >&2
    return 2
  fi
  # ⚠ A KILL IS THE NEEDLE IN A DIAGNOSTIC'S LABEL SLOT, NOT ANYWHERE IN THE
  # OUTPUT: `CONTROL FAILED (<needle>)` (the `!kill` arm's form) or
  # `CONTROL NOT EXERCISED (<needle>)`. A bare substring match counted a label
  # that is printed on a GREEN line (a summary naming a control, say): any
  # unrelated late red then "killed as named" though no check under that label
  # failed. The slot closes that for every summary line at once.
  case "$_mt_out" in
    *"CONTROL FAILED ($2)"* | *"CONTROL NOT EXERCISED ($2)"*) return 0 ;;
    *) [ "$_mt_rc" -eq 1 ] && return 3
       _mut_wrong_reason "$1" "$2"
       return 2 ;;
  esac
}

# The capability excuse's callback (`_cap_rows group`): the needle is this group's own label.
_mut_skip_glabel() { [ "${_cr_f[3]}" != "$_pr_needle" ] || { _mk_g="${_cr_f[1]}"; _cr_stop=1; }; }
# The census's membership callback (`_cap_rows member`): the member label is in `$_mc_ctl`.
_mut_cap_member() {
  case "$_mc_ctl" in *"$_mc_nl${_cr_f[2]}$_mc_nl"*) ;; *)
    echo "!! CONTROL FAILED ($_cen_lbl): declared capability member \"${_cr_f[2]}\" is no \`_control\` this run made" >&2
    _mut_corr_bad=1 ;;
  esac
}
# The always-on half: the records against the labels the run used.
_mut_correspondence() {
  _mut_corr_bad=0
  # ---- THE SIBLING GUARD, ALWAYS ON --------------------------------------------
  # The shipped files beside the wire must be exactly `$_MUT_PARTS`: a part left
  # off the list is not copied beside a mutant, which then exits 2 for a reason
  # that is not its mutation. The test is on the PART NAME (what follows the
  # wire's own stem), never the full path, so a checkout whose path holds
  # `.mutant.` is not mistaken for a copy; a concurrent run's copies
  # (`mutant.<pid>.<part>`) are skipped by that same name. Inside a mutant the
  # stem is `….mutant.<pid>`, and its copies are exactly the parts.
  _mp_have=""
  for _mp_f in "${SELF%.sh}".*.sh; do
    [ -e "$_mp_f" ] || continue
    _mp_n="${_mp_f#"${SELF%.sh}".}"; _mp_n="${_mp_n%.sh}"
    case "$_mp_n" in mutant.*) continue ;; esac
    _mp_have="$_mp_have $_mp_n"
  done
  if [ "$(printf '%s\n' $_mp_have | sort | tr '\n' ' ')" != "$(printf '%s\n' $_MUT_PARTS | sort | tr '\n' ' ')" ]; then
    echo "!! the parts beside the wire are [$_mp_have ], but \`_MUT_PARTS\` is [ $_MUT_PARTS ]," >&2
    echo "   so a mutant would run without one of them, or beside one nobody copies." >&2
    _mut_corr_bad=1
  fi
  # ---- THE CORRESPONDENCE, CHECKED ON EVERY RUN ---------------------------------
  # ⚠ THESE TWO USED TO LIVE INSIDE THE MUTATION BLOCK, which is opt-in and costs
  # minutes — so a PR that deleted a record or renamed a control stayed green
  # until somebody happened to run the harness by hand. They are static
  # properties of this shipped file and run no mutant, so they belong where
  # every run sees them. (The mutation RUN stays opt-in; only its bookkeeping
  # moved.)
  _mutants > "$_VFY/.mutants"
  _mut_orphan=0
  # THE NEEDLE, MATERIALISED ONCE: `_mut_parse` stays the one parser of the
  # record format, and this loop writes the `_pr_needle` it sets for every
  # record to `.needles`, one per line.
  # Every other reader (the `!survive` count, the record count, the label
  # ratchet) tests EXACT equality against that file, never a substring of a raw
  # `.mutants` line — a label that is a prefix of a longer needle is not recorded.
  : > "$_VFY/.needles"
  while IFS= read -r _mline; do
    [ -n "$_mline" ] || continue
    _mut_parse "$_mline"
    # ⚠ A LINE THAT IS NOT A RECORD IS AN ERROR HERE, NOT SOMETHING TO SKIP, and
    # the asymmetry it replaces is why: this loop dropped every line with no
    # needle, so a line the here-document turns into DATA passed this check in
    # silence while `_mut_run` applied it as a sed script and reported MATCHED
    # NOTHING. Four such lines were added and shipped that way; what caught them
    # was the opt-in run, minutes long, and what should have is this, which costs
    # nothing. `_mutants` now drops `#` comments at its own materialiser, so what
    # reaches here is a line that is neither — prose with no `#`, or a record
    # whose TAB an editor ate.
    if [ "$_pr_n" -lt 1 ] || [ -z "$_pr_needle" ]; then
      echo "!! mutation set line \"$_mline\" has no TAB-separated needle, so it is not a" >&2
      echo "   record. Write a note as a \`#\` comment; anything else in that" >&2
      echo "   here-document is data, and the mutation run applies it as a sed expression." >&2
      _mut_orphan=1; continue
    fi
    printf '%s\n' "$_pr_needle" >> "$_VFY/.needles"
  done < "$_VFY/.mutants"
  # ---- THE CENSUS: THE LABELS THE RUN USED --------------------------------------
  # Two sources: the run registry (`$_VFY/.registry` — what bash RAN: the run
  # primitives' `_ctl_reg` appends `<file>:<line>	<kind>	<label>` under each
  # `_control` / `_ctl_run` / `_ctl_gate` / `_fgit_window_verdict_exit` call
  # made under this `_VFY`, kind `control` when
  # `_control` made it, `direct` otherwise), and the value of
  # each name in `$_lbd_names` (the `_*_lbl` names the parts spell — a text
  # derivation, each asserted defined by the controls file's `_lbd` check, so a
  # caller's environment cannot add one). `.labels` is their union; `.ctllabels`
  # (the `!kill` allowlist) is the `control`-kind labels. A `_control` LABEL IS
  # LITERAL: one of that kind equal to a variable's value is red — the variables'
  # producers are `echo` lines, not calls, so a second `_control` under a
  # recorded variable label would otherwise pass under that record. The rule is
  # `_control`'s only: a direct `_ctl_run` / `_ctl_gate` / verdict call may pass
  # a variable label, so a second producer made as a direct block under
  # an echo-only producer's variable label passes under that record (a residual,
  # named with the same-line one below). The registry's length
  # is noted (`.census`) before it is read: a label-taking call after this point
  # would count nowhere, so the controls file's last statement reds if the file
  # grew (`_mut_census_end`, below; a file, so a `( … )` or `$(…)` call grows it too).
  # A control a machine cannot pose is registered all the same (the table's
  # decline, in the run primitives), so an orphan needle is a rename, a deletion or an
  # unreached call on every machine. A run that ends before this point is red
  # without it (the window's exit 2, `_lbd`, a missing part).
  # ONE LABEL, ONE SITE: a label registered from two lines is red — a second
  # producer reusing a recorded label would otherwise pass under the first's
  # record, and the text alone cannot tell two calls apart. The line is the
  # finest site bash reports: two calls under one label on ONE line are one site,
  # and so are calls a `trap` body or a multi-line `eval` string fold onto one
  # reported line (the run primitives' registry header); the controls file has
  # no `trap` or `eval` call. And a direct block (an `if ! _ctl_run` passing a
  # variable label) under a label whose own producer is an `echo` line is one
  # site, as the literal rule above does not reach it.
  wc -l < "$_VFY/.registry" | tr -d '[:space:]' > "$_VFY/.census"
  for _cen_v in $_lbd_names; do printf '%s\n' "${!_cen_v}"; done | LC_ALL=C sort -u > "$_VFY/.vlabels"
  K2_CEN="$_cen_lbl" awk -F'\t' 'FILENAME == ARGV[1] { isv[$0] = 1; next }
    { l = $0; sub(/^[^\t]*\t[^\t]*\t/, "", l)
      if (!((l, $1) in seen)) { seen[l, $1] = 1; sites[l] = sites[l] " " $1; n[l]++ }
      if ($2 == "control" && (l in isv) && !(l in lit)) { lit[l] = 1
        print "!! CONTROL FAILED (" ENVIRON["K2_CEN"] "): \"" l "\" is a `_*_lbl` value passed to `_control` (" $1 "):" > "/dev/stderr"
        print "   a `_control` label is literal — a second producer under a variable label would pass under its record." > "/dev/stderr"
        bad = 1 } }
    END { for (l in n) if (n[l] > 1) {
        print "!! CONTROL FAILED (" ENVIRON["K2_CEN"] "): \"" l "\" was registered from more than one call site:" sites[l] > "/dev/stderr"
        print "   — one label, one call site: a second producer under a recorded label would pass under that record." > "/dev/stderr"
        bad = 1 }
      exit bad }' "$_VFY/.vlabels" "$_VFY/.registry" || _mut_corr_bad=1
  awk -F'\t' '$2 == "control" { l = $0; sub(/^[^\t]*\t[^\t]*\t/, "", l); print l }' "$_VFY/.registry" | LC_ALL=C sort -u > "$_VFY/.ctllabels"
  cut -f3- "$_VFY/.registry" | LC_ALL=C sort -u "$_VFY/.vlabels" - > "$_VFY/.labels"
  # READER 4 OF THE CAPABILITY TABLE: every declared member is a `control`-kind label of THIS run's
  # registry (the list just written), else the table names a control that was renamed or never made.
  _mc_nl=$'\n'; _mc_ctl="$_mc_nl$(cat "$_VFY/.ctllabels")$_mc_nl"
  _cap_rows member _mut_cap_member
  _mut_arc=0
  K2_CEN="$_cen_lbl" K2_BARE="$_VFY/.bare" awk 'BEGIN { printf "" > ENVIRON["K2_BARE"] }
    FILENAME == ARGV[1] { nd[++nn] = $0; isn[$0] = 1; next }
    { lb[++nl] = $0; isl[$0] = 1 }
    END {
      for (i = 1; i <= nn; i++) if (nd[i] != "!survive" && !(nd[i] in isl)) {
        print "!! CONTROL FAILED (" ENVIRON["K2_CEN"] "): mutation record needle \"" nd[i] "\" is no label this run used (none registered" > "/dev/stderr"
        print "   by a call, no `_*_lbl` value): its control was renamed or deleted, or the needle is some other string." > "/dev/stderr"
        bad = 1
      }
      for (i = 1; i <= nl; i++) if (!(lb[i] in isn)) print lb[i] > ENVIRON["K2_BARE"]
      exit bad
    }' "$_VFY/.needles" "$_VFY/.labels" || _mut_arc=$?
  [ "$_mut_arc" -le 1 ] || echo "!! the needle/label scan failed (awk exited $_mut_arc): nothing was decided." >&2
  [ "$_mut_arc" -eq 0 ] || _mut_orphan=1
  [ "$_mut_orphan" -eq 0 ] || _mut_corr_bad=1
  # ---- THE GENERATED SET IS NOT EMPTY, ALWAYS ON --------------------------------
  # A static property of `_mut_regex_mutants`' output over the running wire's
  # `$K2RE` and `$K2RE_PATH`, so it is checked where every run (and every
  # record's trial) sees it -- never in the opt-in run alone, which an empty
  # generated set would otherwise pass silently (`0 mutant(s)`, `0 neither
  # killed nor argued equivalent`).
  _mut_gen_floor || _mut_corr_bad=1
  # ---- EVERY RECORD STILL APPLIES, ALWAYS ON ------------------------------------
  # Each record's expression must change the file it targets. Records anchor on
  # harness and fixtures text that an equivalent edit can change (spelling an
  # option long, adding an allowlist entry), and a record that matches nothing
  # tests a copy identical to the shipped file. The opt-in run would catch it,
  # minutes later and only when someone runs it; this costs one `sed` and one
  # `diff` per expression.
  # ⚠ NOT INSIDE A MUTANT: there one target is edited ON PURPOSE, so the records
  # anchored on the edited text no longer apply, by design.
  case "${SELF##*/}" in
    *.mutant.*) : ;;
    *) while IFS= read -r _ma_rec; do
         [ -n "$_ma_rec" ] || continue
         _mut_parse "$_ma_rec"
         _mut_src_for "$_pr_which"
         _ma_i=0
         while [ "$_ma_i" -lt "$_pr_n" ]; do
           _ma_x="${_pr_expr[$_ma_i]}"; _ma_i=$((_ma_i + 1))
           # ⚠ EACH EXPRESSION ALONE, AND IT MUST CHANGE EXACTLY ONE LINE of its
           # target. Whole-record, whole-file was the old granularity: an
           # expression that also matched a COMMENT passed on the comment after the
           # code line was respelled, and the second half of a `range-s;s` pair
           # kept a record "fresh" when the range half had gone stale. `diff`
           # counts a changed line once whether `s` rewrote it or `d` removed it.
           if ! sed -e "$_ma_x" "$_mut_src" > "$_VFY/.anchor" 2>/dev/null; then
             echo "!! mutation record expression \"$_ma_x\" is not a valid sed expression." >&2; _mut_corr_bad=1; continue
           fi
           _ma_drc=0; diff "$_mut_src" "$_VFY/.anchor" > "$_VFY/.anchordiff" || _ma_drc=$?
           if [ "$_ma_drc" -gt 1 ]; then
             echo "!! mutation record expression \"$_ma_x\": diff of ${_mut_src##*/} failed (exit $_ma_drc)." >&2; _mut_corr_bad=1; continue
           fi
           # Counted in the shell, not by a command substitution per expression:
           # the larger of the `<` lines and the `>` lines of the diff.
           _ma_d=0; _ma_a=0
           while IFS= read -r _ma_l; do
             case "$_ma_l" in '<'*) _ma_d=$((_ma_d + 1)) ;; '>'*) _ma_a=$((_ma_a + 1)) ;; esac
           done < "$_VFY/.anchordiff"
           _ma_n=$(( _ma_d > _ma_a ? _ma_d : _ma_a ))
           if [ "$_ma_n" -eq 0 ]; then
             echo "!! mutation record expression \"$_ma_x\" no longer matches ${_mut_src##*/}: its anchor is stale," >&2
             echo "   so the record would test a copy identical to the shipped file." >&2
             _mut_corr_bad=1
           elif [ "$_ma_n" -ne 1 ]; then
             echo "!! mutation record expression \"$_ma_x\" changes $_ma_n lines of ${_mut_src##*/}, not exactly one:" >&2
             echo "   it is anchored on more than the line it is about (a comment, a second site), so" >&2
             echo "   it stays \"fresh\" after the intended line is respelled. Anchor it on one line;" >&2
             echo "   a record that needs several edits is several TAB-separated expressions." >&2
             _mut_corr_bad=1
           fi
         done
       done < "$_VFY/.mutants"
       command rm -f "$_VFY/.anchor" "$_VFY/.anchordiff" ;;
  esac
  # …the standing negative control must BE there, and the set may not shrink.
  # ⚠ `awk`, NOT `grep -c`: `grep` exits 1 when it selects nothing, and under
  # `pipefail` that aborts the required gate — the same trap as the `wc -l`
  # below, and the failing case would be exactly the one being reported.
  _mut_surv="$(awk '$0=="!survive"{n++} END{print n+0}' "$_VFY/.needles")"
  if [ "$_mut_surv" -ne 1 ]; then
    echo "!! the standing negative control (\`!survive\`) is not in the mutation set" >&2
    echo "   exactly once (found $_mut_surv), so a broken harness cannot be told" >&2
    echo "   from a real kill." >&2
    _mut_corr_bad=1
  fi
  _mut_recs="$(awk 'END{print NR+0}' "$_VFY/.needles")"
  if [ "$_mut_recs" -lt "$_MUT_RECORDS_MIN" ]; then
    echo "!! the mutation set holds $_mut_recs records, against a floor of $_MUT_RECORDS_MIN." >&2
    echo "   A record was deleted. If that is deliberate, say why and LOWER the floor" >&2
    echo "   in the same edit — so a smaller set is a visible decision." >&2
    _mut_corr_bad=1
  fi
  # …and the direction that actually finds things: labels with NO record (`.bare`, above).
  # ⚠ `wc -l`, NOT `grep -c .`. `grep` exits 1 when no line is selected, so on an
  # EMPTY `.bare` — the state this ratchet exists to let you reach — the
  # assignment failed and `set -e` aborted the required gate before the
  # comparison ran. The success case was the one that broke it.
  _mut_bare="$(wc -l < "$_VFY/.bare" | tr -d '[:space:]')"
  if [ "$_mut_bare" -gt "$_MUT_UNRECORDED_MAX" ]; then
    echo "!! CONTROL FAILED ($_cen_lbl): $_mut_bare labels have no mutation record, against a ratchet of $_MUT_UNRECORDED_MAX." >&2
    echo "   Either the new label needs a record, or a record was deleted. The bare ones:" >&2
    sed 's/^/     /' "$_VFY/.bare" >&2
    echo "   If a label genuinely cannot have one, say why and RAISE the ratchet in the" >&2
    echo "   same edit — so widening the gap is a visible decision rather than a silence." >&2
    _mut_corr_bad=1
  elif [ "$_mut_bare" -lt "$_MUT_UNRECORDED_MAX" ]; then
    echo "!! CONTROL FAILED ($_cen_lbl): only $_mut_bare labels have no mutation record, below the ratchet of $_MUT_UNRECORDED_MAX:" >&2
    echo "   a label gained a record, or went away. LOWER the ratchet to $_mut_bare in the" >&2
    echo "   same edit — a slack ratchet lets the next label go bare in silence." >&2
    _mut_corr_bad=1
  fi
  command rm -f "$_VFY/.bare" "$_VFY/.labels"
  return "$_mut_corr_bad"

}
# THE REGISTRY MAY NOT GROW AFTER THE CENSUS. Asked by the controls file as its
# last statement (the wire makes no label-taking call after that file returns):
# the registry's length now against the note the census left. A late call
# counted nowhere, whichever shell made it (a `( … )`, a `$(…)`: the file is
# shared), so it is red under the census's label, listing the late lines; no
# note, or no count, is red too.
_mut_census_end() {
  local _cend_n _cend_now
  _cend_n="$(cat "$_VFY/.census" 2>/dev/null)" || _cend_n=""
  _cend_now="$(wc -l < "$_VFY/.registry" 2>/dev/null | tr -d '[:space:]')" || _cend_now=""
  if [ -n "$_cend_n" ] && [ -n "$_cend_now" ] && [ "$_cend_now" -eq "$_cend_n" ]; then return 0; fi
  echo "!! CONTROL FAILED ($_cen_lbl): the run registry grew after the label census (${_cend_n:-no note} → ${_cend_now:-no count}); the late calls:" >&2
  tail -n "+$((${_cend_n:-0} + 1))" "$_VFY/.registry" 2>/dev/null | sed 's/^/     /' >&2
  return 1
}

# The opt-in half: actually apply each record. Costs one full control pass
# per entry, so it is gated — but its BOOKKEEPING above is not.
_mut_run() {
  if [ -n "${WEBREF_WIRE_MUTANTS:-}" ]; then
    # The copy must sit BESIDE the wire: `$ROOT` is derived from `$0`, and the
    # scope it reads is this repository's real generic core. Its own controls file
    # has to be beside it too, under the name the wire derives (`${SELF%.sh}`), or
    # the copy refuses to run — which is criterion 2 working, not a harness bug.
    # ⚠ PER-RUN NAMES, BECAUSE THE WORKING TREE IS SHARED BY DESIGN. A fixed
    # `${SELF%.sh}.mutant.sh` is one path per checkout, and CLAUDE.md's
    # parallel-session rule makes two runs in one tree the EXPECTED case — so the
    # second run's `sed >` and its `rm -f` overwrote and deleted the first's
    # files mid-flight. The victim's copy then lost its controls and took the
    # wire's own `exit 2` arm, which this harness reported as
    # `killed for the WRONG REASON`: **it blamed the mutation set for an
    # environment collision.** Reproduced independently three times during this
    # slice's review, at `16`, `11` and `10 not killed as named` on a head that
    # measures `0` when run alone.
    # ⚠ AND THE COPY MUST SIT BESIDE THE WIRE, so `$SCRATCH` is not an option:
    # `$ROOT` is derived from `$0`, and the scope it must read is this
    # repository's real generic core. `$$` is what makes the two runs disjoint.
    _mut_base="${SELF%.sh}.mutant.$$"
    _mut_wire="$_mut_base.sh"
    # ⚠ A LEFTOVER IS A REPORT, NOT A FILE TO CLEAN UP. `trap` does not run on
    # SIGKILL, so a killed run leaves mode-755 artifacts in `.claude/tools/`
    # where a `git add -A` would stage them. Say so; do not delete another run's.
    for _stale in "${SELF%.sh}".mutant.*.sh; do
      case "$_stale" in *'.mutant.*.sh') break ;; esac
      case "$_stale" in "$_mut_wire"|"$_mut_base".*) continue ;; esac
      echo "  note: a previous mutation run left $_stale behind (SIGKILL?); it is" >&2
      echo "        not this run's to remove. Delete it once no run is using it." >&2
    done
    # The wire's own trap body (`_k2_exit`: cleanup, and no status 0 without the
    # wire's orderly-end flag), not a second copy of it.
    trap '_k2_rc=$?; _mut_rm_copies; _k2_exit "$_k2_rc"' EXIT
    # Every part once; the wire and the targets are re-copied per entry by
    # `_mut_restore_copies`, because entries edit them.
    for _mp in $_MUT_PARTS; do cp "${SELF%.sh}.$_mp.sh" "$_mut_base.$_mp.sh"; done
    # Through a FILE, not a pipe: the counters below must survive the loop, and a
    # `_mutants | while` runs the body in a subshell that discards them.
    _mutants > "$_VFY/.mutants"
    _mut_n=0; _mut_bad=0; _mut_exc=0
    while IFS= read -r _mrec; do
      [ -n "$_mrec" ] || continue
      _mut_n=$((_mut_n + 1))
      # WHICH SHIPPED FILE THIS ENTRY EDITS, and what it applies — the record's
      # one reader, shared with the always-on checks.
      _mut_parse "$_mrec"
      # A record whose needle is the label of a postcondition THIS MACHINE
      # cannot evaluate (a `machine_limits` line from the window: `P-x — …`)
      # may be unable to die here however correct the wire — but only MAY: a
      # record pinning the limitation test's own narrowness still dies on such
      # a machine. So every record RUNS; a SURVIVAL of one of these is reported
      # as not exercisable here and not counted (PR #527 Codex R17; the first
      # form skipped them unrun — the re-check after it). The labels are
      # derived from the ID — `_p<x>_lbl` and `_p<x>l_lbl` in `$_lbd_names` — not listed.
      # REJECTED, by design: the excuse keys on the `_p<x>_lbl` / `_p<x>l_lbl` NAMING convention (the ID's postcondition label and its limitation-test label, exactly; the P-x half reads any `P-x — …` line of `$_fw_limits`, whose ID is a limit or a fact row's), so a renamed label falls out of it and the opt-in run reds on a limited machine, the FAIL-SAFE side.
      _mskip=""
      while IFS= read -r _ml; do
        _mid="${_ml%% —*}"
        case "$_mid" in P-[a-z]) ;; *) continue ;; esac
        for _mlv in $_lbd_names; do
          case "$_mlv" in "_p${_mid#P-}_lbl"|"_p${_mid#P-}l_lbl") [ "${!_mlv:-}" != "$_pr_needle" ] || _mskip="$_mid" ;; esac
        done
      done <<EOF_MLIM
${_fw_limits:-}
EOF_MLIM
      # The capability half: a needle that is a member of a group whose flag is 0, or that group's own label.
      # Only a machine that DECLARED the absence reaches here with a flag at 0 (the table check `_cap_check` reds
      # an undeclared flag-0 group first, before `_mut_run`); a needle whose group no `group` row declares, or an
      # unset flag, is no 0, so nothing is excused (fail-safe).
      _mskip_g=""; _mk_g=""
      _cap_group_of "$_pr_needle"; _mk_g="$_cap_g"
      [ -n "$_mk_g" ] || _cap_rows group _mut_skip_glabel
      if [ -n "$_mk_g" ]; then _cap_flag_of "$_mk_g"; [ "$_cap_v" != 0 ] || _mskip_g="$_mk_g"; fi
      _mut_restore_copies || { _mut_bad=$((_mut_bad + 1)); continue; }
      _mut_target "$_pr_which"
      if ! _mut_apply "$_mut_src" "$_mut_tgt"; then
        echo "!! MUTANT $_mut_n: the expression is not a valid sed script: $_pr_show" >&2
        _mut_bad=$((_mut_bad + 1)); continue
      fi
      # ⚠ `|| _rc=$?`, NOT `cmd; _rc=$?`: under `set -e` a simple command that
      # returns non-zero ends the run THERE, with no diagnostic at all — which
      # is what a survived mutant did while this refactor was being written.
      _mrc2=0; _mut_trial "$_mut_n ($_pr_needle)" "$_pr_needle" || _mrc2=$?
      [ "$_mrc2" -ne 2 ] || { _mut_bad=$((_mut_bad + 1)); continue; }
      # ⚠ A 2 IS NEVER EXCUSED: it is also how a harness break (MATCHED NOTHING, a
      # clobbered copy) reports, and excusing it would hide one. P-x excuses a
      # survival (1) only; the capability excuse a survival or a wrong-reason kill
      # whose mutant exited 1 (3), because a declined member's label never stands in
      # a label slot on that machine. A limited machine whose caller leaks
      # configuration can therefore still report a record "killed for the wrong
      # reason" — a false RED on an opt-in run, the fail-safe side. A 3 nothing
      # excuses is red.
      if [ -n "$_mskip_g" ] && { [ "$_mrc2" -eq 1 ] || [ "$_mrc2" -eq 3 ]; }; then
        echo "  mutant $_mut_n ($_pr_needle) $([ "$_mrc2" -eq 1 ] && echo survived || echo 'was killed for another reason'), and is not exercisable on this machine: the \`$_mskip_g\` group is declared absent"
        _mut_exc=$((_mut_exc + 1)); continue
      fi
      if [ "$_mrc2" -eq 3 ]; then
        _mut_wrong_reason "$_mut_n ($_pr_needle)" "$_pr_needle"
        _mut_bad=$((_mut_bad + 1)); continue
      fi
      if [ "$_mrc2" -eq 1 ] && [ -n "$_mskip" ]; then
        echo "  mutant $_mut_n ($_pr_needle) survived, and is not exercisable on this machine: $_mskip is a machine limitation here"
        _mut_exc=$((_mut_exc + 1)); continue
      fi
      [ "$_mrc2" -ne 1 ] || {
        echo "!! MUTANT $_mut_n ($_pr_needle) SURVIVED: the wire still exited 0 with this" >&2
        echo "   applied, so nothing above is testing it: $_pr_show" >&2
        _mut_bad=$((_mut_bad + 1)); }
    done < "$_VFY/.mutants"
    command rm -f "$_VFY/.mutants"
    echo "  mutation set: $_mut_n entr(ies), $_mut_bad not killed as named, $_mut_exc not exercisable on this machine"
    # …and the population the wire's own regexes define, which no list here
    # enumerates. Run whatever the hand set did, so one run answers both
    # questions and a failure in either is reported before the exit.
    _mut_gen_n=0; _mut_gen_bad=0
    _mut_gen_run
    # …and the run must have TRIED what the generator derives. The always-on
    # floor guards the generator's OUTPUT; this guards `_mut_gen_run`'s PATH
    # (its file, its read-back, its TAB split), which no record's trial
    # reaches (a trial runs with `WEBREF_WIRE_MUTANTS` unset). Checked here,
    # in the caller, so an early `return` inside `_mut_gen_run` cannot skip it.
    if [ "$_mut_gen_n" -ne "${_mut_gen_floor_n:-0}" ] || [ "$_mut_gen_n" -eq 0 ]; then
      echo "!! the generated boundary set ran $_mut_gen_n mutant(s), but the generator derives" >&2
      echo "   ${_mut_gen_floor_n:-0} from the wire's regexes: this run did not test every rule." >&2
      _mut_gen_bad=$((_mut_gen_bad + 1))
    fi
    echo "  generated boundary set: $_mut_gen_n mutant(s) from \$K2RE and \$K2RE_PATH, $_mut_gen_bad neither killed nor argued equivalent"
    _mut_rm_copies
    [ "$_mut_bad" -eq 0 ] && [ "$_mut_gen_bad" -eq 0 ] || exit 1
    if [ "$_mut_exc" -eq 0 ]; then
      echo "  every entry above was shown to red, and to red for its own reason;"
    else
      echo "  every entry above but the $_mut_exc not exercisable here was shown to red,"
      echo "  and to red for its own reason (a machine with that capability runs them all);"
    fi
    echo "  generated: every class member, quantifier, alternation branch and"
    echo "  escape of those two regexes is pinned by a control. Widening a branch"
    echo "  list is not generated — only the hand records above reach it"
    # ⚠ NO `exit 0` HERE, AND THAT IS THE WHOLE POINT. This block used to end the
    # run — so `WEBREF_WIRE_MUTANTS` merely PRESENT IN THE ENVIRONMENT (exported
    # once, in a shell that later runs `mise run trip-wires`) made the required
    # gate exit 0 having never scanned `_webref/`, with the driver recording it as
    # a wire that ran. Every other path in this file refuses to be green over
    # something it did not read; this was the one that did. The mutation set is
    # now something the run does BEFORE its verdict, not INSTEAD of it.
  fi
}
