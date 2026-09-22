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
# earlier revision of the plan memo booked this as a defer slot to be done after
# the PR landed — which inverts "prereq" into "afterwards" and uses the
# single-commit requirement as a reason to merge the oversized file first. The
# external reviewer caught that (P1). This file is that standalone commit.
#
# ⚠ AND THE COUNTER-ARGUMENT IS ANSWERED, not ignored. The memo previously
# argued against this seam on the ground that "the two lists must be edited
# together, so splitting them puts the two halves of one assertion in two
# files". They must — and the correspondence check below is what ENFORCES that
# rather than hoping for it. Being cross-file is the point: the check reads the
# controls file for labels and this file for records, and reds when they drift.
#
# WHAT IT CONSUMES: `$CTL`, `$_CONTROLS` and `$_MUTATIONS` (from the controls
# file — the last is this file's own path, which `_mut_run` copies beside each
# mutant), plus `$SELF` and `$SCRATCH` (from the wire).
# WHAT IT DEFINES: `_MUT_UNRECORDED_MAX`, `_mutants`, `_mut_correspondence`,
# `_mut_run`.
# ⚠ AND IT WRITES NOTHING OF THE CALLER'S. `_mut_correspondence` used to set
# `ctl_ok` — a variable owned by the controls file — so a rename there would
# have left this file assigning an unused global while the caller's status
# stayed green: the exact cross-file drift the entry guard exists to prevent,
# reintroduced by the split that added the guard. It RETURNS a status now and
# the caller decides, which is a contract a rename cannot silently break.
# ⚠ Asserted at entry, for the same reason the controls file asserts its own:
# a stated interface nobody checks drifts like any other unexecuted claim.
_mut_missing=
for _n in CTL _CONTROLS _MUTATIONS SELF SCRATCH; do
  [ -n "${!_n:-}" ] || _mut_missing="$_mut_missing \$$_n"
done
if [ -n "$_mut_missing" ]; then
  echo "!! This file is the MUTATION SET for \`webref-generic-core-trip-wire.sh\`. It is" >&2
  echo "   SOURCED by that wire's controls and has no meaning alone; missing:$_mut_missing" >&2
  echo "   Run the wire instead." >&2
  exit 2
