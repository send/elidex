# Shared part of the re-derivation harness — sourced by
# `2026-07-citation-hygiene-A-rederive.sh`, which is the only entry point.
# Not executable on its own: it defines no dispatch and sets no shell options,
# and it reads `$REPO_ROOT` and `_measure` from `-integrity.sh`, which the
# dispatcher sources first.
#
# What lives here: the shared plumbing (`$MAIN`, `$PF`, `say`, `$AUTHOR_LOCAL`,
# the §6 fixture bodies, the §4.2.3 prototype) and every block MORE THAN ONE
# memo cites -- `citations` (A-i, A-ii), `couplings` and `budget` (A-i, A-ii,
# A-iii), `lanes` (A-ii, A-iii, umbrella; author-local). The harness's integrity
# machinery is `-integrity.sh`.

# THE BASELINE IS PINNED, NOT TRACKED. Every `$MAIN` assertion in this harness is
# of the form "what the tree looked like BEFORE this slice": `readercensus` wants
# `label_for` to have no readers there, `couplings` wants exactly the two
# pre-existing path hits, `suites` wants the four pre-A-ii suite files. A
# remote-tracking branch is not that -- the moment this PR lands, `origin/main`
# CONTAINS the slice and every one of those flips, so `all` would go red on the
# landed tree and on every slice rebased onto it. The harness would stop being
# able to reproduce its own checks at exactly the moment it became the record of
# them. (Codex R55.)
#
# The pin is this branch's merge-base with `main`, and it is durable for the same
# reason A-i §14 gives: it is an ANCESTOR of `origin/main`, so no ref deletion or
# squash can orphan it -- unlike a branch SHA. Verify with
# `git merge-base --is-ancestor 44cd165db2f039b772acdd5dcf42c3c739bbfaf6 origin/main`.
MAIN=44cd165db2f039b772acdd5dcf42c3c739bbfaf6
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
    echo '| HTML §4.10.21 Constraints | s | b | t | ✓ | no |'
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
  # The trailing HTML comment is DATA the `citations` block reads (the gate
  # ignores it: not a table row): it names the shortname the title is verified
  # against, and it asserts the premise -- `citations` fails the day the map DOES
  # resolve `CSSOM VIEW`, because then this fixture no longer carries the
  # all-unmapped arm it exists for. One home for both facts, here.
  { echo '# fixture'; echo; echo '## §3. Spec coverage map'; echo; echo "$HDR"
    echo '| CSSOM VIEW §4.2 The MediaQueryList Interface | s | b | t | ✓ | no |'
    echo; echo '<!-- unmapped-by-design: CSSOM VIEW = cssom-view-1 -->'
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
    echo; echo '<!-- unmapped-by-design: CSSOM VIEW = cssom-view-1 -->'
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
  # Codex R4: four hard-coded lookups and a separate `awk` listing of the cells is
  # a list and a print with no comparison between them -- a fixture gaining a
  # pair, or a title drifting from the heading, left this block GREEN. The cells
  # are now the input: every `<label> §<n> <title>` cell in the generated
  # fixtures is resolved through the map A-i ships (`shortname_for`), the heading
  # is read with `webref heading --exact`, and the TITLE must equal it, because
  # the fixture comment's own rule is "the title is the real one". A label the
  # map does not know is exactly what `allunmapped` exists to carry: the fixture
  # file itself says so in an `<!-- unmapped-by-design: <label> = <shortname> -->`
  # line, which is both where the title's shortname comes from and a PREMISE this
  # block asserts -- if the map ever resolves that label, the fixture has lost the
  # arm it exists for and this block says so (a first draft kept the shortname in
  # a dict here, a second home for the fixture's fact, and fell back to the map
  # silently). A cell with no `§` is `malformed`'s by design and is reported as
  # such, not verified. Zero parsed cells is a failure of this block.
  local rc=0
  local F; F=$(mktemp -d) || { echo "!! cannot allocate the fixture directory"; return 1; }
  fixtures "$F" >/dev/null || rc=1
  python3 - "$F" <<'CITPY' || rc=1
import re, subprocess, sys, pathlib
sys.path.insert(0, ".claude/tools")
from _webref.spec_labels import shortname_for
UNMAPPED_RE = re.compile(r"^<!-- unmapped-by-design: (.+?) = (\S+) -->$")
cells, by_design, verified = set(), {}, set()
for p in sorted(pathlib.Path(sys.argv[1]).glob("*.md")):
    for ln in p.read_text().splitlines():
        if ln.startswith("| ") and not ln.startswith("| Spec section") and not ln.startswith("|---"):
            cells.add(ln.split("|")[1].strip())
        elif (u := UNMAPPED_RE.match(ln)):
            by_design.setdefault(u.group(1), set()).add(u.group(2))
if not cells:
    print("!! no fixture cells parsed"); sys.exit(1)
bad = 0
for label, sns in sorted(by_design.items()):   # the premise each fixture states, measured
    if len(sns) != 1:
        print(f"!! fixtures disagree on {label!r}'s shortname: {sorted(sns)}"); bad += 1
    if shortname_for(label) is not None:
        print(f"!! {label!r} is declared unmapped-by-design but the map resolves it to "
              f"{shortname_for(label)!r}: the all-unmapped fixtures no longer carry their arm"); bad += 1
for cell in sorted(cells):
    m = re.match(r"^(.*?)\s*§([0-9.]+)\s+(.*)$", cell)
    if not m:
        print(f"  {cell!r:60} -> no §: malformed by design, not verified"); continue
    label, sec, title = m.group(1).strip(), m.group(2), m.group(3).strip()
    if not label:   # `unlabelled`'s cell by design: no label, so no pair to verify
        print(f"  {cell!r:60} -> label-less by design, not a pair"); continue
    sn, via = shortname_for(label), "map"
    if sn is None and label in by_design:
        sn, via = next(iter(by_design[label])), "fixture"
    if sn is None:  # a label neither the map nor a fixture accounts for
        print(f"  {cell!r:60} -> unmapped label {label!r} with no fixture-stated shortname"); bad += 1; continue
    r = subprocess.run([".claude/tools/webref", "heading", "--exact", sn, sec], capture_output=True, text=True)
    hm = re.match(r"\s*§(\S+)\s+(.*?)\s+#\S+\s*$", r.stdout)
    if r.returncode != 0 or not hm:
        print(f"  {cell!r:60} -> {sn} §{sec}: NO HEADING (rc={r.returncode})"); bad += 1; continue
    got = hm.group(2).strip()
    ok = got == title
    print(f"  {cell!r:60} -> {sn} §{sec} [{via}] title {'==' if ok else '!='} {got!r}")
    bad += 0 if ok else 1
    if ok:
        verified.add((sn, sec))

# THE ADVERTISED COVERAGE, NOT JUST A NON-EMPTY POPULATION. `if not cells` only
# rejected the ALL-empty case, so deleting one fixture silently dropped whatever
# citation only it exercised while the remaining cells kept the block green.
# The population this block claims to cover is not a count -- it is the memo's
# §0.5 table, which is the canonical statement of what A-i cites. Read it and
# require every row to have been verified, resolved through the same map the
# cells go through so a variant spelling in a fixture still counts.
memo = pathlib.Path("docs/plans/2026-07-citation-hygiene-Ai-spec-label-map.md")
if not memo.exists():
    print(f"!! {memo} is missing; the coverage claim cannot be derived"); sys.exit(1)
rows, in_table = [], False
for ln in memo.read_text(encoding="utf-8").splitlines():
    if ln.startswith("## §0.5"):
        in_table = True; continue
    if in_table and ln.startswith("## "):
        break
    if in_table and ln.startswith("| `"):
        m0 = re.match(r"^\|\s*`([^`]+?)\s+§([0-9.]+)`\s*\|", ln)
        if m0:
            rows.append((m0.group(1).strip(), m0.group(2)))
if not rows:
    print("!! §0.5's citation table parsed EMPTY; this coverage check would then "
          "certify any fixture set at all"); sys.exit(1)
for label, sec in rows:
    sn = shortname_for(label)
    if sn is None:
        print(f"!! §0.5 cites {label!r}, which the map does not resolve"); bad += 1
    elif (sn, sec) not in verified:
        print(f"!! §0.5 cites {label} §{sec} and no fixture cell verified it — the "
              f"fixture that carried it is gone, or stopped resolving"); bad += 1
print(f"cells={len(cells)} §0.5 rows={len(rows)} failing={bad}")
sys.exit(1 if bad else 0)
CITPY
  rm -rf "$F"
  return "$rc"
}

