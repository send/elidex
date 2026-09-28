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
# splitting them puts the two halves of one assertion in two files". They
# must — and the correspondence check below is what ENFORCES that rather than
# hoping for it. Being cross-file is the point: the check reads the
# controls file for labels and this file for records, and reds when they drift.
#
# WHAT IT CONSUMES: `$CTL` (from the controls harness), `$_CONTROLS` (the
# controls file, read for labels), and `$SELF`, `$SCRATCH`, `$K2RE` and
# `$K2RE_PATH` (from the wire); every other part it names is derived from
# `$SELF` through `_MUT_PARTS`. `_mut_run` copies every part in that list — the
# controls, the harness, this file, the fixture build and the generated half —
# beside each mutant.
# ⚠ THE TWO REGEXES ARE CONSUMED AS VALUES, not just named in a `for` list:
# `_mut_gen_run` compares what the wire's assignment LINE reads back as against
# what the RUNNING wire HOLDS, and that second half is these variables. They
# were left off this list and out of the guard by the commit that added the
# generator — the same drift the guard exists to catch, in the edit that
# widened what there is to drift.
# WHAT IT DEFINES: `_MUT_UNRECORDED_MAX`, `_MUT_RECORDS_MIN`, `_MUTGEN`,
# `_MUT_TARGETS`, `_mutants`, `_mut_correspondence`, `_mut_target`,
# `_mut_restore_copies`, `_mut_rm_copies`, `_mut_trial`, `_mut_run`. The generated
# half — `_mut_equivalent`, `_mut_assign_value`, `_mut_regex_mutants`,
# `_mut_splice`, `_mut_gen_run` — is in `…trip-wire.mutgen.sh`, sourced below.
#
# TWO POPULATIONS, AND THE BOUNDARY IS WHAT EACH ONE'S UNIT IS.
#   * `_mutants` — HAND-WRITTEN, and its unit is a LABEL: every `_control`'s
#     label and every `_lbl="…"` definition in the controls file (the labels of
#     the blocks that are not `_control`s, and of the fixture build window's
#     producers). Every label has to be named by some record
#     (`_MUT_UNRECORDED_MAX` below is what enforces that), and the record says
#     which edit that label is about. Its claim: *THAT* label is about *THAT*
#     edit. It is a floor and cannot be a
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
for _n in CTL _CONTROLS SELF SCRATCH K2RE K2RE_PATH; do
  [ -n "${!_n:-}" ] || _mut_missing="$_mut_missing \$$_n"
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
# The expression is applied to a COPY of the file it names — the wire, unless the
# record says otherwise (below); the copy must (a) differ from
# the original — a stale anchor that matches nothing is a FAILED entry, not a
# passing one — (b) exit non-zero, and (c) print the named control's own
# diagnostic, so an entry that reds for an unrelated reason is caught too.
#
# ⚠ AND THE FILE IT EDITS IS PART OF THE RECORD. An expression may carry the
# prefix `harness:` or `fixtures:`, which aims it at the CONTROL HARNESS or the
# FIXTURE BUILD instead of the wire. The run copies every shipped part beside
# each mutant, so a record says only which one to `sed`: the prefixes are the
# words of `_MUT_TARGETS`, and `_mut_target` derives each one's pair.
#
# RUN IT — it is NOT part of the gate (it costs one full control pass per entry,
# minutes rather than seconds), and a required check nobody can afford to run is
# how gates get switched off:
#
#     WEBREF_WIRE_MUTANTS=1 bash .claude/tools/webref-generic-core-trip-wire.sh
#
# ⚠ ADDING A LABEL MEANS ADDING A RECORD — a `_control` or an `_lbl="…"`
# definition, whatever it is about — the ratchet below is what makes that true. What does NOT belong here is a
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
#     or an `_lbl="…"` definition (the blocks that are not `_control`s, and the
#     fixture build window's producers) — since a stale anchor would otherwise
#     report "wrong reason" forever. It is a grep for the quoted text, not a
#     parse of the call. AND the number of LABELS with NO record — `_control`
#     labels and `_lbl="…"` definitions alike — is ratcheted:
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
# WHICH FILES A RECORD MAY EDIT, BESIDES THE WIRE: the prefix parts, spelled
# once. A record with no prefix edits the wire. `wire` is not a prefix: POSIX `w`
# takes a wfile operand, so `wire:p` is a VALID write to a file called `ire:p`
# (measured: BSD sed exits 0), and a record beginning with it must not be
# re-aimed. A prefix that is not listed here reaches `sed` with the prefix
# attached, where it fails to parse (measured on BSD sed: `sed 'harness:p'` is
# "extra characters at the end of h command", `sed 'fixtures:p'` is "invalid
# command code f"; GNU sed is not measured here).
_MUT_TARGETS="harness fixtures"
# THE PARTS, SPELLED ONCE: every file beside the wire, under the name a copy
# derives from its own `$SELF` (`<wire>.<part>.sh`). The sibling guard, the
# per-run copies, the trap's `rm -f` and the stale-report skip all read it.
_MUT_PARTS="controls harness mutations fixtures mutgen"
_MUT_UNRECORDED_MAX=21
_MUT_RECORDS_MIN=122
# ⚠ A FUNCTION, NOT `x="$(cat <<'EOF' … )"`. Under bash 3.2 — the stock macOS
# shell this wire commits to — a quoted here-document nested inside a command
# substitution is still parsed for expansions, and the `unset "$_v"` in one of
# the records below aborted the WHOLE FILE with `_v: unbound variable`, on every
# run, mutation mode or not. Measured: 3.2 red, 5.3 green.
# ⚠ AND THE FORMAT HAS A COMMENT SYNTAX — a `#` in column 1 — dropped HERE, at
# the single materialiser both readers go through, so one filter covers both, and
# a note can sit beside the records it is about. `^#` is safe to reserve: a record's expression cannot begin with a literal `#`
# except as a sed comment, which is inert. The always-on validator below stays —
# a line that is neither a comment nor a record is still an error, not a skip.
# ⚠ FILTERED IN BASH, NOT THROUGH `grep -v`. This file runs under `pipefail`:
# `grep` exits 1 when it selects nothing, so a set that happened to be all
# comments — or a grep on `PATH` that answers differently — would abort the
# required gate from inside the thing reporting it. Same trap as the `wc -l`
# and `awk` notes below. Measured byte-identical to the `grep -v '^#'` pipe on
# bash 3.2.57 and 5.3, with status 0 at EOF on both.
_mutants() {
  while IFS= read -r _mu_line; do
    case "$_mu_line" in
      '#'*) : ;;
      *) printf '%s\n' "$_mu_line" ;;
    esac
  done <<'MUTANTS'
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
# THE FIXTURE BUILD WINDOW's records (docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §6).
# One or more per label; most edit the harness, where the window's producers
# live, and the rest the fixtures file, whose own shape the window checks.
harness:s/^    [.] [.]\/prelude[.]sh$/    . .\/prelude.sh; exit 0/	the fixture build window completed
harness:s/printf 'set -euo pipefail\\n'/printf 'set -eo pipefail\\n'/	the fixture build window completed
harness:s/printf 'set -euo pipefail\\n'/printf 'set -eu\\n'/	the fixture build window completed
harness:s/printf 'set -euo pipefail\\n'/printf 'set -uo pipefail\\n'/	the fixture build window completed
fixtures:s/^mkdir -p "[$]CTL\/walk\/sub"$/return 0/	the fixture build window completed
fixtures:s/^: > "[$]_FW_DIR\/built"$/set +e; : > "$_FW_DIR\/built"/	the fixture build window completed
# W2's record removes the incomplete-window exit AND leaves the window
# incomplete (its `done` marker renamed), in one harness expression: a record
# edits one file. W2's producers are `_control`'s first statement and the gates
# of the three non-`_control` blocks (relative scratch, fsmonitor, umask); the
# record pins that W2 is reported when the exit is gone, which `_control`'s gate
# alone satisfies. ⚠ A DECLARED GAP: removing one of the three block gates
# survives this set — the block would run over the unbuilt tree, but W2 is
# already reported by the first `_control` — so those gates are pinned only by
# the traced `w2rec` cell (companion §A.14), not by a record.
harness:/^_fgit_window_incomplete_exit()/,/^}/s/^  exit 2$/  :/;s/: > "[$]_FW_DIR\/done"'/: > "$_FW_DIR\/notdone"'/	no control runs over an incomplete fixture build window
fixtures:s/^mkdir -p "[$]CTL\/walk\/sub"$/mkdir -p "$CTL\/walk\/sub"; _ar=$(( 1\/0 ))/	the fixtures file ran without a shell diagnostic
harness:s/"LC_ALL=C")$/"LC_ALL=C" GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=k2.probe GIT_CONFIG_VALUE_0=1)/	a window git whose inputs no fixtures-file command altered reads configuration only from its repo's config file
harness:s/git -c a[.]b=c config --list/git config --list/	this git reports a non-local configuration scope
harness:s/ GIT_CONFIG_NOSYSTEM=1 GIT_ATTR_NOSYSTEM=1 / GIT_ATTR_NOSYSTEM=1 /	the fixture git has no system or global layer outside the void
harness:s/GIT_CONFIG_NOSYSTEM=0 GIT_ATTR_NOSYSTEM=0 git -C/git -C/	this git names its system files through git var
harness:s/^mkdir "[$]_FGIT_VOID" || exit 2$/mkdir "$_FGIT_VOID" \&\& : > "$_FGIT_VOID\/k2plant" || exit 2/	nothing is written into the fixture git's void
harness:s/ "GIT_TEMPLATE_DIR=[$]_FGIT_VOID"//	the fixture git copies no template
harness:s/"LC_ALL=C")$/"LC_ALL=C" GIT_EXEC_PATH=\/nonexistent-k2)/	no exec-path override reaches the fixture git
harness:s/ "LC_ALL=C")$/)/	the fixture build window reads in the wire's locale
harness:s/ GIT_DEFAULT_REF_FORMAT=files / /	the fixture repos use the files ref format
harness:s/chmod "[$]_sm" "[$]CTL\/[$]_sp" 2>\/dev\/null/false/	every mode restriction a fixture sealed was applied
harness:s/"[$]_FGIT_ENVBIN" -i /"$_FGIT_ENVBIN" /	the fixture build window's environment holds only its allowlist
harness:s/"[$]_pfenv" -0 > /{ "$_pfenv" -0; false; } > /	the fixture build window's environment holds only its allowlist
harness:s/"[$]_pfenv" -0 > /false > /	the fixture build window's environment holds only its allowlist
harness:s/"[$]_pfenv" -0 > /{ echo k2 >\&2; } > /	the fixture build window's environment holds only its allowlist
harness:s/_pfenv="[$]_FGIT_ENVBIN"/_pfenv=\/nonexistent-k2\/env/	the fixture build window's environment holds only its allowlist
fixtures:s/^: > "[$]_FW_DIR\/built"$/echo '[include] path = \/nonexistent-k2' >> "$CTL\/clean\/.git\/config"; : > "$_FW_DIR\/built"/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^( cd "[$]CTL\/cachedir" && git init -q [.] /( cd "$CTL\/cachedir" \&\& git init -q --separate-git-dir="$CTL\/.gd-cachedir" . /	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^: > "[$]_FW_DIR\/built"$/git -C "$CTL\/clean" config --unset core.filemode; : > "$_FW_DIR\/built"/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^: > "[$]_FW_DIR\/built"$/( mkdir -p "$CTL\/zz\/inner" \&\& cd "$CTL\/zz\/inner" \&\& git init -q . \&\& git config core.excludesFile \/nonexistent-k2 ); : > "$_FW_DIR\/built"/	every fixture repo persists only the configuration a plain git init writes
s/^# Run from anywhere\./# Run from anywhere (edited by the negative control)./	!survive
MUTANTS
}

# The resolver: $1 = `wire` or a word of `$_MUT_TARGETS`; out = `$_mut_src` (the
# shipped file) and `$_mut_tgt` (the mutable copy `sed` writes and `_mut_trial`
# compares) — `_mut_run`'s per-run names, derived rather than listed per part.
_mut_target() {
  case "$1" in
    wire) _mut_src="$SELF"; _mut_tgt="$_mut_wire" ;;
    *)    _mut_src="${SELF%.sh}.$1.sh"; _mut_tgt="$_mut_base.$1.sh" ;;
  esac
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
# caller resolved, which for a `harness:` or `fixtures:` entry is NOT the wire;
# it is the wire copy that gets EXECUTED either way.
# $1 = how to name it in a diagnostic, $2 = what is required:
# a NEEDLE the output must contain, `!survive`, or `!kill` (red, raised by a
# control, with no control named).
# Returns 0 = as required, 1 = SURVIVED (the caller decides what that means),
# 2 = failed some other way, already reported.
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
  # properties of this shipped file and cost milliseconds, so they belong where
  # every run sees them. (The mutation RUN stays opt-in; only its bookkeeping
  # moved.)
  _mutants > "$CTL/.mutants"
  _mut_orphan=0
  while IFS="$(printf '\t')" read -r _mline _mwant; do
    [ -n "${_mline:-}" ] || continue
    # ⚠ A LINE THAT IS NOT A RECORD IS AN ERROR HERE, NOT SOMETHING TO SKIP, and
    # the asymmetry it replaces is why: this loop dropped every line with no
    # needle, so a line the here-document turns into DATA passed this check in
    # silence while `_mut_run` applied it as a sed script and reported MATCHED
    # NOTHING. Four such lines were added and shipped that way; what caught them
    # was the opt-in run, minutes long, and what should have is this, which costs
    # nothing. `_mutants` now drops `#` comments at its own materialiser, so what
    # reaches here is a line that is neither — prose with no `#`, or a record
    # whose TAB an editor ate.
    if [ -z "${_mwant:-}" ]; then
      echo "!! mutation set line \"$_mline\" has no TAB-separated needle, so it is not a" >&2
      echo "   record. Write a note as a \`#\` comment; anything else in that" >&2
      echo "   here-document is data, and the mutation run applies it as a sed expression." >&2
      _mut_orphan=1; continue
    fi
    # `!survive` is the standing negative control; it names no control by design.
    [ "$_mwant" != '!survive' ] || continue
    grep -qF -- "\"$_mwant\"" "$_CONTROLS" || {
      echo "!! mutation record needle \"$_mwant\" appears nowhere in the controls file" >&2
      echo "   as a quoted label. A record whose control was renamed reports" >&2
      echo "   \"wrong reason\" forever." >&2
      _mut_orphan=1; }
  done < "$CTL/.mutants"
  [ "$_mut_orphan" -eq 0 ] || _mut_corr_bad=1
  # ---- EVERY RECORD STILL APPLIES, ALWAYS ON ------------------------------------
  # Each record's expression must change the file it targets. Records anchor on
  # harness and fixtures text that an equivalent edit can change (spelling an
  # option long, adding an allowlist entry), and a record that matches nothing
  # tests a copy identical to the shipped file. The opt-in run would catch it,
  # minutes later and only when someone runs it; this costs one `sed` per record.
  # ⚠ NOT INSIDE A MUTANT: there one target is edited ON PURPOSE, so the records
  # anchored on the edited text no longer apply, by design.
  case "${SELF##*/}" in
    *.mutant.*) : ;;
    *) while IFS="$(printf '\t')" read -r _ma_x _; do
         [ -n "${_ma_x:-}" ] || continue
         _ma_src="$SELF"
         for _ma_t in $_MUT_TARGETS; do
           case "$_ma_x" in "$_ma_t":*) _ma_src="${SELF%.sh}.$_ma_t.sh"; _ma_x="${_ma_x#"$_ma_t":}"; break ;; esac
         done
         if ! sed "$_ma_x" "$_ma_src" > "$CTL/.anchor" 2>/dev/null; then
           echo "!! mutation record \"$_ma_x\" is not a valid sed expression." >&2; _mut_corr_bad=1
         elif cmp -s "$CTL/.anchor" "$_ma_src"; then
           echo "!! mutation record \"$_ma_x\" no longer matches ${_ma_src##*/}: its anchor is stale," >&2
           echo "   so the record would test a copy identical to the shipped file." >&2
           _mut_corr_bad=1
         fi
       done < "$CTL/.mutants"
       command rm -f "$CTL/.anchor" ;;
  esac
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
  # …and the direction that actually finds things: labels with NO record. A
  # label is the third quoted argument of a `_control` call, or the value of an
  # `_lbl="…"` definition — a block that is not a `_control` prints one, and so
  # does each of the fixture build window's producers.
  _mut_bare=0
  { awk -F'"' '/^ *_control /{print $6}' "$_CONTROLS"
    awk -F'"' '/^ *_[A-Za-z0-9_]+_lbl="/{print $2}' "$_CONTROLS"; } \
    | sed '/^$/d' | while IFS= read -r _lbl; do
    grep -qF -- "	$_lbl" "$CTL/.mutants" || printf '%s\n' "$_lbl"
  done > "$CTL/.bare"
  # ⚠ `wc -l`, NOT `grep -c .`. `grep` exits 1 when no line is selected, so on an
  # EMPTY `.bare` — the state this ratchet exists to let you reach — the
  # assignment failed and `set -e` aborted the required gate before the
  # comparison ran. The success case was the one that broke it.
  _mut_bare="$(wc -l < "$CTL/.bare" | tr -d '[:space:]')"
  if [ "$_mut_bare" -gt "$_MUT_UNRECORDED_MAX" ]; then
    echo "!! $_mut_bare labels have no mutation record, against a ratchet of $_MUT_UNRECORDED_MAX." >&2
    echo "   Either the new label needs a record, or a record was deleted. The bare ones:" >&2
    sed 's/^/     /' "$CTL/.bare" >&2
    echo "   If a label genuinely cannot have one, say why and RAISE the ratchet in the" >&2
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
    # ⚠ A LEFTOVER IS A REPORT, NOT A FILE TO CLEAN UP. `trap` does not run on
    # SIGKILL, so a killed run leaves mode-755 artifacts in `.claude/tools/`
    # where a `git add -A` would stage them. Say so; do not delete another run's.
    for _stale in "${SELF%.sh}".mutant.*.sh; do
      case "$_stale" in *'.mutant.*.sh') break ;; esac
      case "$_stale" in "$_mut_wire"|"$_mut_base".*) continue ;; esac
      echo "  note: a previous mutation run left $_stale behind (SIGKILL?); it is" >&2
      echo "        not this run's to remove. Delete it once no run is using it." >&2
    done
    trap '_mut_rm_copies; case "$SCRATCH" in /*/*) chmod -R u+rwX "$SCRATCH" 2>/dev/null || true; rm -rf "$SCRATCH";; esac' EXIT
    # Every part once; the wire and the targets are re-copied per entry by
    # `_mut_restore_copies`, because entries edit them.
    for _mp in $_MUT_PARTS; do cp "${SELF%.sh}.$_mp.sh" "$_mut_base.$_mp.sh"; done
    # Through a FILE, not a pipe: the counters below must survive the loop, and a
    # `_mutants | while` runs the body in a subshell that discards them.
    _mutants > "$CTL/.mutants"
    _mut_n=0; _mut_bad=0
    while IFS="$(printf '\t')" read -r _mx _mwant; do
      [ -n "$_mx" ] || continue
      _mut_n=$((_mut_n + 1))
      # WHICH SHIPPED FILE THIS ENTRY EDITS — the wire unless the expression
      # carries a word of `$_MUT_TARGETS` as a prefix, which is stripped before
      # `sed` sees it.
      # ⚠ ANCHORED ON THE KNOWN NAMES, NEVER A GENERIC `<word>:` PARSE:
      # `sed 'y/a:/b;/'` is a valid expression whose `${_mx%%:*}` is `y/a`, so a
      # parse would re-aim a record at a target that does not exist.
      _mt_which=wire
      for _mut_t in $_MUT_TARGETS; do
        case "$_mx" in
          "$_mut_t":*) _mt_which="$_mut_t"; _mx="${_mx#"$_mut_t":}"; break ;;
        esac
      done
      _mut_restore_copies || { _mut_bad=$((_mut_bad + 1)); continue; }
      _mut_target "$_mt_which"
      if ! sed "$_mx" "$_mut_src" > "$_mut_tgt" 2>/dev/null; then
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
    _mut_rm_copies
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
