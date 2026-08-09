#!/usr/bin/env python3
"""Machine-check the claim-gate plan memos against themselves and against the tree.

Why this exists rather than another prose rule: within this one program, seven
statements that the author believed had been executed landed false, and each
remedy was a rule added to the memo's preamble. Round 2's three CRITs were each
detectable in one command. A rule cannot fix that, because the author believes
the rule was followed -- so the memo gets a checker instead.
(memory `feedback_prose-rules-cannot-fix-unexecuted-claims`.)

Relationship to `.claude/tools/plan-xcheck.py` (the line-box lane's sibling,
which reached this same conclusion independently): the two share no predicates.
That one's checks are bound to its memo's schema -- PR labels, cell numbers,
M-rows, flip partitions -- and none of them name anything in this memo. What is
genuinely common is about 25 lines of markdown-table parsing, and even the
section splitter differs (`## §N.` there, `## §N <title>` here). They are also
in different unlanded branches, so sharing today would couple this slice's
landing to a 13-round-deep plan review. Collapse them when a *third* memo wants
one, and only then -- at that point the reason N existed is writable and the
harness is worth extracting (CLAUDE.md *One issue, one way*, and memory
`feedback_one-issue-one-way`'s precondition).

What this adds over that sibling is the class it explicitly disclaims: it
declares itself "tabular-surface only: nothing here reads the memo's prose
claims", which is exactly where round 2's CRIT 1 lived. ANNO / PASTE /
TRANSCRIPT / FENCE / PREFLIGHT all *execute* something and compare.

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
FENCE_MARK = re.compile(r"^[ \t]*(```|~~~)")
RS_FENCE_MARK = re.compile(r"^[ \t]*//[/!]?[ \t]*(```|~~~)")
BAD_ESCAPE = re.compile(r"\\[bBdDsSwW<>]")

# §4.2's surface table: which surfaces have a fence arm at all.
FENCE_ARM = {".md": FENCE_MARK, ".rs": RS_FENCE_MARK, ".sh": None, ".toml": None}


def fence_state(lines: list[str], marker_re) -> list[bool]:
    """Per-line 'is fenced'. Only a matching marker closes; marker lines are
    themselves fenced -- the shape both in-repo trackers use (see check_fence)."""
    state = [False] * len(lines)
    if marker_re is None:
        return state
    inside = False
    marker = None
    for i, line in enumerate(lines):
        m = marker_re.match(line)
        if m:
            cur = m.group(1)
            if not inside:
                inside, marker = True, cur
            elif marker == cur:
                inside, marker = False, None
            state[i] = True
        else:
            state[i] = inside
    return state


def classify(rel: str, lines: list[str]) -> list[tuple[int, str, dict | None]]:
    """(line-number, tier, fields) for every candidate line in one file."""
    suffix = "." + rel.rsplit(".", 1)[-1]
    fenced = fence_state(lines, FENCE_ARM.get(suffix, FENCE_MARK))
    out = []
    for i, line in enumerate(lines):
        m = LAX.search(line)
        if not m:
            continue
        k = line[: m.start()].count("`")
        if fenced[i] or k % 2 == 1:
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
        res[rel] = classify(rel, (REPO / rel).read_text().split("\n"))
    return res


def check_annotations() -> dict:
    if WIRE.exists():
        rc, out = sh(["bash", str(WIRE)])  # noqa
        if rc != 0:
            bad("ANNO", f"the wire reds: {out.strip()[:400]}")
        return {}
    tree = scan_tree()
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


def check_fence(memo: Memo) -> None:
    try:
        diverge, n = fence_verdict()
    except Exception as e:  # pragma: no cover
        bad("FENCE", f"cannot run the two in-repo fence trackers: {e}")
        return
    verdict = (f"disagree on line(s) {diverge} of {n}" if diverge
               else f"agree on all {n}")
    for i, line in enumerate(memo.lines, 1):
        if not RESTATES_FENCE.search(line):
            continue
        if NAMES_BOTH.search(line) or (FENCE_TOPIC.search(line) and PAIR_REF.search(line)):
            bad("FENCE", f"{memo.path.name}:{i} restates how the two fence "
                         f"trackers relate; that fact is measured by this "
                         f"check (they {verdict} lines) and must be cited, "
                         f"not written out: {line.strip()[:64]}")


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


def fenced_blocks(memo: Memo) -> list[tuple[int, list[str]]]:
    """(1-based line of the opener, body lines) for every ```-fenced block."""
    out, cur, start = [], None, 0
    for i, line in enumerate(memo.lines, 1):
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


def check_transcripts(memo: Memo) -> None:
    unchecked = 0
    for start, body in fenced_blocks(memo):
        i = 0
        while i < len(body):
            raw = body[i]
            ln = start + 1 + i
            line = raw.strip()

            # form A:  <command>  →  <answer>
            m = ARROW.match(line)
            if m and RUNNABLE.match(m.group(1).strip()):
                cmd, want = m.group(1).strip(), m.group(2).strip()
                if needs_state(cmd):
                    unchecked += 1
                else:
                    got = run_line(cmd)
                    head = re.split(r"\s{2,}|\s+[#(]", want)[0].strip()
                    first = got.split("\n")[0].strip()
                    if head and head not in (first, got) and first not in head:
                        bad("TRANSCRIPT", f"{memo.path.name}:{ln} pastes `{cmd[:56]}` "
                                          f"→ {head!r}; re-running gives {first!r}")
                i += 1
                continue

            # form B:  $ <command>  then its pasted output, to the next $ or EOB
            if line.startswith("$ "):
                cmd = line[2:].strip()
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
                    unchecked += 1
                elif pasted:
                    actual = {m2.group(0) for l2 in run_line(cmd).split("\n")
                              if (m2 := FILELINE.match(l2.strip()))}
                    for miss in sorted(actual - pasted):
                        bad("TRANSCRIPT", f"{memo.path.name}:{ln} pastes the output of "
                                          f"`{cmd[:56]}` but omits the hit at {miss}")
                    for extra in sorted(pasted - actual):
                        bad("TRANSCRIPT", f"{memo.path.name}:{ln} pastes a hit at {extra} "
                                          f"that `{cmd[:56]}` no longer produces")
                i = j
                continue
            i += 1
    if unchecked:
        print(f"  transcripts: {unchecked} pasted command(s) not re-run "
              f"(they need shell state the memo does not also paste)")


# --------------------------------------------------------------------------
# 9. SITES -- every line that describes the wire SET, listed for disposal
#
# The TRANSCRIPT check proves a paste matches its command; it cannot prove the
# command was the right one. Round 2's two missed sibling sites are exactly
# that failure -- `ci.yml`'s "the wires are grep-only" and `mise.toml`'s
# enumeration of four disjoint surfaces are both invalidated by a fifth wire,
# and neither contains the string the memo grepped for.
#
# So this reports rather than fails: the governing files are a closed set of
# five, and every line in them that speaks about the wires *collectively* has
# to be consciously disposed of by §4.6 -- edited, or cited as deliberately
# left alone. Per-wire headers are out of scope by §4.6's own rule.
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
    if not pending:
        return
    print(f"  wire-set sites not cited by the memo ({len(pending)}) -- §4.6 must "
          f"dispose of each, by editing it or saying why not:")
    for rel, i, line in pending:
        star = "  <- named file, unnamed line" if rel.split("/")[-1] in loose else ""
        print(f"    {rel}:{i}  {line[:76]}{star}")


# --------------------------------------------------------------------------
# 10. PREFLIGHT -- §8.3's pasted verdict is what preflight prints today
# --------------------------------------------------------------------------

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
    tiers = [t for _, t, _ in classify("f.md", [
        '<!-- claim count=1 pattern="x" in="a.md" -->',            # strict
        '`<!-- claim count=1 pattern="x" in="a.md" -->`',          # mention, odd k
        '<!-- claim cont=3 pattern="x" in="a.md" -->',             # malformed verb
        r'<!-- claim count=1 pattern="\d" in="a.md" -->',          # malformed dialect
        "```",
        '<!-- claim count=1 pattern="x" in="a.md" -->',            # mention, fenced
        "```",
    ])]
    probes.append(("ANNO classifier tiers",
                   tiers == ["strict", "mention", "malformed", "malformed", "mention"],
                   str(tiers)))
    ok, got = execute({"verb": "count", "n": 999999, "icase": False,
                       "pattern": "fn ", "spec": "crates/**/*.rs"})
    probes.append(("ANNO executor returns a live count", ok and got != 999999, str(got)))
    ok, got = execute({"verb": "lines", "n": 0, "icase": False,
                       "pattern": None, "spec": "crates/**/*.rs"})
    probes.append(("ANNO `lines=` rejects a multi-file pathspec", not ok, str(got)))

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
    probes.append(("TRANSCRIPT `$NF` in an awk program is not shell state",
                   not needs_state("git grep -cE 'x' -- '*.rs' | awk -F: "
                                   "'{s+=$NF} END{print s}'"), ""))

    probe("FENCE ban ignores fence prose with no sameness claim", False,
          capture(check_fence, synth(
              "## §6 x\nThe fence arm is defined per surface; only a matching "
              "marker closes the block, in both implementations.")))

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
    check_preflight(memo)

    mode = "wire" if WIRE.exists() else "bootstrap"
    print(f"claim-gate plan check ({mode}): {len(FAILS)} finding(s)")
    for f in FAILS:
        print("  " + f)
    return 1 if FAILS else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
