# Slice B's part of the re-derivation harness (`…-B-detector-correctness.md`) —
# sourced by `2026-07-citation-hygiene-A-rederive.sh`, the only entry point.
#
# B cites no block by name; these four are routed by the quantity they derive.
# B §4.1.2 and §4.1.8 embed `partition`'s round-trip census (203/948) as an
# inline script; `offline` asserts §4.1.7's CONTRACT (not its reproducer). `bmemo` and `staleclaims` derive the classes of edit B's memo
# needs; `staleclaims` is author-local and excluded from `all`.

partition() {  # §0 — 203/948, and the 195/8 vs 190/13 split under both criteria
  python3 - <<'PY'
import sys, hashlib, subprocess
sys.path.insert(0, ".claude/tools")
from _webref import spec_labels as s
if not hasattr(s, "_catalog"):
    print("!! spec_labels has no catalog fall-through at this head (A-i K3) — B-owned RED; no census can be taken")
    sys.exit(1)
res = s._catalog()
# B §4.1.7: `_catalog()` returns `CatalogResult(available, entries, cause)`.
# Iterating the result itself raised the moment B landed the contract (Codex
# R21); the census ranges over `entries`, and an unavailable catalog is not a
# partition of anything.
if getattr(res, "available", None) is not True:
    print("!! catalog unavailable (%r) — no round-trip census can be taken" % (getattr(res, "cause", res),))
    sys.exit(1)
cat = res.entries
# `available=True` with no entries (an upstream/schema regression) is not a
# catalog: every population and partition count below would print 0 and the
# block would certify a census that ranged over nothing (Codex R37).
if not cat:
    print("!! catalog available but EMPTY — no round-trip census can be taken")
    sys.exit(1)
bad = []
for short in cat:
    lab = s.label_for(short)
    if lab is None: continue
    back = s.shortname_for(lab)
    if back != short: bad.append((short, lab, back))
print(f"catalog={len(cat)}  non-round-trip={len(bad)}")
def ser(x): return ((cat.get(x) or {}).get("series") or {}).get("shortname")
print("  by series      : same=%d diff=%d" % (
    sum(1 for a,_,b in bad if b and ser(a)==ser(b)),
    sum(1 for a,_,b in bad if not b or ser(a)!=ser(b))))
print("  by catalog key : same=%d diff=%d" % (
    sum(1 for a,_,b in bad if (cat.get(a) or {}).get("shortname")==(cat.get(b) or {}).get("shortname")),
    sum(1 for a,_,b in bad if (cat.get(a) or {}).get("shortname")!=(cat.get(b) or {}).get("shortname"))))
def dig(sn):
    """None means NOT MEASURED. `webref heading <bogus> ''` exits 1 with empty
    stdout, so digesting stdout alone made two lookups that both FAILED hash
    identically and land in `same` -- "these two shortnames agree" asserted of a
    comparison that never happened. A shortname of None (no round-trip result at
    all) is the same non-answer, one step earlier."""
    if not sn:
        return None
    r = subprocess.run([sys.executable, ".claude/tools/webref", "heading", sn, ""],
                       capture_output=True, text=True)
    if r.returncode != 0:
        return None
    return hashlib.md5(r.stdout.encode()).hexdigest()
same = diff = unresolved = 0; examples = []
for a, lab, b in bad:
    da, db = dig(a), dig(b)
    if da is None or db is None:
        unresolved += 1; examples.append(("NOT-MEASURED", a, lab, b)); continue
    if da == db: same += 1
    else: diff += 1; examples.append(("DIFFERENT", a, lab, b))
print(f"  by `webref heading` output : same={same} diff={diff} not-measured={unresolved}"
      "   <- the memo's criterion")
for e in examples: print("    ", e[0] + ":", e[1:])
# A comparison that was NOT MEASURED is not a partition result: `dig` says so per
# pair, and the block still exited 0 on a run that reported `not-measured=N`
# (Codex R10). An incomplete catalog measurement cannot certify B §4.1.8's split.
if unresolved:
    print(f"!! {unresolved} comparison(s) NOT MEASURED — the partition figures above are incomplete")
    sys.exit(1)
PY
  return $?    # the heredoc'd command IS the measurement; say so
}

