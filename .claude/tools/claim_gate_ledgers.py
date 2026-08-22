#!/usr/bin/env python3
"""The two checks whose population comes from OUTSIDE the prose, and whose
escape hatch is a positive declaration read by position.

Split out of `claim-gate-plan-check.py` at 1123 lines (CLAUDE.md's touch-time
split discipline, which fires at 1000 and is not deferrable). The seam is a
shared mechanism rather than a line budget, and it is the one this program kept
re-deriving:

  1. the population is enumerated by something that is not the document --
     `CONTROLS` RUNS the self-test and reads the ids it prints; `PATHS` asks git
     which paths `origin/main..HEAD` removes. Neither asks what the memo's
     sentences look like, because every previous attempt that did was defeated
     by a rephrasing (`feedback_checks-must-not-be-defined-by-the-symptom-vocabulary`);
  2. a legitimate mention of an absent thing is licensed by a POSITIVE
     DECLARATION in a designated section -- §8's retirement ledger, §0.1's
     removal ledger -- rather than by a pattern exempting whatever wording
     today's withdrawals happen to use;
  3. the licence is scoped BY POSITION: the row records the fact, and it does
     not license a use of it a section away. Both checks measure mentions
     outside the ledger's own span, so a row cannot satisfy the obligation it
     exists to record;
  4. the ledger fails in both directions. A row whose subject is back -- a
     control the self-test ships, a path the tree has -- is a finding, or the
     section becomes somewhere to park a citation rather than a record of one.

`host` is the entry point, bound by it at import time, exactly as
`claim_gate_reruns` and `claim_gate_selftest` are. `bad` and `skip` are resolved
through it on every call and never imported by name: the probe file REBINDS
`host.FAILS` and `host.SKIPPED` for the duration of one check, so a name bound
once here would append to the list that was swapped out, `capture()` would
return empty, and every negative probe would silently invert.

The same call-time rule runs the other way for the two globals the probe file
swaps -- `SELFTEST_FILE` and `deleted_on_branch`. They live HERE, so a probe
that rebinds them on `host` changes a name nothing reads. Both are reached as
attributes of this module, which the entry point re-exports by name.
"""
from __future__ import annotations

import re
from pathlib import Path
from types import ModuleType

host: ModuleType  # bound by the entry point at import time

# Module-level constants are built at IMPORT time, which is before the entry
# point can bind `host` -- so the two paths below are derived here rather than
# read off it. `host.REPO` is the same expression one directory up.
TOOLS = Path(__file__).resolve().parent


# --------------------------------------------------------------------------
# 13. CONTROLS -- every control the memo NAMES must exist, with the arm named
#
# This replaces FIXTURE, which derived §8.1's Fixture column from the row's own
# pathspec. §8.1 is gone -- the control table is `claim-provenance-selftest.sh`
# now -- and a check gated on a section that no longer exists is not dormant,
# it is a `ban_control` that reports coverage. What it was really defending is
# the same thing this defends one level up: that a control named in the memo is
# a control that exists.
#
# Round 8 restructured the specification out of the memo and into the wire and
# its self-test, and nothing was left holding the two together. What the memo
# went on saying: "§8 C0 keeps a control for it" (there is no C0, and it was
# not on the unwritten list either); "C10's positive" and "C11's positive", as
# the evidence for the clone-fixture decision (both rows are negative-only);
# and a stale control count in the shipped wire's own header.
# Rounds of prose review read past all of them, because a control citation
# reads exactly like coverage whether or not the control is there.
#
# The population is every `C<number>` token in the memo -- defined by the
# property being checked, not by the vocabulary a live citation happens to use.
# Historical mentions are real and must stay writable ("C34 is withdrawn",
# "C26 was deleted"), so the escape hatch is a POSITIVE declaration rather than
# a pattern exempting whatever phrasing today's retirements happen to have:
# §8's retirement ledger names them in a table, and this reads that table. An
# entry there that turns out to BE implemented fails too -- otherwise the
# ledger becomes a place to park a citation rather than a record of one.
#
# Round 9 shipped that hatch too wide. `if cid in retired: continue` licensed
# the id ANYWHERE, in any role, so a retired control could go on being cited as
# live support -- and two were: §8.2 kept "the acceptance corpus takes one of
# the five" in the present tense, and §4.1.1 then cited that paragraph as its
# authority for withdrawing the corpus. The ledger licenses a MENTION, at the
# one site that records the retirement; it does not license a USE. So a retired
# id may appear inside §8's ledger rows and nowhere else, and prose elsewhere
# says what happened without the number. That is decidable by POSITION rather
# than by reading intent out of a sentence's vocabulary, which is the whole
# reason this check exists.
# (memory `feedback_enumerated-exemptions-leave-the-next-class-authoritative`.)
# --------------------------------------------------------------------------

