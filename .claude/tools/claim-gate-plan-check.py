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
    s2 = memo.sect("2")
    if not s2:
        bad("PAIR", "no §2 section found")
        return
    invariants = sorted(set(re.findall(r"\*\*(I\d)\*\*", s2)))
    if len(invariants) < 2:
        bad("PAIR", f"§2 defines {len(invariants)} invariant(s); nothing to pair")
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

def check_section_refs(memo: Memo, umbrella: Memo | None) -> None:
    own = memo.numbers()
    umb = umbrella.numbers() if umbrella else None
    for i, line in enumerate(memo.lines, 1):
        for m in re.finditer(r"(\w[\w'’-]*[ \t]+)?§([\d.]+)", line):
            lead = (m.group(1) or "").strip().lower()
            num = m.group(2).rstrip(".")
            # `("umbrella", "the")` carried a second member that did nothing:
            # on "the §9", the outer test passed, the inner one failed, no
            # `continue` ran, and control reached the same `num not in own` it
            # would have reached with no branch at all.
            if lead == "umbrella":  # "umbrella §6"
                if umb is not None and num not in umb:
                    bad("SECT", f"{memo.path.name}:{i} cites umbrella §{num}, "
                                f"which the umbrella has no heading for")
                continue
            if num not in own:
                bad("SECT", f"{memo.path.name}:{i} cites §{num}, which this memo "
                            f"has no heading for")


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
# 9. SITES -- every line that describes the wire SET, listed for disposal
#
# The TRANSCRIPT check proves a paste matches its command; it cannot prove the
# command was the right one. Round 2's two missed sibling sites are exactly
# that failure -- `ci.yml`'s "the wires are grep-only" and `mise.toml`'s
# enumeration of four disjoint surfaces are both invalidated by a fifth wire,
# and neither contains the string the memo grepped for.
#
# `GOVERNING` is a SEED (`wire_set_lines` states the measurement), and every
# line in the files it names that speaks about the wires *collectively* has to
# be consciously disposed of --
# edited, or cited as deliberately left alone. Per-wire headers are out of
# scope: a new wire only adds to those, it does not falsify them.
#
# This FAILS rather than reports. It shipped as a report, and a report was the
# wrong shape twice over: `check_sites` never called `bad()`, so the one
# mechanism credited with catching what a grep cannot contributed nothing to
# the exit code -- the `ban_control` lesson reproduced in the checker written to
# enforce it. That lesson's words are at `layout-box-reader-trip-wire.sh:472-473`:
# "a ban wire that cannot fire is indistinguishable from a clean tree". This
# passage quoted it as "a ban that cannot fire", against `:276-299` -- a range
# that holds the `ban_control` function body and no such sentence. Two words
# dropped and a range that does not contain the quotation is the third altered
# quotation this program has produced; `git grep -F` decides these.
# --------------------------------------------------------------------------

GOVERNING = [".github/workflows/ci.yml", "CLAUDE.md", "CONTRIBUTING.md",
             "mise.toml", "scripts/trip-wires.sh"]
COLLECTIVE = re.compile(
    r"(every|all|each of|the wires|both directions|disjoint|grep-only|"
    r"cover|layering trip-wires|trip-wires job|no list)", re.I)
WIRE_TOPIC = re.compile(r"trip.?wire|the wires", re.I)


WIRE_FILE = re.compile(r"[\w./-]*-trip-wire\.sh$")


def changes_the_wire_set() -> bool | None:
    """Does THIS BRANCH add or remove a wire? None if the range cannot form.

    The property is the PR's diff, not the memo's prose. It was decided by
    a regex over `memo.text` -- the memo NAMING a wire
    -- which is a gate by name where the comment above it states a gate by
    behaviour. Measured: a four-line memo reading "follow the precedent of
    `layout-box-reader-trip-wire.sh`, which this PR does not touch" collects
    seventeen SITES findings for a wire set it does not change, and
    `docs/plans/2026-07-terminal-z-c3a-impl-plan.md` takes the same seventeen.
    Both probes varied naming and shipping TOGETHER, so the discriminating arm
    -- names one, ships none -- did not exist.
    (memory `feedback_gate-criterion-by-behaviour-not-by-name`.)

    Rename detection used to hide the removal half: a wire renamed out of the
    set reported only its destination, so the branch that took the last wire
    away answered False. `claim_gate_git` disables it for every derivation, for
    that reason at every one of them."""
    paths = claim_gate_git.changed_paths()
    return None if paths is None else any(WIRE_FILE.match(x) for x in paths)


