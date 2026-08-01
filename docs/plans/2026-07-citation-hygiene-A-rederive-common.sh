# Shared part of the re-derivation harness — sourced by
# `2026-07-citation-hygiene-A-rederive.sh`, which is the only entry point.
# Not executable on its own: it defines no dispatch and sets no shell options.
#
# What lives here: the plumbing (`$MAIN`, `$PF`, `say`, `$AUTHOR_LOCAL`, the §6
# fixture bodies, the §4.2.3 prototype) and every block MORE THAN ONE memo cites
# -- `citations` (A-i, A-ii), `couplings` and `budget` (A-i, A-ii, A-iii),
# `lanes` (A-ii, A-iii, umbrella; author-local).

MAIN=origin/main
PF=.claude/skills/elidex-plan-review/preflight.py
say() { printf '\n=== %s ===\n' "$1"; }

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
  .claude/tools/webref heading --exact html 4.10.21
  .claude/tools/webref heading --exact html 4.10.21.2
  .claude/tools/webref heading --exact fetch 2.2.5
  .claude/tools/webref heading --exact cssom-view-1 4.2
  echo "-- and the pairs actually present in the fixture bodies --"
  local F; F=$(mktemp -d); fixtures "$F" >/dev/null
  awk -F'|' '/^\| [A-Za-z§]/ && $2 !~ /Spec section/ {gsub(/^ +| +$/,"",$2); print "  "$2}' \
    "$F"/*.md | sort -u
  rm -rf "$F"
}

couplings() {  # §7 / §12(3) — every elidex coupling in the generic tree, by concept
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
  local AHALF=()
  while IFS= read -r f; do
    printf '%s' "$f" | grep -qE "$BFILES" || AHALF+=("$f")
  done < <(git ls-files '.claude/tools/_webref/*.py' '.claude/tools/_webref/**/*.py' \
                        '.claude/tools/_webref/*.md')
  echo "   A-half files: ${#AHALF[@]}"
  echo "-- concept, origin/main baseline --"
  git grep -nE "$CONCEPT" "$MAIN" -- .claude/tools/_webref/ | cat
  echo "-- concept, HEAD (A's files and B's together) --"
  git grep -nE "$CONCEPT" -- .claude/tools/_webref/ | cat
  echo "-- FILE PATHS only, origin/main (the pre-existing baseline §7 argues from) --"
  git grep -noE "$PATHRE" "$MAIN" -- .claude/tools/_webref/ | cat
  echo "   count: $(git grep -oE "$PATHRE" "$MAIN" -- .claude/tools/_webref/ | wc -l | tr -d ' ')"
  # SUPERSEDED, A-i round 1: this block used to gate on the DELTA -- `comm -13`
  # base against head, "ADDED BY A must be empty" -- reasoning that discharging
  # `cli.py`'s pre-existing path was not A's scope. K2 is an ABSOLUTE now (§2,
  # §6 pin S8, and §12(3), which names this block as its check): A-i already
  # edits `cli.py`, and Slice C, the earlier routing target, has no `cli.py`
  # mandate. So the gate is a plain grep over the whole generic tree, and the
  # pre-existing instance counts against it like any other.
  echo "-- FILE PATHS only, HEAD, the whole generic tree — §12(3)'s actual check --"
  git grep -noE "$PATHRE" -- .claude/tools/_webref/ | cat
  local n_head n_base n_ahalf
  n_head=$(git grep -oE "$PATHRE" -- .claude/tools/_webref/ | wc -l | tr -d ' ')
  n_base=$(git grep -oE "$PATHRE" "$MAIN" -- .claude/tools/_webref/ | wc -l | tr -d ' ')
  n_ahalf=$(git grep -oE "$PATHRE" -- "${AHALF[@]}" | wc -l | tr -d ' ')
  echo "   elidex file paths at HEAD (K2 / S8 — MUST BE 0) : $n_head"
  echo "   of which in A's half                            : $n_ahalf"
  echo "   pre-existing on origin/main (A-i discharges it)  : $n_base"
  if [ "$n_head" = 0 ]; then
    echo "   VERDICT: GREEN — the generic core names no elidex file path"
  else
    echo "   VERDICT: RED — K2 is an absolute; every path listed above must go"
  fi
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
  echo "-- origin/main base, the touch set --"
  for f in "$PF" .claude/tools/_webref/commands/coverage_map.py .claude/tools/_webref/cli.py \
           .claude/tools/_webref/DESIGN.md mise.toml .github/workflows/ci.yml; do
    echo "$(git show "$MAIN:$f" | wc -l) $f"; done
  echo "-- on this branch --"
  for m in Ai-spec-label-map Aii-gate-failure-semantics Aiii-suite-scheduler \
           umbrella B-detector-correctness C-policy-retirement; do
    f="docs/plans/2026-07-citation-hygiene-$m.md"
    [ -f "$f" ] && echo "$(wc -l < "$f") $m"
  done
  # The harness is SIX files since the slice-seam split; one line-count is no
  # longer a statement about it, and the §8 band applies per file.
  for f in docs/plans/2026-07-citation-hygiene-A-rederive*.sh; do
    echo "$(wc -l < "$f") the re-derivation harness — ${f##*/2026-07-citation-hygiene-A-rederive}"
  done
  echo "$(cat docs/plans/2026-07-citation-hygiene-A-rederive*.sh | wc -l | tr -d ' ') the re-derivation harness, all parts"
  echo "-- preflight.py's LOGIC growth under A --"
  # `wc -l` on the armmatrix proto is not a usable estimate: the proto trims
  # argparse help and abbreviates diagnostics, so it comes out SHORTER than the
  # file it grows. Statement count is the honest measure of what A adds.
  local T; T=$(mktemp -d); git worktree add -q "$T" HEAD; _proto "$T"
  python3 - "$T/${PF%/*}/preflight_proto.py" <<'PY'
