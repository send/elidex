# Slice A-i's part of the re-derivation harness (`…-Ai-spec-label-map.md`) —
# sourced by `2026-07-citation-hygiene-A-rederive.sh`, the only entry point.
#
# Blocks A-i cites and no other memo does. A-i's shared blocks -- `citations`,
# `couplings`, `budget` -- are in `-common.sh`.

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

regions() {  # §4.2 — spec_labels.py A/B region boundaries, by named artifact
  # Round 1: this printed the docstring's braces and none of the two regions
  # §4.2's alias rows depend on. Widened to the alias rationale, the
  # comprehension comment, the tuple shape line and its variadic annotation.
  grep -nE '^"""|^SPECS|^def |^#: |_catalog|fallback|pinned = |catalog = |entry\.get|parse alias|Aliases exist|shifted the alias|entry\[3:\]|tuple\[tuple' \
    .claude/tools/_webref/spec_labels.py
}

readers() {  # THE recurring root, made checkable: every reader of a piece of state
  # R7, R8, R9 and A-i R1 all reduced to "a write-path changed; its OTHER readers
  # were not reconciled". That is an authoring step, not a review finding, and it
  # was never a command. It is now. Usage: `rederive readers SPEC_LABEL_REVERSE`.
  # Prints CODE readers and PROSE readers separately, because the edit sets that
  # failed did so by assigning code and leaving prose -- and by assigning one
  # prose site out of three.
  # The census MUST range over a ref, and default to the baseline the memos
  # declare. Run at HEAD it reports zero readers of a symbol the branch already
  # deleted -- which is the reassuring-and-useless answer.
  local sym=${2:-${SYM:-}} ref=${3:-$MAIN}
  [ -n "$sym" ] || { echo "usage: rederive readers <symbol> [ref]   (ref defaults to $MAIN)"; return 2; }
  echo "== $sym  @ $ref =="
  echo "-- code readers (non-comment, non-docstring lines) --"
  git grep -nwE "$sym" "$ref" -- .claude |
    grep -vE ':[0-9]+: *#' | sed 's/^/   /'
  echo "-- prose readers (comments, docstrings, markdown) --"
  git grep -nwE "$sym" "$ref" -- .claude docs | sed 's/^/   /'
  git grep -nE "^ *#.*$sym" "$ref" -- .claude | sed 's/^/   /'
  echo "-- inside docstrings (grep cannot tell; review these by eye) --"
  python3 - "$sym" "$ref" <<'READERSPY'
import ast, subprocess, sys
sym, ref = sys.argv[1], sys.argv[2]
files = subprocess.run(["git", "ls-tree", "-r", "--name-only", ref, ".claude/"],
                       capture_output=True, text=True).stdout.split()
n = 0
for f in files:
    if not f.endswith(".py"):
        continue
    try:
        src = subprocess.run(["git", "show", f"{ref}:{f}"], capture_output=True,
                             text=True).stdout
        tree = ast.parse(src)
    except Exception:
        continue
    for node in ast.walk(tree):
        doc = ast.get_docstring(node) if isinstance(
            node, (ast.Module, ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)) else None
        if doc and sym in doc:
            name = getattr(node, "name", "<module>")
            print(f"   {f}: docstring of {name} (line {getattr(node, 'lineno', 1)})")
            n += 1
print(f"   ({n} docstring site(s))")
READERSPY
}
