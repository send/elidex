#!/usr/bin/env python3
"""Machine-check the claim-gate plan memos against themselves and against the tree.

Why this exists rather than another prose rule: within this one program, seven
statements that the author believed had been executed landed false, and each
remedy was a rule added to the memo's preamble. Round 2's three CRITs were each
detectable in one command. A rule cannot fix that, because the author believes
the rule was followed -- so the memo gets a checker instead.
(memory `feedback_prose-rules-cannot-fix-unexecuted-claims`.)

Relationship to `.claude/tools/plan-xcheck.py` (the line-box lane's sibling,
which reached this same conclusion independently). This passage used to say that
"the two share no predicates", that what is genuinely common is "about 25 lines
of markdown-table parsing", and that the sibling "declares itself tabular-surface
only: nothing here reads the memo's prose claims". All three were false. The
third was a quotation of a sentence that is not in that file, which says the
opposite -- "Checks 11b/11c DO read prose." Nobody had opened the two side by
side; the passage justifying a duplication was itself the unexecuted-claim shape
the rest of this program exists to catch. So it is stated as commands, with
`$X` the sibling's path (it is not on this branch):

  # the only finding code the two share -- and both spend it on §2's pair table
  comm -12 <(grep -o 'bad("[A-Z]*"' "$X" | sort -u) \
            <(grep -oh 'bad("[A-Z]*"' .claude/tools/claim*.py | sort -u)

The glob is the point. This command named `claim-gate-plan-check.py` alone, and
so enumerated eight of this program's twelve codes -- FENCE and TRANSCRIPT live
in `claim_gate_reruns.py`, CONTROLS and PATHS in `claim_gate_ledgers.py`. The
verdict happens to survive the correction, which is why it went unnoticed
through two splits: a comparison that silently measures part of its population
returns the right answer until it does not.

Note a near-collision the collapse will have to resolve rather than merge: the
sibling's `PATH` reports a cited file that does not exist, this program's
`PATHS` reports a file its own branch deleted, and file existence here is
`CITE`'s. One letter apart, opposite subjects.

This section audits the FAR sibling and says nothing about the near ones.
`preflight.py` and `grep_pass.py` are imported and executed by this program,
and share at least a GFM row splitter, a section-span rule and a line-count
with it. That overlap is unaudited here, and saying so is the honest state
rather than a claim of coverage.

Checked one at a time, what they share is: a `bad(code, msg)` collector emitting
`[CODE] msg`; a GFM data-row splitter that drops the separator row (this one
also honours `\\|`, that one splits naively); a `§`-numbered section splitter
(the splitters themselves do differ -- `## §N.` there, level-aware
`## §N <title>` here); an audit of §2's pairwise table; resolution of a
backticked path citation by SUFFIX against the tracked file set, which both
adopted after the same discovery that a memo cites by basename as often as by
path; a restated quantity re-derived and failed on disagreement (its `K=`/`M=`
against §3's own table, TOOLING's numerals against git); a contradiction spelled
as two co-present markers, with a third "this was deliberate" marker exempting
it (its `MARK`, UMBRELLA's amendment column); and a prose harvest scoped to
exclude non-accounting sentences, which both grew after the unscoped form
returned a wrong verdict (its `NON_LANDING`, `check_pairs`'s "only ACCOUNTING
sites count").

What does NOT overlap is the class this file exists for, and it is the larger
difference. `plan-xcheck.py` imports `re, sys, pathlib` and runs nothing: its
only measurement from outside the document is a `crates/**/*.rs` glob for path
existence. Everything here that matters EXECUTES something -- git, bash,
preflight.py, grep_pass.py -- and compares the answer to what the memo pastes,
which is exactly where round 2's CRIT 1 lived. ANNO / TRANSCRIPT / FENCE /
CONTROLS / PATHS / PREFLIGHT are all of that kind. The sibling is also bound to its memo's
schema -- PR labels, cell numbers, M-rows, flip partitions -- and none of those
name anything in this memo.

The old disposition, "collapse them when a *third* memo wants one", does not
survive its own premise: it was priced off the undercount. One parsing helper is
worth duplicating; the list above is not, and duplicating it is the decision
surface CLAUDE.md *One issue, one way* is about. The precondition
`feedback_one-issue-one-way` asks for -- being able to write down why N existed
-- is already met, and this is it: N=2 because the two were written in parallel
in two lanes, neither author aware the other was solving the same problem. What
still argues for waiting is only landing order, since neither file is on `main`:

  git ls-tree origin/main -- .claude/tools/plan-xcheck.py \
                             .claude/tools/claim-gate-plan-check.py   # prints nothing

Collapsing today would couple this slice's landing to the other branch's review.
So: collapse them once both have landed -- not on a third memo, which is a
trigger that no longer has a measurement behind it.

  PAIR        §2's pairwise enumeration is exhaustive and non-redundant
  CONTROLS    every control the memo names exists, with the arm it names
  SECT        every §-reference resolves, in the memo it is attributed to
  CITE        every `path:LINE` is tracked, unambiguous and in range
  ANNO        every `claim` annotation in the tree is classified and re-run
  FENCE       the "A differs from B" claim, decided by running A and B
  TRANSCRIPT  every pasted command is re-run and its pasted answer checked
  SITES       (report) lines describing the wire SET, for conscious disposal
  PATHS       a file this branch deletes, still named, must be re-declared
  TOOLING     a numeral about this program's own tooling, re-derived from git
  UMBRELLA    a binding umbrella decision the slice may not silently withdraw
  PREFLIGHT   §8.3's pasted verdict is what preflight prints today

That list is the index, so it is checked against the program rather than
recalled: `grep -oh 'bad("[A-Z]*"' .claude/tools/claim*.py | sort -u`. It
carried ten of twelve for as long as TOOLING and UMBRELLA have existed.

A check that reaches no verdict -- the wire owning classification, a section
absent from the memo, an argv path not passed -- says so through `skip()`, and
the closing line carries that count beside the finding count. The precedent is
`report_transcripts`: a count, the entries, and a reason on each. It exists
because `0 finding(s)` got printed by a run in which PASTE never executed.

ANNO does not carry a grammar. It runs the wire, which owns the tiers and the
verbs outright; the second transcription that used to live here as
`claim_gate_grammar.py` was deleted with the module, which is what "a file is a
bound that can be deleted whole" was for. The firing proof for every check above is
`claim_gate_selftest.py`. Both are companions of this one rather than libraries;
the binding block below says how they reach back into it.

Usage:  claim-gate-plan-check.py <s1-memo> [<umbrella-memo>]
        claim-gate-plan-check.py --self-test
Exit 1 if any check fails.
"""
from __future__ import annotations