offline() {  # B §4.1.7 — the offline CONTRACT: no SystemExit escapes the catalog path
  # B §4.1.7 embeds a reproducer that records "SystemExit ESCAPED" -- the
  # DEFECT. A block that asserts the defect turns permanently RED the moment B
  # fixes it and then misdiagnoses the fix as "no catalog fall-through" (Codex
  # R17): a control that blesses the defect is worse than none. So this block
  # asserts B's stated contract instead -- the catalog path is present AND a
  # poisoned network yields no SystemExit -- and B §6 S12 pins the unavailable
  # result's shape. At A-i's head `spec_labels` has no `_catalog` (A-i's K3), so
  # the first half is RED until B lands -- an expected, owner-routed RED,
  # recorded in A-i §13.1 -- and says so by name.
  # The precondition is an EMPTY cache: a fixed `/tmp` path persists across runs
  # and users and can hold a valid catalog, satisfying the lookup without ever
  # touching the poisoned `urlopen` (Codex R12). Fresh directory, removed after.
  local C; C=$(mktemp -d) || { echo "!! cannot create an empty cache dir"; return 1; }
  local rc=0
  XDG_CACHE_HOME="$C" python3 - <<'PY' || rc=1
import sys, urllib.request, urllib.error, os
sys.path.insert(0, ".claude/tools")
import re
urllib.request.urlopen = lambda *a, **k: (_ for _ in ()).throw(urllib.error.URLError("offline"))
from _webref import spec_labels
if not hasattr(spec_labels, "_catalog"):
    print("!! spec_labels has no catalog fall-through at this head (A-i K3) — B-owned RED, the contract cannot be exercised yet")
    sys.exit(1)
try:
    print("returned:", spec_labels.shortname_for("CSS Text 3"))
    cat = spec_labels._catalog()
except SystemExit as e:
    print("!! SystemExit ESCAPED _catalog():", e, "— B §4.1.7's contract (discriminated unavailable, no escape) does not hold")
    sys.exit(1)
# The second half of the contract: offline is UNAVAILABLE, never an available
# empty catalog -- `{}` is the fail-open §4.1.7 names (Codex R18). B §6 S12
# pins the exact shape; what this block can say without it is that a plain
# dict is the wrong answer.
# B §4.1.7 names the shape: `CatalogResult(available, entries, cause)`. Read
# the DISCRIMINATOR -- a type test let an available wrapper around an empty
# catalog, or a bare None, pass as "discriminated" (Codex R19).
if getattr(cat, "available", None) is not False:
    print("!! _catalog() offline did not take the UNAVAILABLE branch: %r — B §4.1.7 requires available=False" % (cat,))
    sys.exit(1)
# The WHOLE unavailable shape (B §4.1.7): entries empty, cause naming the
# exception. `available=False` with stale entries or no cause passed (Codex R25).
if getattr(cat, "entries", None) != {}:
    print("!! unavailable result carries entries %r — B §4.1.7 requires entries={}" % (getattr(cat, "entries", None),))
    sys.exit(1)
if not re.match(r"URLError\b", str(getattr(cat, "cause", ""))):
    print("!! unavailable result does not name the poisoned URLError as cause: %r" % (getattr(cat, "cause", None),))
    sys.exit(1)
print("catalog offline -> available=False, entries={}, cause=%r" % (cat.cause,))
PY
  rm -rf "$C"
  return "$rc"    # the heredoc'd command IS the measurement; say so
}

