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
            <(grep -o 'bad("[A-Z]*"' .claude/tools/claim-gate-plan-check.py | sort -u)

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
which is exactly where round 2's CRIT 1 lived. ANNO / PASTE / TRANSCRIPT /
FENCE / PREFLIGHT are all of that kind. The sibling is also bound to its memo's
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
  PREFLIGHT   §8.3's pasted verdict is what preflight prints today

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

import importlib.util
import itertools
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

import claim_gate_selftest  # noqa: E402

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
    def __init__(self, path: Path):
        self.path = path
        self.text = path.read_text()
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
            if lead in ("umbrella", "the"):  # "umbrella §6"
                if lead == "umbrella":
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
# This comment used to end "It retires itself -- once the wire is tracked, this
# module shells out to it instead of classifying anything, so the two
# implementations never coexist." That was false, the memo's §0 withdrew it, and
# the comment was not swept with it. The guard below covers ONE function; the
# self-test imports `classify`, `execute` and `verdict_over` directly, so the
# second transcription outlives the wire's arrival and both are reachable in the
# same run. Retirement is a task somebody has to do -- delete this module and
# prove by a control that its grammar is unreachable -- not a property the
# `WIRE.exists()` branch confers. §6 of the memo carries it as a bound, and
# There is no second copy left to keep in agreement.
# --------------------------------------------------------------------------

WIRE = REPO / ".claude" / "tools" / "claim-provenance-trip-wire.sh"


def check_annotations() -> dict | None:
    """The classified tree, or None once the wire owns classification.

    `{}` was the wire-mode return, and PASTE's `if not live: return` read that
    as "nothing to check" -- so the first commit that creates the wire file
    (presence is the whole test; untracked is enough) silently retired PASTE.
    None is not a falsy payload but the ABSENCE of one, which PASTE now has to
    dispose of out loud."""
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
# The governing files are a closed set of five, and every line in them that
# speaks about the wires *collectively* has to be consciously disposed of --
# edited, or cited as deliberately left alone. Per-wire headers are out of
# scope: a new wire only adds to those, it does not falsify them.
#
# This FAILS rather than reports. It shipped as a report, and a report was the
# wrong shape twice over: `check_sites` never called `bad()`, so the one
# mechanism credited with catching what a grep cannot contributed nothing to
# the exit code -- `layout-box-reader-trip-wire.sh:276-299`'s `ban_control`
# ("a ban that cannot fire is indistinguishable from a clean tree") reproduced
# in the checker written to enforce that lesson.
# --------------------------------------------------------------------------

GOVERNING = [".github/workflows/ci.yml", "CLAUDE.md", "CONTRIBUTING.md",
             "mise.toml", "scripts/trip-wires.sh"]
COLLECTIVE = re.compile(
    r"(every|all|each of|the wires|both directions|disjoint|grep-only|"
    r"cover|layering trip-wires|trip-wires job|no list)", re.I)
WIRE_TOPIC = re.compile(r"trip.?wire|the wires", re.I)


# A memo owes the wire-set disposal only if its PR changes the wire set. That is
# the property, and it is decidable: the memo names a `*-trip-wire.sh` it ships.
SHIPS_A_WIRE = re.compile(r"[\w./-]*-trip-wire\.sh")


def check_sites(memo: Memo) -> None:
    # Round 10: this fired on every memo in the tree, so the prerequisite PR --
    # which lands one PR BEFORE the memo this tool is for -- put a checker on
    # `main` that exits 1 on all 68 of them, for an obligation none of them
    # has. The obligation arises from changing the wire set, not from being a
    # plan-memo, and a check whose population is "every document handed to me"
    # reports a defect that is really a category error.
    if not SHIPS_A_WIRE.search(memo.text):
        skip("SITES", f"{memo.path.name} names no `*-trip-wire.sh`, so its PR does "
                      f"not change the wire set and no line describing that set "
                      f"needs disposing of here")
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

    pending = []
    for rel in GOVERNING:
        if not (REPO / rel).exists():
            continue
        for i, line in enumerate((REPO / rel).read_text().split("\n"), 1):
            if WIRE_TOPIC.search(line) and COLLECTIVE.search(line):
                if f"{rel}:{i}" not in cited:
                    pending.append((rel, i, line.strip()))
    for rel, i, line in pending:
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

TOOLING_SUBJECT = re.compile(
    r"claim-gate-plan-check|claim-provenance-trip-wire|claim-provenance-selftest"
    r"|`claim-gate-plan-check`|prerequisite PR|--self-test", re.I)
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
    derivable: dict[str, int | None] = {}
    rc, out = sh(["git", "rev-list", "--count", "origin/main..HEAD"])
    derivable["commits"] = int(out.strip()) if rc == 0 and out.strip() else None
    rc, out = sh(["git", "diff", "--name-only", "origin/main..HEAD"])
    derivable["files"] = len([x for x in out.split("\n") if x]) if rc == 0 else None
    p = REPO / ".claude" / "tools" / "claim-gate-plan-check.py"
    derivable["lines"] = nr(p) if p.exists() else None
    src = p.read_text() if p.exists() else ""
    derivable["checks"] = len(re.findall(r"^def check_\w+", src, re.M)) or None
    derivable["probes"] = None  # only knowable by running --self-test; see below

    for i, line in enumerate(memo.lines, 1):
        if not TOOLING_SUBJECT.search(line):
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