import ast
import io
import itertools
import os
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SKILL = REPO / ".claude" / "skills" / "elidex-plan-review"
sys.path.insert(0, str(SKILL))  # preflight.py imports grep_pass as a sibling

# --------------------------------------------------------------------------
# the two companions
#
# `claim_gate_selftest` is the firing proof for every check below. It is not a
# library: it calls back into this file, for `bad`, `sh`, `nr`, `REPO` and the
# checks themselves. This file's own name has hyphens and cannot be imported,
# so the dependency is handed over from here -- each companion gets `host`,
# this module -- rather than imported over there.
#
# `bad` crosses as an attribute of `host`, resolved on every call, and `FAILS`
# never crosses at all. `capture()` REBINDS this module's `FAILS` for the
# duration of one check; a call-time `host.bad` sees the replacement, while a
# name bound once to the list itself would keep appending to the list `capture`
# swapped out -- `capture` would return empty and every negative probe would
# silently invert.
# --------------------------------------------------------------------------

sys.path.insert(0, str(Path(__file__).resolve().parent))

# `claim_gate_git` is the lowest layer: every question this program puts to git,
# with one answer per question. It is first because three checks had been asking
# the same question three ways -- that file's header states the convention and
# what the disagreement cost.
import claim_gate_git  # noqa: E402
import claim_gate_selftest  # noqa: E402

claim_gate_git.host = sys.modules[__name__]
claim_gate_selftest.host = sys.modules[__name__]

FAILS: list[str] = []

# Non-coverage is REPORTED, never taken silently. `check_annotations` returned
# `{}` the moment the wire file merely existed and the paste check
# opened with `if not live: return`, so S1's own first commit -- the one that
# creates the wire -- would have retired the PASTE check with no finding, no
# notice, and a run that still printed `0 finding(s)`. A check that did not run
# is indistinguishable from a check that found nothing, which is the memo's
# `ban_control` one level up: it reports a clean tree in exactly the way an
# unexecuted claim reports a true one. So every early return that produces no
# verdict says so through `skip()`, and the summary line carries the count.
SKIPPED: list[str] = []


def bad(code: str, msg: str) -> None:
    FAILS.append(f"[{code}] {msg}")


def skip(code: str, why: str) -> None:
    """One check that produced no verdict, and the condition that stopped it.

    Resolved through this module's globals on every call, like `bad`: the
    self-test's `capture_skipped()` swaps the list out for the duration of one
    check, and a name bound once to the list itself would append where nobody
    reads it -- the inversion `capture`'s own comment describes."""
    SKIPPED.append(f"{code} — {why}")


def sh(cmd: list[str]) -> tuple[int, str]:
    p = subprocess.run(cmd, cwd=REPO, capture_output=True, text=True)
    return p.returncode, p.stdout


def shipped_files() -> list[Path]:
    """Every file this program ships -- the population for any rule about what
    the program may say about itself.

    A GLOB, not a list. Round 9 wrote the list out as [memo, wire, shell
    self-test]: the checker enforced against three files a rule it was breaking
    in the fourth, and a rule whose population excludes its own author is the
    enumerated-exemption shape one level in. The hand-written repair had the
    defect back within two rounds, because each split added a companion and
    nobody edits a list they are not looking at. Two checks consume this --
    CONTROLS' count arm and `check_tooling_quantities` -- so it lives here
    rather than in either of them."""
    return sorted((REPO / ".claude" / "tools").glob("claim[-_]*"))


def finding_codes() -> set[str]:
    """Every code this program can report, read off the calls that report them.

    Three places wanted this set and each derived it again: TOOLING's `checks`
    quantity, the docstring-roster probe, and the decline path in `main`, which
    has to name what it is declining. A fourth derivation is how the roster came
    to carry ten of twelve -- so it is one function, and the population is
    `shipped_files()` rather than this file, which is the narrowing both splits
    already caused once."""
    return {c for p in shipped_files() if p.suffix == ".py"
            for c in re.findall(r'bad\("([A-Z]+)"', p.read_text())}


# --------------------------------------------------------------------------
# markdown surface
# --------------------------------------------------------------------------

HEADING_RE = re.compile(r"^(#{2,4})[ \t]+§([\d.]+)", re.M)
SEP_CELL = re.compile(r"^:?-{2,}:?$")


