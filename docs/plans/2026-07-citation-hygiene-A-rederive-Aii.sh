# Slice A-ii's part of the re-derivation harness
# (`…-Aii-gate-failure-semantics.md`) — sourced by
# `2026-07-citation-hygiene-A-rederive.sh`, the only entry point.
#
# Blocks A-ii cites and no other memo does, plus `_runner` (whose four callers
# are all here), `anchors` (7 preflight symbols: 26 hits in A-ii, 1 in A-i) and
# `timing` (the CLI-subprocess axis §4.2.1 instruments). A-ii's shared blocks --
# `citations`, `couplings`, `budget`, `lanes` -- and `_proto`, which `budget`
# also calls, are in `-common.sh`.

# A gate VERDICT is a line the gate prints, not an exit status. `preflight.py`'s
# `main()` returns 0 or 1 -- but python also exits 1 on an unhandled exception
# (import error, parse error, traceback), so `rc <= 1` certifies a crash as a
# reading. Codex R3 reproduced it: `python3` shadowed by a process that prints a
# traceback and exits 1 gave 18 rows of `EXIT=1` and `column` returned 0. The
# verdict vocabulary is read off the gate's RETURN SITES, not off sample output
# (a first draft keyed on `citation verify:` and flagged every passing fixture
# with zero parsed citations; a second keyed on `split decision:` and flagged
# every `--no-verify` row of the graft, which prints neither). THREE gates are
# graded here -- `origin/main`'s `preflight.py` (`column`), this branch's
# (`carvecolumn`, `remedies`) and the grafted proto (`_proto`, A-ii's prototype;
# `armmatrix`) -- and at every one of them the `§3 Spec coverage map preflight`
# header is printed on every path that reaches `return 0` and on no path that
# returns 1 before it, and every verdict `return 1` follows a `preflight: HARD
# FAIL` print (em dash in the shipped gates, hyphen in the proto). The
# plan-memo-not-found path prints `preflight: plan-memo not found` (shipped) or
# nothing (proto); neither is a verdict and neither matches. That the three
# vocabularies agree is not assumed here: `column` grades `origin/main`'s gate
# with this predicate on 18 rows, so a divergence shows as a loud RED, never as
# green. Output with neither line is not a verdict, whatever the status says.
# One predicate, four callers (`column`, `carvecolumn`, `remedies`,
# `armmatrix`'s `_row`) -- the exit-code-only test had three copies and `_row`
# had none (Codex R4).
# The gate verifies a citation through `webref heading`, which serves from a
# cache and, cold, from the network. A block that runs the gate on the mapped
# fixture rows therefore has a PRECONDITION -- the lookup the rows make can run
# here -- and a cold cache or no network makes every verified row a HARD FAIL
# that is about the environment, not the gate (Codex R12). Measured once, loud.
_webref_warm() {
  local n
  _measure n .claude/tools/webref heading --exact html 4.10.21 \
    || { echo "!! webref lookup unavailable (cold cache / offline) — the baseline rows are NOT MEASURABLE here"; return 1; }
  return 0
}

