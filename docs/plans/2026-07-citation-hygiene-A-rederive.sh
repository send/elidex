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

column() {  # §5 — the origin/main column, every fixture shape, BOTH CLI states
  # The "map" axis does not exist on origin/main (a module-local dict with no
  # import to fail), which is why §5's rows 6-9/14 read n/a there. The CLI axis
  # does exist, and rows 3/4/5 need it — draft 6's `column` never varied it.
  local T; T=$(mktemp -d); git worktree add -q "$T" "$MAIN"
  local F; F=$(mktemp -d); fixtures "$F" >/dev/null
  for st in both nocli; do
    [ "$st" = nocli ] && mv "$T/.claude/tools/webref" "$T/.shim"
    for f in labelled dedup unlabelled allunmapped alias nospec nospec-and-table nospec-and-header fenced-marker; do
      local out rc
      out=$( cd "$T" && python3 "$PF" --no-grep-pass "$F/$f.md" 2>&1 ); rc=$?
      printf '%-8s %-18s EXIT=%d  %s\n' "$st" "$f" "$rc" \
        "$(echo "$out" | grep -oE 'citation verify: +.*|HARD FAIL — [^.]*' | head -1)"
    done
    [ "$st" = nocli ] && mv "$T/.shim" "$T/.claude/tools/webref"
  done
  git worktree remove --force "$T"; rm -rf "$F"
}

carvecolumn() {  # the same fixtures at the carve — what §12(2)'s red-check can detect
  # ALL nine, not three: §6's "fails at the carve?" column is only checkable
  # against the carve's exit code, and draft 7 asserted "yes" for a fixture the
  # carve already exits 0 on (the fenced marker is inert prose there, so the
  # behavioural half of that pin passes at the carve BY ACCIDENT).
  local F; F=$(mktemp -d); fixtures "$F" >/dev/null
  for f in labelled dedup unlabelled allunmapped alias nospec nospec-and-table nospec-and-header fenced-marker; do
    local out rc
    out=$(python3 "$PF" --no-grep-pass "$F/$f.md" 2>&1); rc=$?
    printf '%-18s EXIT=%d  %s\n' "$f" "$rc" \
      "$(echo "$out" | grep -oE 'citation verify: +.*|HARD FAIL — [^.]*|⚠ unrecognized.*' | head -1)"
  done
  rm -rf "$F"
}

# --- capability instruments ---------------------------------------------------
# §5's two axes are "CLI" and "map". Getting either wrong invalidates every row
# measured with it, and draft 7 got the map axis wrong: `mv .claude/tools/_webref`
# leaves `WEBREF.is_file()` TRUE — `.claude/tools/webref` is a separate 16-line
# shim — while the CLI dies `ModuleNotFoundError` at invocation. That is a state
# §5 has no row for: A's static verdict names only the map, and the CLI is in
# fact broken. `instruments` measures all three candidates so the choice is not
# taken on faith.
#
#   map axis : an in-process `sys.meta_path` block. Tree intact ⇒ the child
#              process the gate spawns still resolves ⇒ genuinely "CLI ✓ / map ✗".
#   CLI axis : rename `.claude/tools/webref`. That file, and only that file, is
#              what `WEBREF.is_file()` reads.
_runner() {  # emit the capability-state runner into $1/runpf.py
  cat > "$1/runpf.py" <<'PY'
import atexit, os, runpy, subprocess, sys
from pathlib import Path

class _BlockWebref:
    def find_spec(self, fullname, path=None, target=None):
        if fullname == "_webref" or fullname.startswith("_webref."):
            raise ModuleNotFoundError("blocked by runpf: %s" % fullname)
        return None

if os.environ.get("BLOCK_MAP") == "1":
    sys.meta_path.insert(0, _BlockWebref())
elif os.environ.get("PINNED_ONLY") == "1":
    # Slice A ships `shortname_for` PINNED-MAP-ONLY; the catalog fall-through is
    # Slice B's. Measuring A's control flow against the BRANCH resolver silently
    # resolves labels A will not resolve — `CSSOM VIEW` -> `cssom-view-1` — which
    # turns the all-rows-unmapped fixture into a fully-verified one.
    sys.path.insert(0, str(Path(".claude/tools").resolve()))
    from _webref import spec_labels as _sl
    _sl.shortname_for = (lambda lab: _sl.LABEL_TO_SHORTNAME.get(lab.strip().lower())
                         if lab else None)

if os.environ.get("SPY_SUBPROCESS") == "1":
    # T-net(a) at the level the fetch happens. Asserts on the WEBREF PATH, not a
    # "webref" substring: `grep_pass` also calls `subprocess.run`, with author
    # symbols in argv, and this memo is full of the string.
    _w = str((Path(".claude/tools/webref")).resolve())
    _hits, _orig = [], subprocess.run
    def _spy(argv, *a, **k):
        if any(str(x) == _w for x in (argv or [])): _hits.append(1)
        return _orig(argv, *a, **k)
    subprocess.run = _spy
    atexit.register(lambda: sys.stderr.write("SPY webref-subprocess=%d\n" % len(_hits)))

pf = sys.argv[1]
sys.path.insert(0, str(Path(pf).resolve().parent))   # run_path() does not
sys.argv = [pf] + sys.argv[2:]
runpy.run_path(pf, run_name="__main__")
PY
}