def wire_set_lines() -> list[tuple[str, int, str]]:
    """(path, 1-based line, text) for every governing-file line about the wire SET.

    One site, because the probe file needs the same population to build the
    fixture its SITES probes cite. Written twice, a narrowing here would leave
    both probes citing the old superset and passing against a check that
    changed -- the `ban_control` shape one level up.

    `GOVERNING` is a SEED, not the closed set its first draft called it: the
    same predicate over `git ls-files` matches fourteen tracked files, of which
    several are legitimately out of scope (a per-wire header is not a statement
    about the set). Nothing here tells "deliberately excluded" from "never
    considered", so `check_sites` prints what it did not look at."""
    out: list[tuple[str, int, str]] = []
    for rel in GOVERNING:
        f = REPO / rel
        if not f.exists():
            # A silent `continue` here loses a fifth of the population to a
            # rename, with no finding and no notice.
            skip("SITES", f"{rel} is named as a governing file and is not in the "
                          f"tree, so no line of it was examined")
            continue
        for i, line in enumerate(f.read_text().split("\n"), 1):
            if WIRE_TOPIC.search(line) and COLLECTIVE.search(line):
                out.append((rel, i, line.strip()))
    return out


def report_governing_complement() -> None:
    """What `GOVERNING` does not cover, printed so the seed cannot read as one.

    `wire_set_lines`' docstring said "`check_sites` prints what it did not look
    at" and nothing printed anything: the seed declared itself a seed and then
    asserted a compensating mechanism it did not have, one function away from
    `report_umbrella_coverage`, which does exactly this for `BINDING`. Two
    seed-declaring checks, one honouring the contract and one claiming it.
    (memory `feedback_universal-claims-need-the-complement-measured`.)

    Per-wire headers are out of scope by the rule above, so they are excluded by
    a PREDICATE rather than by absence from a list -- an exclusion nobody can
    read is what makes an enumeration authoritative by default."""
    rc, out = sh(["git", "ls-files"])
    if rc != 0:
        return
    rows: list[tuple[str, int]] = []
    for rel in out.split("\n"):
        if not rel or rel in GOVERNING or WIRE_FILE.match(rel):
            continue
        f = REPO / rel
        if not f.is_file():
            continue
        try:
            text = f.read_text()
        except (UnicodeDecodeError, OSError):
            continue
        n = sum(1 for line in text.split("\n")
                if WIRE_TOPIC.search(line) and COLLECTIVE.search(line))
        if n:
            rows.append((rel, n))
    if rows:
        print(f"  governing-file seed: {len(rows)} tracked file(s) outside "
              f"`GOVERNING` carry a line matching the same predicate, and were "
              f"not examined (per-wire headers are excluded by predicate):")
        for rel, n in sorted(rows, key=lambda r: -r[1]):
            print(f"    {rel} ({n} line(s))")


