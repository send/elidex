# Shared part of the re-derivation harness — sourced by
# `2026-07-citation-hygiene-A-rederive.sh`, which is the only entry point.
# Not executable on its own: it defines no dispatch and sets no shell options,
# and it reads `$REPO_ROOT` and `_measure` from `-integrity.sh`, which the
# dispatcher sources first.
#
# On this branch (slice A-iii, which lands before A-ii) the harness carries
# only what A-iii's memo cites. What lives here: the shared plumbing (`$MAIN`,
# `say`, `$AUTHOR_LOCAL`) and the blocks other memos of the program cite too --
# `suites` (umbrella), `budget` (every slice), `lanes` (umbrella; author-local).
# The A-i / A-ii / B parts and their common blocks return when PR #514 is
# reworked onto this harness after PR #501 lands.

MAIN=origin/main
say() { printf '\n=== %s ===\n' "$1"; }

# --- suites --------------------------------------------------------------------
# A-iii §1 / §4.2 / §4.3: what `scripts/python-suites.sh` WILL run, run now at
# $MAIN by the same procedure, so the memo's figures are this block's output.
#
# THE PROCEDURE (A-iii §4.2, decision D2): the suite set is the L2 set
# `git ls-files ':(glob).claude/**/test_*.py'`; the roots are its unique
# dirnames; each root runs in ITS OWN interpreter, loading exactly the set
# members whose dirname is that root as top-level modules (an explicit module
# list -- the loader never walks a directory, so no root re-runs another's
# suites); a root that runs zero tests, or an empty set, is a failure.
#
# The interpreter is `${REDERIVE_PY:-python3}` so the same block measures the
# floor locally (`REDERIVE_PY=/usr/bin/python3` on a machine whose system
# python is 3.9). Every status is read: a failing suite, a zero-test root, an
# unmeasured or nonzero `urlopen` count each make the block RED.
suites() {
  local T rc=0 n py=${REDERIVE_PY:-python3}
  T=$(mktemp -d)
  git worktree add -q "$T" "$MAIN" || { echo "!! cannot create the $MAIN worktree"; return 1; }
  _measure n git -C "$T" ls-files ':(glob).claude/**/test_*.py' || rc=1
  echo "$n suite files at $MAIN"
  # A-iii §1 says FOUR. The figure has a lifetime -- #501's `test_spec_labels.py`
  # makes it five -- and when it moves, this line is what says so.
  [ "$n" = 4 ] || { echo "!! $n suite files at $MAIN; A-iii §1 says 4 — the memo's figure has moved"; rc=1; }
  local files=$_MEASURE_OUT ns
  # A-iii §4.3: no suite can fetch through a child -- none imports `subprocess`
  # or names the `webref` entry script. A parent-process urlopen spy (below)
  # cannot see a child, so this is the half that covers it.
  # shellcheck disable=SC2086  # the file list is word-split on purpose (no spaces in tracked suite paths)
  _measure --nomatch 1 ns git -C "$T" grep -nE '\bsubprocess\b|tools/webref' -- $files || rc=1
  _measured
  [ "$ns" = 0 ] || { echo "!! a suite at $MAIN can spawn a child — §4.3's no-child premise does not hold"; rc=1; }
  "$py" -VV || rc=1
  "$py" - "$T" "$files" <<'SUITESPY' || rc=1
import os, re, subprocess, sys
t, files = sys.argv[1], sys.argv[2].split()
if not files:
    raise SystemExit("!! empty suite set -- nothing to run is not a pass")
roots = sorted({os.path.dirname(f) for f in files})
# The child: urlopen spy first, then the root's own modules by name, nothing else.
CHILD = r"""
import importlib, sys, unittest, urllib.request
_c = []; _o = urllib.request.urlopen
urllib.request.urlopen = lambda r, *a, **k: (_c.append(r), _o(r, *a, **k))[1]
root, mods = sys.argv[1], sys.argv[2:]
sys.path.insert(0, root)
s = unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromModule(importlib.import_module(m)) for m in mods)
r = unittest.TextTestRunner(verbosity=0).run(s)
sys.stderr.write("RAN=%d URLOPEN=%d\n" % (r.testsRun, len(_c)))
sys.exit(0 if r.wasSuccessful() else 1)
"""
rc = 0
for root in roots:
    mods = [os.path.basename(f)[:-3] for f in files if os.path.dirname(f) == root]
    r = subprocess.run([sys.executable, "-c", CHILD, os.path.join(t, root)] + mods,
                       capture_output=True, text=True)
    m = re.search(r"RAN=(\d+) URLOPEN=(\d+)", r.stderr)
    print(f"  {root}: {len(mods)} file(s), rc={r.returncode}, "
          + (f"{m.group(1)} tests, urlopen={m.group(2)}" if m else "UNMEASURED"))
    if r.returncode != 0 or m is None:
        rc = 1; print(f"!! {root}: the run did not pass"); sys.stdout.flush(); sys.stderr.write(r.stderr)
    elif m.group(1) == "0":
        rc = 1; print(f"!! {root}: zero tests ran -- an uncollected root, L2's case")
    elif m.group(2) != "0":
        rc = 1; print(f"!! {root}: the suites fetch -- L4's 0-urlopen claim does not hold")
sys.exit(rc)
SUITESPY
  # The cleanup must not BE the answer.
  git worktree remove --force "$T"
  return "$rc"
}