def split_row(line: str) -> list[str] | None:
    """GFM row cells. Respects `\\|`, which naive `split('|')` mangles -- C9's
    positive cell contains one, and an awk -F'|' pass over this table silently
    drops its diagnostic."""
    t = line.strip()
    if not t.startswith("|"):
        return None
    out: list[str] = []
    cur: list[str] = []
    esc = False
    for ch in t[1:]:
        if esc:
            cur.append(ch)
            esc = False
        elif ch == "\\":
            cur.append(ch)
            esc = True
        elif ch == "|":
            out.append("".join(cur).strip())
            cur = []
        else:
            cur.append(ch)
    tail = "".join(cur).strip()
    if tail:
        out.append(tail)
    return out


def data_rows(text: str) -> list[list[str]]:
    out = []
    for line in text.split("\n"):
        c = split_row(line)
        if c and not all(SEP_CELL.match(x) for x in c if x):
            out.append(c)
    return out


class Memo:
    def __init__(self, path: Path, text: str | None = None):
        """`text` overrides the file, so a synthetic probe memo is built by THIS
        constructor rather than a second one. The probe file used to reach
        `__new__` and rebuild `heads` from its own copy of `HEADING_RE`, which
        left every probe proving a grammar the checks do not use."""
        self.path = path
        self.text = path.read_text() if text is None else text
        self.lines = self.text.split("\n")
        # (level, number, char pos)
        self.heads: list[tuple[int, str, int]] = [
            (len(m.group(1)), m.group(2).rstrip("."), m.start())
            for m in HEADING_RE.finditer(self.text)
        ]

    def numbers(self) -> set[str]:
        return {n for _, n, _ in self.heads}

    def sect(self, num: str) -> str:
        """Body of §num, ending at the next heading of same-or-shallower level."""
        for i, (lvl, n, pos) in enumerate(self.heads):
            if n != num:
                continue
            end = len(self.text)
            for lvl2, _, pos2 in self.heads[i + 1:]:
                if lvl2 <= lvl:
                    end = pos2
                    break
            return self.text[pos:end]
        return ""


# --------------------------------------------------------------------------
# 1. PAIR -- §2's pairwise enumeration is exhaustive and non-redundant
# --------------------------------------------------------------------------

def check_pairs(memo: Memo) -> None:
    # Absence of a pairwise table is a check with no subject, not a memo with a
    # defect. Both of these used to `bad()`, and together with SECT's bare arm
    # that is what made the tool exit 1 on all sixty-nine plan-memos -- which in
    # turn made a TOOL-wide applicability gate look like the remedy, suppressing
    # eleven checks that were working in order to silence two arms that were not.
    s2 = memo.sect("2")
    if not s2:
        skip("PAIR", f"{memo.path.name} has no §2 section, so no pairwise "
                     f"enumeration was audited")
        return
    invariants = sorted(set(re.findall(r"\*\*(I\d)\*\*", s2)))
    if len(invariants) < 2:
        skip("PAIR", f"{memo.path.name} §2 defines {len(invariants)} "
                     f"invariant(s); nothing to pair")
        return
    want = {tuple(sorted(p)) for p in itertools.combinations(invariants, 2)}

    # Only ACCOUNTING sites count: the pairwise table's rows, and the paragraph
    # that discharges the rest as independent. Prose *about* the table -- "two
    # revisions asserted this was complete while it stood at 20" -- names pairs
    # without accounting for them, and counting those turns an honest note about
    # a past miscount into a fresh double-booking finding.
    acct: list[str] = []
    in_indep = False
    for line in s2.split("\n"):
        if re.search(r"remaining \w+ pairs? (?:are|is|were|was) independent", line):
            in_indep = True
        elif in_indep and not line.strip():
            in_indep = False
        if line.lstrip().startswith("|") or in_indep:
            acct.append(line)

    seen: dict[tuple[str, ...], int] = {}
    for m in re.finditer(r"(I\d)[ \t]*[x×][ \t]*(I\d)", "\n".join(acct)):
        key = tuple(sorted((m.group(1), m.group(2))))
        seen[key] = seen.get(key, 0) + 1

    # A §2 that accounts for NO pair is not an incomplete accounting -- it is a
    # memo that never claimed to make one. Tabulating invariants is common; the
    # pairwise obligation is this program's own convention, and reading it into
    # every §2 produced 77 findings across four other-lane memos, all of the
    # form "never accounts for I1×I2". A §2 that accounts for SOME pairs and
    # misses others is still a finding, which is the whole point of the check.
    if not seen:
        skip("PAIR", f"{memo.path.name} §2 tabulates {len(invariants)} invariants "
                     f"and contains no pairwise accounting, so there is nothing "
                     f"to audit for exhaustiveness")
        return

    for p in sorted(want - set(seen)):
        bad("PAIR", f"§2 never accounts for {p[0]}×{p[1]} "
                    f"({len(invariants)} invariants ⇒ {len(want)} pairs required)")
    for p in sorted(set(seen) - want):
        bad("PAIR", f"§2 names {p[0]}×{p[1]}, not a pair of its own invariants")
    for p, n in sorted(seen.items()):
        if p in want and n > 1:
            bad("PAIR", f"§2 accounts for {p[0]}×{p[1]} {n} times "
                        f"(orientation-insensitive: `{p[0]}×{p[1]}` and "
                        f"`{p[1]}×{p[0]}` are one pair)")


# --------------------------------------------------------------------------
# 3. SECT -- every §-reference resolves, in the memo it is attributed to
# --------------------------------------------------------------------------

ATTRIBUTED = re.compile(r"`([\w./-]+\.md)`[ \t]+§([\d.]+)")