instruments() {  # the three candidate instruments, measured on all three signals
  local T; T=$(mktemp -d); git worktree add -q "$T" HEAD
  local R; R=$(mktemp -d); _runner "$R"
  cat > "$R/probe.py" <<'PY'
import subprocess, sys
from pathlib import Path
W = Path(".claude/tools/webref").resolve()
print("    WEBREF.is_file() =", W.is_file())
sys.path.insert(0, ".claude/tools")
try:
    from _webref.spec_labels import shortname_for  # noqa: F401
    print("    map import       = OK")
except Exception as e:
    print("    map import       = FAIL:", type(e).__name__)
r = subprocess.run([sys.executable, str(W), "heading", "--exact", "html", "4.10.21"],
                   capture_output=True, text=True)
print("    CLI subprocess   = rc", r.returncode)
PY
  ( cd "$T"
    echo "  [0] intact";                     python3 "$R/probe.py"
    echo "  [a] mv _webref  (draft 7 used this)"
    mv .claude/tools/_webref _hidden; python3 "$R/probe.py"; mv _hidden .claude/tools/_webref
    echo "  [b] sys.meta_path block  -> the map axis"
    BLOCK_MAP=1 python3 "$R/runpf.py" "$R/probe.py"
    echo "  [c] mv the webref shim   -> the CLI axis"
    mv .claude/tools/webref _shim; python3 "$R/probe.py"; mv _shim .claude/tools/webref )
  git worktree remove --force "$T"; rm -rf "$R"
}

reloadstale() {  # §4.2.4 — an except-arm global survives a SUCCEEDING reload
  local T; T=$(mktemp -d); git worktree add -q "$T" HEAD
  local R; R=$(mktemp -d); _runner "$R"
  cat > "$T/_reload_probe.py" <<'PY'
import importlib, sys
from pathlib import Path
sys.path.insert(0, str(Path(".claude/skills/elidex-plan-review").resolve()))

class _Block:
    def find_spec(self, fullname, path=None, target=None):
        if fullname.startswith("_webref"):
            raise ModuleNotFoundError(fullname)
        return None

# Shape only: `_err` assigned solely in the except arm vs initialised first.
for label, prefix in (("except-arm only", ""), ("initialised first", "_err = None\n")):
    g = {}
    exec(prefix + "try:\n raise ImportError('boom')\nexcept Exception as e:\n _v=None; _err=e\n", g)
    first = repr(g.get("_err"))
    exec(prefix + "try:\n _v=1\nexcept Exception as e:\n _v=None; _err=e\n", g)
    print(f"  {label:18s} after fail={first}  after reload={g.get('_err')!r}")
PY
  ( cd "$T" && python3 _reload_probe.py )
  git worktree remove --force "$T"; rm -rf "$R"
}