import ast, subprocess, sys
def stmts(src): return sum(isinstance(n, ast.stmt) for n in ast.walk(ast.parse(src)))
base = subprocess.run(["git", "show",
                       "origin/main:.claude/skills/elidex-plan-review/preflight.py"],
                      capture_output=True, text=True).stdout
proto = open(sys.argv[1]).read()
b, p = stmts(base), stmts(proto)
print(f"  origin/main={b} statements   +A={p}   delta={p - b:+d} ({100 * (p - b) / b:+.0f}%)")
print("  caveat: the proto collapses several multi-line diagnostics into one print,")
print("  and each print is a statement, so the shipped delta is somewhat larger.")
PY
  git worktree remove --force "$T"
}

lanes() {  # §13 — base, open PRs, worktrees authoring plan-memos, the two carve commits
  git rev-list --left-right --count "$MAIN"...HEAD
  gh pr list --state open --json number,headRefName --jq '.[] | "\(.number) \(.headRefName)"'
  git log --format='%h %s' --grep='carve the cite-audit detector'
  git log --format='%h %s' --grep='re-carve the shared spec-label map'
  echo "-- worktrees carrying plan-memo diffs --"
  for w in $(git worktree list --porcelain | awk '/^worktree /{print $2}'); do
    n=$(git -C "$w" diff --name-only "$MAIN"...HEAD -- docs/plans/ 2>/dev/null | wc -l)
    [ "$n" -gt 0 ] && echo "  $n $w"
  done
  # A's REAL contention is CI topology, and draft 8's version of this block could
  # not see it: `gh pr list` misses an unpushed branch, and a docs/plans/ filter
  # misses a branch whose collision is in ci.yml / mise.toml. The Layout lane's
  # `layout-trip-wire-ci` was invisible to both halves while committing an
  # OPPOSITE answer on all three files A edits.
  echo "-- worktrees touching the files A contends on (ci.yml / mise.toml / .claude/tools) --"
  for w in $(git worktree list --porcelain | awk '/^worktree /{print $2}'); do
    local f
    f=$(git -C "$w" diff --name-only "$MAIN"...HEAD -- \
          .github/workflows/ mise.toml .claude/tools/ 2>/dev/null)
    [ -n "$f" ] && { echo "  $w  [$(git -C "$w" rev-parse --short HEAD)]"
                     echo "$f" | sed 's/^/      /'; }
  done
}

# AUTHOR-LOCAL: these reach a per-user memory directory and sibling worktrees, so
# they cannot run for a second reader. `all` excludes them; run them by name.
AUTHOR_LOCAL="lanes staleclaims"