def check_section_refs(memo: Memo, umbrella: Memo | None) -> None:
    """Every §-reference resolves, in the document it is attributed to.

    Both documents, not one. `check_citations` had always run over the memo and
    the umbrella; this ran over the memo alone, an asymmetry no comment gave a
    reason for, so a whole document's worth of `§N` was examined by nothing.
    Measured on the real pair before it was fixed: the umbrella cites
    `S1 §4.1.1`, a number it has no heading of its own for, and no run had ever
    looked at that line."""
    umb = umbrella.numbers() if umbrella else None
    for doc in [memo] + ([umbrella] if umbrella else []):
        own = doc.numbers()
        for i, line in enumerate(doc.lines, 1):
            # A reference to ANOTHER document names it the way CITE already
            # names one: a backticked path. Attribution had been a closed list
            # of a single word, `umbrella`, so every other form fell through to
            # "this memo has no such heading" -- and the nine references that
            # surfaced the moment this check began reading the umbrella were
            # cross-document to a one.
            consumed: list[tuple[int, int]] = []
            for m in ATTRIBUTED.finditer(line):
                consumed.append(m.span())
                rel, num = m.group(1), m.group(2).rstrip(".")
                hits = resolve(rel)
                if len(hits) != 1:
                    bad("SECT", f"{doc.path.name}:{i} attributes §{num} to `{rel}`, "
                                f"which names {'no tracked file' if not hits else
                                               f'{len(hits)} tracked files'}")
                elif num not in Memo(REPO / hits[0]).numbers():
                    bad("SECT", f"{doc.path.name}:{i} cites `{rel}` §{num}, which "
                                f"that document has no heading for")
            for m in re.finditer(r"(\w[\w'’-]*[ \t]+)?§([\d.]+)", line):
                if any(a <= m.start(2) < b for a, b in consumed):
                    continue
                lead = (m.group(1) or "").strip().lower()
                num = m.group(2).rstrip(".")
                # `("umbrella", "the")` carried a second member that did
                # nothing: on "the §9", the outer test passed, the inner one
                # failed, no `continue` ran, and control reached the same
                # `num not in own` it would have reached with no branch at all.
                if lead == "umbrella":  # "umbrella §6"
                    if umb is not None and num not in umb:
                        bad("SECT", f"{doc.path.name}:{i} cites umbrella §{num}, "
                                    f"which the umbrella has no heading for")
                    continue
                # The BARE arm, and the one place in this program bound to a
                # per-document convention. Measured, rather than assumed: in
                # `2026-07-app-mode-turn-completion-drain.md` a bare `§7.4.4`
                # is an HTML section and a bare `§4.2` is that memo's own
                # sub-item, written identically, so no textual discriminator
                # exists. Self-numbering is not one either -- 61 of the 69
                # plan-memos carry `## §N` headings of their own and still hold
                # 2364 unresolved bare refs between them.
                #
                # This is a gate by NAME, kept deliberately and scoped to the
                # arm that needs it. `feedback_gate-criterion-by-behaviour-not-
                # by-name` asks for behaviour where behaviour discriminates,
                # and here it is measured not to. What the rule does forbid is
                # what this replaces: the same gate on the whole TOOL, which
                # silenced eleven working checks to quiet this one arm.
                if not IS_SUBJECT.search(doc.text):
                    continue
                if num not in own:
                    bad("SECT", f"{doc.path.name}:{i} cites §{num}, which "
                                f"{doc.path.name} has no heading for")


# --------------------------------------------------------------------------
# 4. CITE -- `path:LINE` / `path:A-B` resolves to a tracked file, in range
# --------------------------------------------------------------------------

CITE_RE = re.compile(r"`([\w./-]+\.(?:sh|py|rs|toml|yml|md)):(\d+)(?:-(\d+))?`")


def nr(path: Path) -> int:
    """`awk 'END{print NR}'` semantics -- the count the wire's `lines` verb uses, and the one
    that differs from `len(text.split('\\n'))` by exactly 1 on every file that
    ends with a newline. Getting this wrong makes every `lines=` verdict off by
    one in the direction that reads as staleness."""
    t = path.read_text()
    return t.count("\n") + (1 if t and not t.endswith("\n") else 0)


_resolved: dict[str, list[str]] = {}


def resolve(rel: str) -> list[str]:
    """Tracked paths a citation names. Memos cite by basename (`preflight.py:87`)
    as often as by path, so a literal miss falls back to a basename search."""
    if rel in _resolved:
        return _resolved[rel]
    hits = [x for x in sh(["git", "ls-files", "--", rel])[1].split("\n") if x]
    if not hits:
        hits = [x for x in sh(["git", "ls-files", "--", f"*/{rel}", rel])[1].split("\n") if x]
    _resolved[rel] = hits
    return hits


def check_citations(memo: Memo) -> None:
    for i, line in enumerate(memo.lines, 1):
        for m in CITE_RE.finditer(line):
            rel, a, b = m.group(1), int(m.group(2)), m.group(3)
            hits = resolve(rel)
            if not hits:
                bad("CITE", f"{memo.path.name}:{i} cites `{rel}:{a}`, not a tracked file")
                continue
            if len(hits) > 1 and not any(h.endswith("/" + rel) and rel.count("/") for h in hits):
                bad("CITE", f"{memo.path.name}:{i} cites `{rel}:{a}`, which names "
                            f"{len(hits)} tracked files ({', '.join(hits[:3])}…); a "
                            f"line number against an ambiguous path is unverifiable")
                continue
            n = nr(REPO / hits[0])
            hi = int(b) if b else a
            if hi > n:
                bad("CITE", f"{memo.path.name}:{i} cites `{rel}:{m.group(2)}"
                            f"{'-' + b if b else ''}` but {hits[0]} has {n} lines")
            if b and int(b) < a:
                bad("CITE", f"{memo.path.name}:{i} cites the inverted range `{rel}:{a}-{b}`")


