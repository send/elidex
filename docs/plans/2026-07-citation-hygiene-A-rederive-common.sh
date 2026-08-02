# Shared part of the re-derivation harness — sourced by
# `2026-07-citation-hygiene-A-rederive.sh`, which is the only entry point.
# Not executable on its own: it defines no dispatch and sets no shell options.
# It does resolve `$REPO_ROOT` at source time (below), because every other part
# and the dispatcher itself depend on that being settled before anything runs.
#
# What lives here: the plumbing (`$REPO_ROOT`, `$MAIN`, `$PF`, `say`,
# `$AUTHOR_LOCAL`, the §6 fixture bodies, the §4.2.3 prototype) and every block
# MORE THAN ONE memo cites -- `citations` (A-i, A-ii), `couplings` and `budget`
# (A-i, A-ii, A-iii), `lanes` (A-ii, A-iii, umbrella; author-local).

# THE REPO THIS HARNESS LIVES IN, derived from THIS FILE's own path -- never from
# cwd. `_wtscan`'s roots are relative (`.claude/tools/`), so before this they
# resolved against whatever directory the caller happened to be standing in: the
# dispatcher used to `cd "$(git rev-parse --show-toplevel)"`, and that `cd`
# NO-OPS when the substitution fails. Measured, with a violation planted on the
# branch: invoked with cwd `/` the `cd` printed `fatal: not a git repository`,
# did nothing, both counts came back 0 and `couplings` printed VERDICT: GREEN;
# invoked from a SIBLING worktree it audited that worktree instead of this one.
# Both are the drift `memory/feedback_worktree-cwd-drift.md` records. Failing to
# resolve the root is now fatal rather than silent.
REPO_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && git rev-parse --show-toplevel) || {
  printf 'FATAL: cannot resolve the repo root from %s\n' "${BASH_SOURCE[0]}" >&2
  printf '       -- run the harness from a checkout of the repo it lives in.\n' >&2
  exit 2
}

MAIN=origin/main
PF=.claude/skills/elidex-plan-review/preflight.py
say() { printf '\n=== %s ===\n' "$1"; }

# --- THE MEASUREMENT PRIMITIVE ------------------------------------------------
# EVERY quantity this harness reports goes through here. The umbrella's `:91`
# constraint is *"Counts are commands. No slice memo carries a quantity it did not
# derive"*, so the one thing this harness must not be able to print is a count
# whose command NEVER RAN. The idiom it replaces -- `n=$(cmd | wc -l)` -- could
# not tell that apart from a real zero on EITHER axis:
#
#   STATUS  `$(...)` discards the pipeline's status and `wc -l` of nothing is
#           `0`, which is the PASS condition at every call site here. Measured,
#           in a checkout with no remote-tracking ref: `git grep … origin/main`
#           died `fatal: unable to resolve revision: origin/main`, the count read
#           `0`, and `couplings` printed `VERDICT: GREEN` and exited 0 -- it
#           certified a derivation that did not occur, which is the negation of
#           this harness's charter. `37c7eb02` fixed exactly this for `_wtscan`
#           and left the two `git grep` baselines on the adjacent lines.
#   EMPTY   `printf '%s\n' "$var" | wc -l` is **1** for an empty `$var`, because
#           printf emits the newline unconditionally. A match count that reads 1
#           when there are no matches is the same class from the other side. The
#           `if [ -n … ]` guards that used to hold this off are not needed here:
#           the count is taken from the command's own output, not from a re-print.
#
#   _measure [--nomatch <status>] <var> <cmd> [arg...]
#
# runs <cmd> ONCE, and:
#   * captures its stdout in `$_MEASURE_OUT`, so a caller that must also PRINT
#     the hits reads that instead of running the command a second time (running
#     it twice is how the pre-`37c7eb02` sites threw the status away twice, and
#     it lets the listing and the count disagree),
#   * on success sets <var> to `wc -l` of that output -- 0 for no output,
#   * on failure sets <var> to `!FAILED(rc=N)`, prints a loud diagnostic, and
#     returns 1.
#
# The sentinel is what makes "the command did not run" UNREPRESENTABLE AS A PASS
# rather than merely detected at the sites someone remembered: every gate in this
# harness is `= 0`, `!FAILED(rc=N)` is not `0` and is not a number, so no failed
# measurement can satisfy any of them -- including at a call site written later by
# someone who has not read this comment. Callers should still propagate the
# return status so the block's exit code carries it too; a caller that forgets
# still cannot print GREEN.
#
# `--nomatch <status>` is the status a SEARCH returns when it ran and matched
# nothing (`git grep` -> 1). It is REQUIRED on the baselines and forbidden
# elsewhere: without it "no matches" -- the expected, correct result -- would be
# indistinguishable from a broken ref, which is the confusion this function
# exists to end. Every OTHER status stays a failure, so git's 128 for an
# unresolvable revision and 127 for a missing interpreter are still fatal.
_MEASURE_OUT=""
_measure() {
  local __nomatch=""
  [ "${1:-}" = "--nomatch" ] && { __nomatch=$2; shift 2; }
  local __var=$1; shift
  local __raw __rc=0
  # The `\034` sentinel is not decoration: `$(...)` strips ALL trailing newlines,
  # so output ending in a blank line would count one line short. Appending a
  # non-newline byte and stripping it back makes the count byte-identical to
  # `cmd | wc -l`.
  __raw=$( { "$@"; __r=$?; printf '\034'; exit "$__r"; } ) || __rc=$?
  __raw=${__raw%$'\034'}
  if [ "$__rc" -ne 0 ] && [ "$__rc" != "$__nomatch" ]; then
    _MEASURE_OUT=""
    printf '!! MEASUREMENT FAILED (rc=%s): %s\n' "$__rc" "$*" >&2
    printf '!!   no count is reported -- a command that did not run measured nothing.\n' >&2
    printf -v "$__var" '!FAILED(rc=%s)' "$__rc"
    return 1
  fi
  _MEASURE_OUT=$__raw
  printf -v "$__var" '%s' "$(printf '%s' "$__raw" | wc -l | tr -d ' ')"
  return 0
}

