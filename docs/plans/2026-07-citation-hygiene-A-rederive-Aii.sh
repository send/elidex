# Slice A-ii's part of the re-derivation harness
# (`…-Aii-gate-failure-semantics.md`) — sourced by
# `2026-07-citation-hygiene-A-rederive.sh`, the only entry point.
#
# Blocks A-ii cites and no other memo does, plus `_runner` (whose four callers
# are all here), `anchors` (7 preflight symbols: 26 hits in A-ii, 1 in A-i) and
# `timing` (the CLI-subprocess axis §4.2.1 instruments). A-ii's shared blocks --
# `citations`, `couplings`, `budget`, `lanes` -- and `_proto`, which `budget`
# also calls, are in `-common.sh`.

column() {  # §5 — the origin/main column, every fixture shape, BOTH CLI states
  # The "map" axis does not exist on origin/main (a module-local dict with no
  # import to fail), which is why §5's rows 6-9/14 read n/a there. The CLI axis
  # does exist, and rows 3/4/5 need it — draft 6's `column` never varied it.
  # A worktree that was not created is not a measurement of `origin/main`: every
  # row below would print an `EXIT=` for a `preflight.py` that is not there, and
  # the block would still exit 0 on the status of its own cleanup.
  # `preflight.py`'s `main()` returns 0 or 1 and nothing else, so an EXIT of 2 or
  # more is not a row of §5's column -- it is python failing to run the gate at
  # all (no such file, argparse abort, missing interpreter), printed in the same
  # shape as a real reading -- and the block could not report it even once
  # noticed, because it ended on `rm -rf`.
  local failed=0
  local T; T=$(mktemp -d)
  git worktree add -q "$T" "$MAIN" || { echo "!! cannot create the $MAIN worktree"; return 1; }
  local F; F=$(mktemp -d); fixtures "$F" >/dev/null || { echo "!! fixtures failed"; return 1; }
  for st in both nocli; do
    [ "$st" = nocli ] && mv "$T/.claude/tools/webref" "$T/.shim"
    for f in labelled dedup unlabelled allunmapped alias nospec nospec-and-table nospec-and-header fenced-marker; do
      local out rc
      out=$( cd "$T" && python3 "$PF" --no-grep-pass "$F/$f.md" 2>&1 ); rc=$?
      printf '%-8s %-18s EXIT=%d  %s\n' "$st" "$f" "$rc" \
        "$(echo "$out" | grep -oE 'citation verify: +.*|HARD FAIL — [^.]*' | head -1)"
      [ "$rc" -le 1 ] || { echo "       !! EXIT=$rc is not a verdict — the gate did not RUN on this row."
                           failed=1; }
    done
    [ "$st" = nocli ] && mv "$T/.shim" "$T/.claude/tools/webref"
  done
  git worktree remove --force "$T"; rm -rf "$F"
  return "$failed"
}

carvecolumn() {  # the same fixtures at the carve — what §12(2)'s red-check can detect
  # ALL nine, not three: §6's "fails at the carve?" column is only checkable
  # against the carve's exit code, and draft 7 asserted "yes" for a fixture the
  # carve already exits 0 on (the fenced marker is inert prose there, so the
  # behavioural half of that pin passes at the carve BY ACCIDENT).
  # Same `EXIT >= 2` reading as `column`, and the same reason this block could not
  # act on it: it ended on `rm -rf`.
  local failed=0
  local F; F=$(mktemp -d); fixtures "$F" >/dev/null || { echo "!! fixtures failed"; return 1; }
  for f in labelled dedup unlabelled allunmapped alias nospec nospec-and-table nospec-and-header fenced-marker; do
    local out rc
    out=$(python3 "$PF" --no-grep-pass "$F/$f.md" 2>&1); rc=$?
    printf '%-18s EXIT=%d  %s\n' "$f" "$rc" \
      "$(echo "$out" | grep -oE 'citation verify: +.*|HARD FAIL — [^.]*|⚠ unrecognized.*' | head -1)"
    [ "$rc" -le 1 ] || { echo "   !! EXIT=$rc is not a verdict — the carve's gate did not RUN here."
                         failed=1; }
  done
  rm -rf "$F"
  return "$failed"
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
  local T; T=$(mktemp -d)
  git worktree add -q "$T" HEAD || { echo "!! cannot create the HEAD worktree"; return 1; }
  local R; R=$(mktemp -d); _runner "$R" || { echo "!! _runner failed"; return 1; }
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
  # `probe.py` CATCHES the import failure it is measuring and prints it, so a
  # nonzero status from it means python did not run the probe at all -- the whole
  # instrument reading is then missing, not negative. Both the subshell (which
  # ended on a restoring `mv`) and this function (which ended on `rm -rf`) threw
  # that away: four probes could each fail to start and the block still exited 0.
  local rc=0
  ( cd "$T" || exit 1
    r=0
    echo "  [0] intact";                     python3 "$R/probe.py" || r=1
    echo "  [a] mv _webref  (draft 7 used this)"
    mv .claude/tools/_webref _hidden; python3 "$R/probe.py" || r=1; mv _hidden .claude/tools/_webref
    echo "  [b] sys.meta_path block  -> the map axis"
    BLOCK_MAP=1 python3 "$R/runpf.py" "$R/probe.py" || r=1
    echo "  [c] mv the webref shim   -> the CLI axis"
    mv .claude/tools/webref _shim; python3 "$R/probe.py" || r=1; mv _shim .claude/tools/webref
    [ "$r" = 0 ] || echo "  !! a probe did not RUN; the axis it names is unmeasured, not negative."
    exit "$r" ) || rc=1
  git worktree remove --force "$T"; rm -rf "$R"
  return "$rc"
}