remedies() {  # §4.2.4 / P5 — which remedy strings co-print when the map is absent
  local T; T=$(mktemp -d); git worktree add -q "$T" HEAD
  local R; R=$(mktemp -d); _runner "$R"
  local F; F=$(mktemp -d); fixtures "$F" >/dev/null
  echo "-- the carve, map absent (in-process block; tree and CLI intact) --"
  ( cd "$T" && BLOCK_MAP=1 python3 "$R/runpf.py" "$PF" --no-grep-pass "$F/labelled.md" 2>&1 |
      grep -E 'unrecognized|extend|SPECS|unmapped|citation verify' )
  ( cd "$T" && BLOCK_MAP=1 python3 "$R/runpf.py" "$PF" --no-grep-pass "$F/labelled.md" \
      >/dev/null 2>&1; echo "EXIT=$?" )
  git worktree remove --force "$T"; rm -rf "$F" "$R"
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
# implementation PR lands this control flow; nothing here is shipped.
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
    # are not "unmapped" - the mapper never ran. The counter states its basis.
    if unavailable:
        print(f"  unclassified rows:    {unmapped_rows}  (label map unavailable)")
    else:
        print(f"  unmapped-label rows:  {unmapped_rows}")
    basis = f" ({unmapped_rows} of {M} counted by label spelling)" if unmapped_rows else ""
    print(f"  unique specs (K):     {K}{basis}")

    # §4.2.4: four remedies, each for its own cause and no other.
    if unrecognized_labels and not unavailable:
        print(f"  remedy1 unrecognized: {sorted(set(unrecognized_labels))}", file=sys.stderr)
    if labelless_rows and not unavailable:
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

armmatrix() {  # §4.2.3 item 5 / §5 — every row, every capability state, 3 predicates
  local T; T=$(mktemp -d); git worktree add -q "$T" HEAD
  local R; R=$(mktemp -d); _runner "$R"
  local F; F=$(mktemp -d); fixtures "$F" >/dev/null
  _proto "$T"
  local PROTO="${PF%/*}/preflight_proto.py"
  _row() {  # $1=label $2=state $3=fixture $4...=flags
    local lbl=$1 st=$2 fx=$3; shift 3
    local moved=0 blk=0 out rc
    case "$st" in
      nocli)   mv "$T/.claude/tools/webref" "$R/.shim"; moved=1 ;;
      nomap)   blk=1 ;;
      neither) mv "$T/.claude/tools/webref" "$R/.shim"; moved=1; blk=1 ;;
    esac
    if [ "$blk" = 1 ]; then
      out=$( cd "$T" && SPY_SUBPROCESS=1 BLOCK_MAP=1 python3 "$R/runpf.py" "$PROTO" \
               --no-grep-pass "$@" "$F/$fx.md" 2>&1 ); rc=$?
    else
      out=$( cd "$T" && SPY_SUBPROCESS=1 PINNED_ONLY=1 python3 "$R/runpf.py" "$PROTO" \
               --no-grep-pass "$@" "$F/$fx.md" 2>&1 ); rc=$?
    fi
    [ "$moved" = 1 ] && mv "$R/.shim" "$T/.claude/tools/webref"
    printf '%-4s %-8s %-18s %-12s EXIT=%d  %s  %s  %s\n' "$lbl" "$st" "$fx" "$*" "$rc" \
      "$(echo "$out" | grep -o 'PROTO-ARM.*' || echo 'PROTO-ARM  n/a  (marker path)')" \
      "$(echo "$out" | grep -o 'SPY webref-subprocess=[0-9]*')" \
      "$(echo "$out" | grep -oE 'citation verify: +.*|HARD FAIL - [a-z§ ]*' | head -1)"
  }
  echo "row  state    fixture            flags        exit  predicates                        webref-subprocess  verdict"
  _row 1   both    labelled;            _row 2   both    labelled --no-verify
  _row 2b  both    dedup;               _row 3   nocli   labelled
  _row 4   nocli   unlabelled;          _row 5   nocli   labelled --no-verify
  _row 6   nomap   labelled;            _row 7   nomap   unlabelled
  _row 8   nomap   labelled --no-verify; _row 9  neither labelled
  _row 10  both    alias;               _row 11  both    allunmapped
  _row 11b both    unlabelled;          _row 12  both    nospec
  _row 12b both    nospec-and-header;   _row 13  both    nospec-and-table
  _row 14  nomap   nospec;              _row 15  both    fenced-marker
  echo "-- states §5 does not tabulate, checked for a fourth predicate divergence --"
  _row x1  nocli   allunmapped;         _row x2  nomap   allunmapped
  _row x3  nocli   nospec;              _row x4  neither unlabelled
  _row x5  nocli   dedup --no-verify;   _row x6  both    unlabelled --no-verify
  _row x7  both    allunmapped --no-verify
  git worktree remove --force "$T"; rm -rf "$F" "$R"
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
  echo "-- origin/main base, the touch set --"
  for f in "$PF" .claude/tools/_webref/commands/coverage_map.py .claude/tools/_webref/cli.py \
           .claude/tools/_webref/DESIGN.md mise.toml .github/workflows/ci.yml; do
    echo "$(git show "$MAIN:$f" | wc -l) $f"; done
  echo "-- on this branch --"
  echo "$(wc -l < docs/plans/2026-07-citation-hygiene-A-enforcement-plumbing.md) A's memo"
  echo "$(wc -l < docs/plans/2026-07-citation-hygiene-A-rederive.sh) the re-derivation harness"
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

all() { for f in citations partition keysets column carvecolumn instruments remedies reloadstale armmatrix suites \
                 anchors regions offline couplings suiteset marker budget filters ruleset timing \
                 lanes bmemo staleclaims; do
          say "$f"; "$f"; done; }

"${1:-all}"