# --------------------------------------------------------------------------
# 5/6. ANNO + PASTE -- classify and EXECUTE every claim annotation in the tree
#
# The grammar these two run is the wire's own, not a copy: §4.2 transcribed from
# the memo's literal text, so that the memo's own live annotations are
# re-executed now, while the wire that will own this does not yet exist.
#
# This comment used to argue that retirement was a task somebody had to do
# rather than a property `WIRE.exists()` confers, and it named the three
# functions the self-test still imported to prove the second transcription was
# reachable. `git grep` now finds no definition of `classify`, `execute` or
# `verdict_over`: the module holding them is the file this branch deletes, and
# the argument's own subject went with it. (The paragraph correcting a stale
# claim went stale itself, which is the shape §0's rule is about, and it kept
# a mid-sentence splice from the edit that started the correction.) What is
# true is one sentence: no grammar lives here, and ANNO shells out to the wire.
# --------------------------------------------------------------------------

WIRE = REPO / ".claude" / "tools" / "claim-provenance-trip-wire.sh"


def check_annotations() -> None:
    """Run the wire over the tree; the verdict is a finding or a `skip()`.

    It was annotated `-> dict | None` and documented as returning "the
    classified tree" long after both `return`s became `None` and the caller
    stopped reading the value: PASTE consumed that payload, and PASTE is
    retired. A signature describing a payload no path produces is the same
    unexecuted claim one level down."""
    if not WIRE.exists():
        # A missing wire is not a memo defect, and this file lands one PR BEFORE
        # the wire does. Round 9 measured what `bad()` here costs: cloned at this
        # branch, the checker exits 1 on every memo -- so the prerequisite PR
        # would put a tool on `main` that fails for a reason no memo can fix,
        # until S1 lands. The honest verdict is that the check did not run, which
        # is the whole point of `skip()`; making it a finding is the mirror error
        # of taking non-coverage silently, and it is the louder of the two.
        skip("ANNO", f"{WIRE.name} is not in this tree, so no `claim` annotation "
                     f"was re-executed — this checker carries no second "
                     f"implementation to fall back on, by design")
        return None
    rc, out = sh(["bash", str(WIRE)])  # noqa
    if rc != 0:
        bad("ANNO", f"the wire reds: {out.strip()[:400]}")
    return None


# PASTE is retired. It re-derived §8.2's pasted tier counts from the payload
# `check_annotations` returned, and the wire returns none: classification is the
# wire's, and it prints its inventory rather than handing back a dict. The guard
# that made this loud rather than silent did its job -- the self-test asserted,
# so the retirement is a decision taken here instead of a check that stopped
# mattering unnoticed.



# --------------------------------------------------------------------------
# 7/8. FENCE and TRANSCRIPT -- `claim_gate_reruns.py`
#
# The two checks that decide a claim by RUNNING something. Split out at the
# touch-time threshold; the seam is that everything else here decides a
# question by reading the document, and these two answer one the document
# cannot. Re-exported so `main` and the probe file keep reaching them by name.
# --------------------------------------------------------------------------

import claim_gate_reruns  # noqa: E402

claim_gate_reruns.host = sys.modules[__name__]

RUNNABLE = claim_gate_reruns.RUNNABLE
fence_verdict = claim_gate_reruns.fence_verdict
check_fence = claim_gate_reruns.check_fence
fenced_blocks = claim_gate_reruns.fenced_blocks
needs_state = claim_gate_reruns.needs_state
run_line = claim_gate_reruns.run_line
check_transcripts = claim_gate_reruns.check_transcripts
report_transcripts = claim_gate_reruns.report_transcripts


# --------------------------------------------------------------------------
# 9. SITES and 12. UMBRELLA -- `claim_gate_registries.py`
#
# The two checks whose coverage is a HAND-DECLARED registry rather than a
# derived population, and which therefore each print their complement. Split
# out at the touch-time threshold; that file's header states the seam, and
# records the measurement that the pair is the whole population rather than
# the two that were noticed. Re-exported so `main` and the probe file keep
# reaching them by name.
# --------------------------------------------------------------------------

import claim_gate_registries  # noqa: E402

claim_gate_registries.host = sys.modules[__name__]

wire_set_lines = claim_gate_registries.wire_set_lines
report_governing_complement = claim_gate_registries.report_governing_complement
check_sites = claim_gate_registries.check_sites
report_umbrella_coverage = claim_gate_registries.report_umbrella_coverage
check_umbrella = claim_gate_registries.check_umbrella
# `GOVERNING`, `BINDING` and `changes_the_wire_set` are NOT re-exported. The
# first two are the seeds those checks read, and a second name for a registry is
# how a registry acquires a second maintainer. The third is withheld for the
# reason `SELFTEST_FILE` and `deleted_on_branch` are: the probe file SWAPS it,
# and `check_sites` resolves it through the module that defines it, so an alias
# here would be the name a probe rebinds while the check goes on calling the
# real one -- a stub silently ignored and a probe silently green.

# --------------------------------------------------------------------------
# 10. PREFLIGHT -- §8.3's pasted verdict is what preflight prints today
# --------------------------------------------------------------------------

# --------------------------------------------------------------------------
# 11. TOOLING -- a numeral in a sentence about this program's own tooling
#
# Four revisions, four false counts, each in the paragraph crediting the tool:
# "29 probes" (34), "a positive and a negative control for every check" (four of
# ten had none), "It finds 13" (7, then 0), "three commits" (five, and five when
# it was written). Revision 4's remedy was a PROSE RULE -- "this memo states no
# quantity about its own tooling" -- and it was falsified eighteen lines above
# itself. The rule cannot work for the same reason the others could not: the
# author believes it was followed.
#
# So the rule becomes a check. Any numeral in a sentence naming the checker, a
# branch of this program, or the wire is re-derived from git where it can be,
# and flagged as unanchored where it cannot.
# --------------------------------------------------------------------------