# Print what the last `_measure` captured, without the extra blank line a
# `printf '%s\n'` on an already-newline-terminated capture would add.
_measured() { [ -n "$_MEASURE_OUT" ] && printf '%s' "$_MEASURE_OUT"; return 0; }

# --- fixtures -----------------------------------------------------------------
# The §6 fixture set, emitted into $1. §5's origin/main column and every pin read
# these exact bodies.
HDR='| Spec section | Step | Branch | Touch | Full enum? | User-input flow |
|---|---|---|---|---|---|'
fixtures() {
  local d=$1; mkdir -p "$d"
  { echo '# fixture'; echo; echo '## §3. Spec coverage map'; echo; echo "$HDR"
    echo '| WHATWG HTML §4.10.21 Constraints | s | b | t | ✓ | no |'
    echo '| WHATWG HTML §4.10.21.2 Constraint validation | s | b | t | ✓ | no |'
  } > "$d/labelled.md"
  # two rows resolving to ONE (shortname, section) pair — the only shape that
  # exercises seen_pairs' dedup `continue`, which no earlier fixture reached.
  { echo '# fixture'; echo; echo '## §3. Spec coverage map'; echo; echo "$HDR"
    echo '| WHATWG HTML §4.10.21 Constraints | s | b | t | ✓ | no |'
    echo '| HTML §4.10.21 Constraints, again | s | b | t | ✓ | no |'
  } > "$d/dedup.md"
  { echo '# fixture'; echo; echo '## §3. Spec coverage map'; echo; echo "$HDR"
    echo '| §4.10.21 Constraints | s | b | t | ✓ | no |'
  } > "$d/unlabelled.md"
  # `CSSOM VIEW` is absent from the 24-key pinned map, so this is all-unmapped
  # AFTER A. It is NOT all-unmapped at the carve (the catalog resolves it to
  # `cssom-view-1`) and stops being so again when Slice B lands the fall-through
  # -- see the memo's §6 hand-off. The title is the real one: a citation-hygiene
  # program must not author spec-shaped text with a fabricated §-title, and
  # `verify_citation` checks only that the number exists, so nothing would catch it.
  { echo '# fixture'; echo; echo '## §3. Spec coverage map'; echo; echo "$HDR"
    echo '| CSSOM VIEW §4.2 The MediaQueryList Interface | s | b | t | ✓ | no |'
  } > "$d/allunmapped.md"
  # item 5's denominator clause -- "N = len(data_rows), MALFORMED ROWS INCLUDED".
  # No other fixture has a row without a section mark, so through draft 8 the one
  # clause of item 5 that is a claim about N was the one clause no state measured.
  # Row 1 is unmapped, row 2 is malformed => citations empty, capability present,
  # so the reporting arm fires with N=2 AND `malformed_hard_fail` exits 1: the
  # co-print item 5 asserts is decided separately.
  { echo '# fixture'; echo; echo '## §3. Spec coverage map'; echo; echo "$HDR"
    echo '| CSSOM VIEW §4.2 The MediaQueryList Interface | s | b | t | ✓ | no |'
    echo '| WHATWG HTML Constraints, no section mark | s | b | t | ✓ | no |'
  } > "$d/malformed.md"
  # the alias spelling row 10 needs; unreachable by any draft-6 fixture.
  { echo '# fixture'; echo; echo '## §3. Spec coverage map'; echo; echo "$HDR"
    echo '| Fetch §2.2.5 Requests | s | b | t | ✓ | no |'
  } > "$d/alias.md"
  { echo '# fixture'; echo; echo '## §3. Spec coverage map'; echo
    echo '**No spec surface** — tooling only.'; } > "$d/nospec.md"
  { echo '# fixture'; echo; echo '## §3. Spec coverage map'; echo
    echo '**No spec surface** — tooling only.'; echo; echo "$HDR"
    echo '| WHATWG HTML §4.10.21 Constraints | s | b | t | ✓ | no |'
  } > "$d/nospec-and-table.md"
  { echo '# fixture'; echo; echo '## §3. Spec coverage map'; echo
    echo '**No spec surface** — tooling only.'; echo; echo "$HDR"; } > "$d/nospec-and-header.md"
  # rule (b): a fenced quotation of the marker must NOT be recognised.
  { echo '# fixture'; echo; echo '## §3. Spec coverage map'; echo; echo '```'
    echo '**No spec surface** — quoted, not declared.'; echo '```'; echo; echo "$HDR"
    echo '| WHATWG HTML §4.10.21 Constraints | s | b | t | ✓ | no |'
  } > "$d/fenced-marker.md"
  ls "$d"
}

