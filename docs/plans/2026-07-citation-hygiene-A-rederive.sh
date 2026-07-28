#!/usr/bin/env bash
# Re-derivation harness for docs/plans/2026-07-citation-hygiene-A-enforcement-plumbing.md
#
# The memo carries no measured digits of its own: every quantity it relies on is
# printed by a function here, and the memo cites the function name. Run one:
#
#     bash docs/plans/2026-07-citation-hygiene-A-rederive.sh <name>
#     bash docs/plans/2026-07-citation-hygiene-A-rederive.sh all
#
# Rationale: five review rounds produced a stale-or-underived-coordinate finding
# four times. Prose descriptions of executable things rot and self-contradict;
# an executable does neither. This file is also where the §6 fixture bodies live,
# so the fixtures a reviewer measures are byte-identical to the ones A ships.
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"

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
  { echo '# fixture'; echo; echo '## §3. Spec coverage map'; echo; echo "$HDR"
    echo '| CSSOM VIEW §4.2 Foo | s | b | t | ✓ | no |'
  } > "$d/allunmapped.md"
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

citations() {  # §0.5 / §3 — the two fixture citations
  .claude/tools/webref heading --exact html 4.10.21
  .claude/tools/webref heading --exact html 4.10.21.2
}

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
    r = subprocess.run([sys.executable, ".claude/tools/webref", "heading", sn, ""],
                       capture_output=True, text=True)
    return hashlib.md5(r.stdout.encode()).hexdigest()
same = diff = 0; examples = []
for a, lab, b in bad:
    if dig(a) == dig(b): same += 1
    else: diff += 1; examples.append((a, lab, b))
print(f"  by `webref heading` output : same={same} diff={diff}   <- the memo's criterion")
for e in examples: print("     DIFFERENT:", e)
PY
}

keysets() {  # §0.1 item 1 / §3.1 — 15 -> 24, the 9 added spellings, equal value sets
  python3 - <<'PY'
import sys, re, subprocess
sys.path.insert(0, ".claude/tools")
from _webref import spec_labels as s
src = subprocess.run(["git","show","origin/main:.claude/skills/elidex-plan-review/preflight.py"],
                     capture_output=True, text=True).stdout
body = src[src.index("SPEC_LABEL_REVERSE = {"):]
body = body[:body.index("}")+1]
main = dict(re.findall(r'"([^"]+)":\s*"([^"]+)"', body))
mk = {k.lower(): v for k, v in main.items()}
a = s.LABEL_TO_SHORTNAME
print(f"origin/main keys={len(main)}  A keys={len(a)}")
print("superset:", all(a.get(k) == v for k, v in mk.items()),
      " changed:", [k for k, v in mk.items() if a.get(k) not in (None, v)],
      " lost:", [k for k in mk if k not in a])
print("added spellings:", sorted(set(a) - set(mk)))
print("value sets equal:", set(a.values()) == set(mk.values()),
      f"({len(set(a.values()))} specs)")
alias_free = {k.lower(): e[0] for e in s.SPECS for k in (e[0], e[1])}
print("parse aliases:", [x for e in s.SPECS for x in e[3:]])
print("deleting every alias changes the map?", alias_free != a,
      f"(alias-free size={len(alias_free)})")
PY
}

column() {  # §5 — the origin/main column, every fixture shape, on a real worktree
  local T; T=$(mktemp -d); git worktree add -q "$T" "$MAIN"
  local F; F=$(mktemp -d); fixtures "$F" >/dev/null
  for f in labelled dedup unlabelled allunmapped alias nospec nospec-and-table nospec-and-header fenced-marker; do
    printf '\n--- %s ---\n' "$f"
    ( cd "$T" && python3 "$PF" --no-grep-pass "$F/$f.md" 2>&1 |
        grep -E 'citation verify|unmapped-label|unique specs|HARD FAIL|unrecognized|extend|total entries' )
    ( cd "$T" && python3 "$PF" --no-grep-pass "$F/$f.md" >/dev/null 2>&1; echo "EXIT=$?" )
  done
  git worktree remove --force "$T"; rm -rf "$F"
}