def check_sites(memo: Memo, ships: bool | None = None) -> None:
    # Round 10: this fired on every memo in the tree, so the prerequisite PR --
    # which lands one PR BEFORE the memo this tool is for -- put a checker on
    # `main` that exits 1 on all 68 of them, for an obligation none of them
    # has. The obligation arises from changing the wire set, not from being a
    # plan-memo, and a check whose population is "every document handed to me"
    # reports a defect that is really a category error.
    ships = changes_the_wire_set() if ships is None else ships
    if ships is None:
        skip("SITES", "`origin/main...HEAD` could not be formed in this clone, so "
                      "whether this PR changes the wire set was not decided")
        return
    if not ships:
        skip("SITES", f"`git diff --name-only origin/main...HEAD` adds no "
                      f"`*-trip-wire.sh`, so this PR does not change the wire set "
                      f"and no line describing that set needs disposing of here "
                      f"({memo.path.name} may still name a wire it does not ship)")
        return
    # Both spellings count as citing a line: the backticked `path:12-14` form,
    # and the bare `path:12:` form a pasted `git grep -n` listing produces --
    # §4.6's edit set is entirely of the second kind, and reading only the first
    # reported eleven sites as undisposed when §4.6 pastes six of them verbatim.
    cited = set()
    for m in CITE_RE.finditer(memo.text):
        rel, a, b = m.group(1), int(m.group(2)), m.group(3)
        for h in resolve(rel):
            for k in range(a, (int(b) if b else a) + 1):
                cited.add(f"{h}:{k}")
    for m in re.finditer(r"^([\w./-]+):(\d+):", memo.text, re.M):
        for h in resolve(m.group(1)):
            cited.add(f"{h}:{m.group(2)}")
    loose = set(re.findall(r"`?([\w./-]+\.(?:yml|toml|md|sh))`?[: ]", memo.text))

    for rel, i, line in wire_set_lines():
        if f"{rel}:{i}" in cited:
            continue
        star = (" (the memo names the file but not this line)"
                if rel.split("/")[-1] in loose else "")
        bad("SITES", f"{rel}:{i} speaks about the wire set and the memo does not "
                     f"cite it{star} -- edit it or cite it as deliberately left "
                     f"alone: {line[:60]}")


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
    codes: set[str] = set()
    for path in shipped_files():
        if path.suffix == ".py":
            codes |= set(re.findall(r'bad\("([A-Z]+)"', path.read_text()))
    derivable["checks"] = len(codes) or None
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
# 12. UMBRELLA -- a slice may not silently contradict a binding decision
#
# Round 4's sharpest finding: adopting §7 Q3 option (c) deleted the fence arm
# from the slice while umbrella D2 still mandates `.rs` fence matching, D3 still
# defines the mention tier as "backticked or fenced", and umbrella §6 still lists
# a rustdoc-fenced control among S1's. The slice's own preamble says it does not
# reopen D1-D10. A withdrawal was swept through the sites that STATE it and not
# through the document that OBLIGES it.
#
# Generalising is not possible from text alone, so this is a registry: each entry
# names a decision, the phrase that shows the umbrella still mandates it, and the
# phrase that shows the slice has withdrawn it. Both present = contradiction.
# --------------------------------------------------------------------------

# (decision, phrase showing the umbrella still MANDATES it,
#  phrase showing the slice has WITHDRAWN it,
#  phrase showing the umbrella has been AMENDED to match)
#
# The amendment marker matters: a binding decision's original text stays as the
# record of what was decided, and the amendment is appended beneath it -- the
# form this umbrella already uses. Without the third column the check would push
# authors to rewrite history instead of amending it, which is the opposite of
# what it is for.
BINDING = [
    ("D2 (.rs fence strip)",
     r"before fence matching in `?\.rs`?|strip a\s*\n?\s*leading `//\[/!\]\?` before fence",
     r"there is no fence arm|The `\.rs` rustdoc strip disappears",
     r"AMENDED by S1 round 4 \(option \(c\)\): the fence half"),
    # RETIRED, round 6, and recorded rather than deleted silently.
    #
    #   ("D3 (mention = backticked or fenced)", r"\*\*mention\*\* \(backticked or fenced", ...)
    #   ("umbrella §6 (S1 must control a rustdoc-fenced annotation)", r"a rustdoc-fenced annotation", ...)
    #
    # Both were inert: their `mandates` patterns stopped matching because the
    # umbrella amended those two decisions by editing the original text IN PLACE
    # and adding a note, not by leaving the original and appending -- which is
    # the form the comment above assumes and which D2 and D4 do follow. An entry
    # that cannot fire is a row that reads as coverage, so they are removed
    # rather than left looking like watchers. Nothing is lost: the text they
    # watched no longer exists, so there is nothing left for a slice to
    # contradict. The self-test's per-entry probes are what surfaced this, and
    # they are what will surface the next one.
    #
    # BINDING is a SEED, not an inventory, and `report_umbrella_coverage` is
    # what keeps it readable as one. Two entries cannot cover ten decisions, and
    # a registry that lists what it watches leaves everything else authoritative
    # by default -- which is not hypothetical here: round 9 withdrew an
    # obligation carried by an umbrella §3 COVERAGE-MAP row, a shape no entry
    # could ever have watched, and the withdrawal ran unamended until review
    # found it. So the complement is computed and printed on every run: every
    # D-decision and every §3 row that names S1, with whether anything watches
    # it. The registry says what it covers; the report says what it does not.
    # (memory `feedback_enumerated-exemptions-leave-the-next-class-authoritative`)
    # Round 6. D4's stated mechanism is `git ls-files` alone, which reports the
    # index; the slice additionally requires a working-tree presence test, which
    # rejects annotations D4-as-written accepts. That is a decision change and it
    # ran four revisions without an amendment -- found by review, not here,
    # because until this entry the registry held ONE withdrawal spread over three
    # rows. Three rows is not three decisions covered: 7 of the 10 D-decisions
    # have no entry, so a green UMBRELLA means "the fence withdrawal is still
    # amended", not "no decision is being silently reversed".
    ("D4 (liveness = `git ls-files` matches >=1)",
     r"`in=` must match ≥1 tracked file or\s*\n?the annotation fails",
     r"present in the working tree|filtered to the paths that exist",
     r"AMENDED by S1 round 6: the precondition is about the working tree"),
]


