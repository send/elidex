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
# ⚠ THE SPLIT IS THE RULE'S OWN SHAPE, AND THE ORDERING MATTERS. CLAUDE.md's
# touch-time discipline says a >1000-line file gets a "standalone prereq split"
# as its own PR **or its own commit**, and its heading is *defer しない*. An
# earlier revision of 2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md booked
# this as a defer slot to be done after the PR landed — which inverts "prereq"
# into "afterwards" and uses the single-commit requirement as a reason to merge
# the oversized file first. The
# external reviewer caught that (P1). This file is that standalone commit.
#
# ⚠ AND THE COUNTER-ARGUMENT IS ANSWERED, not ignored.
# 2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md previously argued against
# this seam on the ground that "the two lists must be edited together, so
# splitting them puts the two halves of one assertion in two files". They must — and the correspondence check below is what ENFORCES that
# rather than hoping for it. Being cross-file is the point: the check reads the
# controls file for labels and this file for records, and reds when they drift.
#
# WHAT IT CONSUMES: `$CTL` (from the controls harness), `$_HARNESS` and
# `$_MUTATIONS` (from the controls file — the last is this file's own path),
# and `$SELF`, `$SCRATCH`, `$_CONTROLS`, `$K2RE` and `$K2RE_PATH` (from the
# wire). `_mut_run` copies every part in its `_MUT_PARTS` list — the controls,
# the harness, this file and the fixture build — beside each mutant.
# ⚠ THE TWO REGEXES ARE CONSUMED AS VALUES, not just named in a `for` list:
# `_mut_gen_run` compares what the wire's assignment LINE reads back as against
# what the RUNNING wire HOLDS, and that second half is these variables. They
# were left off this list and out of the guard by the commit that added the
# generator — the same drift the guard exists to catch, in the edit that
# widened what there is to drift.
# WHAT IT DEFINES: `_MUT_UNRECORDED_MAX`, `_MUT_RECORDS_MIN`, `_MUTGEN`,
# `_mutants`, `_mut_correspondence`, `_mut_trial`, `_mut_run`. The generated
# half — `_mut_equivalent`, `_mut_assign_value`, `_mut_regex_mutants`,
# `_mut_splice`, `_mut_gen_run` — is in `…trip-wire.mutgen.sh`, sourced below.
#
# TWO POPULATIONS, AND THE BOUNDARY IS WHAT EACH ONE'S UNIT IS.
#   * `_mutants` — HAND-WRITTEN, and its unit is a CONTROL. Every control has
#     to be named by some record (`_MUT_UNRECORDED_MAX` below is what enforces
#     that), and the record says which edit that control is about. Its claim:
#     *THAT* control is about *THAT* edit. It is a floor and cannot be a
#     census — a record exists only for something somebody thought of.
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
for _n in CTL _CONTROLS _HARNESS _MUTATIONS SELF SCRATCH K2RE K2RE_PATH; do
  [ -n "${!_n:-}" ] || _mut_missing="$_mut_missing \$$_n"
done
# …and the generated half, which lives beside this file (below).
_MUTGEN="${SELF%.sh}.mutgen.sh"
[ -r "$_MUTGEN" ] || _mut_missing="$_mut_missing $_MUTGEN (the generated half)"
if [ -n "$_mut_missing" ]; then
  echo "!! This file is the MUTATION SET for \`webref-generic-core-trip-wire.sh\`. It is" >&2
  echo "   SOURCED by that wire's controls and has no meaning alone; missing:$_mut_missing" >&2
  echo "   Run the wire instead." >&2
  exit 2