_wtscan() {  # $1 = ERE, $2.. = roots RELATIVE TO $REPO_ROOT. Prints `path:line:match`.
  # Why not `git grep`: it sees TRACKED content only, so a violation in a file
  # that exists but is not yet added reads GREEN. Not hypothetical -- measured:
  # with `cite-audit` planted in an UNTRACKED file under `.claude/skills/`, the
  # git-grep form of this block counted 0 and printed `VERDICT: GREEN`, and a
  # bare `git add -N` on the same file flipped it to RED (measured when the scan
  # still ranged over `.claude/skills/`; the property is the same for an
  # untracked file under `_webref/`). The unit suite these entry-script limbs
  # came from walked the filesystem for exactly that reason; moving them here
  # must not trade the property away. The origin/main
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
unreadable = []
def files_under(root):
    # A root may be a FILE: the generic core is `_webref/` PLUS the entry script
    # `.claude/tools/webref`, and `os.walk` on a file yields NOTHING -- so the
    # entry script was never scanned at HEAD and a plant in it read GREEN
    # (second design re-gate of #501, measured). A file root is one file.
    if os.path.isfile(root):
        yield root; return
    if not os.path.isdir(root):
        print(f"!! root does not exist, NOT scanned: {root}", file=sys.stderr); sys.exit(2)
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in (".git", "__pycache__")]
        for fn in sorted(filenames):
            yield os.path.join(dirpath, fn)