SELFTEST_FILE = TOOLS / "claim-provenance-selftest.sh"
# One line of the self-test's own output: `  ok    C14 positive  count = ...`
# or `  FAIL  C18 (b)  the stub ran ...`. Both statuses are read -- a failing
# run still says which controls exist, and the memo's citations are no less
# checkable while one of them is red.
INVENTORY_LINE = re.compile(
    r"^\s+(?:ok|FAIL)\s+(?P<id>C\d+)\s*(?:\((?P<sub>[a-z])\))?\s*"
    r"(?P<arm>positive|negative)?")
# In the memo: `C10`, `C10's positive`, `C18's negative arm`, `C19`–`C22`.
# Two patterns, because one cannot do both jobs. `MEMO_CONTROL` finds every id
# and any sub-arm letter; `MEMO_CONTROL_ARM` finds an id whose arm word follows
# within a bounded window, with the arm REQUIRED so the match cannot succeed by
# skipping it. Round 9 required the arm to abut the id, and one inserted word
# ("C10's *intended* positive") defeated the check at two live sites; making the
# gap optional in one pattern does not work either -- the lazy quantifier
# prefers zero words and reports no arm at all, which is how the first attempt
# at this fix passed its own probe while changing nothing.
MEMO_CONTROL = re.compile(r"`?\b(?P<id>C\d+)`?(?:'s)?\s*(?:\((?P<sub>[a-z])\))?")
MEMO_CONTROL_ARM = re.compile(
    r"`?\b(?P<id>C\d+)`?(?:'s)?(?:\s+\w+){0,2}\s+(?P<arm>positive|negative)\b")
CONTROL_RANGE = re.compile(r"`?\b(C\d+)`?\s*[–—-]\s*`?(C\d+)`?")
# `§8 control` is a section reference, not a count, so the digits may not be
# preceded by a section sign -- the first draft of this pattern reported §9's
# "no §8 control" as a stale tally.
COUNT_OF_CONTROLS = re.compile(r"(?<!§)\b(\d+)[- ]controls?\b")
# Every file this program ships that can make a claim about itself. Round 9
# shipped the rule with a source list of [memo, wire, shell self-test]: the
# checker enforced against three files a rule it was breaking in the fourth. A
# rule whose population excludes its own author is the enumerated-exemption
# shape, one level in -- and the hand-written repair had the defect back within
# two rounds, because splitting `claim_gate_reruns.py` out of the entry point
# added a fifth file and nobody edits a list they are not looking at. So the
# population is a GLOB over what this program actually ships, and a companion
# added tomorrow is in it without an edit.
DEFAULT_COUNT_SOURCES = sorted(TOOLS.glob("claim[-_]*"))
# The self-test's closing line, printed only after every control has run.
COMPLETED = re.compile(r"^claim-provenance self-test: \d+/\d+ controls ok$", re.M)


def implemented_controls() -> dict[str, set[str]]:
    """id -> the arms the self-test actually ships, read from RUNNING it.

    Not from its source. Four controls are emitted by a `for surface in md sh
    toml rs` loop whose label is built from a variable, so their ids appear
    nowhere in the file as literals -- a source-regex derivation reported C4,
    C5, C24 and C36 as controls the memo had invented. Defining the population
    by a syntactic form rather than by the property is the trap
    `feedback_checks-must-not-be-defined-by-the-symptom-vocabulary` names; the
    self-test prints one line per control it runs, and that is the property."""
    out: dict[str, set[str]] = {}
    if not SELFTEST_FILE.exists():
        return out
    _, listing = host.sh(["bash", str(SELFTEST_FILE)])
    # A run that ABORTED is the hazard, not a run that failed. A failing control
    # still prints its label, so a red self-test answers this question as well
    # as a green one; a self-test killed part-way by `set -e` prints a PREFIX,
    # and every control after the abort would then be reported as a citation the
    # memo invented -- a flood of findings whose real cause is one line above.
    # The closing summary is the completion sentinel: no summary, no inventory.
    if not COMPLETED.search(listing):
        return {}
    for line in listing.split("\n"):
        m = INVENTORY_LINE.match(line)
        if not m:
            continue
        out.setdefault(m.group("id"), set())
        for word in (m.group("arm"), m.group("sub")):
            if word:
                out[m.group("id")].add(word)
    return out