fi

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
# ⚠ ADDING AN ARM MEANS ADDING A RECORD. Nothing here can detect an arm that
# never had one — this set is a floor, not a census. ⚠ AND A DECLARED FLOOR IS
# NOT A DETECTOR: this file used to say exactly the sentence above and assert
# nothing, so deleting a record shrank the set in silence and the run stayed
# green. Three things now hold it up, and each is checked below rather than
# described:
#   * a CORRESPONDENCE, IN BOTH DIRECTIONS AND ALWAYS-ON. Every record's needle
#     must name an actual `_control` label in this file (a stale anchor would
#     otherwise report "wrong reason" forever), AND the number of controls with
#     NO record is ratcheted: `_MUT_UNRECORDED_MAX` may only come down.
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
_MUT_UNRECORDED_MAX=21
# ⚠ A FUNCTION, NOT `x="$(cat <<'EOF' … )"`. Under bash 3.2 — the stock macOS
# shell this wire commits to — a quoted here-document nested inside a command
# substitution is still parsed for expansions, and the `unset "$_v"` in one of
# the records below aborted the WHOLE FILE with `_v: unbound variable`, on every
# run, mutation mode or not. Measured: 3.2 red, 5.3 green.
_mutants() { cat <<'MUTANTS'
s/grep -aEn --/grep -En --/	K2 fires inside binary content, and the content is READ
s/"$_mrc" -gt 1/"$_mrc" -gt 99/	a failed NAME matcher fails closed
s/^K2RE_PATH=.*/K2RE_PATH="$K2RE"/	a quote inside a name segment
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
s/\[ -e "$p" \] || \[ -L "$p" \]/[ -e "$p" ]/	a symlinked EXTRA entry is scanned
s/REL_DIR="${SCOPE_DIR#"$ROOT"\/}"/REL_DIR="${SCOPE_DIR#$ROOT\/}"/	a glob character in the checkout path does not widen the scope
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
s/\]*\[^\]\/\[:space:\]"'"'"'`)}>,;\]'/]*]'/	punctuation BEFORE a slash is part of the path
s/(^|\/)\\.claude/\\.claude/	a segment merely ENDING in .claude is not the host path
s#^K2RE_PATH=.*#K2RE_PATH='\\.claude/(skills|tools)/[^/]+/[^/]+'#	a segment merely ENDING in .claude is not the host path
s/\[ "${WEBREF_WIRE_SELFTEST_PPID:-}" != "$PPID" \]/false/	an inherited SELFTEST export cannot redirect the gate
s/^# Run from anywhere\./# Run from anywhere (edited by the negative control)./	!survive
MUTANTS
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
      echo "!! mutation record needle \"$_mwant\" matches no _control label in this file." >&2
      echo "   A record whose control was renamed reports \"wrong reason\" forever." >&2
      _mut_orphan=1; }
  done < "$CTL/.mutants"
  _mut_corr_bad=0
  [ "$_mut_orphan" -eq 0 ] || _mut_corr_bad=1
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
    _mut_wire="${SELF%.sh}.mutant.$$.sh"
    _mut_ctl="${SELF%.sh}.mutant.$$.controls.sh"
    # ⚠ AND THIS FILE TOO. The copy derives its own `_MUTATIONS` from its name,
    # so a mutant with the controls beside it but not the mutation set exits 2
    # ("decided nothing") for a reason that has nothing to do with the mutation
    # — which the harness then reports as the entry failing. Caught by the
    # standing negative control in the same run that split this file out.
    _mut_mut="${SELF%.sh}.mutant.$$.mutations.sh"
    # ⚠ A LEFTOVER IS A REPORT, NOT A FILE TO CLEAN UP. `trap` does not run on
    # SIGKILL, so a killed run leaves mode-755 artifacts in `.claude/tools/`
    # where a `git add -A` would stage them. Say so; do not delete another run's.
    for _stale in "${SELF%.sh}".mutant.*.sh; do
      case "$_stale" in *'.mutant.*.sh') break ;; esac
      case "$_stale" in "$_mut_wire"|"$_mut_ctl") continue ;; esac
      echo "  note: a previous mutation run left $_stale behind (SIGKILL?); it is" >&2
      echo "        not this run's to remove. Delete it once no run is using it." >&2
    done
    trap 'command rm -f "$_mut_wire" "$_mut_ctl" "$_mut_mut"; case "$SCRATCH" in /*/*) chmod -R u+rwX "$SCRATCH" 2>/dev/null || true; rm -rf "$SCRATCH";; esac' EXIT
    cp "$_CONTROLS" "$_mut_ctl"
    cp "$_MUTATIONS" "$_mut_mut"
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
      # ⚠ AND IT MUST BE EXECUTABLE. `_control` invokes `"$SELF"` DIRECTLY, not
      # through `bash`, so a copy written by `sed` (mode 644) exits 126
      # "Permission denied" for EVERY control — which reds the run, prints every
      # control's diagnostic, and therefore satisfies a "did it name the right
      # control?" test VACUOUSLY. Measured: all 18 entries below "passed" that
      # way, and a deliberately inert entry (a comment-only edit that cannot
      # change any verdict) was the negative control that exposed it. The probe's
      # subject was the permission bit, not the mutation.
      chmod +x "$_mut_wire"
      if cmp -s "$_mut_wire" "$SELF"; then
        echo "!! MUTANT $_mut_n ($_mwant): the expression MATCHED NOTHING, so this entry" >&2
        echo "   tested a copy identical to the wire: $_mx" >&2
        _mut_bad=$((_mut_bad + 1)); continue
      fi
      _mrc2=0
      _mout="$(env -u WEBREF_WIRE_MUTANTS bash "$_mut_wire" 2>&1)" || _mrc2=$?
      if [ "$_mwant" = '!survive' ]; then
        # THE STANDING NEGATIVE CONTROL. Its edit is to COMMENT TEXT, so it
        # cannot change any verdict by construction and the wire MUST still be
        # green. If it reds, the harness is failing mutants for a reason that has
        # nothing to do with the mutation — a missing execute bit, a clobbered
        # copy, a full disk — and every "killed" above is unearned.
        [ "$_mrc2" -eq 0 ] || {
          echo "!! THE NEGATIVE CONTROL DIED (exit $_mrc2). An edit that changes no verdict" >&2
          echo "   reddened the wire, so this run cannot tell a real kill from a broken" >&2
          echo "   harness, and every kill reported above is unearned. Output:" >&2
          printf '%s\n' "$_mout" | sed 's/^/     /' >&2
          _mut_bad=$((_mut_bad + 1)); }
        continue
      fi
      if [ "$_mrc2" -eq 0 ]; then
        echo "!! MUTANT $_mut_n ($_mwant) SURVIVED: the wire still exited 0 with this" >&2
        echo "   applied, so nothing above is testing it: $_mx" >&2
        _mut_bad=$((_mut_bad + 1)); continue
      fi
      case "$_mout" in
        *"$_mwant"*) ;;
        *) echo "!! MUTANT $_mut_n killed for the WRONG REASON (exit $_mrc2): the output" >&2
           echo "   does not name \"$_mwant\", so another check masked the one under test." >&2
           _mut_bad=$((_mut_bad + 1)) ;;
      esac
    done < "$CTL/.mutants"
    command rm -f "$_mut_wire" "$_mut_ctl" "$_mut_mut"
    echo "  mutation set: $_mut_n entr(ies), $_mut_bad not killed as named"
    [ "$_mut_bad" -eq 0 ] || exit 1
    echo "  every entry above was shown to red, and to red for its own reason"
    # ⚠ NO `exit 0` HERE, AND THAT IS THE WHOLE POINT. This block used to end the
    # run — so `WEBREF_WIRE_MUTANTS` merely PRESENT IN THE ENVIRONMENT (exported
    # once, in a shell that later runs `mise run trip-wires`) made the required
    # gate exit 0 having never scanned `_webref/`, with the driver recording it as
    # a wire that ran. Every other path in this file refuses to be green over
    # something it did not read; this was the one that did. The mutation set is
    # now something the run does BEFORE its verdict, not INSTEAD of it.
  fi
}
