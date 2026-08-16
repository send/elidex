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
  UNIQ        §8.1's `$3` column keeps the property §8's preamble asserts
  SECT        every §-reference resolves, in the memo it is attributed to
  CITE        every `path:LINE` is tracked, unambiguous and in range
  ANNO        every `claim` annotation in the tree is classified and re-run
  PASTE       §8.2's pasted tier counts and verdicts are re-derived
  FENCE       the "A differs from B" claim, decided by running A and B
  TRANSCRIPT  every pasted command is re-run and its pasted answer checked
  SITES       (report) lines describing the wire SET, for conscious disposal
  PREFLIGHT   §8.3's pasted verdict is what preflight prints today

A check that reaches no verdict -- the wire owning classification, a section
absent from the memo, an argv path not passed -- says so through `skip()`, and
the closing line carries that count beside the finding count. The precedent is
`report_transcripts`: a count, the entries, and a reason on each. It exists
because `0 finding(s)` got printed by a run in which PASTE never executed.

ANNO's grammar -- §4.2's tiers and §4.4's verbs, transcribed a second time
because the wire does not own them yet -- is `claim_gate_grammar.py`, in a file
of its own because the memo bounds that transcription and a file is a bound that
can be deleted whole. The firing proof for every check above is
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
# `claim_gate_grammar` is §4.2's tiers and §4.4's verbs transcribed a second
# time, which this file is bound to carry until the wire exists;
# `claim_gate_selftest` is the firing proof for every check below. Neither is a
# library: both call back into this file, for `bad`, `sh`, `nr`, `REPO` and the
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

import claim_gate_grammar as grammar  # noqa: E402
import claim_gate_selftest  # noqa: E402

grammar.host = claim_gate_selftest.host = sys.modules[__name__]

FAILS: list[str] = []

# Non-coverage is REPORTED, never taken silently. `check_annotations` returned
# `{}` the moment the wire file merely existed and `check_pasted_transcripts`
# opened with `if not live: return`, so S1's own first commit -- the one that
# creates the wire -- would have retired the PASTE check with no finding, no
# notice, and a run that still printed `0 finding(s)`. A check that did not run
# is indistinguishable from a check that found nothing, which is §4.3's
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
# 2. UNIQ -- §8.1's `$3` column keeps the property §8's preamble asserts
# --------------------------------------------------------------------------