bmemo() {  # §13 — the classes of edit B's memo needs, grep-derived not read
  # §13 names ELEVEN classes. Draft 8's version had blocks for seven, and two of
  # those greps returned something other than their own label (`the carve` matched
  # a perf comment; the line-count grep's first hit was a §3 coverage-map row).
  # A block that does not derive its label is the defect this file exists to end.
  local B=docs/plans/2026-07-citation-hygiene-B-detector-correctness.md
  # The eleven greps are a LISTING, and for several of them "matched nothing" is
  # the ANSWER -- item 6 already spells that out, and item 11 matching nothing is
  # the desired END STATE. So no grep's status is this block's verdict, and until
  # now the verdict was item 11's by accident: fixing the stale table this block
  # exists to find would have reported `bmemo(exit 1)` in `all`'s roster, and a
  # roster that reddens on success stops being read. What IS this block's verdict
  # is whether the memo was there to grep at all -- without it, eleven silent
  # greps read as eleven clean items.
  [ -f "$B" ] || { echo "!! $B is not there — eleven empty greps are not eleven clean items."
                   return 1; }
  # Each item carries its EXPECTED reading: `yes` = the class must be present in
  # B (a memo that lost it silently is a different memo), `no` = the class is a
  # defect B must not carry. A grep whose status was discarded certified neither
  # (Codex R25). `_bm expect label pattern [grep-flags]`.
  local rc=0
  _bm() {
    local want=$1 label=$2 pat=$3; shift 3
    echo "-- $label --"
    local hits; hits=$(grep -n "$@" -- "$pat" "$B")
    [ -n "$hits" ] && printf '%s\n' "$hits" | head -8
    case "$want:${hits:+y}" in
      yes:y|no:) ;;
      yes:) echo "   !! expected this class PRESENT in B, found nothing — the memo lost it"; rc=1 ;;
      no:y)  echo "   !! expected NONE — B still carries this defect"; rc=1 ;;
    esac
  }
  _bm yes "1. file-creation claims for files A creates" 'test_spec_labels'
  _bm yes "2. pin names colliding with A's" '^\- \*\*P[0-9]' -E
  _bm yes "3. spec_labels.py line anchors" 'spec_labels\.py:'
  # Items 4/5/7/9 were fixed in B's text by PR #501 (cumulative /elidex-review
  # over R6-R32: a control that pins a known-false sentence in place blesses the
  # defect -- the inverse of `offline`'s R17 lesson). They are now `no`, each
  # pattern being the defect's own wording.
  _bm no  "4. Slice A section refs (swapped §4.1/§4.2)" 'byte-identical on purpose|\(A §4\.[12]\)' -E
  _bm no  "5. §4.1.8's falsified consequence sentence" 'silently runs against' -E
  _bm no  "6. present-tense 'extant defect' framing of what the carve did" 'is an? (extant|existing) defect|today the resolver|currently (the )?resolv' -E
  _bm no  "7. §0.1 provenance paragraph naming a base B no longer has" '26721cfa|96a8e47b' -E
  _bm yes "8. §4.2's seam list — must name the generic-core scope as a third seam" 'the third seam'
  _bm no  "9. coverage_map's changed last-resort cited as pre-existing" 'already chose' -E
  # The probe-window cap is decided in ONE place (§10-Q2); a restatement with
  # a figure elsewhere drifts from it. `yes` = the pointer exists; `no` = no
  # site states the cap as a figure (a bare `cap` matched `caps = {…}` and the
  # canonical rule itself, so deleting the pointer stayed green -- Codex R37).
  _bm yes "10. the probe-cap pointer §10-Q2 is present" '§10-Q2'
  _bm no  "10b. a probe cap restated as a figure outside §10-Q2" '[0-9]+-word cap' -E
  _bm no  "11. line-count table measured at a base where 2 files do not exist" '^\|[^|]*(cite_audit|spec_labels|webref_data)[^|]*\|[^|]*[0-9]{2,}' -E
  return "$rc"
}

staleclaims() {  # §13 — the cross-file claims this memo corrects, by concept not string
  local M=/Users/kazuaki/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory
  local rc=0
  # CONCEPT, not string. Draft 8 grepped `10 in-flight\|10 memos`, which does not
  # match MEMORY.md's Japanese `10 memo` -- so it missed one of the two live sites,
  # in a memo whose §3.1 mandates concept-greps. The concept is "a count of
  # in-flight memos in the c3-plan worktree".
  echo "-- 'N in-flight memos in elidex-wt-c3-plan' concept --"
  grep -rnE '[0-9]+ *(in-flight|memos?|memo)[^.]{0,40}(c3-plan|in-flight)|c3-plan[^.]{0,40}[0-9]+ *memo' \
    docs/plans/ "$M" 2>/dev/null
  echo "-- actual in-flight memo count in elidex-wt-c3-plan --"
  # The count the greps above are checked AGAINST. `| wc -l` printed 0 when that
  # worktree is gone, which would "confirm" every stale claim of a nonzero count
  # as merely too high rather than unverified.
  local n
  _measure n git -C /Users/kazuaki/repos/send.sh/elidex-wt-c3-plan \
                 diff --name-only "$MAIN"...HEAD -- docs/plans/ || rc=1
  echo "$n"
  echo "-- 'wrong document' consequence --"
  grep -rn 'wrong document' docs/plans/ "$M" 2>/dev/null
  echo "-- dangling shas in memory --"
  # Scoped to the shas THIS memo's §13 acts on. Draft 8 ran the check repo-wide
  # and `head -20`'d 571 non-ancestor results, so all three of item 5's shas sorted
  # past the cut -- the pointer named a block that could not derive the claim.
  for s in d3173bed 53558963 99a3e2c3; do
    printf '  %s: ' "$s"
    if git cat-file -e "$s^{commit}" 2>/dev/null; then
      git merge-base --is-ancestor "$s" HEAD 2>/dev/null && echo "ancestor" || echo "NON-ANCESTOR"
    else echo "UNKNOWN"; fi
    grep -rn "$s" "$M" docs/plans/ 2>/dev/null | sed 's/^/      /'
  done
  return "$rc"
}
