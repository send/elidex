#!/usr/bin/env bash
# Layering trip-wire for the generic webref core (`.claude/tools/_webref/`
# plus the `.claude/tools/webref` entry script) — plan-memo
# 2026-07-citation-hygiene-Ai-spec-label-map.md §2 invariant K2, which that
# memo's §12(3) names as an exit criterion.
#
# `_webref/DESIGN.md` says the package "should stay generic enough to move to
# a standalone repository later" and closes with "keep new generic behavior
# free of elidex-specific file paths".
#
# TWO CHECKS, AND ONLY ONE IS AN ABSOLUTE.  Say which is which, because a green
# here was read for four review rounds as more than it is.
#
# (A) THE PIN — closed, decidable, absolute.  A-i removed exactly two host
#     paths from this tree: `_webref/cli.py` and the `webref` entry script each
#     named `.claude/skills/elidex-review/axes.md` (measured at base
#     `44cd165d`: 1 each; at HEAD: 0).  This wire FAILS if either comes back.
#     Same shape as its four siblings, which pin the re-introduction of a named
#     deleted construct rather than recognising an open category.
#
#     ⚠ The pin is exercised on every run against a planted fixture before the
#     real tree is scanned (`_control` below).  A wire that cannot fire is
#     indistinguishable from a clean tree, and `scripts/trip-wires.sh:105-112`
#     rejects hand-verification for exactly this reason: a check nobody
#     re-runs is a transcript, not a gate.  Codex #501 R66.
#
# (B) THE SEED — a syntactic scan for `<top-level entry>/<something>`, printed
#     but NOT asserted, and deliberately not widened again.
#
#     It was widened four times, each fix opening the next hole: syntactic
#     over-reached (`docs/note.md` inside a test assertion; the prose "affected
#     docs/code"); resolving-on-disk under-reached (a planned or renamed-away
#     path is still a host path); a string-keyed exemption over-suppressed (the
#     same text in any file); a basename-keyed one collided (any nested
#     `DESIGN.md`).  Interpolation — `docs/${x}/y.md` — is the fifth.
#     This repo's own recorded rule is that when a predicate cannot return its
#     population it is a SEED, and widening the regex is the wrong repair
#     (`feedback_checks-must-not-be-defined-by-the-symptom-vocabulary`).  So the
#     seed reports and does not fail: recognising arbitrary path-shaped text in
#     arbitrary source is not decidable here, and a check that claims otherwise
#     teaches readers to trust a green it has not earned.  What covers the open
#     class is the diff — every line entering this tree passes review, and
#     `git diff origin/main...HEAD -- .claude/` is finite.
#
# Run from anywhere.  Exits non-zero if (A) fails.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
for p in "$ROOT/.claude/tools/_webref" "$ROOT/.claude/tools/webref"; do
  [ -e "$p" ] || { echo "!! $p does not exist — this wire would pass over a tree it never read" >&2; exit 2; }
done

# A filesystem walk, not `git grep`: an untracked file under the package is
# exactly where a violation lands during authoring, and `git grep` reads the
# index.  (Measured on this repo: a plant in an unstaged file read GREEN.)
python3 - "$ROOT" <<'PY'
import os, re, subprocess, sys, tempfile

root = sys.argv[1]
scope = [os.path.join(root, ".claude/tools/_webref"), os.path.join(root, ".claude/tools/webref")]

# (A) THE PIN. The host paths A-i removed, spelled out, closed.
PINNED = [".claude/skills/elidex-review/axes.md"]

# (B) THE SEED. Derived vocabulary, syntactic match -- printed, not asserted.
tops = [t for t in subprocess.run(["git", "-C", root, "ls-tree", "--name-only", "HEAD"],
                                  capture_output=True, text=True, check=True).stdout.split()
        if t not in (".gitignore", ".gitattributes")]
if len(tops) < 4:
    raise SystemExit("!! derived only %d top-level entries; the seed would be near-vacuous" % len(tops))
SEED_RE = re.compile(
    r"(?<![\w-])(?:\.{1,2}/|(?:\.\./)+|/)?(" + "|".join(re.escape(t) for t in tops)
    + r")/[\w./*?${}-]+")
# Known non-paths, keyed by path RELATIVE TO `.claude/tools` -- a basename key
# granted any nested `DESIGN.md` the root document's exemption.
NOT_A_PATH = {
    "_webref/test_agent_brief.py": {"docs/note.md"},
    "_webref/DESIGN.md": {"docs/code"},
}
INSTALL_PATH = {".claude/tools/webref"}