carvecolumn() {  # the same fixtures at the carve — what §12(2)'s red-check can detect
  local F; F=$(mktemp -d); fixtures "$F" >/dev/null
  for f in labelled alias allunmapped; do
    printf '\n--- %s (carve) ---\n' "$f"
    python3 "$PF" --no-grep-pass "$F/$f.md" 2>&1 |
      grep -E 'citation verify|unmapped-label|unique specs|unrecognized'
  done
  rm -rf "$F"
}

remedies() {  # §4.2.4 / P5 — which remedy strings co-print when the map is absent
  local T; T=$(mktemp -d); git worktree add -q "$T" HEAD
  local F; F=$(mktemp -d); fixtures "$F" >/dev/null
  mv "$T/.claude/tools/_webref" "$T/_hidden"
  ( cd "$T" && python3 "$PF" --no-grep-pass "$F/labelled.md" 2>&1 | grep -E 'unrecognized|extend|SPECS|unmapped' )
  mv "$T/_hidden" "$T/.claude/tools/_webref"
  git worktree remove --force "$T"; rm -rf "$F"
}

suites() {  # §1 / §4.3.1 / §4.3.3 — 47 tests, 4 files, and the fetch count
  local T; T=$(mktemp -d); git worktree add -q "$T" "$MAIN"
  ls "$T"/.claude/tools/_webref/test_*.py "$T"/.claude/skills/elidex-plan-review/test_*.py | wc -l
  python3 - "$T" <<'PY'
import subprocess, sys
spy = ("import sys, urllib.request\n_c=[]\n_o=urllib.request.urlopen\n"
       "urllib.request.urlopen=lambda r,*a,**k:(_c.append(getattr(r,'full_url',r)),_o(r,*a,**k))[1]\n"
       "import atexit; atexit.register(lambda: sys.stderr.write('URLOPEN=%d\\n'%len(_c)))\n")
t = sys.argv[1]
for args in (["discover","-s",f"{t}/.claude/tools/_webref","-p","test_*.py","-t",f"{t}/.claude/tools"],
             ["discover","-s",f"{t}/.claude/skills/elidex-plan-review","-p","test_*.py"]):
    code = spy + "import unittest,sys;sys.argv=['x']+%r;unittest.main(module=None)" % args
    r = subprocess.run([sys.executable,"-c",code], capture_output=True, text=True)
    print(" | ".join(l for l in r.stderr.splitlines() if l.startswith(("Ran ","URLOPEN","OK"))))
PY
  git worktree remove --force "$T"
}

anchors() {  # §3.1 / §4.2 — origin/main by symbol, never by stored line number
  git show "$MAIN:$PF" | grep -n \
    'SECTION_REF_RE\|^def parse_spec_cell\|^def shortname_from_label\|^def verify_citation\|dest="grep_pass"\|unique_specs\|seen_pairs\|elif seen_pairs\|HARD FAIL'
}