def tooling_subject() -> re.Pattern[str]:
    """Sentences whose subject is this program's own tooling.

    DERIVED from `shipped_files()`, not enumerated. The literal alternation this
    replaces named `claim-gate-plan-check` and the two shell artifacts and
    nothing else, so the three modules the branch's own splits created were
    invisible: "`claim_gate_ledgers.py` carries seven checks" -- two false
    figures -- produced no finding. That is the same narrowing-by-split the
    `checks` derivation twenty lines below documents and fixes, left in place in
    the predicate that decides whether `checks` is even consulted, in the edit
    that fixed the other one."""
    names = sorted({p.name for p in shipped_files()}
                   | {p.stem for p in shipped_files()})
    return re.compile("|".join([re.escape(n) for n in names]
                               + ["prerequisite PR", "--self-test"]), re.I)
NUMERAL = re.compile(r"(?<![\w.:/-])(\d+|one|two|three|four|five|six|seven|eight|nine|ten)"
                     r"\s+(commits?|files?|lines?|checks?|probes?|findings?|"
                     r"controls?|stages?|CRITs?|IMPs?)\b", re.I)
WORDNUM = {"one": 1, "two": 2, "three": 3, "four": 4, "five": 5,
           "six": 6, "seven": 7, "eight": 8, "nine": 9, "ten": 10}


def _num(tok: str) -> int | None:
    return WORDNUM.get(tok.lower()) if not tok.isdigit() else int(tok)


def check_tooling_quantities(memo: Memo) -> None:
    # `origin/main..HEAD`, not a named branch. Round 9 keyed both derivations on
    # the literal `claim-gate-plan-check`, so once that branch merges and is
    # deleted -- on `main`, where this file lands -- `commits` and `files` both
    # resolve to None and every quantity they were re-deriving silently becomes
    # "nothing re-derives it". A check whose subject disappears when its own PR
    # lands is a `ban_control` with a delayed fuse. HEAD always resolves, and on
    # `main` the range is empty, which is the honest answer there.
    subject = tooling_subject()
    derivable: dict[str, int | None] = {}
    derivable["commits"] = claim_gate_git.commit_count()
    # Three dots. This read `origin/main..HEAD` -- a two-dot DIFF, which
    # compares tips -- so files `main` gained after the branch point counted as
    # files this branch changed, and the figure drifted upward every time main
    # moved. A decaying number, inside the check that exists to catch decaying
    # numbers, in the one derivation whose sibling two lines up is two-dot for a
    # reason that does not transfer. `claim_gate_git` states the convention.
    _paths = claim_gate_git.changed_paths()
    derivable["files"] = None if _paths is None else len(_paths)
    # `checks` is the set of finding CODES across every file the program ships,
    # not `^def check_` in one of them. Both splits narrowed this silently: the
    # entry point held ten checks before FENCE and TRANSCRIPT moved out and
    # eight after CONTROLS and PATHS did, while the program went on having
    # twelve -- so a memo stating the true figure was RED for being correct,
    # and the check that exists to catch a stale number was the one carrying
    # it. The population is the same glob the control-count arm uses.
    derivable["checks"] = len(finding_codes()) or None
    # `lines` and `probes` have no entry, and that is the verdict rather than an
    # omission: a line count over a multi-file program depends on which files
    # you chose to count, which is exactly the convention-dependent quantity the
    # memo's D10 REMOVES rather than re-measures, and a probe count is only
    # knowable by running `--self-test`. Both fall to the "nothing re-derives
    # it" arm below, which says so. `derivable["probes"] = None` used to be
    # written out explicitly; `.get()` already answers None for every absent
    # unit, so the line stated a decision it was not making.

    for i, line in enumerate(memo.lines, 1):
        if not subject.search(line):
            continue
        for m in NUMERAL.finditer(line):
            n, unit = _num(m.group(1)), m.group(2).lower().rstrip("s")
            if n is None:
                continue
            want = derivable.get(unit + "s")
            if want is None:
                bad("TOOLING", f"{memo.path.name}:{i} states {m.group(0)!r} about this "
                               f"program's own tooling, and nothing re-derives it — "
                               f"state it as a re-runnable command instead")
            elif n != want:
                bad("TOOLING", f"{memo.path.name}:{i} states {m.group(0)!r}; git says "
                               f"{want} {unit}(s)")


# --------------------------------------------------------------------------
# 13/14. CONTROLS and PATHS -- `claim_gate_ledgers.py`
#
# The two checks whose population is enumerated by something other than the
# document, and whose escape hatch is a positive declaration read by position.
# Split out at the touch-time threshold; that file's header states the four
# properties they share, which is the seam. Re-exported so `main` and the probe
# file keep reaching them by name.
# --------------------------------------------------------------------------

import claim_gate_ledgers  # noqa: E402

claim_gate_ledgers.host = sys.modules[__name__]