def check_probe_substrings(memo: Memo) -> None:
    """`$3` discrimination, scoped to the fixture.

    Two controls in separate fixtures cannot mask each other -- each wire
    invocation can only emit its own diagnostics -- so the global uniqueness
    property revision 2 asserted was both false and the wrong requirement.
    What matters is the fixture group, which §8.1 now carries as a column."""
    s81 = memo.sect("8.1")
    if not s81:
        bad("UNIQ", "no §8.1 section found")
        return
    rows = [c for c in data_rows(s81) if re.match(r"^C\d+$", c[0])]
    if not rows:
        bad("UNIQ", "§8.1 has no parseable control rows")
        return
    if not all(len(c) >= 5 for c in rows):
        bad("UNIQ", "§8.1 rows lack the Fixture column the scoped `$3` property "
                    "is decided by")
        return

    # Which fixtures put ONE wire invocation behind each row, and which share one
    # across rows, is decided by §8's Fixture-model table -- so it is READ from
    # that table's own prose rather than transcribed here. `own` was hard-coded
    # and `clone` was not, so when §4.1.1(b) fixed the clone as one invocation
    # per row this check went on enforcing a requirement the memo had withdrawn,
    # for a round, until a new `clone` row collided with C19's `asserted=299`.
    # Transcribing the answer is what made that possible; deriving it cannot
    # drift, and a fixture whose prose says neither is reported rather than
    # guessed at.
    per_row, shared = set(), set()
    for c in data_rows(memo.sect("8")):
        if len(c) < 2:
            continue
        name = c[0].strip("* `")
        if not re.match(r"^[A-Za-z_][A-Za-z0-9_]*$", name):
            continue
        prose = c[1].lower()
        if "one wire invocation per row" in prose or "its own wire invocation" in prose:
            per_row.add(name)
        elif "one fixture carrying" in prose or "in one wire invocation" in prose \
                or "one wire invocation each" in prose:
            shared.add(name)

    # Anything §8's model does not place in one of those two is UNDECIDABLE, and
    # that includes a fixture the table never mentions -- the first draft of this
    # only reported names it found in the table and could classify neither way,
    # so an unlisted fixture fell through to "shared" exactly as `clone` had.
    # Reporting the complement rather than an enumerated exemption is the fix.
    groups: dict[str, dict[str, list[str]]] = {}
    for c in rows:
        fixture = c[2].strip("* `")
        m = re.findall(r"\(`([^`]+)`", c[-1])
        if not m:
            continue
        if fixture not in per_row and fixture not in shared:
            bad("UNIQ", f"§8's Fixture model does not say whether {fixture!r} puts one "
                        f"wire invocation behind each row or shares one across them, so "
                        f"the `$3` scope for {c[0]} is undecidable")
            continue
        key = f"{fixture}:{c[0]}" if fixture in per_row else fixture
        groups.setdefault(key, {}).setdefault(m[-1], []).append(c[0])

    for key, vals in sorted(groups.items()):
        for a in sorted(vals):
            for b in sorted(vals):
                if a != b and a in b:
                    bad("UNIQ", f"in fixture {key!r}, `$3` {a!r} ({','.join(vals[a])}) "
                                f"is inside {b!r} ({','.join(vals[b])}); the first "
                                f"cannot tell its own arm from the second's firing")
        for v, who in sorted(vals.items()):
            if len(who) > 1:
                bad("UNIQ", f"in fixture {key!r}, `$3` {v!r} is shared by "
                            f"{','.join(who)}; neither discriminates its own arm")

    stale = re.search(r"No two `\$3` values below share a substring",
                      memo.sect("8"))
    if stale:
        bad("UNIQ", "§8 still asserts the global `$3` uniqueness property, which "
                    "is the wrong requirement (separate fixtures cannot mask each "
                    "other) and was false where it stood")


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
    """`awk 'END{print NR}'` semantics -- the count §4.4 specifies, and the one
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
# The grammar these two run is `claim_gate_grammar`: §4.2/§4.4 transcribed from
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
# `check_tier_transcription` is what keeps the two copies agreeing until then.
# --------------------------------------------------------------------------

WIRE = REPO / ".claude" / "tools" / "claim-provenance-trip-wire.sh"


def check_annotations() -> dict | None:
    """The classified tree, or None once the wire owns classification.

    `{}` was the wire-mode return, and PASTE's `if not live: return` read that
    as "nothing to check" -- so the first commit that creates the wire file
    (presence is the whole test; untracked is enough) silently retired PASTE.
    None is not a falsy payload but the ABSENCE of one, which PASTE now has to
    dispose of out loud."""
    if WIRE.exists():
        rc, out = sh(["bash", str(WIRE)])  # noqa
        if rc != 0:
            bad("ANNO", f"the wire reds: {out.strip()[:400]}")
        return None
    live, found = grammar.verdict_over(grammar.scan_tree())
    for code, msg in found:
        bad(code, msg)
    return live


def check_pasted_transcripts(memo: Memo, live: dict | None) -> None:
    """Every figure §8.2 pastes as a transcript must be what re-running produces.
    D10's own mechanism reaching the transcript instead of the claim is exactly
    how the three false annotations of v1 survived."""
    if live is None:
        skip("PASTE", "wire mode: the wire owns classification and prints no tier "
                      "counts, so §8.2's pasted figures have nothing to be "
                      "re-derived from — have the wire emit them, or delete §8.2's "
                      "paste along with this check")
        return
    s = memo.sect("8.2")
    if not s:
        skip("PASTE", f"{memo.path.name} has no §8.2 section, so no pasted tier "
                      f"count, lax-candidate count or acceptance row was re-derived")
        return
    m = re.search(r"\{'strict':\s*(\d+),\s*'mention':\s*(\d+)\}", s)
    if m:
        for label, stated in (("strict", int(m.group(1))), ("mention", int(m.group(2)))):
            got = live["tiers"][label]
            if got != stated:
                bad("PASTE", f"§8.2 pastes {label}={stated}; classifying the tracked "
                             f"tree now gives {got}")
    m = re.search(r"claim\[\^A-Za-z\]'\s*\\\n?.*?\|\s*wc -l\n\s*(\d+)", s, re.S)
    if m:
        stated = int(m.group(1))
        got = sum(live["tiers"].values())
        if got != stated:
            bad("PASTE", f"§8.2 pastes a lax-candidate count of {stated}; "
                         f"the scan now finds {got}")
    for m in re.finditer(r"^(OK|FAIL)\s+(count|files|lines)\s+asserted=(\d+)\s+"
                         r"actual=(\d+)\s+in=(\S+)", s, re.M):
        verdict, verb, asserted, actual, spec = m.groups()
        hit = [r for r in live["results"] if r[2]["spec"] == spec and r[2]["verb"] == verb]
        if verdict == "OK" and hit and hit[0][3] != int(actual):
            bad("PASTE", f"§8.2 pastes `{verb} … actual={actual} in={spec}`; "
                         f"re-running gives {hit[0][3]}")
        if verdict == "FAIL" and verb == "lines":
            ok, got = grammar.execute({"verb": verb, "n": int(asserted),
                                       "icase": False, "pattern": None,
                                       "spec": spec})
            if ok and got != int(actual):
                bad("PASTE", f"§8.2's corpus row pastes `{verb} asserted={asserted} "
                             f"actual={actual} in={spec}`; re-running gives {got}")


# --------------------------------------------------------------------------
# 7. FENCE -- the "A differs from B" claim, decided by running A and B
#
# Round 2's CRIT 1: the memo said preflight.py marks a mismatched fence marker
# fenced while grep_pass.py treats it as content, read off `grep_pass.py`'s
# `— content` comment (which means content *of the still-open block*). That
# premise carried the §6 bound accepting a third implementation and all of §7
# Q3. This check does not read either comment.
# --------------------------------------------------------------------------

CORPUS = "\n".join([
    "plain",
    "```",
    "inside a backtick block",
    "~~~",                       # mismatched marker: the disputed case
    "still inside",
    "```",
    "out",
    "~~~",
    "inside a tilde block",
    "```",                       # mismatched the other way
    "still inside",
    "~~~",
    "tail",
    "```",                       # unclosed at EOF
    "runs to the end",
])


def _load(name: str):
    spec = importlib.util.spec_from_file_location(name, SKILL / f"{name}.py")
    assert spec and spec.loader, f"cannot locate {name}.py under {SKILL}"
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def fence_verdict(spans_fn=None) -> tuple[list[int], int]:
    """(lines where the two in-repo trackers disagree, corpus size).

    `spans_fn` overrides the grep_pass side; the self-test feeds it a
    deliberately wrong tracker, because a comparison that structurally cannot
    report a divergence is worth exactly as much as the comment it replaced."""
    pf, gp = _load("preflight"), _load("grep_pass")
    lines = CORPUS.split("\n")
    a = pf._fence_state_array(lines)

    spans = (spans_fn or gp._code_block_spans)(CORPUS)
    b = [False] * len(lines)
    pos = 0
    for i, line in enumerate(lines):
        start, end = pos, pos + len(line)
        b[i] = any(s <= start and end <= e for s, e in spans)
        pos = end + 1
    return [i + 1 for i in range(len(lines)) if a[i] != b[i]], len(lines)


# Any memo line that puts the two trackers and a sameness/difference word
# together is *restating* this verdict. Polarity is deliberately not parsed:
# "they differ" and "they do not differ" are equally forbidden, because a
# checker that reads English polarity is the same unexecuted-claim medium one
# level down. The rule is D10 form 2 applied to a boolean -- a fact the memo's
# own edits can invalidate is stated by reference, not restated.
RESTATES_FENCE = re.compile(
    r"(differ|disagree|divergen|diverge|agree|identical|the same|in step)", re.I)
_A = r"(?:preflight\.py|_fence_state_array)"
_B = r"(?:grep_pass\.py|_code_block_spans|_fenced_spans)"
NAMES_BOTH = re.compile(rf"{_A}.*{_B}|{_B}.*{_A}")
# The load-bearing false claim was spelled "the existing two already differ" --
# it named neither module, so a checker keyed on the two file names would have
# read clean over the very sentence that carried the §6 bound. Match the
# concept, not the spelling (memory
# `feedback_semantic-sibling-selfseed-and-regate-breadth`).
FENCE_TOPIC = re.compile(r"fence", re.I)
PAIR_REF = re.compile(r"\b(the two|both|each other|the existing two|"
                      r"one another|two implementations|two trackers|"
                      r"the other (?:one|implementation|tracker))\b", re.I)


# The ban is SENTENCE-scoped, not line-scoped. It shipped keyed on one physical
# line, and the memo it guards wraps at ~100 columns: the load-bearing sentence,
# "the tier pass is a third fence-tracking implementation, and the existing two
# already differ", escapes a line-scoped ban whenever the wrap falls between "the
# existing two" and "differ" -- exactly the restatement the ban exists to forbid,
# read clean by the checker written to catch it.
#
# The first fix widened the window to the whole paragraph, and running it over
# the memo produced three findings, all false: markdown puts no blank line
# between adjacent table rows or between adjacent bullets, so a paragraph window
# fused §2's I1×I2 row with its I1×I4 row, and fused one bullet's "inside a
# fenced block" with a LATER bullet's sameness word. Widening past the unit the
# claim is actually made in buys the false-positive rate the FENCE_TOPIC +
# PAIR_REF proximity heuristic was designed to avoid. So the window is the
# sentence: wide enough to span a wrap, which is the whole defect, and no wider.
# A blank line still ends it, and so does a structural line -- a table cell and a
# heading cannot wrap, so joining them to a neighbour can only ever fuse.
STRUCTURAL = re.compile(r"^[ \t]*(\||#{1,6}[ \t]|```|~~~)")
# Closing punctuation may follow the stop: the memo's "…is gone** (§7 Q3, §4.2)."
# ends a sentence at `).`, and a bare `(?<=[.!?])\s` boundary does not see it --
# which left the retracted-premise narration on the next line fused to the
# "fence tracker" heading phrase of the sentence before it.
SENT_END = re.compile(r"[.!?][)\]\"'`*_]*\s+")


def windows(lines: list[str]) -> list[tuple[str, list[tuple[int, int]], int]]:
    """(sentence text, [(offset, 1-based line no)], sentence offset).

    Lines are stripped and joined with one space, so a `.*` in NAMES_BOTH spans
    a wrap; the offset table is what turns a match position back into the line
    the offending span STARTS on, which is the line a reader has to edit."""
    out: list[tuple[str, list[tuple[int, int]], int]] = []
    para: list[tuple[int, str]] = []

    def flush() -> None:
        if not para:
            return
        text = " ".join(t for _, t in para)
        idx, pos = [], 0
        for n, t in para:
            idx.append((pos, n))
            pos += len(t) + 1
        start = 0
        for end in [m.end() for m in SENT_END.finditer(text)] + [len(text)]:
            if end > start:
                out.append((text[start:end], idx, start))
            start = end
        para.clear()

    for i, line in enumerate(lines, 1):
        s = line.strip()
        if not s:
            flush()
        elif STRUCTURAL.match(line):
            flush()
            para.append((i, s))
            flush()
        else:
            para.append((i, s))
    flush()
    return out


def line_at(idx: list[tuple[int, int]], off: int) -> int:
    ln = idx[0][1]
    for start, n in idx:
        if start > off:
            break
        ln = n
    return ln


def check_fence(memo: Memo) -> None:
    try:
        diverge, n = fence_verdict()
    except Exception as e:  # pragma: no cover
        bad("FENCE", f"cannot run the two in-repo fence trackers: {e}")
        return
    verdict = (f"disagree on line(s) {diverge} of {n}" if diverge
               else f"agree on all {n}")
    for text, idx, base in windows(memo.lines):
        r = RESTATES_FENCE.search(text)
        if not r:
            continue
        nb = NAMES_BOTH.search(text)
        ft, pr = FENCE_TOPIC.search(text), PAIR_REF.search(text)
        if nb:
            spans = [r.span(), nb.span()]
        elif ft and pr:
            spans = [r.span(), ft.span(), pr.span()]
        else:
            continue
        lo, hi = min(s for s, _ in spans), max(e for _, e in spans)
        first, last = line_at(idx, base + lo), line_at(idx, base + hi - 1)
        # ...and the matches must sit within a WRAP of each other. The defect is
        # exactly one line's worth of widening: a restatement is a local
        # co-occurrence, and stretching the co-occurrence over a whole long
        # sentence is what produced the last false positive on the memo -- §7's
        # compliant citation ("the `FENCE` check now measures that relation by
        # running both …") picked up `the same` four lines later, out of "the
        # same unexecuted-claim medium one level down", a clause about something
        # else. Two physical lines is the span a ~100-column wrap can hide a
        # claim in, and it is the span this ban reads.
        if last - first > 1:
            continue
        bad("FENCE", f"{memo.path.name}:{first} restates how "
                     f"the two fence trackers relate; that fact is measured by "
                     f"this check (they {verdict} lines) and must be cited, "
                     f"not written out: {text[lo:lo + 64]}")


# --------------------------------------------------------------------------
# 8. TRANSCRIPT -- every pasted command is re-run and its pasted answer checked
#
# The memo's provenance contract says every figure "came from a command run in
# this worktree and pasted next to it". That makes the paste a claim, and D10
# says a claim the memo's own landing can invalidate has to be re-executable
# rather than asserted. Round 2 shipped two prose sites missing from a pasted
# `git grep` edit-set listing; re-running the listing is what finds that, and
# no amount of re-reading it was going to.
#
# Only these program names run, and only when the line carries no shell state
# the memo does not also paste. Lines that cannot be re-run are REPORTED as
# unchecked rather than skipped -- a silent cap reads as coverage.
# --------------------------------------------------------------------------

_RUNNABLE_HEAD = r"git (?:grep|ls-files|show|log)|grep|sed|awk"
RUNNABLE = re.compile(rf"^({_RUNNABLE_HEAD}|bash -c)\b")
# `$NF` inside an awk program is not shell state, and a first cut that tested
# the raw line skipped three re-runnable commands as unrunnable -- a cap that
# reads as coverage. Single-quoted spans are removed before the test. `$?` is
# self-contained within one `bash -c`, so it is not state either.
SQUOTED = re.compile(r"'[^']*'")
NEEDS_STATE = re.compile(r"\$[A-Za-z_{#*@]|set --|<prototype|<this memo|<the umbrella")

# A `bash -c` is admitted by WHAT IT DOES, not by the name it starts with.
# Naming the program was the wrong criterion in both directions: it refused the
# memo's `set -euo pipefail` probes -- its highest-value measurements, and the
# ones that overturned a rule five rounds had passed over -- while the reason
# for refusing `git clone` was never the word "clone" but the side effect.
# So every command position inside the script must be a reader. Anything that
# creates, moves or deletes (`mktemp`, `mkdir`, `git init`, `rm`, `cd`, `>`)
# leaves the script unrunnable and REPORTED, which is what keeps the fixture
# blocks out.
_READERS = {
    "git", "grep", "sed", "awk", "wc", "printf", "echo", "true", "false",
    "test", "[", ":", "sort", "uniq", "head", "tail", "cat", "cut", "tr",
    "set", "if", "then", "else", "elif", "fi", "for", "do", "done", "while",
}
_GIT_READERS = {"grep", "ls-files", "show", "log", "rev-parse", "rev-list", "cat-file"}
_SPLIT = re.compile(r"(?:;|\|\||&&|\||\n)")


def bash_is_read_only(script: str) -> bool:
    # Redirection creates; nothing this tool re-runs may.
    if ">" in script:
        return False
    for part in _SPLIT.split(script):
        toks = part.split()
        # An assignment inside the script is not omitted state -- the whole
        # script runs as one unit -- so skip leading assignments to reach the
        # program, and recurse into a command substitution's own head.
        while toks and re.match(r"^[A-Za-z_][A-Za-z0-9_]*=", toks[0]):
            sub = re.match(r"^[A-Za-z_][A-Za-z0-9_]*=\$\((.*)\)?$", toks[0] + " " + " ".join(toks[1:]))
            if sub:
                return bash_is_read_only(sub.group(1).rstrip(")"))
            toks = toks[1:]
        if not toks:
            continue
        head = toks[0].lstrip("(")
        if head not in _READERS:
            return False
        if head == "git" and (len(toks) < 2 or toks[1].lstrip("-") in ("C",)
                              or toks[1] not in _GIT_READERS):
            return False
    return True


def needs_state(cmd: str) -> bool:
    if cmd.startswith("bash -c"):
        m = re.match(r"^bash -c\s+'(.*)'\s*$", cmd, re.S)
        return not (m and bash_is_read_only(m.group(1)))
    return bool(NEEDS_STATE.search(SQUOTED.sub("''", cmd)))
FILELINE = re.compile(r"^([\w./-]+):(\d+):")
ARROW = re.compile(r"^(.*?)[ \t]+(?:→|->)[ \t]+(.*)$")


QUOTE = re.compile(r"^[ \t]*>[ \t]?")


# FENCE_MARK is for the MEMO's own fenced blocks (check_transcripts), which is a
# different subject from an annotation's tier. §4.2 has one inertness rule -- `k`
# backticks before the `<!--`, identical on all four surfaces. The fence arm this
# checker used to carry, with its per-surface table and its `.rs` rustdoc strip,
# is the rule §7 Q3 option (c) deleted; check_tier_transcription is what now
# keeps `claim_gate_grammar`'s copy of §4.2 and the memo's copy from drifting
# apart again.
FENCE_MARK = re.compile(r"^[ \t]*(```|~~~)")


def fenced_blocks(memo: Memo) -> list[tuple[int, list[str]]]:
    """(1-based line of the opener, body lines) for every ```-fenced block.

    A leading `> ` is stripped first: the memo's own status header is a
    blockquote, and its two artifact commands -- the ones the header offers as
    the proof that the memo is machine-checked -- sat outside every block
    because `^[ \\t]*``` ` does not match `> ``` `."""
    out, cur, start = [], None, 0
    for i, line in enumerate(memo.lines, 1):
        line = QUOTE.sub("", line)
        if FENCE_MARK.match(line):
            if cur is None:
                cur, start = [], i
            else:
                out.append((start, cur))
                cur = None
        elif cur is not None:
            cur.append(line)
    return out


def run_line(cmd: str) -> str:
    p = subprocess.run(["bash", "-c", cmd], cwd=REPO, capture_output=True, text=True)
    return (p.stdout or p.stderr).strip()


def compare_arrow(memo: Memo, ln: int, cmd: str, want: str) -> None:
    """One `<cmd> → <answer>` line.

    An empty re-run is a MISMATCH unless the paste is empty too. The first cut
    tested `first not in head`, which is vacuously true when the command prints
    nothing -- so any pasted answer survived the moment its command stopped
    producing output, which is the drift direction the whole program exists to
    catch. `awk '{s+=$NF} END{print s}'` over no input prints an empty line, not
    `0`, and a pasted `→ 0` sailed through on exactly that."""
    got = run_line(cmd)
    head = re.split(r"\s{2,}|\s+[#(]", want)[0].strip()
    if not head:
        return
    first = got.split("\n")[0].strip()
    if not first:
        bad("TRANSCRIPT", f"{memo.path.name}:{ln} pastes `{cmd[:56]}` → {head!r}; "
                          f"re-running prints nothing at all")
        return
    if head not in (first, got) and first not in head:
        bad("TRANSCRIPT", f"{memo.path.name}:{ln} pastes `{cmd[:56]}` "
                          f"→ {head!r}; re-running gives {first!r}")


# A block that builds its own repository is a FIXTURE transcript: its commands
# describe a tree this one is not, so re-running them here compares against the
# wrong world. Recognised by the construction the memo itself specifies for a
# hermetic fixture, and reported as unchecked rather than silently compared --
# a `git grep` inside such a block ran against the real repo and "reproduced"
# nothing, which read as a stale paste.
FIXTURE_BLOCK = re.compile(r"^\s*(?:\S+=\$\()?(?:mktemp|git init)\b", re.M)


def check_transcripts(memo: Memo) -> list[str]:
    """Findings go through `bad`; the lines this could not re-run are RETURNED.

    Returned rather than printed from here, for `verdict_over`'s reason: a probe
    can read a return value, and the arrow form's hole below shipped precisely
    because nothing could assert what this list does and does not contain."""
    unchecked: list[str] = []
    for start, body in fenced_blocks(memo):
        if FIXTURE_BLOCK.search("\n".join(body)):
            unchecked.append(f"{memo.path.name}:{start} (fixture block — builds "
                             f"its own repo, so this tree is not its subject)")
            continue
        i = 0
        while i < len(body):
            ln = start + 1 + i
            # A `$ ` prompt and an ` → ` answer can appear on the SAME line, and
            # the first cut let the `$ `-form branch swallow the arrow into the
            # command string -- silently dropping §4.4's five execution-rule
            # transcripts, the evidence for I3/I5 and D4/D6. Strip the prompt
            # first, then decide the form.
            line = body[i].strip()
            prompt = line.startswith("$ ")
            if prompt:
                line = line[2:].strip()

            # An arrow line whose command is not RUNNABLE used to match neither
            # this branch nor the `$`-form's, so it fell through to `i += 1` and
            # vanished: no comparison, no `unchecked` entry, and the run read as
            # coverage -- the exact opposite of what this check's own header
            # states ("a silent cap reads as coverage"), with the two forms
            # holding opposite policies for one shape. §4.1.1's whole
            # transcript is of that shape (`git clone --local`, and three
            # `git -C "$fixture" …` lines that put an option before the
            # subcommand), and all four were invisible. The remedy is the report,
            # NOT a wider RUNNABLE: cloning a repository to audit a memo is a
            # side effect this tool must not have.
            #
            # A `$ ` prompt still goes to the branch below even when it carries
            # an arrow. That branch owns the output lines beneath the prompt and
            # already reports its own non-runnable commands; taking them here
            # would leave those output lines to be re-read as commands.
            m = ARROW.match(line)
            if m and (not prompt or RUNNABLE.match(m.group(1).strip())):
                cmd, want = m.group(1).strip(), m.group(2).strip()
                if not RUNNABLE.match(cmd):
                    unchecked.append(f"{memo.path.name}:{ln} {cmd[:60]} "
                                     f"(not one of the programs this checker runs)")
                elif needs_state(cmd):
                    unchecked.append(f"{memo.path.name}:{ln} {cmd[:60]} "
                                     f"(needs shell state the memo does not also paste)")
                else:
                    compare_arrow(memo, ln, cmd, want)
                i += 1
                continue

            if prompt:
                cmd = line
                j = i + 1
                while cmd.endswith("\\") and j < len(body):
                    cmd = cmd[:-1] + " " + body[j].strip()
                    j += 1
                out_lines = []
                while j < len(body) and not body[j].strip().startswith("$ "):
                    out_lines.append(body[j])
                    j += 1
                pasted = {m2.group(0) for l2 in out_lines if (m2 := FILELINE.match(l2.strip()))}
                if not RUNNABLE.match(cmd) or needs_state(cmd):
                    why = ("needs shell state the memo does not also paste"
                           if RUNNABLE.match(cmd)
                           else "not one of the programs this checker runs")
                    unchecked.append(f"{memo.path.name}:{ln} {cmd[:60]} ({why})")
                elif pasted:
                    actual = {m2.group(0) for l2 in run_line(cmd).split("\n")
                              if (m2 := FILELINE.match(l2.strip()))}
                    for miss in sorted(actual - pasted):
                        bad("TRANSCRIPT", f"{memo.path.name}:{ln} pastes the output of "
                                          f"`{cmd[:56]}` but omits the hit at {miss}")
                    for extra in sorted(pasted - actual):
                        bad("TRANSCRIPT", f"{memo.path.name}:{ln} pastes a hit at {extra} "
                                          f"that `{cmd[:56]}` no longer produces")
                else:
                    # output that is not a `path:NN:` listing was compared against
                    # an empty set, i.e. passed unconditionally. Say so.
                    body_txt = "\n".join(x for x in out_lines if x.strip())
                    if body_txt.strip():
                        got = run_line(cmd).strip()
                        if got != body_txt.strip():
                            bad("TRANSCRIPT", f"{memo.path.name}:{ln} pastes output for "
                                              f"`{cmd[:56]}` that re-running does not "
                                              f"reproduce ({got.splitlines()[:1]} vs "
                                              f"{body_txt.splitlines()[:1]})")
                i = j
                continue
            i += 1
    return unchecked


def report_transcripts(unchecked: list[str]) -> None:
    """The precedent this file's non-coverage accounting follows.

    The header used to name ONE reason for the whole list -- "they need shell
    state the memo does not also paste" -- which was already false of the
    fixture-block entries beneath it, and would have been false of every entry
    the arrow-form fix adds. The reason moved onto each line, where it is true
    of that line."""
    if unchecked:
        print(f"  transcripts: {len(unchecked)} pasted command(s) not re-run:")
        for u in unchecked:
            print(f"    {u}")


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


def check_sites(memo: Memo) -> None:
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
    branch = "claim-gate-plan-check"
    derivable: dict[str, int | None] = {}
    rc, out = sh(["git", "rev-list", "--count", f"origin/main..{branch}"])
    derivable["commits"] = int(out.strip()) if rc == 0 and out.strip() else None
    rc, out = sh(["git", "diff", "--name-only", f"origin/main..{branch}"])
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
    ("D3 (mention = backticked or fenced)",
     r"\*\*mention\*\* \(backticked or fenced",
     r"one inertness rule|backtick it\b.*all four surfaces|single inertness rule",
     r"the mention tier was \"backticked \*\*or fenced\*\*\""),
    ("umbrella §6 (S1 must control a rustdoc-fenced annotation)",
     r"a rustdoc-fenced annotation",
     r"C2 and C3 \(the `?\.md`? and `?\.rs`? fence arms",
     r"The rustdoc-fenced control was struck"),
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
# 13. FIXTURE -- §8.1's Fixture value must be derivable from the row itself
#
# Round 3 found C10/C11 claiming a hermetic fixture while measuring the real
# tree. Revision 4 closed it by DEFINING the column and did not re-derive the
# assignments, so round 4 found the same rows as a provable contradiction. A
# label a reader must trust is not a specification; a label the row implies is.
# --------------------------------------------------------------------------

LIVE_PREFIX = re.compile(r'in="(crates/|scripts/|\.claude/|\.github/|docs/plans/)')


def check_fixture_column(memo: Memo) -> None:
    for c in data_rows(memo.sect("8.1")):
        if not re.match(r"^C\d+$", c[0]) or len(c) < 5:
            continue
        fixture, row = c[2].strip("* `"), " | ".join(c)
        m = LIVE_PREFIX.search(row)
        if m and fixture == "own":
            bad("FIXTURE", f"§8.1 {c[0]} is Fixture `own` (its own `mktemp -d`) but "
                           f"measures `{m.group(1)}…`, which exists only in this "
                           f"repository — in a fresh `mktemp -d` the liveness probe "
                           f"fires first and the arm asserts a diagnostic it cannot "
                           f"see; the real-tree fixture is `clone` (§4.1.1)")
        if not m and fixture == "clone":
            bad("FIXTURE", f"§8.1 {c[0]} is Fixture `clone` but names no real-tree "
                           f"pathspec, so nothing distinguishes it from a hermetic row")