_verdict() {  # _verdict <rc> <output> — 0 iff the gate RAN and printed a verdict
  case "$1" in
    0) printf '%s\n' "$2" | grep -q 'Spec coverage map preflight' ;;
    1) printf '%s\n' "$2" | grep -qE 'preflight: (❌ )?HARD FAIL' ;;
    *) return 1 ;;
  esac
}

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
  # The rows verify citations through `webref`, which reads a cache and, cold,
  # the network: a cold cache turns row 1 into `EXIT=1` with a well-formed
  # HARD FAIL -- a verdict about the environment, certified as the gate's
  # (Codex R12). The precondition is measured once, before any row: if the
  # lookup cannot run, the baseline is NOT MEASURABLE here, which is a failed
  # measurement rather than a reading.
  _webref_warm || { git worktree remove --force "$T"; rm -rf "$F"; return 1; }
  # Every cell §5's `origin/main` column tabulates is asserted, exit and all
  # (`_verdict` alone accepts ANY well-formed verdict -- Codex R12); the
  # `--no-verify` rows (2, 5) and `malformed.md` (16) are run too, which this
  # loop never did. A cell §5 does not tabulate carries no claim.
  local f mode st
  for st in both nocli; do
    [ "$st" = nocli ] && mv "$T/.claude/tools/webref" "$T/.shim"
    for f in labelled labelled:--no-verify dedup unlabelled allunmapped alias nospec nospec-and-table nospec-and-header fenced-marker malformed; do
      mode=""; case "$f" in *:*) mode=${f#*:}; f=${f%%:*} ;; esac
      local out rc want
      out=$( cd "$T" && python3 "$PF" --no-grep-pass $mode "$F/$f.md" 2>&1 ); rc=$?
      printf '%-8s %-18s %-12s EXIT=%d  %s\n' "$st" "$f" "$mode" "$rc" \
        "$(echo "$out" | grep -oE 'citation verify: +.*|HARD FAIL — [^.]*' | head -1)"
      _verdict "$rc" "$out" || { echo "       !! EXIT=$rc with no verdict line — the gate did not RUN on this row."
                                 failed=1; }
      case "$st/$f/$mode" in                       # §5 `origin/main` column, by row
        both/labelled/)            want=0 ;;  # 1
        both/labelled/--no-verify) want=0 ;;  # 2
        both/dedup/)               want=0 ;;  # 2b
        nocli/labelled/)           want=1 ;;  # 3
        nocli/unlabelled/)         want=0 ;;  # 4
        nocli/labelled/--no-verify) want=0 ;; # 5
        both/alias/)               want=0 ;;  # 10
        both/allunmapped/)         want=0 ;;  # 11
        both/unlabelled/)          want=0 ;;  # 11b
        both/nospec/)              want=1 ;;  # 12
        both/nospec-and-header/)   want=1 ;;  # 12b
        both/nospec-and-table/)    want=0 ;;  # 13
        both/fenced-marker/)       want=0 ;;  # 15
        both/malformed/)           want=1 ;;  # 16
        *)                         want="" ;;
      esac
      [ -z "$want" ] || [ "$rc" = "$want" ] || { echo "       !! EXIT=$rc, §5's origin/main column says $want"; failed=1; }
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
  # EXPECTED status AND mechanism per fixture: these are the readings A-ii §6
  # asserts "fails at A-i's head" AGAINST, so accepting any well-formed verdict
  # (Codex R19) could not substantiate that column. `fenced-marker` exiting 0
  # with the table verified IS P11d's premise (the marker is inert prose here).
  local spec
  for spec in 'labelled:0:citation verify: +ok' 'dedup:0:citation verify: +ok' \
              'unlabelled:0:unrecognized labels' 'allunmapped:0:unrecognized labels' \
              'alias:0:unrecognized labels' 'nospec:1:no markdown table follows' \
              'nospec-and-table:0:citation verify: +ok' 'nospec-and-header:1:has 0 data rows' \
              'fenced-marker:0:citation verify: +ok'; do
    local f want_rc want_re out rc
    f=${spec%%:*}; spec=${spec#*:}; want_rc=${spec%%:*}; want_re=${spec#*:}
    out=$(python3 "$PF" --no-grep-pass "$F/$f.md" 2>&1); rc=$?
    printf '%-18s EXIT=%d  %s\n' "$f" "$rc" \
      "$(echo "$out" | grep -oE 'citation verify: +.*|HARD FAIL — [^.]*|⚠ unrecognized.*' | head -1)"
    _verdict "$rc" "$out" || { echo "   !! EXIT=$rc with no verdict line — the carve's gate did not RUN here."
                               failed=1; }
    [ "$rc" -eq "$want_rc" ] || { echo "   !! expected EXIT=$want_rc at the carve — A-ii §6's premise for this fixture no longer holds"; failed=1; }
    echo "$out" | grep -qE "$want_re" || { echo "   !! expected mechanism /$want_re/ not in the carve's output"; failed=1; }
  done
  # P11e's premise, at the carve: a no-spec memo with a bad `crates/…` path and
  # grep-pass ENABLED exits non-zero here too -- via the no-table hard fail --
  # and the diagnostic does NOT name the path. Every row above passes
  # `--no-grep-pass`, so this block could not observe the mechanism P11e is
  # about (Codex R15); A-ii's "fails at A-ii's head? yes, on the diagnostic" is
  # exactly this reading, asserted rather than recalled.
  printf '# fixture\n\n## §3. Spec coverage map\n\n**No spec surface** — tooling only.\n\nSee `crates/nonesuch/src/lib.rs`.\n' > "$F/nospec-badpath.md"
  local out rc
  out=$(python3 "$PF" "$F/nospec-badpath.md" 2>&1); rc=$?
  printf '%-18s EXIT=%d  (grep-pass ON)\n' "nospec-badpath" "$rc"
  # The premise is the NO-TABLE hard fail, reached with grep-pass on; a crash
  # in `run_grep_pass` also exits non-zero without naming the path (Codex R23).
  _verdict "$rc" "$out" || { echo "   !! EXIT=$rc with no verdict line — the gate did not RUN on the grep-pass fixture"; failed=1; }
  echo "$out" | grep -q 'no markdown table follows' || { echo "   !! the carve did not reach the no-table hard fail — P11e's premise is not this reading"; failed=1; }
  [ "$rc" -ne 0 ] || { echo "   !! exit 0 — the carve no longer hard-fails a no-spec memo"; failed=1; }
  if echo "$out" | grep -q 'crates/nonesuch'; then
    echo "   !! the carve NAMES the grep-pass finding — P11e would be green at the carve, its 'yes' column is stale"; failed=1
  else
    echo "   carve does not name crates/nonesuch — P11e red here, as A-ii §6 states"
  fi
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
# `--help` is the CLI axis without the network: it exercises the shim's file and
# its import of `_webref` (which is what `[a]` and `[c]` remove) and nothing a
# cold cache can turn RED (Codex R12: `heading` made the intact axis read
# invalid with upstream unavailable).
r = subprocess.run([sys.executable, str(W), "--help"], capture_output=True, text=True)
print("    CLI subprocess   = rc", r.returncode)
# One machine-readable line per probe, for the comparison below.
print("PROBE is_file=%s map=%s cli=%s" % (W.is_file(), "OK" if "shortname_for" in dir() else "FAIL",
                                          "0" if r.returncode == 0 else "nonzero"))
PY
  # `probe.py` CATCHES the import failure it is measuring and prints it, so a
  # nonzero status from it means python did not run the probe at all -- the whole
  # instrument reading is then missing, not negative. Both the subshell (which
  # ended on a restoring `mv`) and this function (which ended on `rm -rf`) threw
  # that away: four probes could each fail to start and the block still exited 0.
  # And a probe that RAN is not yet a measurement of the state it models: §4.2.1's
  # table says what each instrument must read on the three signals, and a probe
  # that prints something else -- `BLOCK_MAP` no longer blocking, the shim's
  # removal no longer breaking the CLI -- is an instrument that stopped modelling
  # its axis while the matrix built on it stayed green (Codex R8). `_probe` runs
  # one, prints it, and compares its PROBE line with §4.2.1's row for that mode.
  local rc=0
  ( cd "$T" || exit 1
    r=0
    _probe() {  # $1 = label  $2 = expected `is_file=… map=… cli=…`  $3… = command
      local lbl=$1 want=$2 out; shift 2
      echo "  $lbl"
      out=$("$@" 2>&1); local prc=$?
      printf '%s\n' "$out" | grep -v '^PROBE '
      [ "$prc" = 0 ] || { echo "  !! the probe did not RUN (exit $prc); the axis it names is unmeasured, not negative."; r=1; return; }
      local got; got=$(printf '%s\n' "$out" | grep -o '^PROBE .*' | sed 's/^PROBE //')
      [ "$got" = "$want" ] || { echo "  !! instrument reads [$got], §4.2.1 says [$want] — it no longer models its axis."; r=1; }
    }
    _probe "[0] intact"                               "is_file=True map=OK cli=0"         python3 "$R/probe.py"
    mv .claude/tools/_webref _hidden
    _probe "[a] mv _webref  (draft 7 used this)"      "is_file=True map=FAIL cli=nonzero" python3 "$R/probe.py"
    mv _hidden .claude/tools/_webref
    _probe "[b] sys.meta_path block  -> the map axis" "is_file=True map=FAIL cli=0"       env BLOCK_MAP=1 python3 "$R/runpf.py" "$R/probe.py"
    mv .claude/tools/webref _shim
    _probe "[c] mv the webref shim   -> the CLI axis" "is_file=False map=OK cli=nonzero"  python3 "$R/probe.py"
    mv _shim .claude/tools/webref
    exit "$r" ) || rc=1
  git worktree remove --force "$T"; rm -rf "$R"
  return "$rc"
}