def retired_controls(memo: Memo) -> tuple[dict[str, str], list[tuple[int, int]]]:
    """(id -> the ledger's reason, and the char spans the ledger rows occupy).

    The spans are what holds the hatch to a mention: an id can carry a ledger
    row and still be cited as live one section away, which is what happened."""
    out: dict[str, str] = {}
    spans: list[tuple[int, int]] = []
    sect8 = memo.sect("8")
    base = memo.text.find(sect8) if sect8 else 0
    for line in sect8.split("\n"):
        c = host.split_row(line)
        if not c or len(c) < 2 or not re.fullmatch(r"`?C\d+`?", c[0].strip()):
            continue
        out[c[0].strip().strip("`")] = c[1]
        at = memo.text.find(line, base)
        if at >= 0:
            spans.append((at, at + len(line)))
    return out, spans


def check_controls(memo: Memo, have: dict[str, set[str]] | None = None,
                   sources: list[Path] | None = None) -> None:
    have = implemented_controls() if have is None else have
    if not have:
        host.skip("CONTROLS", f"{SELFTEST_FILE.name} is absent, or its run did not reach "
                         f"the closing summary that says every control ran — either "
                         f"way no control citation in the memo was resolved against "
                         f"anything")
        return
    retired, ledger_spans = retired_controls(memo)

    for cid, why in sorted(retired.items(), key=lambda kv: int(kv[0][1:])):
        if cid in have:
            host.bad("CONTROLS", f"§8's retirement ledger lists {cid} ({why.strip()[:60]}…) "
                            f"but the self-test ships it — a ledger row records a "
                            f"control that is gone, it does not licence naming one")

    cited: dict[str, set[str]] = {}
    # Ranges first: `C19`–`C22` names four controls, not two.
    for m in CONTROL_RANGE.finditer(memo.text):
        lo, hi = int(m.group(1)[1:]), int(m.group(2)[1:])
        if 0 <= hi - lo < 40:
            for n in range(lo, hi + 1):
                cited.setdefault(f"C{n}", set())
    outside: dict[str, list[int]] = {}
    for m in MEMO_CONTROL.finditer(memo.text):
        cited.setdefault(m.group("id"), set())
        if m.group("sub"):
            cited[m.group("id")].add(m.group("sub"))
        if not any(a <= m.start() < b for a, b in ledger_spans):
            outside.setdefault(m.group("id"), []).append(m.start())
    for m in MEMO_CONTROL_ARM.finditer(memo.text):
        cited.setdefault(m.group("id"), set()).add(m.group("arm"))

    for cid, arms in sorted(cited.items(), key=lambda kv: int(kv[0][1:])):
        if cid in retired:
            for at in outside.get(cid, []):
                ln = memo.text.count("\n", 0, at) + 1
                host.bad("CONTROLS", f"{memo.path.name}:{ln} names {cid}, which §8's "
                                f"ledger retires — the ledger licenses the row "
                                f"that records the retirement, not a citation "
                                f"elsewhere; say what happened without the number")
            continue
        if cid not in have:
            host.bad("CONTROLS", f"the memo names {cid} as a live control; "
                            f"{SELFTEST_FILE.name} has no such row, and §8's "
                            f"retirement ledger does not list it either")
            continue
        for arm in sorted(arms - have[cid]):
            host.bad("CONTROLS", f"the memo cites {cid}'s {arm} arm; {SELFTEST_FILE.name} "
                            f"ships {cid} with " +
                            (f"only its {'/'.join(sorted(have[cid]))} arm"
                             if have[cid] else "no arm named either way"))

    # A count of controls is a quantity about this program's own tooling, which
    # §8 forbids in its own voice and the wire's header then carried anyway.
    for path in (sources if sources is not None else DEFAULT_COUNT_SOURCES + [memo.path]):
        if not path.exists():
            continue
        for m in COUNT_OF_CONTROLS.finditer(path.read_text()):
            host.bad("CONTROLS", f"{path.name} states `{m.group(0)}` — that count moves "
                            f"with the next control; state the property and let "
                            f"`bash {SELFTEST_FILE.name} | tail -1` report the number")