reloadstale() {  # §4.2.4 — an except-arm global survives a SUCCEEDING reload
  local T; T=$(mktemp -d)
  git worktree add -q "$T" HEAD || { echo "!! cannot create the HEAD worktree"; return 1; }
  local R; R=$(mktemp -d); _runner "$R" || { echo "!! _runner failed"; return 1; }
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
  # The probe is the whole block; its status was discarded and the function
  # returned `rm -rf`'s.
  local rc=0
  ( cd "$T" && python3 _reload_probe.py ) || rc=1
  git worktree remove --force "$T"; rm -rf "$R"
  return "$rc"
}

remedies() {  # §4.2.4 / P5 — which remedy strings co-print when the map is absent
  local T; T=$(mktemp -d)
  git worktree add -q "$T" HEAD || { echo "!! cannot create the HEAD worktree"; return 1; }
  local R; R=$(mktemp -d); _runner "$R" || { echo "!! _runner failed"; return 1; }
  local F; F=$(mktemp -d); fixtures "$F" >/dev/null || { echo "!! fixtures failed"; return 1; }
  # ONE run, printed AND graded. This was two invocations of the same command --
  # one for the remedy strings, one thrown away for its exit code -- the shape
  # `couplings` had to be cured of, in which the listing and the status can come
  # from different runs and neither is the block's. The carve's gate returns 0 or
  # 1; anything else is python not running it, and the remedy strings above would
  # then be absent for a reason that is not "the remedy did not print".
  local rc=0 out pfrc
  echo "-- the carve, map absent (in-process block; tree and CLI intact) --"
  out=$( cd "$T" && BLOCK_MAP=1 python3 "$R/runpf.py" "$PF" --no-grep-pass "$F/labelled.md" 2>&1 )
  pfrc=$?
  printf '%s\n' "$out" | grep -E 'unrecognized|extend|SPECS|unmapped|citation verify'
  echo "EXIT=$pfrc"
  [ "$pfrc" -le 1 ] || { echo "!! EXIT=$pfrc — the gate did not RUN; no remedy string above was measured."
                         rc=1; }
  git worktree remove --force "$T"; rm -rf "$F" "$R"
  return "$rc"
}

armmatrix() {  # §4.2.3 item 5 / §5 — every row, every capability state, 3 predicates
  local _n=0 _tab=0 rc=0
  local T; T=$(mktemp -d)
  git worktree add -q "$T" HEAD || { echo "!! cannot create the HEAD worktree"; return 1; }
  local R; R=$(mktemp -d); _runner "$R" || { echo "!! _runner failed"; return 1; }
  local F; F=$(mktemp -d); fixtures "$F" >/dev/null || { echo "!! fixtures failed"; return 1; }
  # Without the graft there is no §4.2.3 control flow to run the matrix against,
  # and every row below would be an `EXIT=` from a file that does not exist.
  _proto "$T" || { echo "!! _proto failed — there is no grafted control flow to measure"; \
                   git worktree remove --force "$T"; rm -rf "$F" "$R"; return 1; }
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
    _n=$((_n + 1)); case "$lbl" in x*) ;; *) _tab=$((_tab + 1)) ;; esac
    printf '%-4s %-8s %-18s %-12s EXIT=%d\n' "$lbl" "$st" "$fx" "$*" "$rc"
    # Print every line the memo cites. Draft 8's filter dropped `remedy*` and had
    # no `PROTO-DISPLAY`, so two sections cited a block that did not emit their
    # claim -- the same defect class one level down.
    echo "$out" | grep -oE 'PROTO-(ARM|DISPLAY) .*|SPY webref-subprocess=[0-9]+|remedy[0-9][a-z -]*|citation verify: +.*|(unclassified|unknown-label|label-less) rows: +[0-9]+|unique specs \(K\): +.*|HARD FAIL - [^.]*' |
      sed 's/^/       /'
  }
  echo "row  state    fixture            flags        exit"
  _row 1   both    labelled;            _row 2   both    labelled --no-verify
  _row 2b  both    dedup;               _row 3   nocli   labelled
  _row 4   nocli   unlabelled;          _row 5   nocli   labelled --no-verify
  _row 6   nomap   labelled;            _row 7   nomap   unlabelled
  _row 8   nomap   labelled --no-verify; _row 9  neither labelled
  _row 10  both    alias;               _row 11  both    allunmapped
  _row 11b both    unlabelled;          _row 12  both    nospec
  _row 12b both    nospec-and-header;   _row 13  both    nospec-and-table
  _row 14  nomap   nospec;              _row 15  both    fenced-marker
  _row 16  both    malformed
  echo "-- states §5 does not tabulate, checked for a further predicate divergence --"
  _row x1  nocli   allunmapped;         _row x2  nomap   allunmapped
  _row x3  nocli   nospec;              _row x4  neither unlabelled
  _row x5  nocli   dedup --no-verify;   _row x6  both    unlabelled --no-verify
  _row x7  both    allunmapped --no-verify; _row x8 nomap malformed
  # The memo may not hand-carry these. Draft 8 said "24 states / 17 §5 rows /
  # 20 other states"; measured they were 25 / 18 / 21.
  echo
  echo "STATES total=$_n  §5-tabulated=$_tab  untabulated=$((_n - _tab))"
  local n_fx; _measure n_fx ls "$F" || rc=1
  echo "  (hand-picked cells, NOT a cross-product: 4 capability states x $n_fx fixtures x 2 modes would be far more)"
  git worktree remove --force "$T"; rm -rf "$F" "$R"
  return "$rc"
}