# --------------------------------------------------------------------------
# 14. TIERS -- this file's copy of §4.2 must agree with §4.2
#
# §0 and §6 record a bound: this checker "carries a second transcription of
# §4.2's tiers and §4.4's verbs" until the wire lands. Nothing held the two
# copies together, and they came apart. Revision 4 adopted §7 Q3 option (c) and
# deleted the fence arm from §4.2; revision 5 amended the umbrella to match;
# `classify()` went on classifying a fenced annotation as `mention`, and a
# --self-test probe ASSERTED that it should -- so the gate certified the rule
# the memo had deleted. Meanwhile BINDING's D2 entry, in this same file, already
# keyed on "there is no fence arm" as proof the slice had withdrawn it. Two
# parts of one checker holding opposite beliefs, green, because the tree carries
# no fenced annotation for either to be wrong about.
#
# A sweep would have caught it, and four revisions of sweeping is what produced
# it. So the bound gets a check rather than a fifth sweep. §3's coverage table
# enumerates the classifier's branches and names each one's tier in its own
# prose; every such branch must have a specimen here, and the specimen's tier --
# decided by running classify() -- must be the tier §3 names. A branch added
# without a specimen fails; a tier changed on one side alone fails.
# --------------------------------------------------------------------------

BRANCH_ID = re.compile(r"^\s*\((?P<id>[ivx]+)\)")
# A branch's tier is what it says a line lands IN, so only the text from the
# first `→` onward is read -- not the condition before it. The first draft of
# this check matched the whole cell and read row (iv), "lax-but-not-strict →
# MALFORMED", as naming both tiers. Everything after the first arrow is kept
# rather than only the last clause, so a row that really does name two tiers
# still trips the ambiguity arm below instead of being silently halved.
TIER_WORD = re.compile(r"\b(mention|strict|MALFORMED)\b")