# --------------------------------------------------------------------------
# 14. PATHS -- a file this branch DELETES, still discussed by the memo
#
# Round 10's largest finding: §0 spent four paragraphs on `claim_gate_grammar.py`
# in the present tense -- what it carries, what delegates to it, which commit
# retires it -- after the prerequisite branch had deleted the file. Every check
# was green, because none of them reads what a sentence claims: `SECT` resolves
# a reference, `CITE` resolves a line, `CONTROLS` resolves a control id, and a
# paragraph about a file that is not there resolves to nothing at all.
#
# The population is derived from GIT, not from the memo's vocabulary, which is
# the trap this program keeps re-entering
# (`feedback_checks-must-not-be-defined-by-the-symptom-vocabulary`). Three
# populations were measured before this one was chosen:
#
#   * "every backticked path that `git ls-files` cannot resolve" -- 30 tokens in
#     this memo, 2 unresolved, but 57 of the tree's 70 plan-memos carry at least
#     one, and nearly all of those are ELISIONS of real files
#     (`elidex-dom-api/element/tree.rs` for `crates/dom/.../src/element/tree.rs`)
#     or files outside the repository (`memory/project_open-defer-slots.md`).
#     A check that reds 57 memos for naming files that exist is the category
#     error `check_sites` was just corrected for.
#   * the same, plus a "not in `git log --all` either" branch for fabrication.
#     Withdrawn: `--all` is a property of one clone. `plan-xcheck.py` is on a
#     local branch of another lane and vanishes from that answer in a fresh
#     clone, so the hard-fail arm would fire on the environment, not the memo.
#   * paths removed by THIS branch and not back in the tree. Measured on this
#     branch: exactly one, `.claude/tools/claim_gate_grammar.py` -- the subject
#     of the finding. Zero on every other memo, since the range is empty of
#     deletions for any branch that does not delete anything.
#
# The third is the property: the author cannot be asked to know what exists
# everywhere, but a file their own commits removed is theirs to re-declare, and
# the range `origin/main..HEAD` is the scoping `check_tooling_quantities`
# already uses for the same reason.
#
# What the ledger buys is a re-declaration, not a swept paragraph: it turns the
# deletion from a silent green into a red naming the file. Prose tense is not
# reachable from here, and §0 says so in its own voice.
# --------------------------------------------------------------------------

# `- `path` — reason`, the shape §8's retirement ledger already uses one level
# down. A bullet rather than a table row: preflight counts data rows, so a
# tabular ledger would move §8.3's pasted `total entries (M)` every time a file
# is deleted -- a second, unrelated figure rotting on the first one's edit.
GONE_ROW = re.compile(r"^-[ \t]+`([^`\n]+)`[ \t]+—[ \t]+(\S.*)$", re.M)
GONE_SECTION = "0.1"


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
    if host.sh(["git", "rev-parse", "--verify", "--quiet", "origin/main"])[0] != 0:
        return None
    rc, out = host.sh(["git", "log", "origin/main..HEAD", "--diff-filter=D",
                  "--name-only", "--format="])
    if rc != 0:
        return None
    return gone_from(out, {x for x in host.sh(["git", "ls-files"])[1].split("\n") if x})


def check_paths(memo: Memo, gone: list[str] | None = None) -> None:
    gone = deleted_on_branch() if gone is None else gone
    if gone is None:
        host.skip("PATHS", "`origin/main..HEAD` could not be formed in this clone, so "
                      "no file this branch deletes was compared against what the "
                      "memo still says about it")
        return
    section = memo.sect(GONE_SECTION)
    declared = {m.group(1): m.group(2) for m in GONE_ROW.finditer(section)}
    # The row itself names the path, so containment has to be measured OUTSIDE
    # the ledger -- otherwise "this row has lost its subject" is unreachable and
    # the section becomes somewhere to park a path. Same position-scoped hatch
    # as `CONTROLS`, for the same reason.
    at = memo.text.find(section) if section else -1
    elsewhere = (memo.text[:at] + memo.text[at + len(section):]) if at >= 0 else memo.text

    for path in gone:
        base = path.rsplit("/", 1)[-1]
        named = base in elsewhere
        if named and path not in declared:
            host.bad("PATHS", f"this branch deletes `{path}` and {memo.path.name} still "
                         f"names it; §{GONE_SECTION} owes a row saying what the memo "
                         f"means by it now — the sentences around it were written "
                         f"while the file was there")
        elif not named and path in declared:
            host.bad("PATHS", f"§{GONE_SECTION} declares `{path}`, and no sentence outside "
                         f"that section names it — the row has outlived its subject")
    for path in declared:
        if path not in gone:
            host.bad("PATHS", f"§{GONE_SECTION} declares `{path}` as a file this branch "
                         f"removes; `git log origin/main..HEAD --diff-filter=D` does "
                         f"not name it, or the tree has it back")