SELFTEST_FILE = REPO / ".claude" / "tools" / "claim-provenance-selftest.sh"
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
# Every file this program ships that can make a claim about itself, which
# includes THIS one and its probe file. Round 9 shipped the rule with a
# source list of [memo, wire, shell self-test]: the checker enforced against
# three files a rule it was breaking in the fourth. A rule whose population
# excludes its own author is the enumerated-exemption shape, one level in.
DEFAULT_COUNT_SOURCES = [WIRE, SELFTEST_FILE, Path(__file__).resolve(),
                         Path(__file__).resolve().parent / "claim_gate_selftest.py"]
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
    _, listing = sh(["bash", str(SELFTEST_FILE)])
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
        c = split_row(line)
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
        skip("CONTROLS", f"{SELFTEST_FILE.name} is absent, or its run did not reach "
                         f"the closing summary that says every control ran — either "
                         f"way no control citation in the memo was resolved against "
                         f"anything")
        return
    retired, ledger_spans = retired_controls(memo)

    for cid, why in sorted(retired.items(), key=lambda kv: int(kv[0][1:])):
        if cid in have:
            bad("CONTROLS", f"§8's retirement ledger lists {cid} ({why.strip()[:60]}…) "
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
                bad("CONTROLS", f"{memo.path.name}:{ln} names {cid}, which §8's "
                                f"ledger retires — the ledger licenses the row "
                                f"that records the retirement, not a citation "
                                f"elsewhere; say what happened without the number")
            continue
        if cid not in have:
            bad("CONTROLS", f"the memo names {cid} as a live control; "
                            f"{SELFTEST_FILE.name} has no such row, and §8's "
                            f"retirement ledger does not list it either")
            continue
        for arm in sorted(arms - have[cid]):
            bad("CONTROLS", f"the memo cites {cid}'s {arm} arm; {SELFTEST_FILE.name} "
                            f"ships {cid} with " +
                            (f"only its {'/'.join(sorted(have[cid]))} arm"
                             if have[cid] else "no arm named either way"))

    # A count of controls is a quantity about this program's own tooling, which
    # §8 forbids in its own voice and the wire's header then carried anyway.
    for path in (sources if sources is not None else DEFAULT_COUNT_SOURCES + [memo.path]):
        if not path.exists():
            continue
        for m in COUNT_OF_CONTROLS.finditer(path.read_text()):
            bad("CONTROLS", f"{path.name} states `{m.group(0)}` — that count moves "
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
    answer as a clean one, which is `skip()`'s whole reason for existing."""
    if sh(["git", "rev-parse", "--verify", "--quiet", "origin/main"])[0] != 0:
        return None
    rc, out = sh(["git", "log", "origin/main..HEAD", "--diff-filter=D",
                  "--name-only", "--format="])
    if rc != 0:
        return None
    return gone_from(out, {x for x in sh(["git", "ls-files"])[1].split("\n") if x})


def check_paths(memo: Memo, gone: list[str] | None = None) -> None:
    gone = deleted_on_branch() if gone is None else gone
    if gone is None:
        skip("PATHS", "`origin/main..HEAD` could not be formed in this clone, so "
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
            bad("PATHS", f"this branch deletes `{path}` and {memo.path.name} still "
                         f"names it; §{GONE_SECTION} owes a row saying what the memo "
                         f"means by it now — the sentences around it were written "
                         f"while the file was there")
        elif not named and path in declared:
            bad("PATHS", f"§{GONE_SECTION} declares `{path}`, and no sentence outside "
                         f"that section names it — the row has outlived its subject")
    for path in declared:
        if path not in gone:
            bad("PATHS", f"§{GONE_SECTION} declares `{path}` as a file this branch "
                         f"removes; `git log origin/main..HEAD --diff-filter=D` does "
                         f"not name it, or the tree has it back")


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
    fields = {
        r"total entries\s+\(M\):\s+(\d+)": r"total entries\s+\(M\):\s+(\d+)",
        r"malformed rows:\s+(\d+)": r"malformed rows:\s+(\d+)",
        r"unmapped-label rows:\s+(\d+)": r"unmapped-label rows:\s+(\d+)",
        r"unique specs \(K\):\s+(\d+)": r"unique specs \(K\):\s+(\d+)",
    }
    for label, rx in fields.items():
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


def main(argv: list[str]) -> int:
    if len(argv) > 1 and argv[1] == "--self-test":
        return claim_gate_selftest.selftest()
    memo = Memo(Path(argv[1]).resolve())
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
