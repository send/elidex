#!/usr/bin/env python3
"""A positive and a negative for every check in `claim-gate-plan-check.py`.

A different lifecycle from the checks themselves: nothing in this file decides
whether a memo is right. Each entry is a PROOF that one check can fire, and --
through `probe()`'s needle -- that it fires saying the thing it was written to
say.

§4.3 of the memo records why that is not optional: keying two controls on a
shared diagnostic let each pass when the *other* arm fired, and a ban that
cannot fire is indistinguishable from a clean tree. `ban_control` is the name
this repository already gives that shape
(`.claude/tools/layout-box-reader-trip-wire.sh`), and a check with no firing
proof is in it: it reports a clean tree in exactly the way an unexecuted claim
reports a true one, which is the defect this whole program exists to catch. A
checker whose own checks are never shown to fire is that same transcript one
level up.

`host` is the entry point, which binds it to itself before `selftest()` runs: it
owns the checks fired here and the `FAILS` list they append to, and
`claim-gate-plan-check.py` has hyphens in its name and cannot be imported.
`capture()` therefore REASSIGNS `host.FAILS` rather than a `FAILS` of its own --
`bad()` resolves the list through the entry point's own globals on every call,
so the swap is what it sees. A second name bound here to the list `capture`
swapped out would collect the appends where nobody reads them, `capture` would
return empty, and every negative probe would silently invert.
"""
from __future__ import annotations

import itertools
import re
import sys
import tempfile
from pathlib import Path
from types import ModuleType

from claim_gate_grammar import classify, execute, verdict_over

host: ModuleType  # bound by the entry point at import time

SYNTH_PAIRS = "\n".join(
    ["## §2 Coupled invariants", ""]
    + [f"| **I{i}** | invariant {i} | forces something |" for i in range(1, 5)]
    + [""]
    + [f"| I{a} × I{b} | intersection |" for a, b in itertools.combinations(range(1, 5), 2)]
)

SYNTH_UNIQ = """## §8 Test strategy
**`$3` a substring the diagnostic must contain.**

| Fixture | Means |
|---|---|
| `own` | its own `mktemp -d` and its own wire invocation, torn down after. |
| `live` | one fixture carrying every row's annotations, so their diagnostics must discriminate. |

### §8.1 Controls

| # | clause | Fixture | Positive | Negative |
|---|---|---|---|---|
| C1 | a | live | p | n (`alpha`) |
| C2 | b | live | p | n (`bravo`) |
"""


def capture(fn, *a) -> list[str]:
    saved, host.FAILS = host.FAILS, []
    try:
        fn(*a)
        return host.FAILS
    finally:
        host.FAILS = saved


def capture_skipped(fn, *a) -> list[str]:
    """The `SKIPPED` entries one check appends, isolated as `capture` isolates
    findings. Both lists are swapped: a check that reaches no verdict can still
    emit one on the way out, and a stray finding from a probe must not land in
    the run's own list. Same call-time requirement as `capture` -- `skip()`
    resolves `SKIPPED` through the entry point's globals, so replacing the
    attribute is what it sees."""
    saved_f, saved_s = host.FAILS, host.SKIPPED
    host.FAILS, host.SKIPPED = [], []
    try:
        fn(*a)
        return host.SKIPPED
    finally:
        host.FAILS, host.SKIPPED = saved_f, saved_s


def capture_live() -> dict:
    """The ANNO payload PASTE consumes, without emitting ANNO findings."""
    saved, out = host.FAILS[:], host.check_annotations()
    del host.FAILS[:]
    host.FAILS.extend(saved)
    return out