budget() {  # A-iii §8 — every size it cites
  local failed=0 n f
  echo "-- $MAIN, the files A-iii edits --"
  for f in mise.toml .github/workflows/ci.yml; do
    _measure n git show "$MAIN:$f" || failed=1
    echo "$n $f"; done
  echo "-- on this branch --"
  f=docs/plans/2026-07-citation-hygiene-Aiii-suite-scheduler.md
  [ -f "$f" ] || { echo "!! expected memo missing: $f"; failed=1; }
  [ -f "$f" ] && { _measure n cat "$f" || failed=1; echo "$n ${f##*/}"; }
  for f in docs/plans/2026-07-citation-hygiene-A-rederive*.sh; do
    _measure n cat "$f" || failed=1
    echo "$n the re-derivation harness — ${f##*/2026-07-citation-hygiene-A-rederive}"
  done
  return "$failed"
}

lanes() {  # A-iii §13 — base, open PRs, worktrees touching what A-iii contends on
  local failed=0 n m w wlist
  git rev-list --left-right --count "$MAIN"...HEAD || failed=1
  # `gh pr list` returns 30 items by default; an open PR past the first page is
  # a contending lane this roster would silently omit (Codex R28).
  gh pr list --state open --limit 1000 --json number,headRefName --jq '.[] | "\(.number) \(.headRefName)"' || failed=1
  # The worktree listing is taken ONCE and its status observed (Codex R23).
  wlist=$(git worktree list --porcelain | sed -n 's/^worktree //p'; exit "${PIPESTATUS[0]}") \
    || { echo "!! git worktree list failed — no worktree census was taken"; failed=1; wlist=""; }
  [ -n "$wlist" ] || { echo "!! worktree listing is empty — this worktree itself should be listed"; failed=1; }
  # The contention is CI topology: `gh pr list` misses an unpushed branch, and a
  # docs/plans/ filter misses a branch whose collision is in ci.yml / mise.toml
  # (how the merged memo missed the Layout lane's #496). Committed range AND
  # working tree (Codex R25).
  echo "-- worktrees touching ci.yml / mise.toml / scripts/ / .claude/**/test_*.py --"
  local P=(.github/workflows/ mise.toml scripts/ ':(glob).claude/**/test_*.py')
  while IFS= read -r w; do   # whole path, whitespace-safe (Codex R21)
    git -C "$w" rev-parse --git-dir >/dev/null 2>&1 || { echo "  (prunable, skipped: $w)"; continue; }
    local committed
    if _measure n git -C "$w" diff --name-only "$MAIN"...HEAD -- "${P[@]}" \
       && committed=$_MEASURE_OUT \
       && _measure m git -C "$w" status --porcelain --untracked-files=all -- "${P[@]}"; then
      [ $((n + m)) -gt 0 ] && { echo "  $w  [$(git -C "$w" rev-parse --short HEAD)]"
                                printf '%s%s' "$committed" "$_MEASURE_OUT" | sed 's/^/      /'; }
    else
      echo "  !! $w — NOT MEASURED ($n); silence here is not 'no contention'"; failed=1
    fi
  done <<< "$wlist"
  return "$failed"
}

# AUTHOR-LOCAL: reaches sibling worktrees and the PR list, so it cannot run for
# a second reader. `all` excludes it; run it by name.
AUTHOR_LOCAL="lanes"