# --- blocks -------------------------------------------------------------------

citations() {  # §0.5 / §3 — EVERY label-§ pair the fixture set carries
  # Draft 9 tabled four and derived two, and the two it did not derive were the
  # two it had just changed -- including the title corrected FROM a fabrication.
  # Nothing on the branch would have caught a second one at that same site.
  # A lookup that FAILED prints nothing and used to leave this block exiting 0 on
  # the status of `rm -rf` -- an unverified §-title reported as a verified one.
  local rc=0
  .claude/tools/webref heading --exact html 4.10.21       || rc=1
  .claude/tools/webref heading --exact html 4.10.21.2     || rc=1
  .claude/tools/webref heading --exact fetch 2.2.5        || rc=1
  .claude/tools/webref heading --exact cssom-view-1 4.2   || rc=1
  echo "-- and the pairs actually present in the fixture bodies --"
  local F; F=$(mktemp -d); fixtures "$F" >/dev/null || rc=1
  awk -F'|' '/^\| [A-Za-z§]/ && $2 !~ /Spec section/ {gsub(/^ +| +$/,"",$2); print "  "$2}' \
    "$F"/*.md | sort -u || rc=1
  rm -rf "$F"
  return "$rc"
}

_wtscan() {  # $1 = ERE, $2.. = roots RELATIVE TO $REPO_ROOT. Prints `path:line:match`.
  # Why not `git grep`: it sees TRACKED content only, so a violation in a file
  # that exists but is not yet added reads GREEN. Not hypothetical -- measured:
  # with `cite-audit` planted in an UNTRACKED file under `.claude/skills/`, the
  # git-grep form of this block counted 0 and printed `VERDICT: GREEN`, and a
  # bare `git add -N` on the same file flipped it to RED. The unit suite these
  # cross-tree limbs came from walked the filesystem for exactly that reason;
  # moving them here must not trade the property away. The origin/main
  # baselines below stay `git grep` -- only git can read a ref.
  #
  # Roots resolve against $REPO_ROOT, not cwd; python chdir's there so the printed
  # paths stay repo-relative and read the same as before.
  #
  # ⚠ THE EXIT STATUS IS PART OF THE ANSWER, which is why this is called through
  # `_measure` and never through `$(… | wc -l)`: `wc -l` of nothing is `0`, and
  # `0` is the PASS condition, so a scanner that NEVER RAN is indistinguishable
  # from one that found nothing -- the same inference bug this function's
  # `git grep` note is about, one level up. Measured: with `python3` shadowed by
  # `#!/bin/sh\nexit 127` and a violation planted, the pre-`_measure` callers
  # printed `: 0` and `VERDICT: GREEN`.
  local ere=$1; shift
  python3 - "$REPO_ROOT" "$ere" "$@" <<'WTSCANPY'
import os, re, sys
os.chdir(sys.argv[1])
ere = re.compile(sys.argv[2])
for root in sys.argv[3:]:
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in (".git", "__pycache__")]
        for fn in sorted(filenames):
            path = os.path.join(dirpath, fn)
            try:
                with open(path, encoding="utf-8") as fh:
                    for i, line in enumerate(fh, 1):
                        for m in ere.finditer(line):
                            print(f"{path}:{i}:{m.group(0)}")
            except (OSError, UnicodeDecodeError):
                continue
WTSCANPY
}

couplings() {  # §7 / §12(2) / §12(3) — K2 and K3's CROSS-TREE halves
  # SCOPES, written down because this block now carries two of them and
  # conflating them is what the widening below fixes:
  #   PKG     `.claude/tools/_webref/` — the package. The by-role CONCEPT
  #           listing below is about the prose A-i rewrote, which lives here.
  #           `test_spec_labels.py` pins K2 and K3 over exactly this tree.
  #   GENERIC `.claude/tools/` — what §2 defines the generic core to be, and
  #           what K2 is an absolute over. WIDER than the package: it also
  #           holds five elidex trip-wire artifacts owned by other lanes.
  # The unit suite scans PKG only, so GENERIC minus PKG -- and `.claude/skills/`
  # for K3 -- can be witnessed HERE and nowhere else. Verified by planting a
  # violation in each of the three trees; before this widening a plant in
  # `.claude/tools/` outside the package and a `cite-audit` plant in
  # `.claude/skills/` both left this block GREEN.
  local PKG='.claude/tools/_webref/'
  local GENERIC='.claude/tools/'
  local SKILLS='.claude/skills/'
  local CONCEPT='\.claude/skills|elidex-plan-review|plan-review|plan-memo|memos abbreviate'
  # An elidex FILE PATH is what DESIGN.md's closing rule forbids; by-role prose it
  # permits. Draft 8 offered one mixed 25-line list as the check for a claim about
  # paths in A's half -- a reviewer eyeball, not a check, and it saw one of the
  # couplings it was offered as the check for.
  #
  # THE PREDICATE, written down because it is otherwise implicit in the regex and
  # stated nowhere else: an elidex file path is `.claude/skills/` or
  # `.claude/tools/` followed by TWO further path segments, so the tool's OWN
  # invocation path `.claude/tools/webref` -- one segment, 22 occurrences in
  # `cli.py` on origin/main -- never matches. That exclusion is intended: an
  # install path is not a path into elidex's tree.
  local PATHRE='\.claude/(skills|tools)/[A-Za-z0-9_-]+/[A-Za-z0-9_.-]+'
  # A's half of the split tree (§4.0's A column). cite_audit.py / test_cite_audit.py
  # / webref_data.py are B's and must not be counted against A.
  # DERIVED, not listed: A's half is the generic tree minus B's files. An
  # inclusion list cannot see a file the slice CREATES -- which is exactly what
  # it missed (test_spec_labels.py), twice, in the block written to catch it.
  local BFILES='cite_audit|test_cite_audit|webref_data'
  # `failed` is the block's memory that SOME quantity below was not derived. It
  # is checked before any count is, because a count that was never taken is not a
  # count of zero -- see `_measure`.
  local failed=0 _n
  _measure _n git ls-files '.claude/tools/_webref/*.py' '.claude/tools/_webref/**/*.py' \
                           '.claude/tools/_webref/*.md' '.claude/tools/webref' || failed=1
  local AHALF=()
  while IFS= read -r f; do
    [ -n "$f" ] || continue
    printf '%s' "$f" | grep -qE "$BFILES" || AHALF+=("$f")
  done <<< "$_MEASURE_OUT"
  echo "   A-half files: ${#AHALF[@]}"
  # The `| cat` these listings used to carry masked git's status the same way
  # `| wc -l` masked it for the counts: with the ref unresolvable the listing came
  # back EMPTY and the block read that as "no couplings". Routed through
  # `_measure`, the listing and its status arrive together.
  echo "-- concept, origin/main baseline (PKG — the by-role prose A-i rewrites) --"
  _measure --nomatch 1 _n git grep -nE "$CONCEPT" "$MAIN" -- "$PKG" || failed=1
  _measured
  echo "-- concept, HEAD (PKG; A's files and B's together) --"
  _measure --nomatch 1 _n git grep -nE "$CONCEPT" -- "$PKG" || failed=1
  _measured
  echo "-- FILE PATHS only, origin/main, GENERIC (the baseline §7 argues from) --"
  # ONE run, printed AND counted. This used to be two `git grep` invocations of
  # the same needle over the same ref -- a shape in which the listing and the
  # count can disagree and neither one's status is read.
  local n_base
  _measure --nomatch 1 n_base git grep -noE "$PATHRE" "$MAIN" -- "$GENERIC" || failed=1
  _measured
  echo "   count: $n_base"
  # SUPERSEDED, A-i round 1: this block used to gate on the DELTA -- `comm -13`
  # base against head, "ADDED BY A must be empty" -- reasoning that discharging
  # `cli.py`'s pre-existing path was not A's scope. K2 is an ABSOLUTE now (§2,
  # §6 pin S8, and §12(3), which names this block as its check): A-i already
  # edits `cli.py`, and Slice C, the earlier routing target, has no `cli.py`
  # mandate. So the gate is a plain grep over the whole generic tree, and the
  # pre-existing instance counts against it like any other.
  echo "-- FILE PATHS only, HEAD, GENERIC — §12(3)'s actual check (working tree) --"
  local n_head n_ahalf
  _measure n_head _wtscan "$PATHRE" "$GENERIC" || failed=1
  _measured
  # An EMPTY `AHALF` is not "A's half has no paths": `git grep -- ` with no
  # pathspec ranges over the WHOLE repo, so the census failing above turned this
  # line into a repo-wide count reported as a per-slice one (measured: 59).
  if [ "${#AHALF[@]}" -eq 0 ]; then
    n_ahalf='!FAILED(A-half census empty)'; failed=1
  else
    _measure --nomatch 1 n_ahalf git grep -oE "$PATHRE" -- "${AHALF[@]}" || failed=1
  fi
  echo "   elidex file paths at HEAD (K2 / S8 — MUST BE 0) : $n_head"
  echo "   of which in A's half                            : $n_ahalf"
  echo "   pre-existing on origin/main (A-i discharges it)  : $n_base"
  # K3's CROSS-TREE half. The unit suite scans PKG, so a Slice-B artifact name
  # re-imported anywhere else under GENERIC, or into `.claude/skills/`, is
  # invisible to it. Unlike the suite, this block spells the needles plainly:
  # it lives in `docs/plans/`, which is in NEITHER scope, so it cannot match
  # itself the way an in-tree test file would.
  echo "-- SLICE-B ARTIFACT NAMES, HEAD, GENERIC + SKILLS (K3 / S7 cross-tree, working tree) --"
  local B_ART='cite.?audit' B_FT='_catalog'
  local n_art n_base_art
  _measure n_art _wtscan "$B_ART|$B_FT" "$GENERIC" "$SKILLS" || failed=1
  _measured
  _measure --nomatch 1 n_base_art \
    git grep -oE -e "$B_ART" -e "$B_FT" "$MAIN" -- "$GENERIC" "$SKILLS" || failed=1
  echo "   Slice-B artifact names at HEAD (K3 / S7 — MUST BE 0) : $n_art"
  echo "   pre-existing on origin/main (must also be 0)         : $n_base_art"
  # THE VERDICT IS A RETURN STATUS, not only a printed line. §12(3) names this
  # block as an exit criterion, and an exit criterion that cannot fail a process
  # is a report: measured, with a violation planted this block printed
  # `VERDICT: RED` and still exited 0, and inside `… all` -- 300+ lines -- that
  # RED line is unanchored text nothing is obliged to read.
  #
  # A FAILED MEASUREMENT IS RED, never green -- for EVERY quantity above, not
  # just the two working-tree scans. Codex reproduced the other half: in a
  # checkout with no remote-tracking ref the `origin/main` baselines died
  # `fatal: unable to resolve revision`, their counts read 0, and this block
  # printed GREEN and exited 0. `_measure` now binds each count to the status of
  # the command that produced it, so a count that was never taken is a
  # `!FAILED(rc=N)` string that no `= 0` gate below can accept even if this arm
  # were removed.
  if [ "$failed" -ne 0 ]; then
    echo "   VERDICT: RED — a MEASUREMENT FAILED (see the !! lines above);"
    echo "                  a count that was never taken is not a count of zero."
    return 1
  fi
  if [ "$n_head" = 0 ] && [ "$n_art" = 0 ]; then
    echo "   VERDICT: GREEN — no elidex file path, no Slice-B artifact name"
    return 0
  fi
  [ "$n_head" = 0 ] || echo "   VERDICT: RED — K2 is an absolute; every path listed above must go"
  [ "$n_art" = 0 ] || echo "   VERDICT: RED — K3: a Slice-B artifact is named outside its slice"
  return 1
}

# --- §4.2.3's control flow, executable ----------------------------------------
# Rounds 5, 6 and 7 each found a defect in §4.2.3 that only a REVIEW ROUND could
# find, because the section specifies the control flow of code that does not
# exist yet, in prose. Draft 6's reporting arm was False in the one row it
# exists for; draft 7's fix made it True in six rows where it must be False.
# Two inversions in a row is a method failure, not an attention failure.
#
# `_proto` grafts §4.2.3 + §4.2.5 onto a copy of `preflight.py` in a scratch
# worktree, and `armmatrix` runs every §5 row against every capability state
# with THREE candidate reporting predicates instrumented side by side. The
# implementation PR lands this control flow AND DELETES `_proto` (memo §12(4)).
_proto() {  # $1 = worktree; writes preflight_proto.py beside preflight.py
  python3 - "$1/$PF" "$1/${PF%/*}/preflight_proto.py" <<'PY'
import sys
from pathlib import Path
src = Path(sys.argv[1]).read_text(encoding="utf-8")
head = src[: src.index("def main() -> int:")]
NEW = '''
MARKER_RE = re.compile(r"^\\s*\\*\\*No spec surface\\*\\*")


def find_markers(lines, fence_state, start, end):
    """§4.2.5 recognition: §3-scoped, fence-gated, line-anchored."""
    return [j for j in range(start, end)
            if not fence_state[j] and MARKER_RE.match(lines[j])]


def grep_pass_stage(args, plan_path) -> bool:
    """True on a hard finding. Shared by the table path AND the no-spec-surface
    path: a slice with no spec surface still has §4-§7 structural references, so
    the marker must not disable grep-pass."""
    if not args.grep_pass:
        return False
    findings = run_grep_pass(plan_path, REPO_ROOT, strict_symbols=args.strict_symbols,
                             strict_enum=args.strict_enum)
    hard = [m for sev, m in findings if sev == "HARD"]
    soft = [m for sev, m in findings if sev == "SOFT"]
    print(f"  grep-pass:            {len(hard)} hard, {len(soft)} soft")
    for msg in soft:
        print(f"  \\u26a0 {msg}", file=sys.stderr)
    if hard:
        print(f"\\npreflight: HARD FAIL - grep-pass: {len(hard)} finding(s)", file=sys.stderr)
        return True
    return False


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("plan_memo")
    p.add_argument("--no-verify", action="store_true")
    p.add_argument("--strict-breadth", action="store_true")
    p.add_argument("--no-grep-pass", dest="grep_pass", action="store_false", default=True)
    p.add_argument("--strict-symbols", action="store_true")
    p.add_argument("--strict-enum", action="store_true")
    args = p.parse_args()

    # item 1: two static causes, evaluated ONCE, before any data loop.
    cli_missing = not WEBREF.is_file()
    map_missing = _shortname_for is None
    unavailable = cli_missing or map_missing
    causes = ((["the webref CLI"] if cli_missing else [])
              + (["the spec-label map"] if map_missing else []))

    plan_path = Path(args.plan_memo)
    if not plan_path.is_file():
        return 1
    lines = plan_path.read_text(encoding="utf-8").splitlines()
    fence_state = _fence_state_array(lines)
    section = find_coverage_map_section(lines, fence_state)
    if section is None:
        print("preflight: HARD FAIL - no Spec coverage map heading.", file=sys.stderr)
        return 1
    heading_line, body_start, body_end = section
    markers = find_markers(lines, fence_state, body_start, body_end)
    table = find_table(lines, body_start, body_end, fence_state)

    # §4.2.5. NOTE: this path returns before `data_rows` EXISTS, so item 4's
    # third arm is unreachable here, not merely False.
    if markers:
        if len(markers) > 1 or table is not None:
            why = ("the marker appears twice" if len(markers) > 1
                   else "a §3 table is present alongside the marker")
            print(f"preflight: HARD FAIL - ambiguous §3 declaration: {why}.", file=sys.stderr)
            print("PROTO-STATE path=marker-ambiguous", file=sys.stderr)
            return 1
        print(f"§3 Spec coverage map preflight - {plan_path.name}")
        print("  breadth:              n/a (no spec surface declared)")
        suffix = (f"; {' and '.join(causes)} unavailable" if unavailable else "")
        print(f"  citation verify:      n/a (no spec surface declared{suffix})")
        print(f"PROTO-STATE path=marker cli_missing={cli_missing} map_missing={map_missing}",
              file=sys.stderr)
        return 1 if grep_pass_stage(args, plan_path) else 0

    if table is None:
        print("preflight: HARD FAIL - heading but no table.", file=sys.stderr)
        return 1
    data_rows = table[2:] if len(table) >= 2 and is_separator_row(table[1]) else table[1:]
    if not data_rows:
        print("preflight: HARD FAIL - 0 data rows.", file=sys.stderr)
        return 1

    specs_seen: dict[str, int] = {}
    malformed_rows = 0
    unmapped_rows = 0
    citations: list[tuple[str, str]] = []
    unrecognized_labels: list[str] = []
    labelless_rows = 0                 # item 7b: partitioned OFF unrecognized_labels
    unique_specs: set[str] = set()
    for row in data_rows:
        spec_cell = row[0] if row else ""
        label, section_num = parse_spec_cell(spec_cell)
        if section_num is None:
            malformed_rows += 1
            continue
        shortname = shortname_from_label(label)
        if shortname is None:
            if label:
                unrecognized_labels.append(label)
            else:
                labelless_rows += 1
            unmapped_rows += 1
            unique_specs.add(f"unmapped:{label}" if label else "unmapped:<empty>")
            continue
        specs_seen[shortname] = specs_seen.get(shortname, 0) + 1
        citations.append((shortname, section_num))
        unique_specs.add(shortname)

    K, M = len(unique_specs), len(data_rows)

    # item 4: act-site 1, at the verification stage.
    verify_failed: list[tuple[str, str, str]] = []
    seen_pairs: set[tuple[str, str]] = set()
    capability_hard_fail = False
    verify_ran = False                 # candidate 3: a flag set INSIDE the stage
    if not args.no_verify and (citations or (unavailable and data_rows)):
        if unavailable:
            capability_hard_fail = True
        else:
            verify_ran = True
            for shortname, section_num in citations:
                key = (shortname, section_num)
                if key in seen_pairs:
                    continue
                seen_pairs.add(key)
                ok, msg = verify_citation(shortname, section_num)
                if not ok:
                    verify_failed.append((shortname, section_num, msg))

    print(f"§3 Spec coverage map preflight - {plan_path.name}")
    print(f"  total entries  (M):   {M}")
    # item 7c (J1 at the REPORTING layer): with the capability absent these rows
    # are not "unmapped" - the mapper never ran. And item 7b's partition is a
    # DISPLAY concern too: one merged counter cannot name two classes, and it
    # says "label" for a row that has none.
    if map_missing:
        print(f"  unclassified rows:    {unmapped_rows}  (label map unavailable)")
    else:
        print(f"  unknown-label rows:   {len(unrecognized_labels)}")
        print(f"  label-less rows:      {labelless_rows}")

    # item 6's basis qualifier, under item 7c: with no mapper there is no
    # "label spelling" count to report - every row is unclassified, and the
    # <label> display notation would present a spec the pinned map DOES know as
    # one it does not.
    displayed = sorted(specs_seen)
    if map_missing:
        basis = " (label map unavailable - no row classified)"
    elif unmapped_rows:
        basis = f" ({unmapped_rows} of {M} counted by label spelling)"
        displayed += [f"<{lbl}>" for lbl in sorted(set(unrecognized_labels))]
        unrouted = list(displayed)          # item 7b as drafted: label-less absent
        if labelless_rows:
            displayed.append("<label-less>")
        # item 8: "K and the spec list it prints cannot disagree". MEASURE IT,
        # under both the routed and the unrouted display, because item 7b moves
        # label-less rows OUT of `unrecognized_labels` and item 8 never noticed.
        print(f"PROTO-DISPLAY K={K} routed={len(displayed)} unrouted={len(unrouted)} "
              f"item8_routed={K == len(displayed)} item8_unrouted={K == len(unrouted)}",
              file=sys.stderr)
    else:
        basis = ""
    print(f"  unique specs (K):     {K}{basis} "
          f"({', '.join(displayed) if displayed else '-'})")

    # §4.2.4: four remedies, each for its own cause and no other.
    if unrecognized_labels and not map_missing:
        print(f"  remedy1 unrecognized: {sorted(set(unrecognized_labels))}", file=sys.stderr)
    if labelless_rows and not map_missing:
        print(f"  remedy2 label-less:   {labelless_rows} row(s)", file=sys.stderr)

    # item 5: act-site 2. THREE CANDIDATES, measured side by side.
    arm_d7 = bool(not args.no_verify and data_rows and not seen_pairs)
    arm_avail = bool(not args.no_verify and data_rows and not unavailable and not seen_pairs)
    arm_flag = bool(verify_ran and data_rows and not seen_pairs)
    print(f"PROTO-STATE cli_missing={cli_missing} map_missing={map_missing} "
          f"citations={len(citations)} M={M} seen_pairs={len(seen_pairs)} "
          f"unmapped={unmapped_rows} verify_ran={verify_ran}", file=sys.stderr)
    print(f"PROTO-ARM d7={arm_d7} avail={arm_avail} flag={arm_flag}", file=sys.stderr)

    if not args.no_verify:
        if capability_hard_fail:
            print(f"\\npreflight: HARD FAIL - citation verification unavailable: "
                  f"{' and '.join(causes)} missing.", file=sys.stderr)
            if cli_missing:
                print("  remedy4 cli-missing", file=sys.stderr)
            if map_missing:
                print("  remedy3 import-error", file=sys.stderr)
        elif verify_failed:
            print(f"\\npreflight: HARD FAIL - citation verification: "
                  f"{len(verify_failed)} failure(s)", file=sys.stderr)
        elif seen_pairs:
            print(f"  citation verify:      ok ({len(seen_pairs)} unique citation(s) checked)")
        elif arm_avail:
            print(f"  citation verify:      n/a (0 of {M} rows resolvable)")

    grep_hard = grep_pass_stage(args, plan_path)
    if malformed_rows or capability_hard_fail or verify_failed or grep_hard:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
'''
Path(sys.argv[2]).write_text(head + NEW.lstrip("\n"), encoding="utf-8")
PY
}

budget() {
  # §8's whole content is counts, so every one of them goes through `_measure`.
  # `git show "$MAIN:$f" | wc -l` printed `0 <path>` for a file that does not
  # exist on the ref -- a size claim for a file nobody measured, in the block §8
  # cites for its size claims.
  local failed=0 n
  echo "-- origin/main base, the touch set --"
  for f in "$PF" .claude/tools/_webref/commands/coverage_map.py .claude/tools/_webref/cli.py \
           .claude/tools/_webref/DESIGN.md mise.toml .github/workflows/ci.yml; do
    _measure n git show "$MAIN:$f" || failed=1
    echo "$n $f"; done
  echo "-- on this branch --"
  for m in Ai-spec-label-map Aii-gate-failure-semantics Aiii-suite-scheduler \
           umbrella B-detector-correctness C-policy-retirement; do
    f="docs/plans/2026-07-citation-hygiene-$m.md"
    [ -f "$f" ] || continue
    _measure n cat "$f" || failed=1
    echo "$n $m"
  done
  # The harness is SIX files since the slice-seam split; one line-count is no
  # longer a statement about it, and the §8 band applies per file.
  for f in docs/plans/2026-07-citation-hygiene-A-rederive*.sh; do
    _measure n cat "$f" || failed=1
    echo "$n the re-derivation harness — ${f##*/2026-07-citation-hygiene-A-rederive}"
  done
  _measure n cat docs/plans/2026-07-citation-hygiene-A-rederive*.sh || failed=1
  echo "$n the re-derivation harness, all parts"
  echo "-- preflight.py's LOGIC growth under A --"
  # `wc -l` on the armmatrix proto is not a usable estimate: the proto trims
  # argparse help and abbreviates diagnostics, so it comes out SHORTER than the
  # file it grows. Statement count is the honest measure of what A adds.
  # A worktree that was not created, or a graft that did not happen, would leave
  # the statement delta below measured against nothing at all.
  local T; T=$(mktemp -d)
  git worktree add -q "$T" HEAD || { echo "!! cannot create the HEAD worktree"; return 1; }
  _proto "$T" || failed=1
  python3 - "$T/${PF%/*}/preflight_proto.py" <<'PY' || failed=1