for root in sys.argv[3:]:
    for path in files_under(root):
        if True:
            try:
                with open(path, encoding="utf-8") as fh:
                    for i, line in enumerate(fh, 1):
                        for m in ere.finditer(line):
                            print(f"{path}:{i}:{m.group(0)}")
            except UnicodeDecodeError:
                continue          # non-text: nothing to match, legitimately
            except OSError as e:
                # An UNREADABLE file is not a non-match: skipping it and exiting
                # 0 told `_measure` the scan was complete, and `couplings`
                # printed GREEN over a file it never inspected (Codex R15).
                unreadable.append(f"{path}: {e.strerror}")
if unreadable:
    for u in unreadable:
        print(f"!! unreadable, NOT scanned: {u}", file=sys.stderr)
    sys.exit(2)
WTSCANPY
}

couplings() {  # §7 / §12(2) / §12(3) — K2 and K3 over the whole generic core
  # SCOPES, written down because this block carries two of them:
  #   PKG     `.claude/tools/_webref/` — the package. The by-role CONCEPT
  #           listing below is about the prose A-i rewrote, which lives here.
  #           `test_spec_labels.py` pins K2 and K3 over exactly this tree.
  #   GENERIC the package PLUS the entry script `.claude/tools/webref` — what
  #           A-i §2 defines the generic core to be (DESIGN.md's
  #           by-responsibility split), and what K2/K3 are absolutes over.
  #           REDRAWN at #501 R36: this used to be all of `.claude/tools/`,
  #           which measured added no evidence (both pre-existing K2 sites are
  #           in `_webref/cli.py` and `webref`) and pulled five other-lane
  #           trip-wire artifacts into §12(3); `.claude/skills/` is the
  #           adapter and is out of K3's range for the same reason.
  # The unit suite scans PKG only, so GENERIC minus PKG (the entry script) is
  # witnessed HERE and nowhere else. Verified by planting a violation in each
  # tree.
  local PKG='.claude/tools/_webref/'
  local GENERIC=('.claude/tools/_webref/' '.claude/tools/webref')
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
  _measure _n git ls-files ':(glob).claude/tools/_webref/**/*.py' \
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
  # ⚠ THE TWO CENSUSES BELOW ARE A READING, NOT A GATE, and saying so is the
  # point: `$CONCEPT` matches domain vocabulary the generic tree legitimately
  # uses (`coverage-map`'s whole job is a plan-memo §3 skeleton, and that
  # wording is in the baseline), so an absolute over it would forbid the
  # package describing what it does. Deciding which occurrence is "role" and
  # which is "policy" is a taste judgement no expression holds — so it is not
  # asserted, and a grown count is a prompt to read the diff, not a verdict.
  echo "-- concept, $MAIN baseline (PKG — the by-role prose A-i rewrites; CENSUS, not asserted) --"
  _measure --nomatch 1 _n_con_base git grep -nE "$CONCEPT" "$MAIN" -- "$PKG" || failed=1
  _measured
  echo "-- concept, HEAD (PKG; A's files and B's together; CENSUS, not asserted) --"
  _measure --nomatch 1 _n_con_head git grep -nE "$CONCEPT" -- "$PKG" || failed=1
  _measured
  echo "   by-role prose: $MAIN $_n_con_base -> HEAD $_n_con_head (a reading; see the note above)"
  # WHAT *IS* ASSERTED is the half with no taste in it. This program's own
  # vocabulary -- a `rederive` block name, a slice letter, a `K<n>` invariant,
  # a PR number, a `#11-` ledger slot -- is never domain wording; it can only
  # be host-project state, which `DESIGN.md` puts in the adapter. It reached
  # the generic tree twice, both times in a docstring written to explain why
  # the *previous* instance had been removed, and the census above printed
  # both without objecting because it was never compared to anything.
  local PROGRAM='rederive |Slice [A-C]\b|\bK[0-9]\b|#5[0-9][0-9]\b|#11-'
  echo "-- HOST-PROJECT VOCABULARY, HEAD, GENERIC (MUST BE 0) --"
  local n_prog
  _measure --nomatch 1 n_prog git grep -nE "$PROGRAM" -- "${GENERIC[@]}" || failed=1
  _measured
  echo "   host-project names at HEAD (MUST BE 0)              : $n_prog"
  [ "$n_prog" = 0 ] || { echo "!! $n_prog host-project name(s) in the generic tree — DESIGN.md puts that wording in the adapter"; failed=1; }
  echo "-- FILE PATHS only, $MAIN, GENERIC (the baseline §7 argues from) --"
  # ONE run, printed AND counted. This used to be two `git grep` invocations of
  # the same needle over the same ref -- a shape in which the listing and the
  # count can disagree and neither one's status is read.
  local n_base
  _measure --nomatch 1 n_base git grep -noE "$PATHRE" "$MAIN" -- "${GENERIC[@]}" || failed=1
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
  # CONTROL for the scanner's own reach (second re-gate of #501): a plant in a
  # FILE root and in a DIRECTORY root must both be seen -- `os.walk` on a file
  # yields nothing, which left the entry script unscanned while this block
  # printed GREEN. Temporary roots under the repo, removed after.
  # ⚠ The cleanup below is `rm -rf` on a path built from this allocation, so the
  # allocation is checked BEFORE anything derives from it. Unchecked, a failing
  # `mktemp` (read-only checkout, full filesystem, changed permissions) left
  # `ctl` EMPTY and `rm -rf "$REPO_ROOT/$ctl"` expanded to the repository root
  # itself -- the harness deleting the checkout it is auditing. `set -e` is not
  # on (`-uo pipefail` only), so nothing else would have stopped it. The `case`
  # keeps the invariant by construction rather than by the reader's care: the
  # removal takes the ABSOLUTE path `mktemp` returned, and that path must be a
  # proper subdirectory of `$REPO_ROOT` (`?*` = at least one character after the
  # slash) or the block returns before creating anything.
  local ctl_abs ctl
  ctl_abs=$(mktemp -d "$REPO_ROOT/.wtscan-control.XXXXXX") || {
    echo "!! cannot allocate the _wtscan control directory under $REPO_ROOT"; return 1; }
  case "$ctl_abs" in
    "$REPO_ROOT"/?*) ;;
    *) echo "!! control directory '$ctl_abs' is not a proper subdirectory of $REPO_ROOT"; return 1 ;;
  esac
  ctl=${ctl_abs#"$REPO_ROOT"/}
  mkdir -p "$ctl/d"; printf 'x .claude/skills/elidex-review/axes.md\n' > "$ctl/f"; cp "$ctl/f" "$ctl/d/g"
  local n_ctl_f n_ctl_d
  _measure n_ctl_f _wtscan "$PATHRE" "$ctl/f" || failed=1
  _measure n_ctl_d _wtscan "$PATHRE" "$ctl/d" || failed=1
  rm -rf "$ctl_abs"
  [ "$n_ctl_f" = 1 ] && [ "$n_ctl_d" = 1 ] || { echo "!! _wtscan control: file root saw $n_ctl_f, dir root saw $n_ctl_d (both must be 1)"; failed=1; }
  local n_head n_ahalf
  _measure n_head _wtscan "$PATHRE" "${GENERIC[@]}" || failed=1
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
  echo "   pre-existing at the pinned base (A-i discharges it): $n_base"
  # A-i §4.2 S8 and §13.1 argue from "origin/main has TWO" (`_webref/cli.py:78`,
  # `.claude/tools/webref:5`); the count was printed and never compared (the
  # block-audit of 2026-08-22). The claim has a lifetime -- A-i landing makes it
  # 0 -- and when it moves, this line says so and the two memo sentences get
  # rewritten, rather than staying true-looking beside a green block.
  [ "$n_base" = 2 ] || { echo "!! $MAIN baseline is $n_base, not the 2 A-i §4.2 S8 / §13.1 argue from"; failed=1; }
  # K3's entry-script half. The unit suite scans PKG, so a Slice-B artifact name
  # re-imported into the entry script is invisible to it. Unlike the suite, this block spells the needles plainly:
  # it lives in `docs/plans/`, which is in NEITHER scope, so it cannot match
  # itself the way an in-tree test file would.
  echo "-- SLICE-B ARTIFACT NAMES, HEAD, GENERIC (K3 / S7 entry-script limb, working tree) --"
  # At A-i's head NO exemption: any `cite_audit` / `_catalog` under GENERIC is
  # a K3 violation. Slice B's landing adds its canonical paths
  # (`commands/cite_audit.py`, `spec_labels.py`) as the one exemption and keeps
  # the rest of the scan (B §6, the S7 retirement bullet — Codex R21).
  local B_ART='cite.?audit' B_FT='_catalog'
  local n_art n_base_art
  _measure n_art _wtscan "$B_ART|$B_FT" "${GENERIC[@]}" || failed=1
  _measured
  _measure --nomatch 1 n_base_art \
    git grep -oE -e "$B_ART" -e "$B_FT" "$MAIN" -- "${GENERIC[@]}" || failed=1
  echo "   Slice-B artifact names at HEAD (K3 / S7 — MUST BE 0) : $n_art"
  echo "   pre-existing at the pinned base (must also be 0)   : $n_base_art"
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
  if [ "$n_head" = 0 ] && [ "$n_art" = 0 ] && [ "$n_base_art" = 0 ]; then
    echo "   VERDICT: GREEN — no elidex file path, no Slice-B artifact name"
    return 0
  fi
  [ "$n_head" = 0 ] || echo "   VERDICT: RED — K2 is an absolute; every path listed above must go"
  [ "$n_art" = 0 ] || echo "   VERDICT: RED — K3: a Slice-B artifact is named outside its slice"
  # The baseline line says "must also be 0"; a printed premise outside the
  # verdict let the block certify K3 with that premise false (Codex R18).
  [ "$n_base_art" = 0 ] || echo "   VERDICT: RED — a Slice-B artifact name already exists on $MAIN; the K3 baseline premise is false"
  return 1
}