def write_probe(text: str) -> host.Memo:
    """A probe memo on disk -- `check_preflight` shells out to preflight.py with
    the memo's path, so a purely in-memory Memo cannot exercise it."""
    p = host.REPO / "docs" / "plans" / "zz-probe-preflight.md"
    p.write_text(text)
    return host.Memo(p)


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
    rc, out = host.sh([sys.executable, str(host.SKILL / "preflight.py"),
                       str(m.path.relative_to(host.REPO))])
    keep = [ln for ln in out.split("\n")
            if re.search(r"total entries|malformed rows|unmapped-label|"
                         r"unique specs|grep-pass:", ln)]
    return "\n```\n" + "\n".join(keep) + f"\nEXIT={rc}\n```\n"


def pending_sites() -> list[str]:
    """`rel:line` for every governing-file line that speaks about the wire set."""
    out = []
    for rel in host.GOVERNING:
        if not (host.REPO / rel).exists():
            continue
        for i, line in enumerate((host.REPO / rel).read_text().split("\n"), 1):
            if host.WIRE_TOPIC.search(line) and host.COLLECTIVE.search(line):
                out.append(f"{rel}:{i}")
    return out


def synth(text: str, name: str = "synthetic.md") -> host.Memo:
    m = host.Memo.__new__(host.Memo)
    m.path = Path(name)
    m.text = text
    m.lines = text.split("\n")
    m.heads = [(len(x.group(1)), x.group(2).rstrip("."), x.start())
               for x in re.finditer(r"^(#{2,4})[ \t]+§([\d.]+)", text, re.M)]
    return m