# `SELFTEST_FILE` is deliberately NOT re-exported. The two sites that swap it
# reach `claim_gate_ledgers.SELFTEST_FILE`, because `implemented_controls`
# resolves it through that module -- an alias here would publish exactly the
# name that module's header warns rebinding does nothing to.
implemented_controls = claim_gate_ledgers.implemented_controls
check_controls = claim_gate_ledgers.check_controls
gone_from = claim_gate_git.gone_from
# `deleted_on_branch` is withheld for the same reason as `SELFTEST_FILE`, and
# was published here anyway: the rule was stated for both names and applied to
# one. A probe rebinding it through this alias changed a name `check_paths`
# never resolves, so the stub was ignored and the check ran the real git
# derivation while the probe went green. It now lives in `claim_gate_git` and is
# still withheld -- moving a name does not change which module a swap must
# target, only which module that is.
check_paths = claim_gate_ledgers.check_paths


# --------------------------------------------------------------------------
# 15. TIERS -- RETIRED, and deleted rather than left as data.
#
# It read §3's Branch column and decided each branch by classifying a specimen
# through this file's second copy of the tier rules -- the copy that no longer
# exists. It cannot be re-pointed at the wire: the wire acquires annotations by
# scanning a tree and has no per-string entry point, so a specimen has nowhere
# to go. What it enforced now lives where it executes: the self-test carries a
# both-directions control per surface and per tier.
#
# Round 9's first pass retired the check and left `BRANCH_ID`, `TIER_WORD` and
# `TIER_SPECIMEN` defined and unreferenced -- a retired mechanism's data still
# reading as live specification, which is the same shape as the memo citing a
# control that is gone. `dead code は接続するか削除`.
# --------------------------------------------------------------------------


def check_preflight(memo: Memo) -> None:
    s = memo.sect("8.3")
    if not s:
        skip("PREFLIGHT", f"{memo.path.name} has no §8.3 section, so preflight's "
                          f"verdict was not compared against any paste")
        return
    rc, out = sh([sys.executable, str(SKILL / "preflight.py"),
                  str(memo.path.relative_to(REPO))])
    # A seed, not preflight's whole output: it also prints `heading line`,
    # `parsed citations`, `citation verify` and `split decision`, all pasteable
    # into §8.3 and none of them compared here. Stated because an enumeration
    # that does not say it is one reads as coverage. This was written as a dict
    # mapping each regex to ITSELF, so the "label" in the diagnostic below was
    # the raw pattern -- `§8.3 pastes \`unique specs \\(K\\):\\s+(\\d+)\`` at a reader.
    fields = [
        ("total entries (M)", r"total entries\s+\(M\):\s+(\d+)"),
        ("malformed rows", r"malformed rows:\s+(\d+)"),
        ("unmapped-label rows", r"unmapped-label rows:\s+(\d+)"),
        ("unique specs (K)", r"unique specs \(K\):\s+(\d+)"),
    ]
    for label, rx in fields:
        ms, mo = re.search(rx, s), re.search(rx, out)
        if ms and mo and ms.group(1) != mo.group(1):
            bad("PREFLIGHT", f"§8.3 pastes `{label}` = {ms.group(1)}; preflight "
                             f"now prints {mo.group(1)}")
    ms = re.search(r"grep-pass:\s+(\d+) hard, (\d+) soft", s)
    mo = re.search(r"grep-pass:\s+(\d+) hard, (\d+) soft", out)
    if ms and mo and ms.groups() != mo.groups():
        bad("PREFLIGHT", f"§8.3 pastes grep-pass {ms.group(1)} hard / {ms.group(2)} "
                         f"soft; preflight now prints {mo.group(1)} / {mo.group(2)}")
    ms = re.search(r"EXIT=(\d+)", s)
    if ms and int(ms.group(1)) != rc:
        bad("PREFLIGHT", f"§8.3 pastes EXIT={ms.group(1)}; preflight now exits {rc}")


# --------------------------------------------------------------------------


# --------------------------------------------------------------------------
# Arm coverage -- the one universal this program still stated in prose
#
# `claim_gate_selftest.py` opens "A positive and a negative for every check",
# and it was false: traced, ten `bad()` sites and three `skip()` sites of
# forty-two were reached by no probe. Each is this program's own `ban_control` --
# an arm that reports a clean tree exactly as an unexecuted claim reports a true
# one -- and the claim asserting otherwise is the shape the whole program exists
# to catch, one level up, in the file that enforces it.
#
# A prose rule cannot fix that, for the reason the module docstring gives: the
# author believes it was followed. So it becomes a check. The population is the
# AST -- every call to `bad`/`skip` -- rather than a grep or a hand list, and
# coverage is decided by RUNNING the probes under a tracer rather than by
# reading them. `UNPROBED` below is the positive-declaration hatch, the same
# idiom §8's retirement ledger and §0.1's removal ledger use, and it fails in
# both directions: an entry that turns out to BE reached is a finding too.
# --------------------------------------------------------------------------

ARM_COVERAGE_ENV = "CLAIM_GATE_ARM_COVERAGE"

# site -> why no probe reaches it. Every entry is a bound, stated here rather
# than left as silence.
UNPROBED: dict[str, str] = {}


def arm_sites() -> dict[tuple[str, int], str]:
    """Every `bad()` / `skip()` call site this program ships, by AST.

    Not by regex: `host.bad(...)` and `bad(...)` are the same call through two
    spellings, and a line-oriented pattern also has to guess at continuations."""
    out: dict[tuple[str, int], str] = {}
    for f in shipped_files():
        if f.suffix != ".py":
            continue
        for n in ast.walk(ast.parse(f.read_text())):
            if not isinstance(n, ast.Call):
                continue
            fn = n.func
            name = (fn.attr if isinstance(fn, ast.Attribute)
                    else fn.id if isinstance(fn, ast.Name) else None)
            if name not in ("bad", "skip"):
                continue
            # `bad(code=..., msg=...)` matches the real signature, and a
            # `not n.args` guard made it invisible to the population -- and to
            # the DOCSTRING roster too, which keys on a POSITIONAL literal.
            # (Spelling that regex out here put its own match in the file, and
            # the roster check reported the comment as an unlisted code. Same
            # shape as a `git grep` needle written into the file that greps.)
            a = next((k.value for k in n.keywords if k.arg == "code"),
                     n.args[0] if n.args else None)
            code = a.value if isinstance(a, ast.Constant) else "?"
            out[(f.name, n.lineno)] = f"{name}({code})"
    return out


