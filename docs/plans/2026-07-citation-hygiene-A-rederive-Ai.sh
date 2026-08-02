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
_r = subprocess.run(["git","show","origin/main:.claude/skills/elidex-plan-review/preflight.py"],
                    capture_output=True, text=True)
if _r.returncode != 0:
    sys.stderr.write(_r.stderr)
    raise SystemExit("!! `git show origin/main:…preflight.py` failed (rc=%d); there is no "
                     "baseline key set to compare against." % _r.returncode)
src = _r.stdout
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
  return $?    # the heredoc'd command IS the measurement; say so
}

regions() {  # §4.2 — spec_labels.py A/B region boundaries, by named artifact
  # Round 1: this printed the docstring's braces and none of the two regions
  # §4.2's alias rows depend on. Widened to the alias rationale, the
  # comprehension comment, the tuple shape line and its variadic annotation.
  #
  # grep's status IS this block's: §4.2 cites these artifacts as PRESENT, so both
  # 1 (ran, matched nothing) and 2 (no such file) are failures here, and neither
  # may arrive as an empty listing under a zero exit.
  grep -nE '^"""|^SPECS|^def |^#: |_catalog|fallback|pinned = |catalog = |entry\.get|parse alias|Aliases exist|shifted the alias|entry\[3:\]|tuple\[tuple' \
    .claude/tools/_webref/spec_labels.py
  return $?
}

readers() {  # THE recurring root, made checkable: every reader of a piece of state
  # R7, R8, R9 and A-i R1 all reduced to "a write-path changed; its OTHER readers
  # were not reconciled". That is an authoring step, not a review finding, and it
  # was never a command. It is now. Usage: `rederive readers SPEC_LABEL_REVERSE`.
  # Prints CODE readers and PROSE readers separately, because the edit sets that
  # failed did so by assigning code and leaving prose -- and by assigning one
  # prose site out of three.
  #
  # CODE and PROSE are ONE census PARTITIONED, not two greps. Through A-i round 2
  # this block ran the code pass as `git grep ... -- .claude | grep -v '^#'` and
  # the prose pass as `git grep ... -- .claude docs` -- the UNFILTERED SUPERSET.
  # For any symbol with no leading-`#` reader the two sections therefore printed
  # byte-identical output (`readers SHORTNAME_TO_LABEL HEAD`: the same 5 lines
  # twice), and the separation the block advertises -- the separation the failing
  # edit sets needed -- did not exist. It only LOOKED separable for
  # `SPEC_LABEL_REVERSE`, which happens to have exactly one comment reader.
  # Classification is syntactic: a markdown/prose file is prose; a `#`-comment
  # line is prose; a line inside a docstring is prose, decided by the AST section
  # that used to only report beside the partition and now decides it; everything
  # else is code.
  #
  # The census MUST range over a ref, and default to the baseline the memos
  # declare. Run at HEAD it reports zero readers of a symbol the branch already
  # deleted -- which is the reassuring-and-useless answer, so an empty census and
  # a broken partition are LOUD (`!!`) and exit non-zero. A block that silently
  # reports nothing issues a clean bill of health it did not earn; that is the
  # failure mode four review rounds went past.
  local sym=${2:-${SYM:-}} ref=${3:-$MAIN}
  [ -n "$sym" ] || { echo "usage: rederive readers <symbol> [ref]   (ref defaults to $MAIN)"; return 2; }
  python3 - "$sym" "$ref" <<'READERSPY'
import ast, re, subprocess, sys
sym, ref = sys.argv[1], sys.argv[2]


def git(*args, ok=(0,)):
    """`ok` is the statuses that MEAN something. `git grep` returns 1 for "ran,
    matched nothing", which is a real answer; everything else -- 128 for an
    unresolvable ref above all -- would hand this census an empty string it would
    then report as "no readers". The loud-empty guard below cannot tell those
    apart, so the distinction has to be made here."""
    r = subprocess.run(["git", *args], capture_output=True, text=True)
    if r.returncode not in ok:
        sys.stderr.write(r.stderr)
        raise SystemExit("!! `git %s` failed (rc=%d); an empty census from a command that "
                         "did not run is not a census." % (" ".join(args), r.returncode))
    return r.stdout


prefix = ref + ":"
hits = []
for raw in git("grep", "-nwE", sym, ref, "--", ".claude", "docs", ok=(0, 1)).splitlines():
    if not raw.startswith(prefix):
        continue
    path, lineno, text = raw[len(prefix):].split(":", 2)
    hits.append((path, int(lineno), text))

# Docstring spans, by AST. `git grep` cannot tell a docstring from code, so this
# is what makes the partition real rather than advertised.
spans, docsites = {}, []
for path in sorted({p for p, _, _ in hits if p.endswith(".py")}):
    try:
        tree = ast.parse(git("show", f"{ref}:{path}"))
    except Exception:
        continue
    rs = []
    for node in ast.walk(tree):
        if not isinstance(node, (ast.Module, ast.ClassDef, ast.FunctionDef,
                                 ast.AsyncFunctionDef)):
            continue
        doc = ast.get_docstring(node)
        if doc is None:
            continue
        expr = node.body[0]
        rs.append((expr.lineno, expr.end_lineno))
        if sym in doc:
            docsites.append((path, expr.lineno, getattr(node, "name", "<module>")))
    spans[path] = rs

COMMENT = re.compile(r"\s*(#|//)")


def prose_kind(path, lineno, text):
    """None means code. Anything else is the reason the line is prose."""
    if not path.endswith((".py", ".sh", ".toml", ".yml", ".yaml", ".cfg")):
        return "markdown/prose file"
    for a, b in spans.get(path, []):
        if a <= lineno <= b:
            return "docstring"
    return "comment" if COMMENT.match(text) else None


code, prose = [], []
for path, lineno, text in hits:
    kind = prose_kind(path, lineno, text)
    (prose if kind else code).append((path, lineno, text, kind))


def show(rows, what):
    for path, lineno, text, kind in rows:
        print(f"   {path}:{lineno}:{text.strip()}" + (f"   [{kind}]" if kind else ""))
    if not rows:
        print(f"   (no {what} readers)")


print(f"== {sym}  @ {ref} ==")
print(f"-- code readers ({len(code)}) — not a comment, not a docstring --")
show(code, "code")
print(f"-- prose readers ({len(prose)}) — the COMPLEMENT: comments, docstrings, markdown --")
show(prose, "prose")
print(f"-- inside docstrings, by AST ({len(docsites)} site(s)) --")
for path, lineno, name in docsites:
    print(f"   {path}: docstring of {name} (line {lineno})")

rc = 0
if not hits:
    print(f"!! EMPTY CENSUS: no reader of `{sym}` at {ref}. Either the symbol is")
    print("!! spelled differently, or the ref is wrong (a symbol this branch")
    print("!! deletes has no readers at HEAD). NOT a clean bill of health.")
    rc = 1
elif [(p, n) for p, n, _, _ in code] == [(p, n) for p, n, _, _ in prose]:
    print("!! CODE AND PROSE SECTIONS ARE IDENTICAL — the partition is broken and")
    print("!! the separation this block advertises is not being computed.")
    rc = 1
sys.exit(rc)
READERSPY
}