reloadstale() {  # §4.2.4 — an except-arm global survives a SUCCEEDING reload
  local T; T=$(mktemp -d)
  git worktree add -q "$T" HEAD || { echo "!! cannot create the HEAD worktree"; return 1; }
  cat > "$T/_reload_probe.py" <<'PY'
import sys
# Shape only: `_err` assigned solely in the except arm vs initialised first.
# §4.2.4's claim is the ASYMMETRY -- the except-arm-only global keeps its stale
# error after a succeeding reload, the initialised-first one is `None` -- and a
# probe that printed both readings and exited 0 certified neither (the
# block-audit of 2026-08-22). Both halves are asserted.
after = {}
for label, prefix in (("except-arm only", ""), ("initialised first", "_err = None\n")):
    g = {}
    exec(prefix + "try:\n raise ImportError('boom')\nexcept Exception as e:\n _v=None; _err=e\n", g)
    first = repr(g.get("_err"))
    exec(prefix + "try:\n _v=1\nexcept Exception as e:\n _v=None; _err=e\n", g)
    after[label] = g.get("_err")
    print(f"  {label:18s} after fail={first}  after reload={after[label]!r}")
ok = after["except-arm only"] is not None and after["initialised first"] is None
if not ok:
    print("!! §4.2.4's asymmetry does not hold: stale-after-reload =",
          repr(after["except-arm only"]), "initialised-first =", repr(after["initialised first"]))
sys.exit(0 if ok else 1)
PY
  # The probe is the whole block; its status was discarded and the function
  # returned `rm -rf`'s.
  local rc=0
  ( cd "$T" && python3 _reload_probe.py ) || rc=1
  git worktree remove --force "$T"
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
  _verdict "$pfrc" "$out" || { echo "!! EXIT=$pfrc with no verdict line — the gate did not RUN; no remedy string above was measured."
                               rc=1; }
  # The listing above is not the claim; §4.2.1 is: at the carve the map-absent
  # case "does not exist" -- the gate still reads its module-local dict, so the
  # rows VERIFY and no wrong-cause remedy prints. Both halves are asserted, each
  # by its own grep status (a filter's status was discarded here before, Codex
  # R8): the `citation verify: ok` line must be present, the `unrecognized
  # labels` remedy must be absent. The day A-ii migrates the reader, this block
  # flips -- which is the asymmetry A-ii's §4.2.1 exists to measure.
  printf '%s\n' "$out" | grep -qE 'citation verify: +ok' \
    || { echo "!! the carve did not verify the rows with the map blocked — §4.2.1's 'does not exist' no longer holds"; rc=1; }
  if printf '%s\n' "$out" | grep -q 'unrecognized labels'; then
    echo "!! the carve printed a wrong-cause remedy with the map blocked — §4.2.1's 'does not exist' no longer holds"; rc=1
  fi
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
    local moved=0 blk=0 out prc
    case "$st" in
      nocli)   mv "$T/.claude/tools/webref" "$R/.shim"; moved=1 ;;
      nomap)   blk=1 ;;
      neither) mv "$T/.claude/tools/webref" "$R/.shim"; moved=1; blk=1 ;;
    esac
    if [ "$blk" = 1 ]; then
      out=$( cd "$T" && SPY_SUBPROCESS=1 BLOCK_MAP=1 python3 "$R/runpf.py" "$PROTO" \
               --no-grep-pass "$@" "$F/$fx.md" 2>&1 ); prc=$?
    else
      out=$( cd "$T" && SPY_SUBPROCESS=1 PINNED_ONLY=1 python3 "$R/runpf.py" "$PROTO" \
               --no-grep-pass "$@" "$F/$fx.md" 2>&1 ); prc=$?
    fi
    [ "$moved" = 1 ] && mv "$R/.shim" "$T/.claude/tools/webref"
    _n=$((_n + 1)); case "$lbl" in x*) ;; *) _tab=$((_tab + 1)) ;; esac
    printf '%-4s %-8s %-18s %-12s EXIT=%d\n' "$lbl" "$st" "$fx" "$*" "$prc"
    # The row's status is a VERDICT only if the graft printed one (`_verdict`, same
    # rule as `column`): a proto that crashed with exit 1 is a row that was never
    # measured, and it is the BLOCK's status that must say so -- `rc` here is
    # armmatrix's own, reached through the nested function's dynamic scope.
    _verdict "$prc" "$out" || { echo "       !! EXIT=$prc with no verdict line — the graft did not RUN on this row."
                                rc=1; }
    # A verdict is not yet §5's verdict: the table's "After A-ii" column states
    # an exit per tabulated row, and item 8 states the K line for every
    # map-absent row. Both were printed and never compared (Codex R9). The
    # expected exit lives here, keyed by §5's row label; an untabulated `x*`
    # row has no claim to hold.
    local want
    case "$lbl" in
      1|2|2b|5|8|10|11|11b|12|14|15) want=0 ;;
      3|4|6|7|9|12b|13|16)          want=1 ;;
      *)                             want="" ;;
    esac
    [ -z "$want" ] || [ "$prc" = "$want" ] || { echo "       !! EXIT=$prc, §5 row $lbl says $want"; rc=1; }
    # Row 15 (P11d) asserts on the MECHANISM, not the exit: a marker inside a
    # fence must not be recognised, so no `no spec surface declared` line may
    # appear -- the row fell through to "no expectation" until Codex R10.
    [ "$lbl" != 15 ] || ! printf '%s\n' "$out" | grep -q 'no spec surface declared' \
      || { echo "       !! fenced marker took the no-spec path — P11d's fence gate no longer holds"; rc=1; }
    # Item 8 is a rule about the K line, which only the table path prints (the
    # no-spec-surface path, §5 row 14, prints none): when the line is there and
    # the map is absent, it must read n/a.
    case "$st" in nomap|neither)
      if printf '%s\n' "$out" | grep -q 'unique specs (K):'; then
        printf '%s\n' "$out" | grep -q 'unique specs (K):     n/a (label map unavailable)' \
          || { echo "       !! map absent but K is a number — §4.2.3 item 8 says n/a (label map unavailable)"; rc=1; }
      fi ;;
    esac
    # Print every line the memo cites. Draft 8's filter dropped `remedy*` and had
    # no `PROTO-DISPLAY`, so two sections cited a block that did not emit their
    # claim -- the same defect class one level down.
    # A row whose instrumentation lines are MISSING is a row the memo cannot
    # cite; the filter's empty output returned 1 into nowhere (Codex R26).
    local instr
    instr=$(echo "$out" | grep -oE 'PROTO-(ARM|DISPLAY) .*|SPY webref-subprocess=[0-9]+|remedy[0-9][a-z -]*|citation verify: +.*|(unclassified|unknown-label|label-less) rows: +[0-9]+|unique specs \(K\): +.*|HARD FAIL - [^.]*')
    if [ -n "$instr" ]; then printf '%s\n' "$instr" | sed 's/^/       /'
    else echo "       !! no instrumentation line (PROTO-*/SPY/remedy/count) in this row's output — the memo cites lines this row did not emit"; rc=1; fi
    # Each signal CLASS the memo cites must be present -- "any one line" let a
    # row that stopped emitting SPY pass on its PROTO-ARM line (Codex R27):
    # the arm, the child-invocation spy, and a gate verdict, every row.
    # `PROTO-ARM` is printed by the TABLE arm; the no-spec-surface path (every
    # `nospec*` fixture) has no arm to print and says so in its verdict line.
    local sig
    local -a sigs=('SPY webref-subprocess=' 'citation verify: |HARD FAIL - |remedy[0-9]|unique specs \(K\):')
    case "$fx" in nospec*) ;; *) sigs=('PROTO-ARM ' "${sigs[@]}") ;; esac
    for sig in "${sigs[@]}"; do
      printf '%s\n' "$instr" | grep -qE "$sig" || { echo "       !! required signal class /$sig/ absent from this row"; rc=1; }
    done
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
  # The memo may not hand-carry these, and neither does this comment: draft 8
  # said "24 states / 17 §5 rows / 20 other states", a later revision of this
  # line said "25 / 18 / 21", and both were stale the next time a row was added.
  # The line below is the only statement of the totals.
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
  # EACH anchor independently: one alternation exits 0 while any single symbol
  # survives, so eight renamed anchors and one `HARD FAIL` still read as "all
  # present" (Codex R18). §3.1 cites each by name; each is asserted by name.
  local src rc=0 a
  src=$(git show "$MAIN:$PF") || { echo "!! cannot read $PF at $MAIN"; return 1; }
  for a in 'SECTION_REF_RE' '^def parse_spec_cell' '^def shortname_from_label' '^def verify_citation' \
           'dest="grep_pass"' 'unique_specs' 'seen_pairs' 'elif seen_pairs' 'HARD FAIL'; do
    if printf '%s\n' "$src" | grep -n -- "$a" | head -3; then :; else
      echo "!! anchor not found at $MAIN:$PF — $a"; rc=1; fi
  done
  return "$rc"
}