def unreached_arms() -> list[str]:
    """`file:line name(CODE)` for every arm the probes do not reach.

    Runs the probes under `sys.settrace`. The nested `selftest()` sees
    `ARM_COVERAGE_ENV` set and declines to shell out again, which is the same
    guard-inheritance hazard `C18` records for the wire: a self-test that
    invokes the thing invoking it recurses unless the inner one can tell."""
    sites = arm_sites()
    fired: set[tuple[str, int]] = set()

    def tracer(frame, event, arg):
        # CALL events, not `line`. Keying on line execution counts a site as
        # reached whenever its LINE runs, so `False and bad(...)`, a
        # comprehension over an empty sequence, and a one-line `if`/`for` body
        # all reported covered by zero probes -- a false green in the one check
        # whose whole value is that a false green cannot happen. A call event
        # fires only when the function is actually entered, and
        # `frame.f_back.f_lineno` is the line that entered it.
        if event == "call" and frame.f_code.co_name in ("bad", "skip"):
            back = frame.f_back
            if back is not None:
                fired.add((Path(back.f_code.co_filename).name, back.f_lineno))
        return None

    buf, saved = io.StringIO(), sys.stdout
    sys.stdout = buf
    sys.settrace(tracer)
    try:
        claim_gate_selftest.selftest()
    finally:
        sys.settrace(None)
        sys.stdout = saved
    return [f"{f}:{ln} {w}" for (f, ln), w in sorted(sites.items())
            if (f, ln) not in fired]


def summary(mode: str) -> list[str]:
    """The run's closing lines: what was found, and what was not looked at.

    Split out of `main` so the self-test can prove a skipped check reaches the
    output. It printed the finding count alone, which is how wire mode could
    retire PASTE and still close with `0 finding(s)` -- a number a reader takes
    as "everything ran and everything was clean"."""
    out = [f"claim-gate plan check ({mode}): {len(FAILS)} finding(s), "
           f"{len(SKIPPED)} check(s) not run"]
    out += [f"  {f}" for f in FAILS]
    out += [f"  not run: {s}" for s in SKIPPED]
    return out


# The memo this program is FOR names it: §0's status block pastes the two
# commands that establish the gate. Nothing else in `docs/plans/` does.
IS_SUBJECT = re.compile(r"claim-gate-plan-check")


def main(argv: list[str]) -> int:
    if len(argv) > 1 and argv[1] == "--self-test":
        return claim_gate_selftest.selftest()
    if len(argv) > 1 and argv[1] == "--arm-coverage":
        os.environ[ARM_COVERAGE_ENV] = "1"
        for line in unreached_arms():
            print(line)
        return 0
    memo = Memo(Path(argv[1]).resolve())
    # Applicability is per CHECK, not per tool. The gate that used to stand
    # here declined every check on any memo not naming this program, and the
    # measurement that removed it is in `check_section_refs`: exactly one arm
    # is bound to a per-document convention, and eleven checks were being
    # silenced to quiet it. `CITE` alone reports 645 citations across the
    # tree's plan-memos, sampled and genuine -- a path into a crate this
    # repository deleted, a basename resolving to three files -- and none of
    # them were reachable while the tool refused to look.
    #
    # Each check that can lack a subject now says so through `skip()`: PAIR on
    # a memo with no §2, PREFLIGHT on one with no §8.3, SITES on a PR that
    # changes no wire, and so on. That is the same mechanism, one level in,
    # where it can be true of one check rather than asserted of all twelve.
    umbrella = Memo(Path(argv[2]).resolve()) if len(argv) > 2 else None

    check_pairs(memo)
    # UNIQ and FIXTURE policed §8.1's control table. (Round 9's first draft of
    # this comment said "UNIQ, PROBE and FIXTURE". There has never been a
    # PROBE check in this tree -- `git log -S'PROBE'` matches only the commit
    # that added the word. A retirement list with an invented member reads as
    # coverage history and inflates what the collapse retired.) That table is now
    # `claim-provenance-selftest.sh`, and the property they enforced -- a
    # negative arm's diagnostic substring must discriminate its own failure
    # within its invocation -- is enforced there by construction: st_probe
    # rejects a run that failed for the wrong reason. Two homes for one rule was
    # the shape this program exists to collapse, so the prose-side set retired
    # with the prose. They are DELETED rather than kept callable against a
    # section no memo has: a check gated on an absent heading never fires and
    # never reports, which is the coverage-shaped silence `skip()` exists to
    # prevent. CONTROLS is what replaces them, one level up.
    check_section_refs(memo, umbrella)
    check_citations(memo)
    if umbrella:
        check_citations(umbrella)
    check_annotations()
    check_fence(memo)
    report_transcripts(check_transcripts(memo))
    if umbrella:
        report_transcripts(check_transcripts(umbrella))
    check_sites(memo)
    report_governing_complement()
    check_tooling_quantities(memo)
    check_umbrella(memo, umbrella)
    report_umbrella_coverage(umbrella)
    check_controls(memo)
    check_paths(memo)
    check_preflight(memo)

    for line in summary("wire" if WIRE.exists() else "bootstrap"):
        print(line)
    return 1 if FAILS else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
