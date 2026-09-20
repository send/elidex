#!/usr/bin/env bash
# Layering trip-wire for the generic webref core (`.claude/tools/_webref/`
# plus the `.claude/tools/webref` entry script) — plan-memo
# 2026-07-citation-hygiene-Ai-spec-label-map.md §2 invariant K2, which that
# memo's §12(3) names as an exit criterion.
#
# `_webref/DESIGN.md` says the package "should stay generic enough to move to
# a standalone repository later" and closes with "keep new generic behavior
# free of elidex-specific file paths".  Slice A-i discharged the two instances
# that existed at its base (`_webref/cli.py` and the `webref` entry script both
# named `.claude/skills/elidex-review/axes.md`); this wire is what stops them
# coming back.  Without it the invariant has no enforcement at all: `mise run
# ci` is cargo-only, and the suite beside the package deliberately does not
# carry host-project policy (it is meant to be extractable too).
#
# THE PREDICATE RESOLVES, IT DOES NOT ENUMERATE OR GUESS.  An earlier home for
# this check listed the path shapes it knew — `.claude/(skills|tools)/<a>/<b>`
# — and was measured blind to `crates/`, `docs/`, `scripts/`, `mise.toml` and
# `.github/`; widening that list only moves the blindness to the next shape.
# But the complement alone over-reaches in the other direction: a first pass of
# this wire flagged `docs/note.md` (a synthetic filename inside a test
# assertion) and `docs/code` (the prose phrase "affected docs/code").
#
# So the predicate is SYNTACTIC — `<top-level entry>/<something>`, globs
# included — and the handful of strings that are genuinely not paths are named
# in `NOT_A_PATH` below, where adding one is a visible edit.  ⚠ An earlier draft
# instead asked whether the path RESOLVES on disk, which reads greener than it
# should: a path to a file that is planned, renamed away, or written as a glob
# is still a path into the host tree.  The one deliberate exemption is the
# tool's own install path `.claude/tools/webref` — an invocation example is not
# a path into the tree, and DESIGN.md's usage lines carry it.
#
# ⚠ WHAT THIS WIRE DOES NOT COVER, stated so a green is not read as more than
# it is: a BARE top-level name, used with no separator after it.  The predicate
# requires `<top-level entry>/<something>`, so `"docs"`, `"crates"` and
# `"CLAUDE.md"` as standalone tokens are invisible to it.  That is a real and
# arguably worse class — the package baking in the host's directory layout
# rather than merely naming a file — and it has two live instances, both
# PRE-EXISTING at this slice's base `44cd165d` and so not the slice's to
# discharge:
#
#   _webref/cli.py:217          --paths default ["docs", "crates", "CLAUDE.md"]
#   _webref/commands/refresh.py:52   the same triple, in help text
#
# They are left standing deliberately.  Widening this wire to catch them would
# make it RED on a tree whose only violations are its own inheritance, which is
# how a wire gets disabled rather than obeyed; and the memo this wire serves
# defines K2 as the path-shaped class only.  Whoever generalises the package
# for extraction owns that pair.
#
# Run from anywhere.  Exits non-zero on any violation.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SCOPE=("$ROOT/.claude/tools/_webref" "$ROOT/.claude/tools/webref")

for p in "${SCOPE[@]}"; do
  [ -e "$p" ] || { echo "!! $p does not exist — this wire would pass over a tree it never read" >&2; exit 2; }
done

# The top-level vocabulary is derived INSIDE the python below, not here.  An
# earlier draft read it with `mapfile`, which is bash 4; `/bin/bash` on stock
# macOS is 3.2, so under `mise run ci` this wire aborted on `set -e` before
# scanning anything — a wire that cannot run is worse than no wire, and the
# driver records that same hazard class at `scripts/trip-wires.sh:24-26`.
#
# A filesystem walk, not `git grep`: an untracked file under the package is
# exactly where a violation lands during authoring, and `git grep` reads the
# index.  (Measured on this repo: a plant in an unstaged file read GREEN.)
python3 - "$ROOT" <<'PY'
import os, re, subprocess, sys