fi
# THE GENERATED HALF — `_mut_gen_run` and what it needs — LIVES BESIDE THIS FILE,
# at the boundary this header draws between the two populations.
# shellcheck source=/dev/null
. "$_MUTGEN"

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
#   one record per line, TAB-separated:  <sed expression>  <substring the run must print>
#
# The expression is applied to a COPY of the wire; the copy must (a) differ from
# the original — a stale anchor that matches nothing is a FAILED entry, not a
# passing one — (b) exit non-zero, and (c) print the named control's own
# diagnostic, so an entry that reds for an unrelated reason is caught too.
#
# RUN IT — it is NOT part of the gate (it costs one full control pass per entry,
# minutes rather than seconds), and a required check nobody can afford to run is
# how gates get switched off:
#
#     WEBREF_WIRE_MUTANTS=1 bash .claude/tools/webref-generic-core-trip-wire.sh
#
# ⚠ ADDING A CONTROL MEANS ADDING A RECORD, whatever the control is about —
# the ratchet below is what makes that true. What does NOT belong here is a
# record for a rule of `$K2RE` or `$K2RE_PATH` that needed no new control:
# `_mut_gen_run` derives that population from the assignments themselves, so a
# hand-written copy would be a second spelling of the same class. The boundary
# is stated in full at the head of this file and again at that function.
# ⚠ Nothing here can detect an arm that
# never had one — this set is a floor, not a census. ⚠ AND A DECLARED FLOOR IS
# NOT A DETECTOR: this file used to say exactly the sentence above and assert
# nothing, so deleting a record shrank the set in silence and the run stayed
# green. Three things now hold it up, and each is checked below rather than
# described:
#   * a CORRESPONDENCE, IN BOTH DIRECTIONS AND ALWAYS-ON. Every record's needle
#     must appear in the controls file as a QUOTED STRING — a `_control` label,
#     or the label a block that is not a `_control` prints (the umask and
#     fsmonitor ones) — since a stale anchor would otherwise report "wrong
#     reason" forever. It is a grep for the quoted text, not a parse of the
#     call. AND the number of controls with NO record is ratcheted:
#     `_MUT_UNRECORDED_MAX` may only come down.
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
_MUT_UNRECORDED_MAX=21
_MUT_RECORDS_MIN=95
# ⚠ A FUNCTION, NOT `x="$(cat <<'EOF' … )"`. Under bash 3.2 — the stock macOS
# shell this wire commits to — a quoted here-document nested inside a command
# substitution is still parsed for expansions, and the `unset "$_v"` in one of
# the records below aborted the WHOLE FILE with `_v: unbound variable`, on every
# run, mutation mode or not. Measured: 3.2 red, 5.3 green.
_mutants() { cat <<'MUTANTS'
s/grep -aEn --/grep -En --/	K2 fires inside binary content, and the content is READ
s/"$_mrc" -gt 1/"$_mrc" -gt 99/	a failed NAME matcher fails closed
s#^K2RE_PATH=.*#K2RE_PATH='(^|/)\\.claude/(skills|tools)/[^/"]+/[^/]+'#	a quote inside a name segment
s/^K2RE_PATH=.*/K2RE_PATH="$K2RE"/	a STAGED symlink target is a stored path
s/elif \[ "$_mode" = 120000 \]; then/elif [ "$_mode" = 120000 ] \&\& [ "$_src" = index ]; then/	a COMMITTED symlink target is a stored path
s|${1//$'\\n'/$_REC_SEP}|${1}|	a NEWLINE inside a name segment
s/^  _stored "${rel#"$_dir"\/}"/  : /	an entry's own NAME is the hierarchy
s/--exclude-per-directory=.gitignore/--exclude-standard/	per-clone info/exclude cannot hide an entry
s/\[ "$_hrc" -eq 0 \]/false/	a COMMITTED violation fixed only in the index still fires
s/if ! tr -d .\\000. < "\$_b"/if false/	a NUL-bearing staged symlink blob is not a path
s/readlink -n "$f"/readlink "$f"/	readlink's own newline is not read as stored content
s/export GIT_NO_LAZY_FETCH=1 GIT_NO_REPLACE_OBJECTS=1/export GIT_NO_LAZY_FETCH=1/	a replace ref cannot substitute the staged blob
s|^_esc() .*|_esc() { printf '%s' "$1"; }|	a name cannot forge a verdict record
s/^export LC_ALL=C$/export LC_ALL=C.UTF-8/	a byte no UTF-8 locale can bracket
s/"$SCANNED" -eq 0/"$SCANNED" -eq -1/	an empty scope fails loudly
s/\[ "$_read" -eq 0 \] || printf/printf/	an inventoried entry that was never read is not counted as scanned
s/\[ -e "$p" \] || \[ -L "$p" \]/[ -e "$p" ]/	a symlinked EXTRA entry is scanned
s/REL_DIR="${SCOPE_DIR#"$ROOT"\/}"/REL_DIR="${SCOPE_DIR#$ROOT\/}"/	a glob character in the checkout path does not widen the scope
s/REL_FILE="${SCOPE_FILE#"$ROOT"\/}"/REL_FILE="${SCOPE_FILE#$ROOT\/}"/	a pattern character in the checkout path does not misplace the entry script
s/GIT_CONFIG\*) : ;;/GIT_CONFIG*) unset "$_v" ;;/	the caller's git CONFIGURATION survives the routing purge
s/\[ "$_rc_tracked" -eq 0 \]/true/	a failed TRACKED inventory fails closed
s/\[ "$_rc_worktree" -eq 0 \]/true/	a failed WORKTREE inventory fails closed
s/\[ "$_rc_head" -eq 0 \]/true/	a failed HEAD inventory fails closed
s/the HEAD inventory exited %d/the HEAD inventory was fine %d/	a failed HEAD inventory fails closed
s/ls-files -z --stage/ls-files -z --cached/	a STAGED symlink target is a stored path
s/\[ "$_shrc" -eq 0 \]/false/	a failed HEAD PROBE is not an unborn HEAD
s/\[ "$_shrc" -ne 1 \]/true/	an orphan branch is an unborn HEAD, not a read failure
s/\[^A-Za-z0-9_.~@+%-\]/[^A-Za-z0-9_.~+%-]/	a PATH character before .claude is not a prose boundary
s/\[^A-Za-z0-9_.~@+%-\]/[[:space:]"]/	the leading boundary covers =, --opt=, :, a backtick, ** and ,
s/elif _ancestor_link "$rel"; then/elif false; then/	the worktree read does not traverse an ancestor symlink
s/\[ "$_catrc" -ne 0 \]/false/	a failed staged-blob read is not a clean target
s/)}>,;\]/]/g	closing punctuation is not a path segment
s/elif ! _git -C "$ROOT" --literal-pathspecs/elif false \&\& ! _git -C "$ROOT" --literal-pathspecs/	an inventoried path that vanished is not silently skipped
s/--literal-pathspecs ls-files --error-unmatch/ls-files --error-unmatch/	the vanished-path question is asked of a LITERAL path
s/\[ "$_hrc" -ne 0 \]/false/	a malformed HEAD ref is not an unborn repository
s|\[^/\[:space:\]\]+/(|[^/[:space:])}>,;]+/(|	punctuation BEFORE a slash is part of the path
s/(^|\/)\\.claude/\\.claude/	a segment merely ENDING in .claude is not the host path
s#^K2RE_PATH=.*#K2RE_PATH='\\.claude/(skills|tools)/[^/]+/[^/]+'#	a segment merely ENDING in .claude is not the host path
s/_SELFTEST="$2"/_SELFTEST="${WEBREF_WIRE_SELFTEST:-$2}"/	an exported SELFTEST cannot redirect the scan
s/\[ ! -d "${2:-}" \]/false/	a missing self-test root decides nothing
s/^SCRATCH="$(_phys "$_raw_scratch")"/SCRATCH="$_raw_scratch"/	a relative scratch dir is removed on exit
s/if \[ "$_ENDS" -ne 1 \] || /if false \&\& /	a walk killed mid-scan is not a verdict
s/\[ -x "$_ab_p" \] && return 0/return 0/	a tracked file under an unsearchable dir is not absent
s/|| \[ -L "$_ab_p" \] || continue/|| [ -L "$_ab_p" ] || return 1/	a tracked directory deleted wholesale stays green
s/\[ -d "$_ab_p" \] || return 0/[ -d "$_ab_p" ] || return 1/	a tracked directory replaced by a file stays green
s|\[^/\[:space:\]\]+/|[^]/[:space:]]+/|	a ] inside a first segment
s|\[^/\[:space:\]\]+/|[^/[:space:]"]+/|	a double quote inside a first segment
s|\[^/\[:space:\]\]+/|[^/[:space:]'"'"']+/|	a single quote inside a first segment
s|\[^/\[:space:\]\]+/|[^/[:space:]`]+/|	a backtick inside a first segment
s|\[^/\[:space:\]\]+/|[^]/[:space:]]+/|2	a ] inside an intermediate segment
s|\[^/\[:space:\]\]+/|[^/[:space:]"]+/|2	a double quote inside an intermediate segment
s|\[^/\[:space:\]\]+/|[^/[:space:]'"'"']+/|2	a single quote inside an intermediate segment
s|\[^/\[:space:\]\]+/|[^/[:space:]`]+/|2	a backtick inside an intermediate segment
s|\[^/\[:space:\]\]+/|[^/[:space:]"]+/|	a quoted one-segment reference before a slash token fails safe
s/SCRATCH="$(_phys "$_raw_scratch")" || SCRATCH=""/SCRATCH="$(_phys "$_raw_scratch")"/	a restrictive umask decides nothing
s/_root_p="$(_phys "$ROOT")" || _root_p=""/_root_p="$(_phys "$ROOT")"/	an unresolvable self-test root decides nothing
s/^unset GREP_OPTIONS$/:/	a caller's GREP_OPTIONS cannot hide a file
s/exec git -c core.fsmonitor=false /exec git /	a caller's fsmonitor hook does not run
s/^K2RE='(^|\[/K2RE='([/	a reference at the start of a line fires
s/\[^A-Za-z0-9_.~@+%-\]/[^A-Za-z0-9_.@+%-]/	a PATH character before .claude is not a prose boundary
s/\[^A-Za-z0-9_.~@+%-\]/[^A-Za-z0-9_.~@]/	a PATH character before .claude is not a prose boundary
s/\[^A-Za-z0-9_.~@+%-\]/[^A-Za-z0-9_~@+%-]/	a PATH character before .claude is not a prose boundary
s/\[^A-Za-z0-9_.~@+%-\]/[^A-Za-z0-9.~@+%-]/	a PATH character before .claude is not a prose boundary
s/\[^A-Za-z0-9_.~@+%-\]/[^A-Za-z_.~@+%-]/	a PATH character before .claude is not a prose boundary
s/\[^A-Za-z0-9_.~@+%-\]/[^a-z0-9_.~@+%-]/	a PATH character before .claude is not a prose boundary
s|\[^A-Za-z0-9_.~@+%-\]|[^A-Za-z0-9_.~@+%/-]|	a reference written after a slash fires
s|`\]\*\[^\]|`]+[^]|	a ONE-character final segment fires
s#(\[^/\[:space:\]\]+/|#([^/[:space:]]{2,}/|#	a ONE-character intermediate segment fires
s#^K2RE_PATH='\(.*\)/\[^/\]+/\[^/\]+'#K2RE_PATH='\1/[^/]{2,}/[^/]+'#	a ONE-character first segment in a STORED path fires
s|\[^\]/\[:space:\]"'"'"'`\]\*|[^/[:space:]"'"'"'`]*|	the final segment's middle stops where its last character does
s|\[^\]/\[:space:\]"'"'"'`\]\*|[^][:space:]"'"'"'`]*|	the final segment's middle stops where its last character does
s|\[^\]/\[:space:\]"'"'"'`\]\*|[^]/"'"'"'`]*|	the final segment's middle stops where its last character does
s|\[^\]/\[:space:\]"'"'"'`\]\*|[^]/[:space:]'"'"'`]*|	the final segment's middle stops where its last character does
s|\[^\]/\[:space:\]"'"'"'`\]\*|[^]/[:space:]"`]*|	the final segment's middle stops where its last character does
s|\[^\]/\[:space:\]"'"'"'`\]\*|[^]/[:space:]"'"'"']*|	the final segment's middle stops where its last character does
s/^K2RE='\(.*\)(skills|tools)/K2RE='\1[a-z]+/	running text that only looks like a two-segment reference stays green
s|\[^/\[:space:\]\]+/(|[^/]+/(|	running text that only looks like a two-segment reference stays green
s|\[^/\[:space:\]\]+/(|[^/[:space:]]*/(|	running text that only looks like a two-segment reference stays green
s#(\[^/\[:space:\]\]+/|#([^/]+/|#	running text that only looks like a two-segment reference stays green
s#(\[^/\[:space:\]\]+/|#([^/[:space:]]*/|#	running text that only looks like a two-segment reference stays green
s/)}>,;\])'$/)>,;])'/	running text that only looks like a two-segment reference stays green
s/)}>,;\])'$/)},;])'/	running text that only looks like a two-segment reference stays green
s/)}>,;\])'$/)}>;])'/	running text that only looks like a two-segment reference stays green
s/)}>,;\])'$/)}>,])'/	running text that only looks like a two-segment reference stays green
s/\[^\]\/\[:space:\]"'"'"'`)}>,;\])'$/[^]\/[:space:]'"'"'`)}>,;])'/	running text that only looks like a two-segment reference stays green
s/\[^\]\/\[:space:\]"'"'"'`)}>,;\])'$/[^]\/[:space:]"`)}>,;])'/	running text that only looks like a two-segment reference stays green
s/\[^\]\/\[:space:\]"'"'"'`)}>,;\])'$/[^]\/[:space:]"'"'"')}>,;])'/	running text that only looks like a two-segment reference stays green
s/\[^\]\/\[:space:\]"'"'"'`)}>,;\])'$/[^\/[:space:]"'"'"'`)}>,;])'/	running text that only looks like a two-segment reference stays green
s/^K2RE='\(.*\)\\\.claude/K2RE='\1.claude/	running text that only looks like a two-segment reference stays green
s#^K2RE_PATH='(^|/)#K2RE_PATH='^#	a stored path naming .claude after a slash, under tools, fires
s#^K2RE_PATH='(^|/)\\.claude/(skills|tools)#K2RE_PATH='(^|/)\\.claude/(skills)#	a stored path naming .claude after a slash, under tools, fires
s#^K2RE_PATH='(^|/)\\.claude/(skills|tools)#K2RE_PATH='(^|/)\\.claude/[a-z]+#	a stored path that only looks like one stays green
s#^K2RE_PATH='\(.*\)/\[^/\]+/\[^/\]+'#K2RE_PATH='\1/[^/]*/[^/]+'#	a stored path that only looks like one stays green
s#^K2RE_PATH='(^|/)\\.claude#K2RE_PATH='(^|/).claude#	a stored path that only looks like one stays green
s/^# Run from anywhere\./# Run from anywhere (edited by the negative control)./	!survive
MUTANTS
}

