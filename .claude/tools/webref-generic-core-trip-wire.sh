#!/usr/bin/env bash
# Layering trip-wire for the generic webref core (`.claude/tools/_webref/`
# plus the `.claude/tools/webref` entry script) — plan-memo
# 2026-07-citation-hygiene-Ai-spec-label-map.md §2 invariant K2, which that
# memo's §12(3) names as an exit criterion.
#
# `_webref/DESIGN.md` says the package "should stay generic enough to move to
# a standalone repository later" and closes with "keep new generic behavior
# free of elidex-specific file paths AND PUT ELIDEX POLICY IN ADAPTER COMMANDS
# OR DOCUMENTATION".
#
# ⚠ Those are TWO axes, and this wire reaches only the first.  The header used
# to quote the rule truncated at "file paths", which read as though a green
# here covered the whole sentence; #501 gate 4 found the policy half violated
# in the same commit set the wire was green on (host review history in the
# package's own suite).  Nothing mechanical covers the policy half — it is a
# judgement about wording, and the instrument for it is diff review.
#
# THREE CHECKS, TWO OF THEM ABSOLUTE.  Say which is which, because a green
# here was read for four review rounds as more than it is.
#
# (A) THE PIN — closed, decidable, absolute.  A-i removed exactly one host
#     path from this tree, at two sites: `_webref/cli.py` and the `webref`
#     entry script each named `.claude/skills/elidex-review/axes.md` (measured
#     at base `44cd165d`: 1 each; at HEAD: 0).  This wire FAILS if it comes
#     back.  Same shape as its four siblings, which pin the re-introduction of
#     a named deleted construct rather than recognising an open category.
#
# (A2) THE K2 PREDICATE — closed, decidable, absolute.  The plan-memo's §2
#     states K2 with a predicate of its own: no `.claude/(skills|tools)/` plus
#     two further path segments, anywhere in this tree.  That predicate is
#     NARROW: two fixed directory roots, a fixed segment count.  It enumerates
#     its own population, so it is not a seed, and it measures 0 over the 34
#     files here — asserting it costs nothing today and catches the class A-i
#     exists to remove.
#
#     ⚠ It was asserted once and the assertion was lost to two fixes that each
#     made sense alone.  `3aaad3cb` (R55) deleted
#     `TestSliceBoundary.test_no_elidex_file_path_in_this_package` as "a second
#     copy of a scan this wire already makes over a strictly larger range";
#     `b0912a6c` (R62) then demoted that scan to report-only, voiding the
#     deletion's ground, and nobody restored it.  Between those two commits a
#     file carrying `.claude/skills/elidex-plan-review/preflight.py` passed
#     this wire GREEN.  R62's rule — a predicate that cannot return its
#     population is a seed, and widening the regex is the wrong repair — is
#     true of (B) and was over-applied to this one.  The two are different
#     predicates and get different treatment.
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
# Run from anywhere.  Exits non-zero if (A) or (A2) fails.

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

# (A) THE PIN. The host path A-i removed, spelled out, closed.
PINNED = [".claude/skills/elidex-review/axes.md"]

# (A2) THE K2 PREDICATE, verbatim from the plan memo's §2: two fixed directory
# roots plus two further segments. Closed and decidable -- NOT the seed below.
# No exemption list: §2 states K2 with none, and an exemption with zero members
# is an untested escape hatch (measured at HEAD: 0 hits either way).
K2_RE = re.compile(r"\.claude/(?:skills|tools)/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+")

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
    pinned_hits, k2_hits, seed_hits, scanned = [], [], [], 0
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
            for m in K2_RE.finditer(line):
                k2_hits.append("%s:%d: %s" % (os.path.relpath(path, base), n, m.group(0)))
            for m in SEED_RE.finditer(line):
                cand = re.sub(r"^(?:\.{1,2}/|(?:\.\./)+|/)", "", m.group(0).rstrip(".,;:"))
                if cand in INSTALL_PATH or cand in NOT_A_PATH.get(rel, ()):
                    continue
                seed_hits.append("%s:%d: %s" % (os.path.relpath(path, base), n, cand))
    return scanned, pinned_hits, k2_hits, seed_hits


# ---- CONTROLS, run BEFORE the real scan -------------------------------------
# The samples below are spelled out a SECOND time on purpose. They are not a
# duplicate of `PINNED`: they are the independent subject the pin is tested
# against, exactly as `layout-box-reader-trip-wire.sh`'s `ban_control` passes a
# hand-written sample line beside the pattern. If someone edits `PINNED` to a
# different spelling, the two stop agreeing and THAT is the signal -- which is
# the failure `PINNED`-derived fixtures cannot see.
CONTROL_HIT = ".claude/skills/elidex-review/axes.md"
CONTROL_MISS = ".claude/skills/elidex-review/workflow.md"
# (A2)'s control is a DIFFERENT host path, so the two absolutes cannot pass each
# other's test: `workflow.md` must miss the pin and hit the K2 predicate.

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
    n_hit, hits, hit_k2, _ = scan([hit_dir], ctl)
    n_miss, misses, miss_k2, _ = scan([miss_dir], ctl)
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
    if not miss_k2:
        raise SystemExit("!! K2 CONTROL FAILED: a planted `%s` did NOT fire the K2 "
                         "predicate, so a green below would mean nothing. `K2_RE` is "
                         "broken." % CONTROL_MISS)
print("  controls: pin fires on a planted `%s` and not on a sibling; the K2 predicate "
      "fires on `%s`" % (CONTROL_HIT, CONTROL_MISS))

scanned, pinned_hits, k2_hits, seed_hits = scan(scope, root)

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
    print("        blind to: bare roots with no separator (`\"docs\"`, `\"crates\"`,")
    print("        `\"CLAUDE.md\"` -- live at cli.py's `--paths` default and refresh.py's")
    print("        usage string, both pre-existing) and interpolation (`docs/${x}/y.md`).")
failed = False
if pinned_hits:
    print("!! a host path A-i REMOVED is back in the generic core:")
    for h in pinned_hits:
        print("     %s" % h)
    failed = True
else:
    print("  pin: %d removed host path, none present -- ABSOLUTE" % len(PINNED))
if k2_hits:
    print("!! K2: a `.claude/(skills|tools)/<a>/<b>` host path is named in the generic core:")
    for h in k2_hits:
        print("     %s" % h)
    failed = True
else:
    print("  K2: 0 `.claude/(skills|tools)/<a>/<b>` paths named here -- ABSOLUTE")
if failed:
    raise SystemExit(1)
PY

echo "webref generic-core layering trip-wire PASSED"