root = sys.argv[1]
# The repo's own top-level entries ARE the host-project vocabulary.  Deriving
# the set rather than writing it down is what keeps a new top-level directory
# from being invisible to this wire on the day it appears.
tops = [t for t in subprocess.run(["git", "-C", root, "ls-tree", "--name-only", "HEAD"],
                                  capture_output=True, text=True, check=True).stdout.split()
        if t not in (".gitignore", ".gitattributes")]
if len(tops) < 4:
    raise SystemExit("!! derived only %d top-level entries; the predicate would be near-vacuous" % len(tops))
scope = [os.path.join(root, ".claude/tools/_webref"), os.path.join(root, ".claude/tools/webref")]
# `<top>/<something>` — the trailing segment is what makes it a path INTO the
# tree rather than a bare mention of a directory's name in prose.
# `[\w./-]` alone missed a GLOB (`crates/**/*.rs`), which is still a path into
# the host tree; `*` and `?` join the class.
#
# ⚠ AND THE LOOKBEHIND MUST NOT REJECT A PREFIX.  `(?<![\w/.-])` was written to
# stop `foodocs/x` matching, but it also rejected `./docs/x`, `../docs/x` and
# any absolute path, because each puts `/` or `.` immediately before the entry
# name — measured, all three read GREEN.  A path is not less a path into the
# host tree for being spelled relative to the file or from the filesystem root.
# So an OPTIONAL prefix (`./`, any run of `../`, or a leading `/`) is part of
# the match, and the lookbehind only has to exclude a WORD character before it,
# which is what actually distinguishes `foodocs/x` from `./docs/x`.
PATH_RE = re.compile(
    r"(?<![\w-])(?:\.{1,2}/|(?:\.\./)+|/)?(" + "|".join(re.escape(t) for t in tops)
    + r")/[\w./*?-]+")
EXEMPT = {".claude/tools/webref"}
# ⚠ EXISTENCE IS NOT THE TEST, and an earlier draft made it one.  A path the
# package should not name is no less one for pointing at a file that is planned,
# renamed away, or matched by a glob — `docs/new-policy.md` would have read
# GREEN purely for not existing yet.  So every syntactic match counts, and the
# two things that are genuinely NOT paths are named instead: the synthetic
# filename a test asserts on, and prose that happens to contain a slash.  Both
# are listed, so adding one is a visible edit rather than a silent widening.
NOT_A_PATH = {
    "docs/note.md",   # test_agent_brief.py's synthetic fixture name
    "docs/code",      # DESIGN.md prose: "affected docs/code"
}

def files():
    for s in scope:
        if os.path.isfile(s):
            yield s
        else:
            for dirpath, dirnames, filenames in os.walk(s):
                dirnames[:] = [d for d in dirnames if d not in (".git", "__pycache__")]
                for fn in filenames:
                    yield os.path.join(dirpath, fn)

hits, scanned = [], 0
for path in files():
    try:
        body = open(path, encoding="utf-8").read()
    except UnicodeDecodeError:
        # Not "nothing to match": a partially-decodable file would be scanned
        # in part and reported as whole.  Read it as bytes, lossily, so the
        # census covers every line it claims to cover.
        body = open(path, "rb").read().decode("utf-8", "replace")
    scanned += 1
    for n, line in enumerate(body.splitlines(), 1):
        for m in PATH_RE.finditer(line):
            # Prose ends sentences; a path does not end in a period.
            cand = m.group(0).rstrip(".,;:")
            # The prefix is part of the match now, so normalise it away before
            # comparing against the two name sets — otherwise `./docs/note.md`
            # would be reported while `docs/note.md` is not.
            bare = re.sub(r"^(?:\.{1,2}/|(?:\.\./)+|/)", "", cand)
            if bare in EXEMPT or bare in NOT_A_PATH:
                continue
            hits.append(f"{os.path.relpath(path, root)}:{n}: {cand}")

if not scanned:
    raise SystemExit("!! scanned 0 files; this wire would report no violation for a reason that is not 'there are none'")
print(f"  scanned {scanned} file(s) under the generic core against {len(tops)} top-level entries")
if hits:
    print("!! host-project file path(s) in the generic core — DESIGN.md forbids them here:")
    for h in hits:
        print(f"     {h}")
    raise SystemExit(1)
PY

echo "webref generic-core layering trip-wire PASSED"
