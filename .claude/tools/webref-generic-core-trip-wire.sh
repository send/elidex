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
# A path INTO elidex's tree is one that RESOLVES inside elidex's tree.  That is
# a fact this wire can check rather than a shape it has to predict, and it
# separates the two classes above by construction: neither synthetic name
# exists, `.claude/skills/elidex-review/axes.md` does.  The one deliberate
# exemption is the tool's own install path `.claude/tools/webref` — an
# invocation example is not a path into the tree, and DESIGN.md's usage lines
# carry it.
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

# The repo's own top-level entries ARE the host-project vocabulary.  Deriving
# the set from the filesystem rather than writing it down is what keeps a new
# top-level directory from being invisible to this wire on the day it appears.
mapfile -t TOP < <(cd "$ROOT" && git ls-tree --name-only HEAD | grep -vE '^\.(gitignore|gitattributes)$')
[ "${#TOP[@]}" -gt 3 ] || { echo "!! derived only ${#TOP[@]} top-level entries; the predicate would be near-vacuous" >&2; exit 2; }

# A filesystem walk, not `git grep`: an untracked file under the package is
# exactly where a violation lands during authoring, and `git grep` reads the
# index.  (Measured on this repo: a plant in an unstaged file read GREEN.)
python3 - "$ROOT" "${TOP[@]}" <<'PY'
import os, re, sys

root, tops = sys.argv[1], sys.argv[2:]
scope = [os.path.join(root, ".claude/tools/_webref"), os.path.join(root, ".claude/tools/webref")]
# `<top>/<something>` — the trailing segment is what makes it a path INTO the
# tree rather than a bare mention of a directory's name in prose.
PATH_RE = re.compile(r"(?<![\w/.-])(" + "|".join(re.escape(t) for t in tops) + r")/[\w./-]+")
EXEMPT = {".claude/tools/webref"}

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
            if cand in EXEMPT or not os.path.exists(os.path.join(root, cand)):
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