budget() {
  # §8's whole content is counts, so every one of them goes through `_measure`.
  # `git show "$MAIN:$f" | wc -l` printed `0 <path>` for a file that does not
  # exist on the ref -- a size claim for a file nobody measured, in the block §8
  # cites for its size claims.
  local failed=0 n
  echo "-- pinned base ($MAIN), the touch set --"
  for f in "$PF" .claude/tools/_webref/commands/coverage_map.py .claude/tools/_webref/cli.py \
           .claude/tools/_webref/DESIGN.md mise.toml .github/workflows/ci.yml; do
    _measure n git show "$MAIN:$f" || failed=1
    echo "$n $f"; done
  echo "-- on this branch --"
  for m in Ai-spec-label-map umbrella; do
    f="docs/plans/2026-07-citation-hygiene-$m.md"
    # Both memos are EXPECTED (the slice memos B / A-ii / A-iii / C travel on branch
    # `citation-hygiene-slice-memos`); one that is absent is a failed measurement,
    # not a row to skip -- skipping printed no size for it and exited 0 (Codex R18).
    [ -f "$f" ] || { echo "!! expected memo missing: $f"; failed=1; continue; }
    _measure n cat "$f" || failed=1
    echo "$n $m"
  done
  # The harness is one file per part since the slice-seam split; one line-count
  # is no longer a statement about it, and the §8 band applies per file. The
  # block count is A-i §8's other layout figure, and this is its only home.
  for f in docs/plans/2026-07-citation-hygiene-A-rederive*.sh; do
    _measure n cat "$f" || failed=1
    echo "$n the re-derivation harness — ${f##*/2026-07-citation-hygiene-A-rederive}"
  done
  _measure n cat docs/plans/2026-07-citation-hygiene-A-rederive*.sh || failed=1
  echo "$n the re-derivation harness, all parts"
  _measure n grep -hE '^[A-Za-z_][A-Za-z0-9_]*\(\)' docs/plans/2026-07-citation-hygiene-A-rederive*.sh || failed=1
  echo "$n the re-derivation harness, blocks (function definitions, all parts)"
  # The "+A statement growth" measurement grafted A-ii's prototype; it left with the
  # A-ii part (branch `citation-hygiene-slice-memos`).
  return "$failed"
}

