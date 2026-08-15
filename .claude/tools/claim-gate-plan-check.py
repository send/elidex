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

FAILS: list[str] = []


def bad(code: str, msg: str) -> None:
    FAILS.append(f"[{code}] {msg}")


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

    groups: dict[str, dict[str, list[str]]] = {}
    for c in rows:
        fixture = c[2].strip("* `")
        m = re.findall(r"\(`([^`]+)`", c[-1])
        if not m:
            continue
        # a fixture named `own` is a DIFFERENT fixture per control, so each such
        # row is its own group; anything else is a shared fixture id.
        key = f"own:{c[0]}" if fixture == "own" else fixture
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
# This is a bootstrap: it is the grammar of §4.2/§4.4 transcribed from the
# memo's literal text, so that the memo's own live annotations are re-executed
# now, while the wire that will own this does not yet exist. It retires itself
# -- once `.claude/tools/claim-provenance-trip-wire.sh` is tracked, this module
# shells out to it instead of classifying anything, so the two implementations
# never coexist.
# --------------------------------------------------------------------------

WIRE = REPO / ".claude" / "tools" / "claim-provenance-trip-wire.sh"

LAX = re.compile(r"<!--[ \t]*claim[^A-Za-z]")
STRICT = re.compile(
    r"<!--[ \t]+claim[ \t]+"
    r"(?:"
    r"(?P<cf>count|files)=(?P<n1>[0-9]+)[ \t]+(?:(?P<icase>icase)[ \t]+)?"
    r'pattern="(?P<cfpat>[^"]*)"[ \t]+in="(?P<cfin>[^"]*)"'
    r"|"
    r'lines=(?P<n2>[0-9]+)[ \t]+in="(?P<lnin>[^"]*)"'
    r")"
    r"[ \t]+-->"
)
# FENCE_MARK is for the MEMO's own fenced blocks (check_transcripts), which is a
# different subject from an annotation's tier. §4.2 has one inertness rule -- `k`
# backticks before the `<!--`, identical on all four surfaces. The fence arm this
# file used to carry, with its per-surface table and its `.rs` rustdoc strip, is
# the rule §7 Q3 option (c) deleted; check_tier_transcription is what now keeps
# this copy of §4.2 and the memo's copy from drifting apart again.
FENCE_MARK = re.compile(r"^[ \t]*(```|~~~)")
BAD_ESCAPE = re.compile(r"\\[bBdDsSwW<>]")


def classify(lines: list[str]) -> list[tuple[int, str, dict | None]]:
    """(line-number, tier, fields) for every candidate line in one file.

    Takes no filename: §4.2's inertness rule is a line predicate identical on
    every surface, so nothing here is per-file. That is structural rather than
    incidental -- a surface argument is precisely what a second arm would need."""
    out = []
    for i, line in enumerate(lines):
        m = LAX.search(line)
        if not m:
            continue
        k = line[: m.start()].count("`")
        if k % 2 == 1:
            out.append((i + 1, "mention", None))
            continue
        sm = STRICT.search(line)
        if not sm:
            out.append((i + 1, "malformed", None))
            continue
        g = sm.groupdict()
        if g["cf"]:
            f = {"verb": g["cf"], "n": int(g["n1"]), "icase": bool(g["icase"]),
                 "pattern": g["cfpat"], "spec": g["cfin"]}
        else:
            f = {"verb": "lines", "n": int(g["n2"]), "icase": False,
                 "pattern": None, "spec": g["lnin"]}
        esc = BAD_ESCAPE.search(f["pattern"]) if f["pattern"] is not None else None
        if esc:
            out.append((i + 1, "malformed", {"why": f"pattern contains {esc.group(0)}"}))
            continue
        out.append((i + 1, "strict", f))
    return out


def execute(f: dict) -> tuple[bool, int | str]:
    _, out = sh(["git", "ls-files", "--", f["spec"]])
    live = [x for x in out.split("\n") if x]
    if f["verb"] == "lines":
        if len(live) != 1:
            return False, f"in= resolves to {len(live)} tracked files, not exactly 1"
        return True, nr(REPO / live[0])
    if not live:
        return False, "in= matches no tracked file"
    flag = "-nE" if f["verb"] == "count" else "-lE"
    if f["icase"]:
        flag += "i"
    _, out = sh(["git", "grep", flag, "-e", f["pattern"], "--", f["spec"]])
    return True, len([x for x in out.split("\n") if x])


