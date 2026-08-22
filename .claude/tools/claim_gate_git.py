#!/usr/bin/env python3
"""Everything this program asks git, with one answer per question.

Three call sites asked a variant of *what does this branch change* and answered
it three different ways:

  * `changes_the_wire_set`      `git diff --name-only origin/main...HEAD`
  * `check_tooling_quantities`  `git diff --name-only origin/main..HEAD`
  * `check_paths`               `git log origin/main..HEAD --diff-filter=D`

The first two are the same question with different answers, and the difference
is not a style preference. **A two-dot `log` range and a three-dot `diff` range
denote the same commits; a two-dot `diff` does not.** `git log A..B` is
"reachable from B, not from A", which is merge-base anchored by construction.
`git diff A..B` compares the two TIPS -- so every file `main` gains after the
branch point joins the count of what the branch changed, and the figure decays
as `main` moves. That was under `check_tooling_quantities`, whose entire purpose
is catching figures that decay, and it was unobservable in this repository,
whose merge base IS `origin/main` today. The probe for it builds a history that
has diverged, because a probe run here would have passed either way.

None of the three disabled rename detection, which git performs unasked -- the
version it became the default in is not recorded here, because the probes
measure the behaviour rather than cite it. A rename reports only where the path
ARRIVED, and every consumer here is asking about a departure: whether a wire
left the set, whether a file the memo's sentences describe is gone. Measured on
a fixture that renames `x-trip-wire.sh` to `x.sh`: `changes_the_wire_set()`
answered False and `deleted_on_branch()` answered empty, for a branch that
removed the only wire in the tree.

So, the convention, stated once because three sites disagreeing about it is
what this module exists to end: **three dots for a diff, two for a log or a
commit count, and `--no-renames` everywhere.**

`None` is the answer whenever the range cannot be formed -- no `origin/main` in
this clone, or a HEAD unrelated to it. Callers report that through `host.skip()`
rather than reading an empty list as a clean one; that distinction is why these
return `None` instead of `[]`.

`host` is the entry point, bound by it at import time as the other companions
are, and `REPO` is reached through `host.sh` on every call rather than copied
here. The probe file swaps `host.REPO` to aim these derivations at a fixture
repository, and a copy taken at import time would be a name nothing is swapping
-- the failure `claim_gate_ledgers.py`'s header records for `SELFTEST_FILE`.
"""
from __future__ import annotations

from types import ModuleType

host: ModuleType  # bound by the entry point at import time

BASE = "origin/main"


def have_base() -> bool:
    """Is there a `origin/main` in this clone to anchor a range to?"""
    return host.sh(["git", "rev-parse", "--verify", "--quiet", BASE])[0] == 0


def changed_paths() -> list[str] | None:
    """Paths this branch changes, against the point it left `origin/main`."""
    if not have_base():
        return None
    rc, out = host.sh(["git", "diff", "--name-only", "--no-renames",
                       f"{BASE}...HEAD"])
    return sorted({x for x in out.split("\n") if x}) if rc == 0 else None


def commit_count() -> int | None:
    """How many commits this branch has that `origin/main` does not.

    Two dots, and correctly so: this is a `rev-list` range, where two dots
    already means "reachable from HEAD, not from the base". It is spelled out
    because the sibling above spells three, and an unexplained disagreement
    between two adjacent lines is what produced the one this module fixes.
    """
    if not have_base():
        return None
    rc, out = host.sh(["git", "rev-list", "--count", f"{BASE}..HEAD"])
    return int(out.strip()) if rc == 0 and out.strip() else None


def paths_at_merge_base() -> list[str] | None:
    """Every tracked path as of the point this branch left `origin/main`.

    The other half of a SET question. `changed_paths()` answers "was this path
    touched", which is not the same as "did the set gain or lose a member": a
    one-line edit to an existing member touches a path and changes no set. A
    caller asking about membership compares this against the tree and takes the
    difference, and gets rename handling for free -- a renamed member leaves
    under its old name whatever the diff machinery decides to call the move.
    """
    if not have_base():
        return None
    rc, out = host.sh(["git", "merge-base", BASE, "HEAD"])
    if rc != 0 or not out.strip():
        return None
    rc, out = host.sh(["git", "ls-tree", "-r", "--name-only", out.strip()])
    return [x for x in out.split("\n") if x] if rc == 0 else None


def tracked_paths() -> list[str]:
    """Every tracked path in the working tree."""
    return [x for x in host.sh(["git", "ls-files"])[1].split("\n") if x]


def gone_from(log_out: str, tracked: set[str]) -> list[str]:
    """The deletion log, minus what is back. A path deleted in one commit of the
    range and restored in a later one is not a file this branch removes, and
    `--diff-filter=D` names it either way -- so the tree decides, not the log.
    Split out from the git plumbing so the subtraction has a control."""
    return sorted({p for p in log_out.split("\n") if p and p not in tracked})


def deleted_on_branch() -> list[str] | None:
    """Paths `origin/main..HEAD` removes and the working tree does not have back.

    None when the range cannot be formed -- no `origin/main` in this clone, or a
    HEAD unrelated to it. The caller reports that rather than reading an empty
    answer as a clean one, which is `host.skip()`'s whole reason for existing."""
    if not have_base():
        return None
    rc, out = host.sh(["git", "log", f"{BASE}..HEAD", "--diff-filter=D",
                       "--no-renames", "--name-only", "--format="])
    if rc != 0:
        return None
    return gone_from(out, {x for x in host.sh(["git", "ls-files"])[1].split("\n") if x})


def removed_in_history(path: str) -> bool | None:
    """Did any commit reachable from HEAD delete `path`?

    The question `check_paths`' parking arm had no way to ask. A row in the
    removal ledger whose subject is absent has two possible causes and only one
    of them is a record: the file was removed, or the path was never a file. A
    misspelling and a genuine removal are indistinguishable both to "is it in
    the tree" and to "is it in this branch's range" -- the second deliberately
    so, since a row has to survive its deletion leaving the range when the
    prerequisite PR lands. History is the only thing that separates them.

    Reachable from HEAD, not `--all`: the claim a removal row makes is that
    THIS line of development removed the file. `--all` would also accept a path
    deleted on some other branch, which makes the answer depend on what this
    clone happens to have fetched rather than on what the memo says.

    None -- report, do not decide -- when the history is not here to be asked.
    In a shallow clone `git log HEAD` sees one commit, so every genuine removal
    would read as a path that never existed, and the check would fire hardest
    on the memos that are most correct.
    """
    rc, out = host.sh(["git", "rev-parse", "--is-shallow-repository"])
    if rc != 0 or out.strip() != "false":
        return None
    # Both spellings, as `host.resolve` accepts both: a memo cites by basename
    # as often as by path, and a row that spells one is declaring the same file
    # as a row that spells the other. A bare basename is not a pathspec git
    # matches at any depth, so without the fallback a correctly-spelled row read
    # as a path no commit ever had -- the arm firing hardest on the memo that
    # had done the right thing.
    rc, out = host.sh(["git", "log", "HEAD", "--diff-filter=D", "--no-renames",
                       "--name-only", "--format=", "--", path, f"*/{path}"])
    if rc != 0:
        return None
    return any(x for x in out.split("\n") if x)