# ONE TRIAL, SHARED BY BOTH POPULATIONS. The mutated copy is already at
# `$_mut_wire`. $1 = how to name it in a diagnostic, $2 = what is required:
# a NEEDLE the output must contain, `!survive`, or `!kill` (red, raised by a
# control, with no control named).
# Returns 0 = as required, 1 = SURVIVED (the caller decides what that means),
# 2 = failed some other way, already reported.
_mut_trial() {
  # ⚠ AND IT MUST BE EXECUTABLE. `_control` invokes `"$SELF"` DIRECTLY, not
  # through `bash`, so a copy written by `sed` (mode 644) exits 126
  # "Permission denied" for EVERY control — which reds the run, prints every
  # control's diagnostic, and therefore satisfies a "did it name the right
  # control?" test VACUOUSLY. Measured: every entry the set then held "passed" that
  # way, and a deliberately inert entry (a comment-only edit that cannot
  # change any verdict) was the negative control that exposed it. The probe's
  # subject was the permission bit, not the mutation.
  chmod +x "$_mut_wire"
  if cmp -s "$_mut_wire" "$SELF"; then
    echo "!! MUTANT $1: the edit MATCHED NOTHING, so this entry tested a copy" >&2
    echo "   identical to the wire." >&2
    return 2
  fi
  _mt_rc=0
  _mt_out="$(env -u WEBREF_WIRE_MUTANTS bash "$_mut_wire" 2>&1)" || _mt_rc=$?
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
    case "$_mt_out" in
      *"CONTROL FAILED ("*) return 0 ;;
      *) echo "!! MUTANT $1 was killed BY THE WRONG SUBJECT (exit $_mt_rc): no control" >&2
         echo "   failed, so what reddened the run was the real tree or a refusal, not" >&2
         echo "   a fixture posing the question this rule is about." >&2
         return 2 ;;
    esac
  fi
  case "$_mt_out" in
    *"$2"*) return 0 ;;
    *) echo "!! MUTANT $1 killed for the WRONG REASON (exit $_mt_rc): the output" >&2
       echo "   does not name \"$2\", so another check masked the one under test." >&2
       return 2 ;;
  esac
}