def scan_tree() -> dict[str, list[tuple[int, str, dict | None]]]:
    _, out = sh(["git", "grep", "-lE", r"<!--[[:space:]]*claim[^A-Za-z]", "--",
                  "*.md", "*.rs", "*.sh", "*.toml", ":(exclude).claude/tools/**"])
    res = {}
    for rel in [x for x in out.split("\n") if x]:
        res[rel] = classify((REPO / rel).read_text().split("\n"))
    return res


# Split from `check_annotations` so the verdict is a PURE function of an
# already-scanned tree. It was inlined behind `scan_tree()`, which reads the real
# repository, and three of its arms -- the MALFORMED report, the asserted-vs-
# actual mismatch, and §8.2's per-verb acceptance property -- were therefore
# unprobeable: firing any of them meant planting a bad annotation in a tracked
# file. All three shipped with no firing proof at all, which is the `ban_control`
# state §4.3 names, in the check that exists to execute the memo's own claims.
def verdict_over(tree: dict[str, list[tuple[int, str, dict | None]]]) -> dict:
    tiers = {"strict": 0, "mention": 0, "malformed": 0}
    verbs: set[str] = set()
    results = []
    for rel, hits in sorted(tree.items()):
        for ln, tier, f in hits:
            tiers[tier] += 1
            if tier == "malformed":
                why = (f or {}).get("why", "does not match the §4.4 production")
                bad("ANNO", f"{rel}:{ln} classifies MALFORMED ({why})")
            if tier != "strict" or f is None:
                continue
            verbs.add(f["verb"])
            ok, got = execute(f)
            if not ok:
                bad("ANNO", f"{rel}:{ln} {f['verb']}={f['n']}: {got}")
                continue
            results.append((rel, ln, f, got))
            if got != f["n"]:
                bad("ANNO", f"{rel}:{ln} asserts {f['verb']}={f['n']}, "
                            f"actual={got}  in={f['spec']}")

    # §8.2's acceptance property, executed rather than asserted.
    for v in ("count", "files", "lines"):
        if v not in verbs:
            bad("ANNO", f"§8.2's property requires ≥1 strict `{v}` annotation "
                        f"in the tracked tree; there are none")
    return {"tiers": tiers, "results": results}


def check_annotations() -> dict:
    if WIRE.exists():
        rc, out = sh(["bash", str(WIRE)])  # noqa
        if rc != 0:
            bad("ANNO", f"the wire reds: {out.strip()[:400]}")
        return {}
    return verdict_over(scan_tree())


def check_pasted_transcripts(memo: Memo, live: dict) -> None:
    """Every figure §8.2 pastes as a transcript must be what re-running produces.
    D10's own mechanism reaching the transcript instead of the claim is exactly
    how the three false annotations of v1 survived."""
    if not live:
        return
    s = memo.sect("8.2")
    if not s:
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
            ok, got = execute({"verb": verb, "n": int(asserted), "icase": False,
                               "pattern": None, "spec": spec})
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

RUNNABLE = re.compile(r"^(git (?:grep|ls-files|show|log)|grep|sed|awk)\b")
# `$NF` inside an awk program is not shell state, and a first cut that tested
# the raw line skipped three re-runnable commands as unrunnable -- a cap that
# reads as coverage. Single-quoted spans are removed before the test. `$?` is
# self-contained within one `bash -c`, so it is not state either.
SQUOTED = re.compile(r"'[^']*'")
NEEDS_STATE = re.compile(r"\$[A-Za-z_{#*@]|set --|<prototype|<this memo|<the umbrella")


def needs_state(cmd: str) -> bool:
    return bool(NEEDS_STATE.search(SQUOTED.sub("''", cmd)))
FILELINE = re.compile(r"^([\w./-]+):(\d+):")
ARROW = re.compile(r"^(.*?)[ \t]+(?:→|->)[ \t]+(.*)$")


QUOTE = re.compile(r"^[ \t]*>[ \t]?")


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


def check_transcripts(memo: Memo) -> None:
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

            m = ARROW.match(line)
            if m and RUNNABLE.match(m.group(1).strip()):
                cmd, want = m.group(1).strip(), m.group(2).strip()
                if needs_state(cmd):
                    unchecked.append(f"{memo.path.name}:{ln} {cmd[:60]}")
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
                    unchecked.append(f"{memo.path.name}:{ln} {cmd[:60]}")
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
    if unchecked:
        print(f"  transcripts: {len(unchecked)} pasted command(s) not re-run "
              f"(they need shell state the memo does not also paste):")
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
]


