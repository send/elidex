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
# THE UNGUARDED DIRECTION. Everything above compares `SPECS` with the FROZEN
# `origin/main` baseline, which is the K4 pin. Nothing compared it with the gate
# that is live in this checkout, so adding a spec to `SPECS` without adding it to
# `preflight.SPEC_LABEL_REVERSE` left memos citing the new canonical label
# degrading to verify-skip + exit 0, with no test red. Measured: planting one row
# reddens three unit tests and three claims above, and the live gate still cannot
# resolve the label. That is the direction with teeth, so it is asserted here.
_live = open(".claude/skills/elidex-plan-review/preflight.py", encoding="utf-8").read()
_lb = _live[_live.index("SPEC_LABEL_REVERSE = {"):]
_lb = _lb[:_lb.index("}") + 1]
live = {k.lower(): v for k, v in re.findall(r'"([^"]+)":\s*"([^"]+)"', _lb)}
if not live:
    raise SystemExit("!! the live gate's SPEC_LABEL_REVERSE parsed EMPTY; this arm would "
                     "then pass for a reason that is not 'the sets agree'.")
unreachable = sorted(l for _, l, *_ in s.SPECS if l.lower() not in live)
print("labels the LIVE gate cannot resolve:", unreachable or "none")
alias_free = {k.lower(): e[0] for e in s.SPECS for k in (e[0], e[1])}
print("parse aliases:", [x for e in s.SPECS for x in e[3:]])
print("deleting every alias changes the map?", alias_free != a,
      f"(alias-free size={len(alias_free)})")
# The readings above are the claims §0.1 item 1 / §3.1 / §13.1 make (superset,
# nothing changed, nothing lost, equal value sets over 12 specs, 9 added
# spellings, alias-free map); printing them and exiting 0 certified nothing
# (the block-audit of 2026-08-22). Each is asserted; a failing one is named.
bad = [name for name, ok in (
    ("superset",           all(a.get(k) == v for k, v in mk.items())),
    ("nothing changed",    not [k for k, v in mk.items() if a.get(k) not in (None, v)]),
    ("nothing lost",       not [k for k in mk if k not in a]),
    ("12 specs",           len(set(a.values())) == 12),
    ("equal value sets",   set(a.values()) == set(mk.values())),
    ("9 added spellings",  len(set(a) - set(mk)) == 9),
    ("alias-free map",     alias_free == a and not [x for e in s.SPECS for x in e[3:]]),
    ("live gate resolves every canonical label", not unreachable),
) if not ok]
if bad:
    print("!! keysets: the memo's claim(s) do not hold:", ", ".join(bad))
sys.exit(1 if bad else 0)
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
  # may arrive as an empty listing under a zero exit. ONE grep PER artifact: a
  # single alternation exits 0 when ANY alternative matches, so the block was
  # green with 10 of 14 alternatives matching nothing (the block-audit of
  # 2026-08-22) -- the catalog / fall-through / parse-alias artifacts are Slice
  # B's region, absent at A-i by design (§4.2's "omitted" rows), and are not
  # asserted here; the A-region artifacts the rows rest on are, each by name.
  local f=.claude/tools/_webref/spec_labels.py rc=0 pat
  for pat in '^"""' '^SPECS: tuple\[tuple' '^LABEL_TO_SHORTNAME: dict' '^def label_for' '^def shortname_for' '^#: '; do
    grep -nE "$pat" "$f" || { echo "!! A-region artifact absent: $pat"; rc=1; }
  done
  return "$rc"
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
    except SyntaxError as e:
        # A reader this interpreter cannot parse has UNKNOWN docstring spans;
        # continuing filed its hits as code and the partition below was then
        # a census of nothing (Codex R18). Not a census -- say so and stop.
        raise SystemExit(f"!! `{path}` at {ref} does not parse ({e.msg}, line {e.lineno}); "
                         "its docstring spans are unknown and the partition cannot be derived.")
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
  return $?   # the heredoc'd census IS the measurement; say so
}

readercensus() {  # §15 — the four reader censuses A-i cites, as ONE roster entry
  # `readers` takes a required <symbol> and has no zero-arg form, so it could not
  # sit in `all`; A-i §15 therefore listed the four invocations as prose, and
  # `all`'s exclusion notice named only `lanes staleclaims` (Codex R14). A census
  # the roster never runs is the "authoring step that was never a command" this
  # block's parent exists to end. The expected readings are §4.1's: the three
  # symbols that exist at $MAIN have readers there; `label_for` is NEW in A-i, so
  # at $MAIN its census is EMPTY -- the loud-empty guard firing IS the reading
  # (A-i §4.1 row) -- and at HEAD it is not.
  local rc=0 s
  for s in _SPEC_LABEL_MAP COMMON_SHORTNAMES SPEC_LABEL_REVERSE; do
    readers readers "$s" "$MAIN" || { echo "!! \`readers $s $MAIN\` did not produce a populated, partitioned census"; rc=1; }
  done
  # "Non-zero" is not "empty": the payload aborting for any other reason also
  # exits non-zero, and reading that as the expected-empty result certified a
  # census never taken (Codex R15). Only the explicit `EMPTY CENSUS` line is
  # the reading this arm is named for.
  local lf; lf=$(readers readers label_for "$MAIN" 2>&1); local lfrc=$?
  if [ "$lfrc" -eq 0 ]; then
    echo "!! \`readers label_for $MAIN\` found readers — §4.1's 'none at origin/main' no longer holds"; rc=1
  elif printf '%s\n' "$lf" | grep -q '^!! EMPTY CENSUS'; then
    echo "(readers label_for $MAIN: empty, as §4.1 states — the module is new in A-i)"
  else
    echo "!! \`readers label_for $MAIN\` exited $lfrc WITHOUT the EMPTY CENSUS reading — not a census:"
    printf '%s\n' "$lf" | tail -3 | sed 's/^/   /'; rc=1
  fi
  readers readers label_for HEAD || { echo "!! \`readers label_for HEAD\` is empty or unpartitioned"; rc=1; }
  return "$rc"
}