# Every `**Dn — ...**` heading in the umbrella's §1, and every §3 data row whose
# cells name S1. Derived from the umbrella's own text rather than listed here,
# so a decision or row added there appears in the report without an edit.
UMB_DECISION = re.compile(r"^\*\*(D\d+)\s*—\s*([^*]+?)\.?\*\*", re.M)
NAMES_S1 = re.compile(r"\bS1\b")


def report_umbrella_coverage(umbrella: Memo | None) -> None:
    """What BINDING does not watch, printed so the registry cannot read as one.

    A green UMBRELLA means the seeded entries are not being silently reversed.
    It means nothing about anything else, and this is where that is said."""
    if umbrella is None:
        return
    watched = {name.split()[0] for name, *_ in BINDING}
    rows: list[str] = []
    for m in UMB_DECISION.finditer(umbrella.sect("1")):
        if m.group(1) not in watched:
            rows.append(f"§1 {m.group(1)} — {m.group(2).strip()[:64]}")
    # The amendment marker is APPENDED BENEATH the table, which is the form
    # this umbrella uses and which BINDING's comment above documents -- so the
    # test is on the section, not on the row. Reading the row's own cells
    # reported two already-amended rows as unwatched, half the report's output.
    sect3 = umbrella.sect("3")
    amended_here = "AMENDED" in sect3
    for c in data_rows(sect3):
        joined = " | ".join(c)
        if NAMES_S1.search(joined) and not amended_here:
            rows.append(f"§3 row — {c[2].strip()[:64] if len(c) > 2 else joined[:64]}")
    if rows:
        print(f"  umbrella: {len(rows)} binding item(s) with no BINDING entry "
              f"(the registry is a seed; a withdrawal here is caught by review, "
              f"not by this tool):")
        for r in rows:
            print(f"    {r}")


def check_umbrella(memo: Memo, umbrella: Memo | None) -> None:
    # Invoked with one argv path, this check has nothing to compare against and
    # returned quietly -- and so did the umbrella's own citations, its
    # transcripts, and every `umbrella §N` reference in `check_section_refs`.
    # The whole registry below, whose subject is a withdrawal swept through the
    # sites that STATE it and not the document that OBLIGES it, was disabled by
    # dropping an argument.
    if umbrella is None:
        skip("UMBRELLA", "no umbrella memo on argv: no binding decision was "
                         "compared, and the umbrella's own citations, transcripts "
                         "and `umbrella §N` targets went unchecked with it")
        return
    for name, mandates, withdraws, amended in BINDING:
        if re.search(amended, umbrella.text, re.S):
            continue
        if re.search(mandates, umbrella.text, re.S) and re.search(withdraws, memo.text, re.S):
            bad("UMBRELLA", f"the slice withdraws {name} while the umbrella still "
                            f"mandates it — a binding decision cannot be changed by a "
                            f"slice edit; amend the umbrella in the same commit or "
                            f"withdraw the slice's change")


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
    # Applicability is a property of the TOOL, not an exemption inside one check.
    # Round 10 found `SITES` exiting 1 on every plan-memo in the tree for an
    # obligation none of them has, and fixed SITES. Measured after that fix: all
    # 68 still exited 1, on PAIR ("no §2 section found"), SECT and CITE, which
    # are bound to this memo's schema in exactly the same way. Exempting the one
    # class that was noticed leaves every other class authoritative by default
    # (memory `feedback_enumerated-exemptions-leave-the-next-class-authoritative`),
    # and the remedy was verified on the check rather than re-measured on the
    # tool -- so the category error survived its own fix.
    if not IS_SUBJECT.search(memo.text):
        print(f"claim-gate plan check: {memo.path.name} does not name "
              f"`claim-gate-plan-check`, so it is not a memo this tool is about "
              f"— every check declined")
        return 0
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