def check_umbrella(memo: Memo, umbrella: Memo | None) -> None:
    if umbrella is None:
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
# now, and a rule with no probe is what §4.3 calls a `ban_control`.
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
        hits = classify(spec)
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

# --------------------------------------------------------------------------
# self-test -- a positive and a negative for every check above.
#
# §4.3 of the memo records why: keying two controls on a shared diagnostic let
# each pass when the *other* arm fired, and a ban that cannot fire is
# indistinguishable from a clean tree. A checker whose own checks are never
# shown to fire is that same transcript one level up.
# --------------------------------------------------------------------------

SYNTH_PAIRS = "\n".join(
    ["## §2 Coupled invariants", ""]
    + [f"| **I{i}** | invariant {i} | forces something |" for i in range(1, 5)]
    + [""]
    + [f"| I{a} × I{b} | intersection |" for a, b in itertools.combinations(range(1, 5), 2)]
)

SYNTH_UNIQ = """## §8 Test strategy
**`$3` a substring the diagnostic must contain.**

### §8.1 Controls

| # | clause | Fixture | Positive | Negative |
|---|---|---|---|---|
| C1 | a | live | p | n (`alpha`) |
| C2 | b | live | p | n (`bravo`) |
"""


def capture(fn, *a) -> list[str]:
    global FAILS
    saved, FAILS = FAILS, []
    try:
        fn(*a)
        return FAILS
    finally:
        FAILS = saved


def capture_live() -> dict:
    """The ANNO payload PASTE consumes, without emitting ANNO findings."""
    saved, out = FAILS[:], check_annotations()
    del FAILS[:]
    FAILS.extend(saved)
    return out


def write_probe(text: str) -> Memo:
    """A probe memo on disk -- `check_preflight` shells out to preflight.py with
    the memo's path, so a purely in-memory Memo cannot exercise it."""
    p = REPO / "docs" / "plans" / "zz-probe-preflight.md"
    p.write_text(text)
    return Memo(p)


# The smallest document preflight will parse: a §3 heading, a schema-shaped
# table, one data row. Enough to make it print a verdict a paste can disagree
# with, and nothing else.
PREFLIGHT_FIXTURE = """# probe

## §3 Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| `scripts/trip-wires.sh` §1 (the driver) | discover | (i) glob | `WIRE_GLOB` | ✓ | no |

## §8.3 Preflight
"""


def run_preflight_block(fixture: str) -> str:
    """Preflight's own verdict lines for `fixture`, as a fenced block a memo
    could paste. Derived by running the tool, so the positive probe cannot go
    stale for the same reason these checks exist."""
    m = write_probe(fixture)
    rc, out = sh([sys.executable, str(SKILL / "preflight.py"),
                  str(m.path.relative_to(REPO))])
    keep = [ln for ln in out.split("\n")
            if re.search(r"total entries|malformed rows|unmapped-label|"
                         r"unique specs|grep-pass:", ln)]
    return "\n```\n" + "\n".join(keep) + f"\nEXIT={rc}\n```\n"


def pending_sites() -> list[str]:
    """`rel:line` for every governing-file line that speaks about the wire set."""
    out = []
    for rel in GOVERNING:
        if not (REPO / rel).exists():
            continue
        for i, line in enumerate((REPO / rel).read_text().split("\n"), 1):
            if WIRE_TOPIC.search(line) and COLLECTIVE.search(line):
                out.append(f"{rel}:{i}")
    return out


def synth(text: str, name: str = "synthetic.md") -> Memo:
    m = Memo.__new__(Memo)
    m.path = Path(name)
    m.text = text
    m.lines = text.split("\n")
    m.heads = [(len(x.group(1)), x.group(2).rstrip("."), x.start())
               for x in re.finditer(r"^(#{2,4})[ \t]+§([\d.]+)", text, re.M)]
    return m