# The always-on half: static properties of the two shipped lists.
_mut_correspondence() {
  # ---- THE CORRESPONDENCE, CHECKED ON EVERY RUN ---------------------------------
  # ⚠ THESE TWO USED TO LIVE INSIDE THE MUTATION BLOCK, which is opt-in and costs
  # minutes — so a PR that deleted a record or renamed a control stayed green
  # until somebody happened to run the harness by hand. They are static
  # properties of this shipped file and cost milliseconds, so they belong where
  # every run sees them. (The mutation RUN stays opt-in; only its bookkeeping
  # moved.)
  _mutants > "$CTL/.mutants"
  _mut_orphan=0
  while IFS="$(printf '\t')" read -r _ _mwant; do
    [ -n "${_mwant:-}" ] || continue
    # `!survive` is the standing negative control; it names no control by design.
    [ "$_mwant" != '!survive' ] || continue
    grep -qF -- "\"$_mwant\"" "$_CONTROLS" || {
      echo "!! mutation record needle \"$_mwant\" appears nowhere in the controls file" >&2
      echo "   as a quoted label. A record whose control was renamed reports" >&2
      echo "   \"wrong reason\" forever." >&2
      _mut_orphan=1; }
  done < "$CTL/.mutants"
  _mut_corr_bad=0
  [ "$_mut_orphan" -eq 0 ] || _mut_corr_bad=1
  # …the standing negative control must BE there, and the set may not shrink.
  # ⚠ `awk`, NOT `grep -c`: `grep` exits 1 when it selects nothing, and under
  # `pipefail` that aborts the required gate — the same trap as the `wc -l`
  # below, and the failing case would be exactly the one being reported.
  _mut_surv="$(awk -F'\t' '$2=="!survive"{n++} END{print n+0}' "$CTL/.mutants")"
  if [ "$_mut_surv" -ne 1 ]; then
    echo "!! the standing negative control (\`!survive\`) is not in the mutation set" >&2
    echo "   exactly once (found $_mut_surv), so a broken harness cannot be told" >&2
    echo "   from a real kill." >&2
    _mut_corr_bad=1
  fi
  _mut_recs="$(awk -F'\t' 'NF>1{n++} END{print n+0}' "$CTL/.mutants")"
  if [ "$_mut_recs" -lt "$_MUT_RECORDS_MIN" ]; then
    echo "!! the mutation set holds $_mut_recs records, against a floor of $_MUT_RECORDS_MIN." >&2
    echo "   A record was deleted. If that is deliberate, say why and LOWER the floor" >&2
    echo "   in the same edit — so a smaller set is a visible decision." >&2
    _mut_corr_bad=1
  fi
  # …and the direction that actually finds things: controls with NO record.
  # The label is the third quoted argument of a `_control` call.
  _mut_bare=0
  awk -F'"' '/^ *_control /{print $6}' "$_CONTROLS" | sed '/^$/d' | while IFS= read -r _lbl; do
    grep -qF -- "	$_lbl" "$CTL/.mutants" || printf '%s\n' "$_lbl"
  done > "$CTL/.bare"
  # ⚠ `wc -l`, NOT `grep -c .`. `grep` exits 1 when no line is selected, so on an
  # EMPTY `.bare` — the state this ratchet exists to let you reach — the
  # assignment failed and `set -e` aborted the required gate before the
  # comparison ran. The success case was the one that broke it.
  _mut_bare="$(wc -l < "$CTL/.bare" | tr -d '[:space:]')"
  if [ "$_mut_bare" -gt "$_MUT_UNRECORDED_MAX" ]; then
    echo "!! $_mut_bare controls have no mutation record, against a ratchet of $_MUT_UNRECORDED_MAX." >&2
    echo "   Either the new control needs a record, or a record was deleted. The bare ones:" >&2
    sed 's/^/     /' "$CTL/.bare" >&2
    echo "   If a control genuinely cannot have one, say why and RAISE the ratchet in the" >&2
    echo "   same edit — so widening the gap is a visible decision rather than a silence." >&2
    _mut_corr_bad=1
  fi
  command rm -f "$CTL/.bare"
  return "$_mut_corr_bad"

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
    # THE PARTS, SPELLED ONCE: every file beside the wire that a mutant needs
    # beside IT, under the name the copy derives from its own `$SELF`
    # (`$_mut_base.<part>.sh`). The copy below, the trap's `rm -f` and the
    # stale-report skip all read this one list, so a new part is one word here
    # rather than three sites that have to agree.
    # ⚠ EVERY PART, NOT ONLY THE CONTROLS. Each part is sourced by name from a
    # sibling, so a mutant with the controls beside it but not the mutation set
    # exits 2 ("decided nothing") for a reason that has nothing to do with the
    # mutation — which the harness then reports as the entry failing. Caught by
    # the standing negative control in the same run that split the mutation set
    # out; the harness and the fixture build are sourced the same way.
    _MUT_PARTS="controls harness mutations fixtures mutgen"
    # THE SIBLING GUARD — HERE, BEFORE THE LOOP, AND NOWHERE ELSE. The shipped
    # files beside the wire must be exactly `$_MUT_PARTS`: a part left off the
    # list is not copied, and every mutant would then exit 2 for a reason that is
    # not its mutation. It cannot be always-on: inside a mutant `$SELF` is
    # `….mutant.$$.sh`, every sibling carries `.mutant.`, and the filtered glob
    # below is empty.
    _mp_have=""
    for _mp_f in "${SELF%.sh}".*.sh; do
      case "$_mp_f" in *.mutant.*|*'.*.sh') continue ;; esac
      _mp_n="${_mp_f#"${SELF%.sh}".}"; _mp_have="$_mp_have ${_mp_n%.sh}"
    done
    if [ "$(printf '%s\n' $_mp_have | sort | tr '\n' ' ')" != "$(printf '%s\n' $_MUT_PARTS | sort | tr '\n' ' ')" ]; then
      echo "!! the parts beside the wire are [$_mp_have ], but \`_MUT_PARTS\` is [ $_MUT_PARTS ]," >&2
      echo "   so a mutant would run without one of them, or beside one nobody copies." >&2
      echo "   The mutation run decided nothing." >&2
      exit 2
    fi
    # ⚠ A LEFTOVER IS A REPORT, NOT A FILE TO CLEAN UP. `trap` does not run on
    # SIGKILL, so a killed run leaves mode-755 artifacts in `.claude/tools/`
    # where a `git add -A` would stage them. Say so; do not delete another run's.
    for _stale in "${SELF%.sh}".mutant.*.sh; do
      case "$_stale" in *'.mutant.*.sh') break ;; esac
      case "$_stale" in "$_mut_wire"|"$_mut_base".*) continue ;; esac
      echo "  note: a previous mutation run left $_stale behind (SIGKILL?); it is" >&2
      echo "        not this run's to remove. Delete it once no run is using it." >&2
    done
    trap 'command rm -f "$_mut_wire"; for _mp in $_MUT_PARTS; do command rm -f "$_mut_base.$_mp.sh"; done; case "$SCRATCH" in /*/*) chmod -R u+rwX "$SCRATCH" 2>/dev/null || true; rm -rf "$SCRATCH";; esac' EXIT
    for _mp in $_MUT_PARTS; do cp "${SELF%.sh}.$_mp.sh" "$_mut_base.$_mp.sh"; done
    # Through a FILE, not a pipe: the counters below must survive the loop, and a
    # `_mutants | while` runs the body in a subshell that discards them.
    _mutants > "$CTL/.mutants"
    _mut_n=0; _mut_bad=0
    while IFS="$(printf '\t')" read -r _mx _mwant; do
      [ -n "$_mx" ] || continue
      _mut_n=$((_mut_n + 1))
      if ! sed "$_mx" "$SELF" > "$_mut_wire" 2>/dev/null; then
        echo "!! MUTANT $_mut_n: the expression is not a valid sed script: $_mx" >&2
        _mut_bad=$((_mut_bad + 1)); continue
      fi
      # ⚠ `|| _rc=$?`, NOT `cmd; _rc=$?`: under `set -e` a simple command that
      # returns non-zero ends the run THERE, with no diagnostic at all — which
      # is what a survived mutant did while this refactor was being written.
      _mrc2=0; _mut_trial "$_mut_n ($_mwant)" "$_mwant" || _mrc2=$?
      [ "$_mrc2" -ne 2 ] || { _mut_bad=$((_mut_bad + 1)); continue; }
      [ "$_mrc2" -ne 1 ] || {
        echo "!! MUTANT $_mut_n ($_mwant) SURVIVED: the wire still exited 0 with this" >&2
        echo "   applied, so nothing above is testing it: $_mx" >&2
        _mut_bad=$((_mut_bad + 1)); }
    done < "$CTL/.mutants"
    command rm -f "$CTL/.mutants"
    echo "  mutation set: $_mut_n entr(ies), $_mut_bad not killed as named"
    # …and the population the wire's own regexes define, which no list here
    # enumerates. Run whatever the hand set did, so one run answers both
    # questions and a failure in either is reported before the exit.
    _mut_gen_n=0; _mut_gen_bad=0
    _mut_gen_run
    echo "  generated boundary set: $_mut_gen_n mutant(s) from \$K2RE and \$K2RE_PATH, $_mut_gen_bad neither killed nor argued equivalent"
    command rm -f "$_mut_wire"; for _mp in $_MUT_PARTS; do command rm -f "$_mut_base.$_mp.sh"; done
    [ "$_mut_bad" -eq 0 ] && [ "$_mut_gen_bad" -eq 0 ] || exit 1
    echo "  every entry above was shown to red, and to red for its own reason;"
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