def files(scope, base):
    for s in scope:
        if os.path.isfile(s):
            yield s
        else:
            for dirpath, dirnames, filenames in os.walk(s):
                dirnames[:] = [d for d in dirnames if d not in (".git", "__pycache__")]
                for fn in filenames:
                    yield os.path.join(dirpath, fn)


def scan(scope, base):
    """The ONE engine. The controls below and the real run share it, so a
    control can only pass by proving the same code the verdict comes from."""
    pinned_hits, seed_hits, scanned = [], [], 0
    for path in files(scope, base):
        try:
            body = open(path, encoding="utf-8").read()
        except UnicodeDecodeError:
            # Not "nothing to match": a partially-decodable file would otherwise
            # be scanned in part and reported as whole.
            body = open(path, "rb").read().decode("utf-8", "replace")
        scanned += 1
        rel = os.path.relpath(path, os.path.join(base, ".claude/tools"))
        for n, line in enumerate(body.splitlines(), 1):
            for pin in PINNED:
                if pin in line:
                    pinned_hits.append("%s:%d: %s" % (os.path.relpath(path, base), n, pin))
            for m in SEED_RE.finditer(line):
                cand = re.sub(r"^(?:\.{1,2}/|(?:\.\./)+|/)", "", m.group(0).rstrip(".,;:"))
                if cand in INSTALL_PATH or cand in NOT_A_PATH.get(rel, ()):
                    continue
                seed_hits.append("%s:%d: %s" % (os.path.relpath(path, base), n, cand))
    return scanned, pinned_hits, seed_hits


# ---- CONTROLS, run BEFORE the real scan -------------------------------------
# The samples below are spelled out a SECOND time on purpose. They are not a
# duplicate of `PINNED`: they are the independent subject the pin is tested
# against, exactly as `layout-box-reader-trip-wire.sh`'s `ban_control` passes a
# hand-written sample line beside the pattern. If someone edits `PINNED` to a
# different spelling, the two stop agreeing and THAT is the signal -- which is
# the failure `PINNED`-derived fixtures cannot see.
CONTROL_HIT = ".claude/skills/elidex-review/axes.md"
CONTROL_MISS = ".claude/skills/elidex-review/workflow.md"

with tempfile.TemporaryDirectory() as ctl:
    # Distinguish an environment failure from a dead assertion: with the dir
    # empty the controls would exercise nothing and silently "pass".
    hit_dir = os.path.join(ctl, "hit")
    miss_dir = os.path.join(ctl, "miss")
    os.makedirs(hit_dir)
    os.makedirs(miss_dir)
    with open(os.path.join(hit_dir, "control.py"), "w", encoding="utf-8") as fh:
        fh.write('AXES = "%s"  # planted\n' % CONTROL_HIT)
    with open(os.path.join(miss_dir, "control.py"), "w", encoding="utf-8") as fh:
        fh.write('OTHER = "%s"  # planted, NOT the pinned path\n' % CONTROL_MISS)
    n_hit, hits, _ = scan([hit_dir], ctl)
    n_miss, misses, _ = scan([miss_dir], ctl)
    if n_hit != 1 or n_miss != 1:
        raise SystemExit("!! control scan read %d/%d file(s), not 1/1 -- the controls "
                         "proved nothing about this run" % (n_hit, n_miss))
    if not hits:
        raise SystemExit("!! POSITIVE CONTROL FAILED: a planted `%s` did NOT fire the "
                         "pin, so a green below would mean nothing. `PINNED` is empty, "
                         "misspelled, or the match loop is broken." % CONTROL_HIT)
    if misses:
        raise SystemExit("!! NEGATIVE CONTROL FAILED: `%s` fired the pin, so the pin is "
                         "matching more than the path it names." % CONTROL_MISS)
print("  controls: pin fires on a planted `%s`, and not on a sibling path" % CONTROL_HIT)

scanned, pinned_hits, seed_hits = scan(scope, root)

if not scanned:
    raise SystemExit("!! scanned 0 files; this wire would report no violation for a "
                     "reason that is not 'there are none'")
print("  scanned %d file(s) under the generic core" % scanned)
if seed_hits:
    print("  seed (REPORT ONLY -- see the header; the diff covers this class):")
    for h in seed_hits:
        print("     %s" % h)
else:
    print("  seed: no `<top-level>/...` shaped text (%d entries; NOT an absolute)" % len(tops))
if pinned_hits:
    print("!! a host path A-i REMOVED is back in the generic core:")
    for h in pinned_hits:
        print("     %s" % h)
    raise SystemExit(1)
print("  pin: %d removed host path(s), none present -- ABSOLUTE" % len(PINNED))
PY

echo "webref generic-core layering trip-wire PASSED"