# One specimen per classifier branch of §3, spelled as the lines a file would
# contain. Exactly one line of each is a candidate; that line's tier is what §3
# has to name. The fenced specimen is the one that drifted, so it stays even
# though §4.2 no longer has an arm for it -- "a fence does nothing" is the rule
# now, and a rule with no probe is what §2's I4xI6 row calls a `ban_control`
# (the name comes from `layout-box-reader-trip-wire.sh:276-299`; §4.3 is the
# pattern-body lint and never mentions it -- the memo carried the same
# mis-citation at two sites and it was corrected there in the same round).
TIER_SPECIMEN: dict[str, list[str]] = {
    "i": ['`<!-- claim count=1 pattern="x" in="a.md" -->`'],
    "ii": ["```", '<!-- claim count=1 pattern="x" in="a.md" -->', "```"],
    "iv": ['<!-- claim cont=3 pattern="x" in="a.md" -->'],
    "vi": [r'<!-- claim count=1 pattern="\d" in="a.md" -->'],
}


def check_tier_transcription(memo: Memo) -> None:
    s3 = memo.sect("3")
    if not s3:
        bad("TIERS", "no §3 section, so the classifier branches this checker "
                     "transcribes have no authority to be checked against")
        return
    named: dict[str, str] = {}
    for c in data_rows(s3):
        if len(c) < 3:
            continue
        m = BRANCH_ID.match(c[2])
        if not m or "→" not in c[2]:
            continue  # no stated outcome, so no tier is claimed
        outcome = c[2].split("→", 1)[1]
        tiers = {t.lower() for t in TIER_WORD.findall(outcome)}
        if not tiers:
            continue  # not a classifier branch -- scan, liveness, argv, registration
        if len(tiers) > 1:
            bad("TIERS", f"§3 branch ({m.group('id')}) names {sorted(tiers)}; a "
                         f"branch landing in more than one tier cannot be decided "
                         f"by one specimen — split the row")
            continue
        named[m.group("id")] = tiers.pop()

    for bid, tier in sorted(named.items()):
        spec = TIER_SPECIMEN.get(bid)
        if spec is None:
            bad("TIERS", f"§3 branch ({bid}) names tier `{tier}` and TIER_SPECIMEN "
                         f"has no specimen for it, so this checker's own copy of "
                         f"§4.2 goes unchecked against the memo's — add one")
            continue
        hits = grammar.classify(spec)
        if len(hits) != 1:
            bad("TIERS", f"TIER_SPECIMEN[{bid!r}] yields {len(hits)} candidate "
                         f"lines; a specimen must yield exactly one")
            continue
        got = hits[0][1]
        if got != tier:
            bad("TIERS", f"§3 branch ({bid}) says such a line lands in `{tier}`; "
                         f"this checker's classify() puts the specimen in `{got}` "
                         f"— the memo and its second transcription of §4.2 "
                         f"disagree, and the tree is only green because no "
                         f"annotation exercises the difference")
    for bid in sorted(set(TIER_SPECIMEN) - set(named)):
        bad("TIERS", f"TIER_SPECIMEN carries a specimen for §3 branch ({bid}), "
                     f"which §3 no longer names as a tier branch")


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
    check_probe_substrings(memo)
    check_section_refs(memo, umbrella)
    check_citations(memo)
    if umbrella:
        check_citations(umbrella)
    live = check_annotations()
    check_pasted_transcripts(memo, live)
    check_fence(memo)
    report_transcripts(check_transcripts(memo))
    if umbrella:
        report_transcripts(check_transcripts(umbrella))
    check_sites(memo)
    check_tooling_quantities(memo)
    check_umbrella(memo, umbrella)
    check_fixture_column(memo)
    check_tier_transcription(memo)
    check_preflight(memo)

    for line in summary("wire" if WIRE.exists() else "bootstrap"):
        print(line)
    return 1 if FAILS else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