def selftest() -> int:
    # The checks fired below, and the primitives they need, stay in the entry
    # point; bound to locals here so each proof reads as the call it is.
    check_pairs = host.check_pairs
    check_probe_substrings = host.check_probe_substrings
    check_section_refs = host.check_section_refs
    check_citations = host.check_citations
    check_pasted_transcripts = host.check_pasted_transcripts
    check_fence = host.check_fence
    check_transcripts = host.check_transcripts
    check_sites = host.check_sites
    check_tooling_quantities = host.check_tooling_quantities
    check_umbrella = host.check_umbrella
    check_fixture_column = host.check_fixture_column
    check_tier_transcription = host.check_tier_transcription
    check_preflight = host.check_preflight
    fence_verdict = host.fence_verdict
    needs_state = host.needs_state
    run_line = host.run_line
    sh, REPO = host.sh, host.REPO

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
    # §8's Fixture model is READ, not transcribed -- so a fixture the model does
    # not describe must be reported rather than assumed into either scope. The
    # transcribed form silently assumed "shared" for `clone` for a whole round
    # after §4.1.1(b) made it per-row.
    probe("UNIQ negative (a fixture §8's model does not describe)", True,
          capture(check_probe_substrings, synth(
              SYNTH_UNIQ.replace("| live |", "| ghost |"))),
          "does not say whether")
    probe("UNIQ positive (a per-row fixture reused by two rows does not collide)", False,
          capture(check_probe_substrings, synth(
              SYNTH_UNIQ.replace("`bravo`", "`alpha`").replace("| live |", "| own |"))))
    # This tool EXECUTES what RUNNABLE admits, and it admits by program name.
    # Every shape below was found by a review that attacked the classifier; each
    # is a reader-NAMED program writing. Both directions, so a later relaxation
    # of WRITES fails here rather than silently widening what gets run.
    for c, runs in [
        ("sed -i.bak -e s/a/b/ README.md", False),
        ("awk 'BEGIN{system(\"touch /tmp/PWNED\")}'", False),
        ("sed -n 'w /tmp/PWNED' README.md", False),
        ("grep -c . README.md & touch /tmp/PWNED", False),
        ("grep -c . README.md; rm -rf /tmp/PWNED", False),
        ("git log -1 -p --output=/tmp/PWNED", False),
        ("git show --output=/tmp/PWNED HEAD", False),
        ("git grep -O'touch /tmp/PWNED' -e claim", False),
        ("sort -o /tmp/PWNED README.md", False),
        ("grep x README.md > /tmp/PWNED", False),
        ("cat <(touch /tmp/PWNED)", False),
        # the shapes both memos actually paste must keep running
        ("git grep -nE 'claim' -- '*.md'", True),
        ("git ls-files -- 'crates/**/*.rs'", True),
        ("awk 'END{print NR}' README.md", True),
        ("git show 3045f256:README.md", True),
        ("sed -n '25p;59p' scripts/trip-wires.sh", True),
    ]:
        got = bool(host.RUNNABLE.match(c)) and not host.needs_state(c)
        probe(f"RUNNABLE {'runs' if runs else 'refuses'}: {c[:44]}", False,
              [] if got == runs else [f"got runs={got}, expected {runs}"])

    # An entry whose `mandates` pattern no longer matches the REAL umbrella can
    # never fire again: a row that looks like coverage and is not. Round 6 found
    # two -- D3 and umbrella §6 -- because the umbrella's text was edited in
    # place where BINDING's comment assumes it is appended to. This lives in the
    # self-test rather than in check_umbrella because its subject is the registry
    # against the real document, not whatever pair is on argv; run per memo it
    # would fire on every synthetic fixture and prove nothing.
    _umb = host.REPO / "docs/plans/2026-08-quantitative-claim-staleness-gate.md"
    if _umb.exists():
        _t = _umb.read_text()
        for _name, _mand, _w, _a in host.BINDING:
            probe(f"BINDING entry can still fire: {_name[:40]}", False,
                  [] if re.search(_mand, _t, re.S) else
                  [f"`mandates` no longer matches the umbrella; the entry is inert "
                   f"and {_name} is uncovered"])
    else:
        # NOT silent. The prerequisite branch ships the checker without the memos,
        # so these probes have no subject there -- and a probe that vanishes reads
        # as a probe that passed, which is the defect this file exists to catch.
        # One probe records the absence by name, and it is green because the
        # prerequisite PR is allowed to be green; what it may not be is quiet.
        probe(f"BINDING registry NOT CHECKED here: {len(host.BINDING)} entries have "
              f"no umbrella in this worktree", False, [])

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

    def anno(tree) -> list[str]:
        """`verdict_over` RETURNS its findings, so these probes read the return
        value and never go through `capture()`. That is the point of the arm
        being pure: a finding raised through a binding made before `capture()`
        swapped the FAILS list would land in the swapped-out one, and every
        negative below would read clean — the exact inversion these five probes
        exist to make impossible."""
        return [f"[{c}] {m}" for c, m in verdict_over(tree)[1]]

    true_hits = [(1, "strict", live_anno("count", "REQUIRED_WIRES", "scripts/*.sh")),
                 (2, "strict", live_anno("files", "REQUIRED_WIRES", "scripts/*.sh")),
                 (3, "strict", live_anno("lines", None, "scripts/trip-wires.sh"))]
    probe("ANNO positive (every annotation re-runs to its asserted value)", False,
          anno({"probe.md": true_hits}))
    moved = dict(true_hits[2][2])
    moved["n"] += 7
    probe("ANNO negative (a strict annotation the tree has since moved past)", True,
          anno({"probe.md": true_hits[:2] + [(3, "strict", moved)]}),
          f"asserts lines={moved['n']}, actual={true_hits[2][2]['n']}")
    probe("ANNO negative (a candidate line classifying MALFORMED, with a reason)", True,
          anno({"probe.md": true_hits + [(9, "malformed", {"why": "pattern contains \\d"})]}),
          "probe.md:9 classifies MALFORMED (pattern contains \\d)")
    probe("ANNO negative (a MALFORMED line with no reason recorded)", True,
          anno({"probe.md": true_hits + [(9, "malformed", None)]}),
          "does not match the §4.4 production")
    probe("ANNO negative (§8.2's property: a verb with no strict annotation)", True,
          anno({"probe.md": true_hits[:2]}),
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
    def block(*body: str) -> host.Memo:
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

    def not_rerun(memo) -> list[str]:
        """The lines `check_transcripts` RETURNS as not re-run. Findings still
        go through `bad`, so they are isolated here the way `capture` does it --
        a probe reading the return value must not leave one in the run's list."""
        saved, host.FAILS = host.FAILS, []
        try:
            return check_transcripts(memo)
        finally:
            host.FAILS = saved

    # The hole Defect A closed. An arrow line whose command is not RUNNABLE
    # matched neither branch, so it left no comparison AND no entry here; the
    # `$`-form's identical case had always been reported. §4.1.1's transcript is
    # the specimen: a `git clone --local`, and `git -C <dir>` lines that put an
    # option before the subcommand. The report is the whole fix -- widening
    # RUNNABLE to run them would make auditing a memo clone a repository.
    probe("TRANSCRIPT arrow form reports a non-runnable command", True,
          not_rerun(block('git clone --local -q . "$fixture"   → 0.37s, 70M, 1704 files')),
          "not one of the programs this checker runs")
    probe("TRANSCRIPT arrow form reports an option before the git subcommand", True,
          not_rerun(block('git -C "$fixture" ls-files -- "crates/**/*.rs" | wc -l   → 34')),
          "not one of the programs this checker runs")
    probe("TRANSCRIPT arrow form reports a runnable command needing shell state", True,
          not_rerun(block('git grep -lE -e alpha -- "$p" | wc -l   → 2')),
          "needs shell state the memo does not also paste")
    probe("TRANSCRIPT arrow form reports nothing when it re-ran the line", False,
          not_rerun(block(f"{real}   → {n}")))
    # ...and the `$ ` form keeps its own branch, which owns the output lines
    # beneath the prompt. Taking a prompted arrow line here would leave those
    # to be re-read as commands of their own.
    probe("TRANSCRIPT $-form keeps reporting its non-runnable command", True,
          not_rerun(block('$ python3 <prototype-classifier> <this memo>   → 3 rows')),
          "not one of the programs this checker runs")
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
    assert live is not None, (
        "the wire exists, so `check_annotations` classifies nothing and PASTE's "
        "probes have no payload to build a fixture from — have the wire print the "
        "tier counts, or delete PASTE together with these probes; what must not "
        "happen is this file quietly stopping at the two probes below")
    t = live["tiers"]
    paste = ("## §8.2 Live-tree acceptance\n\n```\n"
             f"--- {{'strict': {t['strict']}, 'mention': {t['mention']}}}\n```\n")
    probe("PASTE positive (tier counts matching the tree)", False,
          capture(check_pasted_transcripts, synth(paste), live))
    probe("PASTE negative (a tier count mutated)", True,
          capture(check_pasted_transcripts,
                  synth(paste.replace(f"'strict': {t['strict']}", "'strict': 99")), live),
          "pastes strict=99")

    # Defect B, replayed. `check_annotations` returned `{}` as soon as the wire
    # FILE existed -- presence is the whole test, untracked is enough -- and
    # PASTE's `if not live: return` read that as "nothing to check". S1's first
    # commit creates that file, so §8.2's verification would have stopped on the
    # commit that starts the slice, with no finding and no notice. The stub is
    # how the transition gets a firing proof at all: the wire is the subject of
    # the memo this checker audits and does not exist yet.
    def with_wire(rc: int) -> tuple[list[str], object]:
        box: list[object] = []
        with tempfile.TemporaryDirectory() as td:
            stub = Path(td) / "claim-provenance-trip-wire.sh"
            stub.write_text(f"#!/usr/bin/env bash\necho 'stub wire'\nexit {rc}\n")
            saved, host.WIRE = host.WIRE, stub
            try:
                found = capture(lambda: box.append(host.check_annotations()))
            finally:
                host.WIRE = saved
        return found, box[0]

    green_wire, wire_live = with_wire(0)
    probe("ANNO delegates to the wire and stays clean when it exits 0", False, green_wire)
    probe("ANNO negative (the wire reds)", True, with_wire(1)[0], "the wire reds")
    probe("ANNO in wire mode hands PASTE no payload rather than an empty one", True,
          ["check_annotations() returned None"] if wire_live is None else
          [f"check_annotations() returned {wire_live!r}, which PASTE reads as clean"],
          "returned None")
    probe("PASTE reports itself not run when the wire owns classification", True,
          capture_skipped(check_pasted_transcripts, synth(paste), None),
          "wire mode")
    probe("PASTE reports itself not run when the memo has no §8.2", True,
          capture_skipped(check_pasted_transcripts, synth("## §9 elsewhere\n"), live),
          "no §8.2 section")
    probe("PASTE reports no skip when it had a payload and a §8.2", False,
          capture_skipped(check_pasted_transcripts, synth(paste), live))

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
    probe("PREFLIGHT reports itself not run when the memo has no §8.3", True,
          capture_skipped(check_preflight, synth("## §9 elsewhere\n")),
          "no §8.3 section")
    probe("PREFLIGHT reports no skip when §8.3 is there to compare", False,
          capture_skipped(check_preflight, write_probe(PREFLIGHT_FIXTURE + real_pf)))
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
    # Called with one argv path this check compared nothing and said nothing --
    # the same shape as wire-mode PASTE, reached by omitting an argument rather
    # than by creating a file.
    probe("UMBRELLA reports itself not run when no umbrella was passed", True,
          capture_skipped(check_umbrella, slice_withdraws, None),
          "no umbrella memo on argv")
    probe("UMBRELLA reports no skip when it had an umbrella to compare", False,
          capture_skipped(check_umbrella, slice_withdraws, umb_mandates))

    # D4, added round 6. Both directions, because the entry exists to catch a
    # decision change that four revisions made silently: the slice requires a
    # working-tree presence test the umbrella's `git ls-files` mechanism does
    # not, which REJECTS annotations D4-as-written accepts.
    d4_mandates = synth("**`in=` must match ≥1 tracked file or\nthe annotation fails**", "umb.md")
    d4_amended = synth("**`in=` must match ≥1 tracked file or\nthe annotation fails**\n\n"
                       "⚠ **AMENDED by S1 round 6: the precondition is about the working tree**",
                       "umb.md")
    d4_withdraws = synth("at least one of which is present in the working tree")
    probe("UMBRELLA negative (D4: slice adds a working-tree test, umbrella unamended)", True,
          capture(check_umbrella, d4_withdraws, d4_mandates),
          "binding decision cannot be changed by a slice edit")
    probe("UMBRELLA positive (D4: the umbrella carries the round-6 amendment)", False,
          capture(check_umbrella, d4_withdraws, d4_amended))

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
    def s3(*branches: str) -> host.Memo:
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

    # The closing line is where a skipped check becomes visible, so it gets its
    # own proof: every `skip()` above is worth nothing if the run still ends on a
    # finding count alone, which is how wire mode retired PASTE in silence.
    def with_skips(entries: list[str]) -> list[str]:
        saved, host.SKIPPED = host.SKIPPED, list(entries)
        try:
            return host.summary("wire")
        finally:
            host.SKIPPED = saved

    probe("SUMMARY counts and names a check that did not run", True,
          [ln for ln in with_skips(["PASTE — wire mode"]) if "not run" in ln],
          "1 check(s) not run")
    probe("SUMMARY names no skipped check when every check ran", False,
          [ln for ln in with_skips([]) if "not run:" in ln])
    probe("SUMMARY states the zero rather than omitting the count", True,
          [ln for ln in with_skips([]) if "check(s) not run" in ln],
          "0 check(s) not run")

    width = max(len(p[0]) for p in probes)
    failed = 0
    for label, ok, detail in probes:
        if not ok:
            failed += 1
        print(f"  {'ok  ' if ok else 'FAIL'}  {label:<{width}}  {detail}")
    print(f"self-test: {len(probes) - failed}/{len(probes)} probes ok")
    return 1 if failed else 0