regions() {  # §4.0 — spec_labels.py A/B region boundaries, including the intra-fn splits
  grep -n '^"""\|^SPECS\|^def \|^#: \|_catalog\|SPECS is a fallback\|pinned = \|catalog = \|entry.get' \
    .claude/tools/_webref/spec_labels.py
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

couplings() {  # §7 / §12(3) — every elidex coupling in the generic tree, by concept
  echo "-- origin/main baseline --"
  git grep -nE '\.claude/skills|elidex-plan-review|plan-review|plan-memo|memos abbreviate' \
    "$MAIN" -- .claude/tools/_webref/ | cat
  echo "-- HEAD --"
  git grep -nE '\.claude/skills|elidex-plan-review|plan-review|plan-memo|memos abbreviate' \
    -- .claude/tools/_webref/ | cat
}

suiteset() {  # §4.3.2 J4 — the set the uncollected-suite check must range over
  git ls-files '.claude/**/test_*.py'
  echo "-- discover roots --"; echo ".claude/tools/_webref"; echo ".claude/skills/elidex-plan-review"
}

marker() { git grep -nE '^[[:space:]]*\*\*No spec surface\*\*' -- docs/plans/ || echo "(none)"; }

budget() {
  for f in "$PF" .claude/tools/_webref/commands/coverage_map.py .claude/tools/_webref/cli.py \
           .claude/tools/_webref/DESIGN.md mise.toml .github/workflows/ci.yml; do
    echo "$(git show "$MAIN:$f" | wc -l) $f"; done
  echo "$(wc -l < docs/plans/2026-07-citation-hygiene-A-enforcement-plumbing.md) (this memo)"
}

filters() { git show "$MAIN:.github/workflows/ci.yml" | sed -n '/filters:/,/^  check:/p'; }

ruleset() {
  gh api repos/send/elidex/rulesets --jq '.[] | {id, name, enforcement, target}'
  local id; id=$(gh api repos/send/elidex/rulesets --jq '.[] | select(.name=="main-protection") | .id')
  gh api "repos/send/elidex/rulesets/$id" --jq \
    '{rules: [.rules[].type], pr: (.rules[]|select(.type=="pull_request").parameters.required_approving_review_count), bypass: [.bypass_actors[].actor_type], mode: [.bypass_actors[].bypass_mode]}'
}

timing() {  # §11 — subprocess vs in-process resolution, 100 reps, warm cache
  python3 - <<'PY'
import sys, time, subprocess
sys.path.insert(0, ".claude/tools")
from _webref.resolver import lookup_section
W = ".claude/tools/webref"
lookup_section("html", "4.10.21")                      # warm
t = time.perf_counter()
for _ in range(100): lookup_section("html", "4.10.21")
inp = (time.perf_counter() - t) / 100
t = time.perf_counter()
for _ in range(10):
    subprocess.run([sys.executable, W, "heading", "--exact", "html", "4.10.21"], capture_output=True)
sub = (time.perf_counter() - t) / 10
print(f"subprocess={sub:.4f}s  in-process={inp:.6f}s  ratio={sub/inp:.0f}x")
PY
}

lanes() {  # §13 — base, open PRs, worktrees authoring plan-memos, the two carve commits
  git rev-list --left-right --count "$MAIN"...HEAD
  gh pr list --state open --json number,headRefName --jq '.[] | "\(.number) \(.headRefName)"'
  git log --format='%h %s' --grep='carve the cite-audit detector'
  git log --format='%h %s' --grep='re-carve the shared spec-label map'
  for w in $(git worktree list --porcelain | awk '/^worktree /{print $2}'); do
    n=$(git -C "$w" diff --name-only "$MAIN"...HEAD -- docs/plans/ 2>/dev/null | wc -l)
    [ "$n" -gt 0 ] && echo "$n $w"
  done
}

bmemo() {  # §13 — the classes of edit B's memo needs, grep-derived not read
  local B=docs/plans/2026-07-citation-hygiene-B-detector-correctness.md
  echo "-- file-creation claims for files A creates --"; grep -n 'test_spec_labels' "$B"
  echo "-- pin names colliding with A's --";            grep -nE '^\- \*\*P[0-9]' "$B"
  echo "-- spec_labels.py anchors --";                  grep -n 'spec_labels\.py:' "$B"
  echo "-- Slice A section refs --";                    grep -nE 'Slice A §|A §4' "$B"
  echo "-- present-tense catalog framing --";           grep -n 'the carve' "$B"
  echo "-- cap-rule statements --";                     grep -n 'cleanup-' "$B"
  echo "-- line-count table --";                        grep -n 'cite_audit.py.*|.*[0-9]' "$B" | head -5
}

staleclaims() {  # §13 — the cross-file claims this memo corrects, by concept not string
  local M=/Users/kazuaki/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory
  echo "-- '10 in-flight memos' concept --"
  grep -rn '10 in-flight\|10 memos' docs/plans/ "$M" 2>/dev/null
  echo "-- actual in-flight memo count in elidex-wt-c3-plan --"
  git -C /Users/kazuaki/repos/send.sh/elidex-wt-c3-plan diff --name-only "$MAIN"...HEAD -- docs/plans/ | wc -l
  echo "-- 'wrong document' consequence --"
  grep -rn 'wrong document' docs/plans/ "$M" 2>/dev/null
  echo "-- dangling shas in memory --"
  for s in $(grep -rhoE '`[0-9a-f]{8}`' "$M" | tr -d '`' | sort -u); do
    git cat-file -e "$s^{commit}" 2>/dev/null && \
      { git merge-base --is-ancestor "$s" HEAD 2>/dev/null || echo "NON-ANCESTOR $s"; } || echo "UNKNOWN $s"
  done | sort -u | head -20
}

all() { for f in citations partition keysets column carvecolumn remedies suites anchors regions \
                 offline couplings suiteset marker budget filters ruleset timing lanes bmemo staleclaims; do
          say "$f"; "$f"; done; }

"${1:-all}"