lanes() {  # §13 — base, open PRs, worktrees authoring plan-memos, the two carve commits
  local failed=0 n m
  git rev-list --left-right --count "$MAIN"...HEAD || failed=1
  # `gh pr list` returns 30 items by default; an open PR past the first page is
  # a contending lane this roster would silently omit (Codex R28).
  gh pr list --state open --limit 1000 --json number,headRefName --jq '.[] | "\(.number) \(.headRefName)"' || failed=1
  # `git log --grep` exits 0 on NO match, so a missing carve commit read as
  # found (Codex R22) -- and the second subject never existed in this history
  # (the shared-map carve is `docs(plans): carve Slice A-i — the shared
  # spec-label map`). Each census is a COUNT that must be >= 1, over --all so
  # a branch-local carve still answers after the squash.
  local carve
  for carve in 'carve the cite-audit detector' 'carve Slice A-i'; do
    _measure n git log --all --format='%h %s' --grep="$carve" || { failed=1; continue; }
    [ "$n" -ge 1 ] || { echo "!! no commit with subject /$carve/ in any ref — a carve commit this lane roster names is absent"; failed=1; }
    git log --all --format='%h %s' --grep="$carve" | head -3
  done
  # The worktree listing is taken ONCE and its status observed: a process
  # substitution that failed fed both loops an empty stream and `lanes` exited
  # 0 having censused no worktree at all (Codex R23).
  local wlist
  wlist=$(git worktree list --porcelain | sed -n 's/^worktree //p'; exit "${PIPESTATUS[0]}") \
    || { echo "!! git worktree list failed — no worktree census was taken"; failed=1; wlist=""; }
  [ -n "$wlist" ] || { echo "!! worktree listing is empty — this worktree itself should be listed"; failed=1; }
  echo "-- worktrees carrying plan-memo diffs --"
  # The `2>/dev/null | wc -l` this used to be reported `0` -- i.e. "this worktree
  # authors no plan-memo" -- for a worktree whose diff could not be taken at all,
  # and §13's lane roster is exactly a claim about which worktrees those are.
  while IFS= read -r w; do   # whole path, whitespace-safe (Codex R21: `$2` truncated it)
    # A prunable entry (its directory is gone) is not a worktree whose diff
    # failed; it is reported as what it is and skipped, or a scratch worktree
    # some earlier block left behind turns this whole roster RED.
    git -C "$w" rev-parse --git-dir >/dev/null 2>&1 || { echo "  (prunable — not a reachable worktree, skipped: $w)"; continue; }
    # Committed range AND the working tree: a memo being authored but not yet
    # committed is exactly what this census promises to list (Codex R25).
    if _measure n git -C "$w" diff --name-only "$MAIN"...HEAD -- docs/plans/ \
       && _measure m git -C "$w" status --porcelain --untracked-files=all -- docs/plans/; then
      [ $((n + m)) -gt 0 ] && echo "  $((n + m)) $w  (committed $n, uncommitted $m)"
    else
      echo "  !! $w — NOT MEASURED ($n); absent from this roster for a reason that"
      echo "     is not 'it authors no plan-memo'"; failed=1
    fi
  done <<< "$wlist"
  # A's REAL contention is CI topology, and draft 8's version of this block could
  # not see it: `gh pr list` misses an unpushed branch, and a docs/plans/ filter
  # misses a branch whose collision is in ci.yml / mise.toml. The Layout lane's
  # `layout-trip-wire-ci` was invisible to both halves while committing an
  # OPPOSITE answer on all three files A edits.
  echo "-- worktrees touching the files A contends on (ci.yml / mise.toml / .claude/tools) --"
  while IFS= read -r w; do   # whole path, whitespace-safe (Codex R21: `$2` truncated it)
    # A prunable entry (its directory is gone) is not a worktree whose diff
    # failed; it is reported as what it is and skipped, or a scratch worktree
    # some earlier block left behind turns this whole roster RED.
    git -C "$w" rev-parse --git-dir >/dev/null 2>&1 || { echo "  (prunable — not a reachable worktree, skipped: $w)"; continue; }
    if _measure n git -C "$w" diff --name-only "$MAIN"...HEAD -- \
                     .github/workflows/ mise.toml .claude/tools/ \
       && _measure m git -C "$w" status --porcelain --untracked-files=all -- \
                     .github/workflows/ mise.toml .claude/tools/; then
      [ $((n + m)) -gt 0 ] && { echo "  $w  [$(git -C "$w" rev-parse --short HEAD)]"
                          _measured | sed 's/^/      /'; }
    else
      echo "  !! $w — NOT MEASURED ($n); silence here is not 'no contention'"; failed=1
    fi
  done <<< "$wlist"
  return "$failed"
}