import ast, subprocess, sys
def stmts(src): return sum(isinstance(n, ast.stmt) for n in ast.walk(ast.parse(src)))
_b = subprocess.run(["git", "show",
                     "origin/main:.claude/skills/elidex-plan-review/preflight.py"],
                    capture_output=True, text=True)
if _b.returncode != 0:
    sys.stderr.write(_b.stderr)
    raise SystemExit("!! `git show origin/main:…preflight.py` failed (rc=%d); there is no "
                     "baseline to compute a delta against." % _b.returncode)
base = _b.stdout
proto = open(sys.argv[1]).read()
b, p = stmts(base), stmts(proto)
print(f"  origin/main={b} statements   +A={p}   delta={p - b:+d} ({100 * (p - b) / b:+.0f}%)")
print("  caveat: the proto collapses several multi-line diagnostics into one print,")
print("  and each print is a statement, so the shipped delta is somewhat larger.")
PY
  # The cleanup runs LAST but must not BE the answer: `git worktree remove`
  # succeeding says nothing about whether anything above was measured.
  git worktree remove --force "$T"
  return "$failed"
}

lanes() {  # §13 — base, open PRs, worktrees authoring plan-memos, the two carve commits
  local failed=0 n
  git rev-list --left-right --count "$MAIN"...HEAD || failed=1
  gh pr list --state open --json number,headRefName --jq '.[] | "\(.number) \(.headRefName)"' || failed=1
  git log --format='%h %s' --grep='carve the cite-audit detector' || failed=1
  git log --format='%h %s' --grep='re-carve the shared spec-label map' || failed=1
  echo "-- worktrees carrying plan-memo diffs --"
  # The `2>/dev/null | wc -l` this used to be reported `0` -- i.e. "this worktree
  # authors no plan-memo" -- for a worktree whose diff could not be taken at all,
  # and §13's lane roster is exactly a claim about which worktrees those are.
  for w in $(git worktree list --porcelain | awk '/^worktree /{print $2}'); do
    if _measure n git -C "$w" diff --name-only "$MAIN"...HEAD -- docs/plans/; then
      [ "$n" -gt 0 ] && echo "  $n $w"
    else
      echo "  !! $w — NOT MEASURED ($n); absent from this roster for a reason that"
      echo "     is not 'it authors no plan-memo'"; failed=1
    fi
  done
  # A's REAL contention is CI topology, and draft 8's version of this block could
  # not see it: `gh pr list` misses an unpushed branch, and a docs/plans/ filter
  # misses a branch whose collision is in ci.yml / mise.toml. The Layout lane's
  # `layout-trip-wire-ci` was invisible to both halves while committing an
  # OPPOSITE answer on all three files A edits.
  echo "-- worktrees touching the files A contends on (ci.yml / mise.toml / .claude/tools) --"
  for w in $(git worktree list --porcelain | awk '/^worktree /{print $2}'); do
    if _measure n git -C "$w" diff --name-only "$MAIN"...HEAD -- \
                     .github/workflows/ mise.toml .claude/tools/; then
      [ "$n" -gt 0 ] && { echo "  $w  [$(git -C "$w" rev-parse --short HEAD)]"
                          _measured | sed 's/^/      /'; }
    else
      echo "  !! $w — NOT MEASURED ($n); silence here is not 'no contention'"; failed=1
    fi
  done
  return "$failed"
}

# AUTHOR-LOCAL: these reach a per-user memory directory and sibling worktrees, so
# they cannot run for a second reader. `all` excludes them; run them by name.
AUTHOR_LOCAL="lanes staleclaims"