marker() {  # §4.2.5 residual — the census must implement the SAME three properties
  # the gate does, or the mitigation is the looser grep the memo denies it is:
  # line-anchored AND fence-aware AND §3-scoped. A bare grep is only the first.
  python3 - <<'MARKERPY'
import re, subprocess, sys
sys.path.insert(0, ".claude/skills/elidex-plan-review")
from preflight import _fence_state_array, find_coverage_map_section
MARKER = re.compile(r"^ {0,3}\*\*No spec surface\*\*")   # indent-gated, same rule as the proto's MARKER_RE
# A census over a file list that was never produced reports 0 markers, which is
# also what "no markers" reports -- `_measure`'s inference bug, in python.
_ls = subprocess.run(["git", "ls-files", "docs/plans/"], capture_output=True, text=True)
if _ls.returncode != 0:
    sys.stderr.write(_ls.stderr)
    raise SystemExit("!! `git ls-files docs/plans/` failed (rc=%d); the counts below "
                     "would read 0 for a reason that is not 'no markers'." % _ls.returncode)
files = _ls.stdout.split()
hits = loose = 0
unreadable = []
for f in files:
    try: lines = open(f, encoding="utf-8").read().splitlines()
    except OSError as e:
        # A tracked plan that cannot be opened is not "no markers": skipping it
        # printed partial counts and exited 0 (Codex R17). Same rule as `_wtscan`.
        unreadable.append(f"{f}: {e.strerror}"); continue
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
if unreadable:
    for u in unreadable: print(f"!! unreadable, NOT censused: {u}", file=sys.stderr)
    sys.exit(2)
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