suites() {  # §1 / §4.3.1 / §4.3.3 — 47 tests, 4 files, and the fetch count
  # THREE ways this block used to certify a run it did not have. (1) The file
  # count was `ls … | wc -l`, which is `0` when the globs match nothing -- see
  # `_measure`. (2) The suite runner discarded `subprocess.run`'s `returncode`,
  # AND the output filter kept only `Ran `/`URLOPEN`/`OK`, so a failing suite
  # printed `Ran 35 tests in 0.03s | URLOPEN=0` -- the failure invisible as well
  # as non-fatal, and the URLOPEN figure the umbrella `:82` cites taken from a run
  # that did not complete. (3) The function then returned the status of the
  # SUCCESSFUL `git worktree remove`, so even a detected failure could not reach
  # `all`'s roster.
  local T rc=0 n
  T=$(mktemp -d)
  git worktree add -q "$T" "$MAIN" || { echo "!! cannot create the $MAIN worktree"; return 1; }
  _measure n ls "$T"/.claude/tools/_webref/test_*.py \
                "$T"/.claude/skills/elidex-plan-review/test_*.py || rc=1
  echo "$n"
  # §4.3.1 / §1 claim FOUR suite files at the base; the count was echoed and
  # never compared (the block-audit of 2026-08-22). The figure has a lifetime --
  # A-ii's `test_preflight.py` makes it five -- and when it moves, this line is
  # what says so, not a reader noticing the memo drifted.
  [ "$n" -eq 4 ] || { echo "!! $n suite files at $MAIN; A-iii §4.3.1 says 4 — the memo's figure has moved"; rc=1; }
  python3 - "$T" <<'SUITESPY' || rc=1
import re, subprocess, sys
spy = ("import sys, urllib.request\n_c=[]\n_o=urllib.request.urlopen\n"
       "urllib.request.urlopen=lambda r,*a,**k:(_c.append(getattr(r,'full_url',r)),_o(r,*a,**k))[1]\n"
       "import atexit; atexit.register(lambda: sys.stderr.write('URLOPEN=%d\\n'%len(_c)))\n")
t = sys.argv[1]
rc = 0
# `FAILED`/`ERROR:`/`FAIL:` are unittest's DIAGNOSTICS, and dropping them is half
# of why a red suite read green here. They are kept, and on a nonzero exit the
# child's whole stderr is replayed so the traceback survives too.
KEEP = ("Ran ", "URLOPEN", "OK", "FAILED", "ERROR:", "FAIL:")
for args in (["discover","-s",f"{t}/.claude/tools/_webref","-p","test_*.py","-t",f"{t}/.claude/tools"],
             ["discover","-s",f"{t}/.claude/skills/elidex-plan-review","-p","test_*.py"]):
    code = spy + "import unittest,sys;sys.argv=['x']+%r;unittest.main(module=None)" % args
    r = subprocess.run([sys.executable,"-c",code], capture_output=True, text=True)
    print(" | ".join(l for l in r.stderr.splitlines() if l.startswith(KEEP)))
    if r.returncode != 0:
        rc = 1
        print(f"!! SUITE FAILED (rc={r.returncode}) under {args[2]} -- the counts on the")
        print("!! line above are from a run that did not pass; they measure nothing.")
        sys.stdout.flush()          # or the replayed traceback lands above its own header
        sys.stderr.write(r.stderr)
    # The umbrella (`:82`) and §4.3.3 cite this block for ZERO `urlopen` calls at
    # the base; the `URLOPEN=` line was printed and never read back, so a suite
    # that started fetching read green (the block-audit of 2026-08-22).
    m = re.search(r"URLOPEN=(\d+)", r.stderr)
    if m is None or m.group(1) != "0":
        rc = 1
        print(f"!! URLOPEN={m.group(1) if m else 'unmeasured'} under {args[2]} -- the suites fetch;"
              " the 0-urlopen claim does not hold")
sys.exit(rc)
SUITESPY
  git worktree remove --force "$T"
  return "$rc"
}

# AUTHOR-LOCAL: these reach a per-user memory directory and sibling worktrees, so
# they cannot run for a second reader. `all` excludes them; run them by name.
AUTHOR_LOCAL="lanes"
