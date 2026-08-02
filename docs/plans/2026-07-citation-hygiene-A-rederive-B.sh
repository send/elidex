# Slice B's part of the re-derivation harness (`…-B-detector-correctness.md`) —
# sourced by `2026-07-citation-hygiene-A-rederive.sh`, the only entry point.
#
# B cites no block by name; these four are routed by the quantity they derive.
# B §4.1.2 and §4.1.8 embed `partition`'s round-trip census (203/948) and
# `offline`'s SystemExit escape as inline scripts -- these blocks are their
# executable twin. `bmemo` and `staleclaims` derive the classes of edit B's memo
# needs; `staleclaims` is author-local and excluded from `all`.

partition() {  # §0 — 203/948, and the 195/8 vs 190/13 split under both criteria
  python3 - <<'PY'
import sys, hashlib, subprocess
sys.path.insert(0, ".claude/tools")
from _webref import spec_labels as s
cat = s._catalog()
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
PY
}

offline() {  # §10-Q1(a) — the SystemExit escape the boundary rests on
  python3 - <<'PY'
import sys, urllib.request, urllib.error, os
os.environ["XDG_CACHE_HOME"] = "/tmp/empty-cache-rederive"; sys.path.insert(0, ".claude/tools")
urllib.request.urlopen = lambda *a, **k: (_ for _ in ()).throw(urllib.error.URLError("offline"))
from _webref import spec_labels
try: print("returned:", spec_labels.shortname_for("CSS Text 3"))
except SystemExit as e: print("SystemExit ESCAPED _catalog():", e)
PY
}

bmemo() {  # §13 — the classes of edit B's memo needs, grep-derived not read
  # §13 names ELEVEN classes. Draft 8's version had blocks for seven, and two of
  # those greps returned something other than their own label (`the carve` matched
  # a perf comment; the line-count grep's first hit was a §3 coverage-map row).
  # A block that does not derive its label is the defect this file exists to end.
  local B=docs/plans/2026-07-citation-hygiene-B-detector-correctness.md
  echo "-- 1. file-creation claims for files A creates --"; grep -n 'test_spec_labels' "$B"
  echo "-- 2. pin names colliding with A's --";             grep -nE '^\- \*\*P[0-9]' "$B"
  echo "-- 3. spec_labels.py line anchors --";              grep -n 'spec_labels\.py:' "$B"
  echo "-- 4. Slice A section refs (swapped §4.1/§4.2) --"; grep -nE 'Slice A §|A §4' "$B"
  echo "-- 5. §4.1.8's falsified consequence sentence --"
  grep -nE 'wrong document|silently runs against' "$B"
  echo "-- 6. present-tense 'extant defect' framing of what the carve did --"
  grep -nE 'is an? (extant|existing) defect|today the resolver|currently (the )?resolv' "$B" || echo "   (none)"
  echo "-- 7. §0.1 provenance paragraph naming a base B no longer has --"
  grep -nE 'branch(es)? from|carve|base' "$B" | head -8
  echo "-- 8. §4.2's seam list — must name the widening as a third seam --"
  grep -nE '^\|.*seam|seams?:' "$B" | head -8
  echo "-- 9. coverage_map's changed last-resort cited as pre-existing --"
  grep -nE '_spec_label|last.resort|upper\(\)' "$B"
  echo "-- 10. cap-rule restatements (must become a pointer) --"; grep -n 'cleanup-\|per-PR ≤3\|cap' "$B"
  echo "-- 11. line-count table measured at a base where 2 files do not exist --"
  grep -nE '^\|[^|]*(cite_audit|spec_labels|webref_data)[^|]*\|[^|]*[0-9]{2,}' "$B"
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