anchors() {  # §3.1 / §4.2 — origin/main by symbol, never by stored line number
  # The pipeline IS the measurement, so its status is this block's: `pipefail`
  # carries an unresolvable ref out of `git show`, and grep's 1 -- no anchor found
  # -- is a real failure here rather than a benign empty result, because §3.1
  # cites these symbols as present. Stated with an explicit `return` so the
  # block's status is a claim rather than a leftover.
  git show "$MAIN:$PF" | grep -n \
    'SECTION_REF_RE\|^def parse_spec_cell\|^def shortname_from_label\|^def verify_citation\|dest="grep_pass"\|unique_specs\|seen_pairs\|elif seen_pairs\|HARD FAIL'
  return $?
}

marker() {  # §4.2.5 residual — the census must implement the SAME three properties
  # the gate does, or the mitigation is the looser grep the memo denies it is:
  # line-anchored AND fence-aware AND §3-scoped. A bare grep is only the first.
  python3 - <<'MARKERPY'
import re, subprocess, sys
sys.path.insert(0, ".claude/skills/elidex-plan-review")
from preflight import _fence_state_array, find_coverage_map_section
MARKER = re.compile(r"^\s*\*\*No spec surface\*\*")
# A census over a file list that was never produced reports 0 markers, which is
# also what "no markers" reports -- `_measure`'s inference bug, in python.
_ls = subprocess.run(["git", "ls-files", "docs/plans/"], capture_output=True, text=True)
if _ls.returncode != 0:
    sys.stderr.write(_ls.stderr)
    raise SystemExit("!! `git ls-files docs/plans/` failed (rc=%d); the counts below "
                     "would read 0 for a reason that is not 'no markers'." % _ls.returncode)
files = _ls.stdout.split()
hits = loose = 0
for f in files:
    try: lines = open(f, encoding="utf-8").read().splitlines()
    except OSError: continue
    raw = [i for i, l in enumerate(lines) if MARKER.match(l)]
    loose += len(raw)
    if not raw: continue
    fence = _fence_state_array(lines)
    sec = find_coverage_map_section(lines, fence)
    if sec is None: continue
    _, start, end = sec
    real = [i for i in raw if not fence[i] and start <= i < end]
    for i in real: print(f"  {f}:{i+1}")
    hits += len(real)
print(f"  recognised (line-anchored + fence-aware + §3-scoped): {hits}")
print(f"  a bare line-anchored grep would report               : {loose}")
MARKERPY
  return $?    # the heredoc'd command IS the measurement; say so
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
    # An invocation that dies on startup is fast, and its speed is not the
    # subprocess cost §11's ratio is about.
    r = subprocess.run([sys.executable, W, "heading", "--exact", "html", "4.10.21"],
                       capture_output=True)
    if r.returncode != 0:
        sys.stderr.write(r.stderr.decode(errors="replace"))
        raise SystemExit("!! the webref CLI exited %d; a failed invocation is not a "
                         "timing of a successful one." % r.returncode)
sub = (time.perf_counter() - t) / 10
print(f"subprocess={sub:.4f}s  in-process={inp:.6f}s  ratio={sub/inp:.0f}x")
PY
  return $?    # the heredoc'd command IS the measurement; say so
}