def selftest() -> int:
    probes: list[tuple[str, bool, str]] = []  # (label, ok, detail)

    def probe(label: str, want_fire: bool, found: list[str], needle: str = "") -> None:
        fired = bool(found)
        ok = fired == want_fire and (not needle or any(needle in f for f in found))
        probes.append((label, ok, "; ".join(found)[:120] or "(clean)"))

    # PAIR
    probe("PAIR positive (all C(4,2) present once)", False,
          capture(check_pairs, synth(SYNTH_PAIRS)))
    probe("PAIR negative (a pair dropped)", True,
          capture(check_pairs, synth(SYNTH_PAIRS.replace("| I2 × I4 | intersection |\n", "")
                                     .replace("| I2 × I4 | intersection |", ""))),
          "I2×I4")
    probe("PAIR negative (a pair restated in the other orientation)", True,
          capture(check_pairs, synth(SYNTH_PAIRS + "\n| I3 × I1 | again |")),
          "2 times")
    probe("PAIR positive (prose about the table is not an accounting site)", False,
          capture(check_pairs, synth(
              SYNTH_PAIRS + "\n\nAn earlier draft carried I1×I3 twice and omitted "
              "I2×I4 altogether, which is why this is checked.")))
    probe("PAIR positive (the independent-list paragraph does count)", False,
          capture(check_pairs, synth(
              SYNTH_PAIRS.replace("| I2 × I4 | intersection |", "")
              + "\n\nThe remaining one pair is independent, for a stated reason:\n"
                "**I2×I4** (they act on disjoint stages).\n")))

    # UNIQ
    probe("UNIQ positive (disjoint `$3` in one fixture)", False,
          capture(check_probe_substrings, synth(SYNTH_UNIQ)))
    probe("UNIQ negative (one `$3` inside another, same fixture)", True,
          capture(check_probe_substrings, synth(SYNTH_UNIQ.replace("`bravo`", "`alphabet`"))),
          "is inside")
    probe("UNIQ negative (two controls share a `$3`, same fixture)", True,
          capture(check_probe_substrings, synth(SYNTH_UNIQ.replace("`bravo`", "`alpha`"))),
          "shared by")
    # the whole point of scoping: the identical collision across fixtures is fine
    probe("UNIQ positive (same `$3`, separate `own` fixtures)", False,
          capture(check_probe_substrings, synth(
              SYNTH_UNIQ.replace("`bravo`", "`alpha`").replace("| live |", "| own |"))))
    probe("UNIQ negative (the withdrawn global property restated)", True,
          capture(check_probe_substrings, synth(
              SYNTH_UNIQ.replace("must contain.**",
                                 "must contain.** No two `$3` values below share a substring."))),
          "wrong requirement")
    probe("UNIQ negative (no Fixture column)", True,
          capture(check_probe_substrings, synth(
              SYNTH_UNIQ.replace("| Fixture ", "").replace("| live ", "")
              .replace("|---|---|---|---|---|", "|---|---|---|---|"))),
          "lack the Fixture column")

    # SECT
    probe("SECT positive (a defined section)", False,
          capture(check_section_refs, synth("## §4 Design\nsee §4 above\n"), None))
    probe("SECT negative (an undefined section)", True,
          capture(check_section_refs, synth("## §4 Design\nsee §9 below\n"), None),
          "§9")

    # CITE
    probe("CITE positive (in range)", False,
          capture(check_citations, synth("see `scripts/trip-wires.sh:1` there")))
    probe("CITE negative (past EOF)", True,
          capture(check_citations, synth("see `scripts/trip-wires.sh:999999` there")),
          "lines")
    probe("CITE negative (untracked)", True,
          capture(check_citations, synth("see `scripts/no-such-file.sh:1` there")),
          "not a tracked file")

    # ANNO -- classifier tiers, then a deliberately wrong live measurement
    tiers = [t for _, t, _ in classify([
        '<!-- claim count=1 pattern="x" in="a.md" -->',            # strict
        '`<!-- claim count=1 pattern="x" in="a.md" -->`',          # mention, odd k
        '<!-- claim cont=3 pattern="x" in="a.md" -->',             # malformed verb
        r'<!-- claim count=1 pattern="\d" in="a.md" -->',          # malformed dialect
        "```",
        '<!-- claim count=1 pattern="x" in="a.md" -->',            # STRICT: §4.2
        "```",                                                     # has no fence arm
    ])]
    # These three went in as bare `probes.append`, which bypasses `probe()` and so
    # asserts no diagnostic substring -- and the tier list was the only ANNO arm
    # any of them reached. The tier detail moves into the label so it stays
    # visible; the two that assert a REJECTION carry needles.
    want = ["strict", "mention", "malformed", "malformed", "strict"]
    probe(f"ANNO classifier tiers {tiers}", False,
          [] if tiers == want else [f"classify() gives {tiers}, not {want}"])
    ok, got = execute({"verb": "count", "n": 999999, "icase": False,
                       "pattern": "fn ", "spec": "crates/**/*.rs"})
    probe("ANNO executor measures rather than echoing the assertion", True,
          [f"execute() returns {got}, not the asserted 999999"]
          if ok and got != 999999 else [], "not the asserted 999999")
    ok, got = execute({"verb": "lines", "n": 0, "icase": False,
                       "pattern": None, "spec": "crates/**/*.rs"})
    probe("ANNO `lines=` rejects a multi-file pathspec", True,
          [str(got)] if not ok else [], "not exactly 1")

    # The three arms `verdict_over` was extracted for. Each fixture's asserted
    # value is DERIVED by executing it, so the positive cannot go stale for the
    # reason this whole program exists; the negatives mutate that derived value
    # rather than hard-coding a second one.
    def live_anno(verb: str, pattern: str | None, spec: str) -> dict:
        f = {"verb": verb, "n": 0, "icase": False, "pattern": pattern, "spec": spec}
        ok, got = execute(f)
        assert ok, f"ANNO fixture `{verb} in={spec}` does not execute: {got}"
        f["n"] = got
        return f

    true_hits = [(1, "strict", live_anno("count", "REQUIRED_WIRES", "scripts/*.sh")),
                 (2, "strict", live_anno("files", "REQUIRED_WIRES", "scripts/*.sh")),
                 (3, "strict", live_anno("lines", None, "scripts/trip-wires.sh"))]
    probe("ANNO positive (every annotation re-runs to its asserted value)", False,
          capture(verdict_over, {"probe.md": true_hits}))
    moved = dict(true_hits[2][2])
    moved["n"] += 7
    probe("ANNO negative (a strict annotation the tree has since moved past)", True,
          capture(verdict_over, {"probe.md": true_hits[:2] + [(3, "strict", moved)]}),
          f"asserts lines={moved['n']}, actual={true_hits[2][2]['n']}")
    probe("ANNO negative (a candidate line classifying MALFORMED, with a reason)", True,
          capture(verdict_over,
                  {"probe.md": true_hits + [(9, "malformed", {"why": "pattern contains \\d"})]}),
          "probe.md:9 classifies MALFORMED (pattern contains \\d)")
    probe("ANNO negative (a MALFORMED line with no reason recorded)", True,
          capture(verdict_over, {"probe.md": true_hits + [(9, "malformed", None)]}),
          "does not match the §4.4 production")
    probe("ANNO negative (§8.2's property: a verb with no strict annotation)", True,
          capture(verdict_over, {"probe.md": true_hits[:2]}),
          "requires ≥1 strict `lines` annotation")

    # FENCE -- the measurement, its ability to report a divergence, and the ban
    diverge, n = fence_verdict()
    probes.append((f"FENCE measurement runs (corpus {n} lines)", n > 0,
                   f"diverge={diverge}"))

    def wrong_spans(content: str):
        """A tracker that lets a MISMATCHED marker close the block -- the exact
        behaviour the memo attributed to `grep_pass.py`. If the real one did
        this, the comparison must say so."""
        out, start, inside = [], 0, False
        for m in re.finditer(r"^[ \t]*(```|~~~)", content, re.M):
            if not inside:
                inside, start = True, m.start()
            else:
                inside = False
                out.append((start, m.end()))
        if inside:
            out.append((start, len(content)))
        return out

    mutant, _ = fence_verdict(wrong_spans)
    probes.append(("FENCE measurement reports a planted divergence",
                   bool(mutant), f"diverge={mutant}"))
    both = "`preflight.py` and `grep_pass.py` %s on the mismatched marker"
    for polarity in ("differ", "do not differ", "agree"):
        probe(f"FENCE ban fires on a restatement ({polarity!r})", True,
              capture(check_fence, synth("## §6 x\n" + both % polarity)),
              "must be cited")
    # the shape that actually shipped: neither module named
    probe("FENCE ban fires on an unnamed-pair restatement", True,
          capture(check_fence, synth(
              "## §6 x\nThe tier pass is a third fence-tracking implementation, "
              "and the existing two already differ.")),
          "must be cited")
    probe("FENCE ban ignores a line naming only one tracker", False,
          capture(check_fence, synth("## §6 x\n`preflight.py` tracks fences.")))
    # The hole Fix A closed: the SAME sentence, wrapped where the memo wraps it.
    # Before the ban was widened to a paragraph this returned no findings at all,
    # and the needle pins the reported line to where the span STARTS (line 3),
    # not to the line the sameness word happens to land on (line 4).
    probe("FENCE ban fires on a restatement wrapped across two lines", True,
          capture(check_fence, synth(
              "## §6 x\n\nThe tier pass would be a third fence-tracking "
              "implementation, and the existing two\nalready differ on the "
              "mismatched marker.")),
          "synthetic.md:3 restates")
    # ...and the boundary that keeps the widening honest. These two halves are one
    # sentence when the break between them is a wrap and two windows when it is a
    # blank line; the first carries the topic and the pair reference with no
    # sameness word, the second the sameness word alone. The break is the whole
    # difference, so BOTH directions are probed -- a window rule whose fixture
    # could not fuse in the first place proves nothing about the boundary.
    halves = ("## §6 x\n\nThe fence arm was struck, so the existing two "
              "implementations are%sno longer in step.")
    probe("FENCE ban does not fuse two halves across a paragraph break", False,
          capture(check_fence, synth(halves % "\n\n")))
    probe("FENCE ban does fuse the same two halves across a wrap", True,
          capture(check_fence, synth(halves % "\n")),
          "must be cited")
    # Found by running the widened ban over the memo: markdown puts no blank line
    # between adjacent table rows, so a paragraph-wide window fused §2's I1×I2
    # row ("silently converts strict → mention") with its I1×I4 row and reported
    # a restatement neither row makes. A table cell cannot wrap, so a row is
    # always its own window.
    probe("FENCE ban does not fuse two adjacent table rows", False,
          capture(check_fence, synth(
              "## §2 x\n\n| I1 × I2 | An over-eager rule converts strict → "
              "mention, so the fenced case is where it shows.\n"
              "| I1 × I4 | The reject-list runs on a body already parsed, and "
              "the two arms must not agree here.\n")))
    # Also found by running it over the memo: §7's COMPLIANT sentence -- the one
    # that cites this check instead of restating it -- is four lines long, and a
    # sentence-wide window read its "the same unexecuted-claim medium" clause as
    # the sameness word for the "`FENCE` check … by running both" clause. The
    # markers must sit within a wrap of each other, so this is clean while the
    # two-line fixture above fires.
    probe("FENCE ban does not reach a sameness word three lines away", False,
          capture(check_fence, synth(
              "## §7 x\n\nThe `FENCE` check now measures that relation by "
              "running both, and forbids this memo from restating the verdict "
              "in\neither polarity — a boolean the memo's own edits can "
              "invalidate is what D10 form 2 says to cite rather than write\n"
              "out, and a checker that read English polarity would be the same "
              "unexecuted-claim medium one level down.")))
    # TRANSCRIPT -- both arms, both directions
    def block(*body: str) -> Memo:
        return synth("\n".join(["```", *body, "```"]))

    real = "git ls-files -- '.claude/tools/*.sh' | wc -l"
    n = run_line(real)
    probe("TRANSCRIPT arrow form, correct answer", False,
          capture(check_transcripts, block(f"{real}   → {n}")))
    probe("TRANSCRIPT arrow form, stale answer", True,
          capture(check_transcripts, block(f"{real}   → {int(n) + 7}")),
          "re-running gives")
    listing = "git grep -n 'REQUIRED_WIRES' -- 'scripts/*.sh'"
    hits = [x for x in run_line(listing).split("\n") if x]
    probe("TRANSCRIPT $-form, complete listing", False,
          capture(check_transcripts, block(f"$ {listing}", *hits)))
    probe("TRANSCRIPT $-form, a site dropped from the listing", True,
          capture(check_transcripts, block(f"$ {listing}", *hits[1:])),
          "omits the hit at")
    probe("TRANSCRIPT $-form, a site that no longer exists", True,
          capture(check_transcripts, block(f"$ {listing}", *hits,
                                           "scripts/gone.sh:42:REQUIRED_WIRES")),
          "no longer produces")
    probe("TRANSCRIPT skips a command needing unpasted shell state", False,
          capture(check_transcripts, block('git grep -lE -e alpha -- "$p" → 2')))
    probe("TRANSCRIPT skips a fixture block instead of comparing it here", False,
          capture(check_transcripts, block(
              "git init -q .; printf 'alpha\\n' > a.txt; git add -A",
              "git grep -c alpha -- a.txt   → a.txt:2")))
    probe("TRANSCRIPT still compares a non-fixture block in the same shape", True,
          capture(check_transcripts, block(f"{real}   → {int(n) + 7}")),
          "re-running gives")
    probes.append(("TRANSCRIPT `$NF` in an awk program is not shell state",
                   not needs_state("git grep -cE 'x' -- '*.rs' | awk -F: "
                                   "'{s+=$NF} END{print s}'"), ""))

    # PASTE / PREFLIGHT / SITES -- these three shipped with NO probe at all,
    # while the docstring above and the memo's header both claimed a positive
    # and a negative for every check. Three `bad()`-emitting checks were
    # therefore in the state §4.3 names: indistinguishable from a clean tree.
    # Each is built from what the tree says right now, so the probes hold in
    # any worktree and never require the memo this tool audits to be present --
    # that memo is a different slice, and a probe that reads it would re-create
    # the bundling the carve-out removed.
    live = capture_live()
    t = live["tiers"]
    paste = ("## §8.2 Live-tree acceptance\n\n```\n"
             f"--- {{'strict': {t['strict']}, 'mention': {t['mention']}}}\n```\n")
    probe("PASTE positive (tier counts matching the tree)", False,
          capture(check_pasted_transcripts, synth(paste), live))
    probe("PASTE negative (a tier count mutated)", True,
          capture(check_pasted_transcripts,
                  synth(paste.replace(f"'strict': {t['strict']}", "'strict': 99")), live),
          "pastes strict=99")

    sites = sorted(pending_sites())
    def disposal(rows):
        return "## §4.6 Disposal\n\n```\n" + "\n".join(f"{r}: x" for r in rows) + "\n```\n"
    probe("SITES positive (every wire-set line cited)", False,
          capture(check_sites, synth(disposal(sites))))
    probe("SITES negative (one disposal dropped)", True,
          capture(check_sites, synth(disposal(sites[1:]))),
          "speaks about the wire set")

    real_pf = run_preflight_block(PREFLIGHT_FIXTURE)
    probe("PREFLIGHT positive (a pasted verdict matching the tool)", False,
          capture(check_preflight, write_probe(PREFLIGHT_FIXTURE + real_pf)))
    probe("PREFLIGHT negative (a soft-finding count mutated)", True,
          capture(check_preflight, write_probe(
              PREFLIGHT_FIXTURE
              + re.sub(r"(\d+) hard, (\d+) soft", r"\1 hard, 99 soft", real_pf))),
          "preflight now prints")
    (REPO / "docs" / "plans" / "zz-probe-preflight.md").unlink(missing_ok=True)

    # TOOLING / UMBRELLA / FIXTURE -- the three classes round 4 found recurring.
    # Probed here at the same time they were written, because shipping a check
    # without a firing proof is the defect the same round found in ANNO.
    n_commits = sh(["git", "rev-list", "--count", "origin/main..claim-gate-plan-check"])[1].strip()
    probe("TOOLING positive (a count git agrees with)", False,
          capture(check_tooling_quantities,
                  synth(f"The prerequisite PR has {n_commits} commits.")))
    probe("TOOLING negative (a count git contradicts)", True,
          capture(check_tooling_quantities,
                  synth(f"The prerequisite PR has {int(n_commits) + 3} commits.")),
          "git says")
    probe("TOOLING negative (a quantity nothing can re-derive)", True,
          capture(check_tooling_quantities,
                  synth("`claim-gate-plan-check` closed six CRITs.")),
          "nothing re-derives it")
    probe("TOOLING ignores a numeral in a sentence not about the tooling", False,
          capture(check_tooling_quantities, synth("The umbrella has ten decisions.")))

    umb_mandates = synth("**mention** (backticked or fenced → ignore).", "umb.md")
    umb_silent = synth("mention is whatever the slice says.", "umb.md")
    slice_withdraws = synth("There is one inertness rule: backtick it, on all four surfaces.")
    probe("UMBRELLA positive (umbrella silent on the withdrawn rule)", False,
          capture(check_umbrella, slice_withdraws, umb_silent))
    probe("UMBRELLA positive (umbrella mandates, slice does not withdraw)", False,
          capture(check_umbrella, synth("The tier pass follows D3."), umb_mandates))
    probe("UMBRELLA negative (slice withdraws what the umbrella mandates)", True,
          capture(check_umbrella, slice_withdraws, umb_mandates),
          "binding decision cannot be changed by a slice edit")
    umb_amended = synth('**mention** (backticked or fenced → ignore).\n\n'
                        'the mention tier was "backticked **or fenced**"', "umb.md")
    probe("UMBRELLA positive (the umbrella carries the amendment)", False,
          capture(check_umbrella, slice_withdraws, umb_amended))

    FIX = ("## §8.1 Controls\n\n| # | clause | Fixture | Positive | Negative |\n"
           "|---|---|---|---|---|\n")
    probe("FIXTURE positive (a hermetic row with a fixture-local pathspec)", False,
          capture(check_fixture_column,
                  synth(FIX + '| C1 | a | own | `in="a.md"` | wrong count (`asserted=`) |\n')))
    probe("FIXTURE negative (`own` row measuring the real tree)", True,
          capture(check_fixture_column,
                  synth(FIX + '| C1 | a | own | `in="crates/**/*.rs"` | x (`asserted=`) |\n')),
          "exists only in this repository")
    probe("FIXTURE positive (a `clone` row measuring the real tree)", False,
          capture(check_fixture_column,
                  synth(FIX + '| C1 | a | clone | `in="crates/**/*.rs"` | x (`asserted=`) |\n')))
    probe("FIXTURE negative (`clone` row naming no real-tree pathspec)", True,
          capture(check_fixture_column,
                  synth(FIX + '| C1 | a | clone | `in="a.md"` | x (`asserted=`) |\n')),
          "names no real-tree pathspec")

    probe("FENCE ban ignores fence prose with no sameness claim", False,
          capture(check_fence, synth(
              "## §6 x\nThe fence arm is defined per surface; only a matching "
              "marker closes the block, in both implementations.")))

    # TIERS -- the class that produced this check: the memo deleted the fence
    # arm and this file kept it, with a probe asserting the deleted behaviour.
    # The negative below is that exact state, replayed.
    def s3(*branches: str) -> Memo:
        head = ("## §3 Spec coverage map\n\n"
                "| Spec section | Step | Branch | Touch | Full enum? | User-input flow |\n"
                "|---|---|---|---|---|---|\n")
        return synth(head + "".join(
            f"| `u.md` §1 | classify | {b} | §4.2 | ✓ | no |\n" for b in branches))

    TRUE_3 = ("(i) odd *k* backticks before the `<!--` → mention",
              "(ii) inside a fenced block with even *k* → **strict, and executed**",
              "(iv) lax-but-not-strict → MALFORMED, fail",
              r"(vi) `\` + `[bBdDsSwW<>]` → MALFORMED, names the escape")
    probe("TIERS positive (every branch's tier is what classify() decides)", False,
          capture(check_tier_transcription, s3(*TRUE_3)))
    probe("TIERS negative (§3 keeps the withdrawn fence arm)", True,
          capture(check_tier_transcription,
                  s3(TRUE_3[0], "(ii) inside a fenced block with even *k* → mention",
                     *TRUE_3[2:])),
          "second transcription of §4.2")
    probe("TIERS negative (a tier branch with no specimen)", True,
          capture(check_tier_transcription,
                  s3(*TRUE_3, "(ix) a new arm → mention")),
          "TIER_SPECIMEN has no specimen")
    probe("TIERS negative (a specimen for a branch §3 dropped)", True,
          capture(check_tier_transcription, s3(*TRUE_3[:3])),
          "§3 no longer names as a tier branch")
    probe("TIERS negative (a branch naming two tiers)", True,
          capture(check_tier_transcription,
                  s3(*TRUE_3, "(ix) `.md` → mention, everything else → strict")),
          "landing in more than one tier")
    probe("TIERS ignores a non-classifier branch", False,
          capture(check_tier_transcription,
                  s3(*TRUE_3, "(vii) every value as one argv element, no `eval`")))
    probe("TIERS reads the outcome, not the condition (`not-strict`)", False,
          capture(check_tier_transcription,
                  s3(TRUE_3[0], TRUE_3[1],
                     "(iv) a strict-looking line that is lax-but-not-strict → "
                     "MALFORMED, fail", TRUE_3[3])))

    width = max(len(p[0]) for p in probes)
    failed = 0
    for label, ok, detail in probes:
        if not ok:
            failed += 1
        print(f"  {'ok  ' if ok else 'FAIL'}  {label:<{width}}  {detail}")
    print(f"self-test: {len(probes) - failed}/{len(probes)} probes ok")
    return 1 if failed else 0


def main(argv: list[str]) -> int:
    if len(argv) > 1 and argv[1] == "--self-test":
        return selftest()
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
    check_transcripts(memo)
    if umbrella:
        check_transcripts(umbrella)
    check_sites(memo)
    check_tooling_quantities(memo)
    check_umbrella(memo, umbrella)
    check_fixture_column(memo)
    check_tier_transcription(memo)
    check_preflight(memo)

    mode = "wire" if WIRE.exists() else "bootstrap"
    print(f"claim-gate plan check ({mode}): {len(FAILS)} finding(s)")
    for f in FAILS:
        print("  " + f)
    return 1 if FAILS else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
