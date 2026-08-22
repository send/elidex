#!/usr/bin/env python3
"""A firing proof for every arm of every check in `claim-gate-plan-check.py`.

A different lifecycle from the checks themselves: nothing in this file decides
whether a memo is right. Each entry is a PROOF that one check can fire, and --
through `probe()`'s needle -- that it fires saying the thing it was written to
say.

That opening sentence used to read "A positive and a negative for every check",
and it was false in exactly the way this program is about: traced, thirteen of
the forty-two `bad()` / `skip()` sites were reached by no probe at all --
including the ANNO arm that is the only one taken on the branch this file ships
from, and both TRANSCRIPT arms whose own comments narrate the bug they were
added for. Thirteen `ban_control`s under a sentence asserting there were none,
in the file that defines the term.

It is not asserted now. `--arm-coverage` collects the sites by AST and runs the
probes under `sys.settrace`; the `ARMS` probe below fails on any site no probe
reaches, and on any entry in `UNPROBED` the probes turn out to reach. The
sentence is true because a command says so on every run, which is the rule §0
of the memo states and the shape the rest of this program exists to impose.

The memo records why that is not optional: keying two controls on a
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

import builtins
import contextlib
import io
import itertools
import os
import re
import subprocess
import symtable
import sys
import tempfile
from pathlib import Path
from types import ModuleType


host: ModuleType  # bound by the entry point at import time

SYNTH_PAIRS = "\n".join(
    ["## §2 Coupled invariants", ""]
    + [f"| **I{i}** | invariant {i} | forces something |" for i in range(1, 5)]
    + [""]
    + [f"| I{a} × I{b} | intersection |" for a, b in itertools.combinations(range(1, 5), 2)]
)


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
    """`rel:line` for every governing-file line that speaks about the wire set.

    Read off `check_sites`' own generator rather than re-implementing its
    predicate: a copy here builds the fixture the SITES probes cite, so
    narrowing the check would leave both probes green against the change."""
    return [f"{rel}:{i}" for rel, i, _ in host.claim_gate_registries.wire_set_lines()]


def synth(text: str, name: str = "synthetic.md") -> host.Memo:
    """A Memo over a string, built by the REAL constructor.

    This used to bypass `Memo.__init__` with `__new__` and rebuild `heads` from
    a character-for-character copy of `HEADING_RE`. Every probe in this file --
    the firing proof for every check -- then parsed headings through the copy,
    so widening the real grammar would leave the whole suite proving the old
    one."""
    return host.Memo(Path(name), text)


def git_fixture(tmp: Path, steps: list[list[str]]) -> Path:
    """A throwaway repository with a history the live one cannot supply.

    Three of this program's git derivations turn on distinctions that are
    unobservable here: the merge base of `origin/main..HEAD` IS `origin/main`
    today, so two-dot and three-dot agree, and nothing on this branch is a
    rename. A probe run against this repository would pass either way, which is
    the shape `ban_control` names -- so the fixture supplies the divergence and
    the rename, and the derivations are pointed at it by swapping the module
    global they read, not by growing a parameter production never passes.
    """
    for cmd in [["git", "init", "-q", "-b", "main"],
                ["git", "config", "user.email", "probe@example.invalid"],
                ["git", "config", "user.name", "probe"],
                ["git", "config", "commit.gpgsign", "false"]] + steps:
        if cmd[0] == "write":
            (tmp / cmd[1]).write_text(cmd[2])
            continue
        p = subprocess.run(cmd, cwd=tmp, capture_output=True, text=True)
        if p.returncode != 0:
            raise RuntimeError(f"fixture step {cmd} failed: {p.stderr.strip()}")
    return tmp


# A branch point, a commit on the branch, and a commit `main` gained after it.
# Two-dot answers "two files changed" here and three-dot answers "one", which is
# the whole difference between comparing tips and comparing against the point
# the branch left.
DIVERGED = [
    ["write", "base.txt", "base\n"],
    ["git", "add", "-A"], ["git", "commit", "-qm", "base"],
    ["git", "checkout", "-qb", "feat"],
    ["write", "mine.txt", "mine\n"],
    ["git", "add", "-A"], ["git", "commit", "-qm", "mine"],
    ["git", "checkout", "-q", "main"],
    ["write", "theirs.txt", "theirs\n"],
    ["git", "add", "-A"], ["git", "commit", "-qm", "theirs"],
    ["git", "update-ref", "refs/remotes/origin/main", "refs/heads/main"],
    ["git", "checkout", "-q", "feat"],
]

# The same shape with a rename instead: a file leaves the set it was in, and
# rename detection -- on by default since git 2.9 -- reports only where it
# arrived. Every consumer of these derivations is asking about paths, so a
# departure that names only the destination is a departure none of them sees.
def _wire_branch(*after: list[str]) -> list[list[str]]:
    """A base commit holding one wire, then whatever the branch does to it."""
    return [
        ["write", "a-trip-wire.sh", "echo a\n"],
        ["git", "add", "-A"], ["git", "commit", "-qm", "base"],
        ["git", "update-ref", "refs/remotes/origin/main", "refs/heads/main"],
        ["git", "checkout", "-qb", "feat"],
    ] + list(after) + [["git", "add", "-A"], ["git", "commit", "-qm", "branch"]]


EDITS_A_WIRE = _wire_branch(["write", "a-trip-wire.sh", "echo a\necho more\n"])
ADDS_A_WIRE = _wire_branch(["write", "b-trip-wire.sh", "echo b\n"])


def renamed(old: str, new: str) -> list[list[str]]:
    return [
        ["write", old, "contents\n"],
        ["git", "add", "-A"], ["git", "commit", "-qm", "base"],
        ["git", "update-ref", "refs/remotes/origin/main", "refs/heads/main"],
        ["git", "checkout", "-qb", "feat"],
        ["git", "mv", old, new],
        ["git", "commit", "-qm", "rename"],
    ]


def selftest() -> int:
    # The checks fired below, and the primitives they need, stay in the entry
    # point; bound to locals here so each proof reads as the call it is.
    check_pairs = host.check_pairs
    check_section_refs = host.check_section_refs
    check_citations = host.check_citations
    check_fence = host.check_fence
    check_transcripts = host.check_transcripts
    check_sites = host.check_sites
    check_tooling_quantities = host.check_tooling_quantities
    check_umbrella = host.check_umbrella
    check_preflight = host.check_preflight
    fence_verdict = host.fence_verdict
    needs_state = host.needs_state
    run_line = host.run_line
    sh, REPO = host.sh, host.REPO
    # The companion that owns CONTROLS and PATHS. Two probes below replace a
    # global for the duration of one check, and a global is only replaceable on
    # the module whose functions read it -- reaching these through the entry
    # point's re-export would rebind a name nothing resolves.
    ledgers = host.claim_gate_ledgers
    # Same rule, one layer down: the git derivations live in their own module
    # now, so a probe that stubs one swaps it there.
    git_ = host.claim_gate_git
    # SITES and UMBRELLA, and the two seed registries they read. Same rule:
    # a probe swapping `GOVERNING` or `changes_the_wire_set` swaps it HERE.
    registries = host.claim_gate_registries

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
        for _name, _mand, _w, _a in registries.BINDING:
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
        probe(f"BINDING registry NOT CHECKED here: {len(registries.BINDING)} entries have "
              f"no umbrella in this worktree", False, [])


    # SECT
    probe("SECT positive (a defined section)", False,
          capture(check_section_refs, synth("## §4 Design\nsee §4 above\n"), None))
    probe("SECT negative (an undefined section)", True,
          capture(check_section_refs, synth("## §4 Design\nsee §9 below\n"), None),
          "§9")
    # The umbrella's OWN section references were examined by nothing. CITE runs
    # over both documents and SECT over one, an asymmetry with no reason stated
    # anywhere -- so a whole document's worth of `§N` went unchecked. Measured
    # on the real pair: the umbrella cites `S1 §4.1.1`, a number it has no
    # heading for, and no run has ever looked at it.
    probe("SECT negative (an undefined section in the UMBRELLA)", True,
          capture(check_section_refs, synth("## §4 Design\nnothing\n"),
                  synth("## §2 Scope\nsee §9 below\n", "umbrella.md")),
          "umbrella.md:2")
    probe("SECT positive (a section the umbrella does define, cited in it)", False,
          capture(check_section_refs, synth("## §4 Design\nnothing\n"),
                  synth("## §2 Scope\nsee §2 above\n", "umbrella.md")))
    # A reference to ANOTHER document is named the way `CITE` already names
    # one -- a backticked path. Attribution was a closed list of one word,
    # `umbrella`, so every other form fell through to "this memo has no such
    # heading": measured on the real pair, nine references in the umbrella,
    # every one of them cross-document (`S1 §4.2`, `` `terminal-z-c3a-impl-plan.md` §7 ``,
    # `v1's §4.1.3`) and not one of them dangling.
    _OTHER = "docs/plans/2026-07-terminal-z-c3a-impl-plan.md"
    probe("SECT positive (a §-ref attributed to another document by path)", False,
          capture(check_section_refs,
                  synth(f"## §4 Design\nsee `{_OTHER}` §7 for the precedent\n"), None))
    # The needle names the ATTRIBUTED document, not the number: `§99` is absent
    # from the containing memo too, so the old self-reference arm fired with a
    # message this probe accepted -- green for a rule it was not exercising.
    probe("SECT negative (a path-attributed §-ref that document lacks)", True,
          capture(check_section_refs,
                  synth(f"## §4 Design\nsee `{_OTHER}` §99 for the precedent\n"), None),
          "that document has no heading for")
    probe("SECT negative (a §-ref attributed to a path that is not tracked)", True,
          capture(check_section_refs,
                  synth("## §4 Design\nsee `docs/plans/zz-no-such-memo.md` §2\n"), None),
          "no tracked file")
    # An AMBIGUOUS attribution is a third outcome, and weakening the guard from
    # "exactly one" to "at least one" left the suite green until this arm
    # existed -- the check would then have read headings off whichever file the
    # resolver happened to list first.
    #
    # The fixture is `SKILL.md`, and the first choice was `README.md`, which is
    # duplicated across the tree and is NOT ambiguous here: `resolve` tries the
    # literal pathspec first, the repository root has a README, and the
    # basename fallback that produces the ambiguity never runs. The assertion
    # below is what said so -- a probe whose fixture is asserted rather than
    # assumed reports its own wrong premise.
    _AMBIG = "SKILL.md"
    probes.append((f"SECT: `{_AMBIG}` really is ambiguous in this tree",
                   len(host.resolve(_AMBIG)) > 1, f"{len(host.resolve(_AMBIG))} hits"))
    probe("SECT negative (a §-ref attributed to an ambiguous path)", True,
          capture(check_section_refs,
                  synth(f"## §4 Design\nsee `{_AMBIG}` §2 for it\n"), None),
          "tracked files")

    # CITE
    probe("CITE positive (in range)", False,
          capture(check_citations, synth("see `scripts/trip-wires.sh:1` there")))
    probe("CITE negative (past EOF)", True,
          capture(check_citations, synth("see `scripts/trip-wires.sh:999999` there")),
          "lines")
    probe("CITE negative (untracked)", True,
          capture(check_citations, synth("see `scripts/no-such-file.sh:1` there")),
          "not a tracked file")

    # The classifier/executor probes retired with the grammar module. They
    # exercised this file's second copy of the tier rules and the verbs, which is
    # the copy the wire replaces; their subjects are now controlled by
    # `claim-provenance-selftest.sh`, whose fixtures run the wire itself. What
    # stays here is ANNO's delegation to the wire, below.

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
    # `fenced_blocks` decides which pasted lines are commands at all, so its
    # boundaries get their own proofs. It carried a toggle that flipped on any
    # marker -- the `wrong_spans` mutant above, shipped as production -- and
    # dropped an unclosed trailing block silently. Both directions, plus the
    # blockquote strip that is the one thing it does beyond the shared tracker.
    _fb = host.fenced_blocks
    _mismatch = _fb(synth("```\nalpha\n~~~\nbeta\n```\ngamma\n"))
    probes.append(("FENCE-BLOCKS a mismatched marker does not close the block",
                   len(_mismatch) == 1 and _mismatch[0] == (1, ["alpha", "~~~", "beta"]),
                   f"{_mismatch}"))
    _unclosed = _fb(synth("```\nalpha\nbeta\n"))
    probes.append(("FENCE-BLOCKS an unclosed block is fenced to EOF, not dropped",
                   len(_unclosed) == 1 and _unclosed[0][1][:2] == ["alpha", "beta"],
                   f"{_unclosed}"))
    _quoted = _fb(synth("> ```\n> alpha\n> ```\n"))
    probes.append(("FENCE-BLOCKS a blockquoted fence is still a block",
                   len(_quoted) == 1 and _quoted[0][1] == ["alpha"],
                   f"{_quoted}"))

    # Every separator is a command position. A review executed `| tee`,
    # `; python3 -c`, `&& python3 -c`, `$(…)` and a backtick form out of a memo
    # and wrote six files into the repo root -- three of them silently, because
    # the paste still matched. `RUNNABLE` gated the first word only.
    _unrunnable = host.claim_gate_reruns.unrunnable
    for _label, _cmd in [
            ("a pipe into a writer", "git ls-files -- x | tee OUT"),
            ("a semicolon", "git ls-files -- x ; python3 -c pass"),
            ("command substitution", "git log --oneline $(python3 -c pass)"),
            ("a backtick", "git log --oneline `python3 -c pass`"),
            ("&&", "git ls-files && python3 -c pass"),
            ("||", "git ls-files || python3 -c pass"),
            ("a pipe into xargs", "grep -c a f | xargs -I{} tee OUT")]:
        probes.append((f"ADMISSION refuses {_label}",
                       bool(_unrunnable(_cmd)), _unrunnable(_cmd) or "ADMITTED"))
    for _label, _cmd in [
            ("a pipe into awk", "git show a:b | awk 'END{print NR}'"),
            ("a pipe into wc", "git log --oneline a..b -- p | wc -l"),
            ("a quoted pathspec", 'git ls-files -- "*.md"')]:
        probes.append((f"ADMISSION still admits {_label}",
                       _unrunnable(_cmd) is None, _unrunnable(_cmd) or "admitted"))

    probe("TRANSCRIPT arrow form, correct answer", False,
          capture(check_transcripts, block(f"{real}   → {n}")))
    probe("TRANSCRIPT arrow form, stale answer", True,
          capture(check_transcripts, block(f"{real}   → {int(n) + 7}")),
          "re-running gives")
    listing = "git grep -n 'REQUIRED_WIRES' -- 'scripts/*.sh'"
    hits = [x for x in run_line(listing).split("\n") if x]
    # The stale-answer probe above uses `n + 7`, which happens to avoid the
    # substring relation for today's `n`. This one pins the relation itself: the
    # abbreviated-paste arm accepted any paste CONTAINING the answer, so `13`
    # passed against a true `3`, silently, in the comparator round 2's CRIT
    # lived in. A trailing gloss must still pass.
    _wc = 'git ls-files -- ".claude/tools/claim_gate_*.py" | wc -l'
    _n = run_line(_wc).strip()
    probe("TRANSCRIPT arrow form, a stale answer that CONTAINS the true one", True,
          capture(check_transcripts, synth(f"## §4 x\n\n```\n{_wc}   → 1{_n}\n```\n")),
          "re-running gives")
    probe("TRANSCRIPT arrow form, the true answer with a trailing gloss", False,
          capture(check_transcripts, synth(f"## §4 x\n\n```\n{_wc}   → {_n} files\n```\n")))
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
    # therefore in the state the memo names: indistinguishable from a clean tree.
    # Each is built from what the tree says right now, so the probes hold in
    # any worktree and never require the memo this tool audits to be present --
    # that memo is a different slice, and a probe that reads it would re-create
    # the bundling the carve-out removed.

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

    sites = sorted(pending_sites())
    def disposal(rows):
        # The gate is INJECTED below rather than bought with this sentence.
        # Round 10's gate was on the memo naming a wire, so the fixture named
        # one to stay in population -- which is why naming and shipping were
        # never separable here, and why the discriminating arm was missing.
        return ("## §4.6 Disposal\n\nS1 ships `.claude/tools/claim-provenance-trip-wire.sh`.\n\n"
                "```\n" + "\n".join(f"{r}: x" for r in rows) + "\n```\n")
    probe("SITES positive (every wire-set line cited)", False,
          capture(check_sites, synth(disposal(sites)), True))
    probe("SITES negative (one disposal dropped)", True,
          capture(check_sites, synth(disposal(sites[1:])), True),
          "speaks about the wire set")

    real_pf = run_preflight_block(PREFLIGHT_FIXTURE)
    probe("PREFLIGHT positive (a pasted verdict matching the tool)", False,
          capture(check_preflight, write_probe(PREFLIGHT_FIXTURE + real_pf)))
    probe("PREFLIGHT negative (a soft-finding count mutated)", True,
          capture(check_preflight, write_probe(
              PREFLIGHT_FIXTURE
              + re.sub(r"(\d+) hard, (\d+) soft", r"\1 hard, 99 soft", real_pf))),
          "preflight now prints")
    # The soft-count arm is its own `re.search` pair; the four-figure loop above
    # it had no firing proof at all -- it was written as a dict mapping each
    # regex to itself, and nobody had run one of its arms. One is enough to
    # prove the loop reaches a verdict; the loop is a stated seed either way.
    probe("PREFLIGHT negative (a figure from the four-field loop mutated)", True,
          capture(check_preflight, write_probe(
              PREFLIGHT_FIXTURE
              + real_pf.replace("total entries  (M):   1", "total entries  (M):   42"))),
          "total entries (M)")
    probe("PREFLIGHT reports itself not run when the memo has no §8.3", True,
          capture_skipped(check_preflight, synth("## §9 elsewhere\n")),
          "no §8.3 section")
    probe("PREFLIGHT reports no skip when §8.3 is there to compare", False,
          capture_skipped(check_preflight, write_probe(PREFLIGHT_FIXTURE + real_pf)))
    (REPO / "docs" / "plans" / "zz-probe-preflight.md").unlink(missing_ok=True)

    # TOOLING / UMBRELLA / FIXTURE -- the three classes round 4 found recurring.
    # Probed here at the same time they were written, because shipping a check
    # without a firing proof is the defect the same round found in ANNO.
    # The probe's subject must be a ref that survives this PR. Round 9 keyed it
    # on the local branch name `claim-gate-plan-check`, so after that branch
    # merges and is deleted -- i.e. on `main`, where this file lands -- the
    # `rev-list` returns empty and `--self-test` dies with a ValueError. The
    # tool the prerequisite PR ships was broken on main by construction, and the
    # memo's own State block pastes `--self-test` as the gate's status. `HEAD`
    # is the ref that is always resolvable.
    rc_c, out_c = sh(["git", "rev-list", "--count", "origin/main..HEAD"])
    if rc_c != 0 or not out_c.strip().isdigit():
        probe("TOOLING NOT PROBED: `origin/main..HEAD` did not resolve to a count",
              False, [])
    else:
        n_commits = out_c.strip()
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

    # Keyed on D2, which survives; the earlier form keyed on D3, retired in round
    # 6 for being inert, so these probes went clean and stopped proving anything.
    umb_mandates = synth("strip a\nleading `//[/!]?` before fence matching in `.rs`.", "umb.md")
    umb_silent = synth("inertness is whatever the slice says.", "umb.md")
    slice_withdraws = synth("There is one inertness rule and there is no fence arm.")
    probe("UMBRELLA positive (umbrella silent on the withdrawn rule)", False,
          capture(check_umbrella, slice_withdraws, umb_silent))
    probe("UMBRELLA positive (umbrella mandates, slice does not withdraw)", False,
          capture(check_umbrella, synth("The tier pass follows D3."), umb_mandates))
    probe("UMBRELLA negative (slice withdraws what the umbrella mandates)", True,
          capture(check_umbrella, slice_withdraws, umb_mandates),
          "binding decision cannot be changed by a slice edit")
    umb_amended = synth("strip a\nleading `//[/!]?` before fence matching in `.rs`.\n\n"
                        "AMENDED by S1 round 4 (option (c)): the fence half", "umb.md")
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


    # CONTROLS. The `have` map and the files scanned for a control COUNT are
    # both injected, so these probes decide the check's logic rather than the
    # tree's current control set -- which is the thing the check is for.
    check_controls = host.check_controls
    HAVE = {"C1": {"positive", "negative"}, "C2": {"negative"}, "C3": set()}
    LEDGER = ("## §8 Test strategy\n\n| Retired | Why |\n|---|---|\n"
              "| `C9` | withdrawn: no probe shape observes it |\n")
    probe("CONTROLS positive (every cited control exists, with the arm cited)", False,
          capture(check_controls, synth(LEDGER + "\nC1's positive and C2 both hold.\n"),
                  HAVE, []))
    probe("CONTROLS negative (a control the self-test does not ship)", True,
          capture(check_controls, synth(LEDGER + "\n§8 C0 keeps a control for it.\n"),
                  HAVE, []),
          "no such row")
    probe("CONTROLS negative (an arm the self-test does not ship)", True,
          capture(check_controls, synth(LEDGER + "\nC2's positive proves it.\n"), HAVE, []),
          "only its negative arm")
    probe("CONTROLS negative (a control with no arm named either way)", True,
          capture(check_controls, synth(LEDGER + "\nC3's negative proves it.\n"), HAVE, []),
          "no arm named either way")
    # Round 10: one inserted word defeated the arm check at two live sites, and
    # a sub-arm letter was unreadable, so `C18 (z)` passed while `C99 (a)` red.
    probe("CONTROLS negative (an arm cited with words between it and the id)", True,
          capture(check_controls, synth(LEDGER + "\nC2's intended positive proves it.\n"),
                  HAVE, []),
          "cites C2's positive arm")
    probe("CONTROLS negative (a sub-arm the self-test does not ship)", True,
          capture(check_controls, synth(LEDGER + "\nC1 (z) asserts it.\n"),
                  {"C1": {"a", "b"}}, []),
          "ships C1 with only its a/b arm")
    probe("CONTROLS positive (a sub-arm it does ship)", False,
          capture(check_controls, synth(LEDGER + "\nC1 (b) asserts it.\n"),
                  {"C1": {"a", "b"}}, []))
    # A range names every control between its ends, which is how `C19`-`C22`
    # read as two citations while four controls went unwritten.
    probe("CONTROLS negative (a range names the controls between its ends)", True,
          capture(check_controls, synth(LEDGER + "\n`C1`–`C4` re-execute the corpus.\n"),
                  HAVE, []),
          "C4")
    probe("CONTROLS positive (a retired control may be named in the ledger row)", False,
          capture(check_controls, synth(LEDGER), HAVE, []))
    probe("CONTROLS negative (a retired control cited outside the ledger)", True,
          capture(check_controls, synth(LEDGER + "\nC9 still covers that case.\n"),
                  HAVE, []),
          "licenses the row that records the retirement")
    probe("CONTROLS negative (the ledger retires a control that is shipped)", True,
          capture(check_controls, synth(
              LEDGER.replace("`C9`", "`C1`") + "\nC1 is withdrawn.\n"), HAVE, []),
          "the self-test ships it")
    # The literals are BUILT, not written: this file is inside the population
    # the rule scans, so a `NN-control` literal here would red the very check it
    # probes -- the self-exemption the rule was just widened to remove.
    _n = 55
    _cnt = Path(tempfile.mkdtemp()) / "counted.sh"
    _cnt.write_text(f"# turns the entire {_n}-control self-test into a no-op\n")
    probe("CONTROLS negative (a control count in a shipped file)", True,
          capture(check_controls, synth(LEDGER), HAVE, [_cnt]),
          f"{_n}-control")
    _cnt.write_text("# turns the whole self-test into a no-op\n")
    probe("CONTROLS positive (the same sentence stating the property instead)", False,
          capture(check_controls, synth(LEDGER), HAVE, [_cnt]))
    # The count population is a glob, and a glob's BREADTH needs a control or
    # narrowing it back to a hand list is a silent no-op -- which is what the
    # hand list was: it omitted `claim_gate_reruns.py` from the round that
    # created it, so the rule went on excluding one of its own authors. The
    # membership is derived from a different source here, the import graph plus
    # the two shell files the checker holds as constants, so a mutation that
    # shrinks the glob has something to disagree with.
    # `.exists()` because this file lands one PR BEFORE the wire and its shell
    # self-test: on the prerequisite branch the two are not there to count, and
    # a probe demanding them would red a branch for not yet shipping them.
    _shipped = {p.resolve() for p in (host.WIRE, ledgers.SELFTEST_FILE) if p.exists()}
    _shipped |= {Path(m.__file__).resolve() for m in list(sys.modules.values())
                 if getattr(m, "__file__", None)
                 and Path(m.__file__).resolve().parent == ledgers.TOOLS}
    # Bound: this derivation shares the glob's directory assumption, so it
    # catches a narrowed PATTERN but not a companion placed in a subdirectory.
    _uncounted = sorted(p.name for p in _shipped
                        - {q.resolve() for q in host.shipped_files()})
    probes.append(("CONTROLS counts every file this program ships, not a list of them",
                   not _uncounted,
                   f"uncounted: {_uncounted}" if _uncounted
                   else f"{len(_shipped)} shipped file(s), all in the population"))

    probe("CONTROLS reports rather than passing when the self-test is absent", False,
          capture(check_controls, synth(LEDGER), {}, []))
    probe("  …and it says so through skip()", True,
          capture_skipped(check_controls, synth(LEDGER), {}, []),
          "did not reach the closing summary")

    # The inventory comes from a RUN, so a run killed part-way by `set -e` would
    # hand back a prefix and every control after the abort would be reported as
    # a citation the memo invented -- a flood of findings one line from their
    # real cause. The closing summary is the completion sentinel. Both
    # directions, against a stand-in self-test.
    #
    # The stand-in is swapped on `ledgers`, not on `host`. `SELFTEST_FILE` lives
    # in that module now, and `implemented_controls` resolves it through that
    # module's globals -- a rebinding on the entry point changes a name nothing
    # reads, and the run silently measures the REAL self-test instead. `C97` is
    # the discriminator: an id no real control carries, so a probe reading the
    # wrong file cannot pass by finding a live id in it. (Keyed on `C1` this
    # probe passed through exactly that mistake while its partner went red.)
    _saved_st = ledgers.SELFTEST_FILE
    _st = Path(tempfile.mkdtemp()) / "claim-provenance-selftest.sh"
    try:
        ledgers.SELFTEST_FILE = _st
        _n2 = 2
        _st.write_text("#!/usr/bin/env bash\n"
                       "echo '  ok    C97 positive  x'\n"
                       "echo '  ok    C97 negative  y'\n"
                       f"echo 'claim-provenance self-test: {_n2}/{_n2} controls ok'\n")
        _inv = host.implemented_controls()
        probe("CONTROLS inventory (a complete run is read)", True,
              [f"{sorted(_inv)}"] if _inv else [], "C97")
        _st.write_text("#!/usr/bin/env bash\n"
                       "echo '  ok    C97 positive  x'\n"
                       "echo 'bash: line 9: syntax error'\nexit 2\n")
        _inv = host.implemented_controls()
        probe("CONTROLS inventory (a run with no closing summary yields nothing)", False,
              [f"{sorted(_inv)}"] if _inv else [])
    finally:
        ledgers.SELFTEST_FILE = _saved_st

    # PATHS. `gone` is injected for the same reason `have` is: these decide the
    # check's logic, not today's branch. The derivation that reads git gets its
    # own proof below, because injecting the population everywhere would leave
    # the half that talks to the repository with no firing proof at all.
    check_paths = host.check_paths
    GONE = ".claude/tools/claim_gate_grammar.py"
    LEDGER_GONE = (f"### §0.1 What this PR removes\n\n- `{GONE}` — deleted on the "
                   f"prerequisite branch; the sentences naming it are past tense\n")
    NAMES_GONE = ("\n## §5 Decomposition\n\nThe second transcription lived in "
                  "`claim_gate_grammar.py` until the prerequisite branch took it.\n")
    probe("PATHS positive (a deleted file the memo declares)", False,
          capture(check_paths, synth(LEDGER_GONE + NAMES_GONE), [GONE]))
    probe("PATHS negative (a deleted file the memo still names, undeclared)", True,
          capture(check_paths, synth(NAMES_GONE), [GONE]),
          "owes a row")
    probe("PATHS negative (a ledger row no sentence outside it names)", True,
          capture(check_paths, synth(LEDGER_GONE), [GONE]),
          "outlived its subject")
    # The parking arm asks whether the SUBJECT came back, so a tracked path is
    # the finding and an absent one is not -- a row for a file the prerequisite
    # PR removed must survive that PR landing, which is when `gone` empties.
    _here = ".claude/tools/claim_gate_ledgers.py"
    probe("PATHS negative (a ledger row whose subject is back in the tree)", True,
          capture(check_paths, synth(
              f"### §0.1 What this PR removes\n\n- `{_here}` — deleted\n"
              + NAMES_GONE.replace("claim_gate_grammar.py", "claim_gate_ledgers.py")),
              []),
          "back in the tree")
    probe("PATHS positive (a row survives its deletion leaving the branch range)", False,
          capture(check_paths, synth(LEDGER_GONE + NAMES_GONE), []))
    # A subject can be absent for two reasons and only one of them is a record:
    # the file was removed, or the path was never a file. The parking arm asks
    # whether the subject came BACK, and both answer no -- so a misspelling
    # parks in silence. Measured before this probe existed: `claim_gate_grammer`
    # declared, named outside, `gone=[]`, zero findings. The probe above is this
    # one's positive control and must stay green: the two differ only in
    # history, which is why the arm reads it.
    _FAB = LEDGER_GONE.replace("claim_gate_grammar", "claim_gate_grammer") \
        + NAMES_GONE.replace("claim_gate_grammar", "claim_gate_grammer")
    probe("PATHS negative (a ledger row naming a path no commit ever had)", True,
          capture(check_paths, synth(_FAB), []),
          "no commit")
    # In a shallow clone `git log HEAD` sees one commit, so every genuine removal
    # would read as a path that never existed and the arm above would fire
    # hardest on the memos that are most correct. It reports instead.
    _saved_hist = git_.removed_in_history
    try:
        git_.removed_in_history = lambda _p: None
        probe("PATHS does not decide when the history cannot be asked", False,
              capture(check_paths, synth(_FAB), []))
        probe("  …and the unaskable history is reported through skip()", True,
              capture_skipped(check_paths, synth(_FAB), []),
              "cannot say whether")
    finally:
        git_.removed_in_history = _saved_hist
    probe("PATHS positive (a deleted file no sentence names owes nothing)", False,
          capture(check_paths, synth("## §5 Decomposition\n\nNothing about it.\n"),
                  [GONE]))
    # The declaration is licensed AT the row and nowhere else, so the row's own
    # mention must not satisfy the memo's obligation to name what it discusses.
    probe("PATHS negative (a ledger under a heading that is not §0.1)", True,
          capture(check_paths, synth(LEDGER_GONE.replace("§0.1", "§0.9")), [GONE]),
          "owes a row")
    # The tree decides, not the log: `--diff-filter=D` names a path deleted in
    # one commit of the range whether or not a later one puts it back, and a
    # restored file is not one this branch removes.
    _sub = host.gone_from("a.py\nb.py\n", {"b.py"})
    probes.append(("PATHS population subtracts a deletion the tree has back",
                   _sub == ["a.py"], f"{_sub}"))
    # Swapped on `git_` for the reason the CONTROLS stand-in above is swapped on
    # `ledgers`: the function lives there and `check_paths` resolves it through
    # that module. It used to live in `ledgers`, and this swap followed it --
    # the rule is about ownership, not about which file the check is in.
    _saved_gone = git_.deleted_on_branch
    try:
        git_.deleted_on_branch = lambda: None
        probe("PATHS reports rather than passing when the range cannot be formed", False,
              capture(check_paths, synth(NAMES_GONE)))
        probe("  …and it says so through skip()", True,
              capture_skipped(check_paths, synth(NAMES_GONE)),
              "could not be formed")
    finally:
        git_.deleted_on_branch = _saved_gone
    _gone = git_.deleted_on_branch()
    probes.append(("PATHS derives its population from this repository's git",
                   _gone is not None, f"origin/main..HEAD deletes {_gone}"))

    # The git substrate. Three sites asked "what does this branch change" and
    # answered it three ways, and the live repository cannot tell them apart:
    # its merge base IS `origin/main`, and it contains no rename. These are the
    # discriminating arms, on histories built to have both.
    _saved_repo, _saved_res = host.REPO, dict(host._resolved)
    try:
        def at(steps: list[list[str]], td: str) -> None:
            host.REPO = git_fixture(Path(td), steps)
            host._resolved.clear()

        with tempfile.TemporaryDirectory() as _td:
            at(DIVERGED, _td)
            probe("git: what the branch changed is anchored at the merge base",
                  False, capture(check_tooling_quantities,
                                 synth("The prerequisite PR touches one file.\n")))
        with tempfile.TemporaryDirectory() as _td:
            at(renamed("old.sh", "new.sh"), _td)
            _d = git_.deleted_on_branch()
            probes.append(("git: a rename is a departure the deletion set reports",
                           _d == ["old.sh"], f"{_d}"))
        with tempfile.TemporaryDirectory() as _td:
            at(renamed("x-trip-wire.sh", "x.sh"), _td)
            _w = registries.changes_the_wire_set()
            probes.append(("git: a wire renamed out of the set changes the set",
                           _w is True, f"{_w}"))
        # "add or remove" is what the docstring says and what SITES is for: a
        # governing file's collective sentences are falsified by the set gaining
        # or losing a member, not by a member changing inside. Touching one line
        # of an existing wire answered True, which puts every set-level line in
        # the tree on the disposal list of a memo whose PR changed no set.
        for _steps, _want, _label in (
                (EDITS_A_WIRE, False, "editing a wire in place does not change the set"),
                (ADDS_A_WIRE, True, "adding a wire does change the set")):
            with tempfile.TemporaryDirectory() as _td:
                at(_steps, _td)
                _w = registries.changes_the_wire_set()
                probes.append((f"SITES: {_label}", _w is _want, f"{_w}"))
    finally:
        host.REPO = _saved_repo
        host._resolved.clear()
        host._resolved.update(_saved_res)

    # Shallowness is a property of the CLONE, not of the tree, so no fixture
    # history supplies it: the probe intercepts the one question that reports it
    # and lets every other call through. Stubbing `removed_in_history` instead
    # would prove the caller handles None, which is what the PATHS pair above
    # proves -- measured, deleting the guard itself left the suite at 153/153
    # until this arm existed, so the pair was not covering it.
    _real_sh = host.sh

    def _shallow_sh(cmd: list[str]) -> tuple[int, str]:
        return (0, "true\n") if cmd[-1] == "--is-shallow-repository" else _real_sh(cmd)

    try:
        host.sh = _shallow_sh
        _shallow = git_.removed_in_history(".claude/tools/claim_gate_grammar.py")
    finally:
        host.sh = _real_sh
    probes.append(("git: a shallow clone cannot say whether a path was removed",
                   _shallow is None, f"{_shallow}"))

    # A companion reaches entry-point names as `host.X`. A BARE name that only
    # resolves over there raises NameError on the first run that takes the line,
    # and until then reads as working code: `CITE_RE` crossed a split that way
    # and was caught because a probe happened to execute its line, which is luck
    # rather than a property. The rule is structural -- a companion has no free
    # global beyond builtins and its own imports -- so it is derived by scoping
    # the source, not by grepping for names anyone remembered to list.
    #
    def _free_globals(src: str, label: str) -> list[str]:
        st = symtable.symtable(src, label, "exec")
        top = {s.get_name() for s in st.get_symbols()
               if s.is_assigned() or s.is_imported()}
        out, todo = [], [st]
        while todo:
            s = todo.pop()
            todo.extend(s.get_children())
            out += [n for n in (y.get_name() for y in s.get_symbols()
                                if y.is_global() and not y.is_assigned())
                    if n not in top and not hasattr(builtins, n)
                    and not n.startswith("__")]
        return sorted(set(out))

    # The population is the IMPORT GRAPH, not the `claim[-_]*` glob. A glob has
    # no control on its own breadth: narrowing it to one file leaves this probe
    # green, because the one file is the entry point and the filtered set is
    # empty -- an empty population passes every membership test there is, which
    # is the `ban_control` shape. A companion is defined by what the entry point
    # did to it: `host` bound to the entry point module. Nothing else in the
    # process satisfies that, and it cannot be narrowed without unbinding a
    # companion, which every check reaching through `host` would notice.
    _me = sys.modules[host.__name__]
    _companions = sorted(
        (m for m in list(sys.modules.values())
         if getattr(m, "host", None) is _me and getattr(m, "__file__", None)),
        key=lambda m: m.__file__)
    _stray = [f"{Path(m.__file__).name}:{n}"
              for m in _companions
              for n in _free_globals(Path(m.__file__).read_text(),
                                     Path(m.__file__).name)]
    probes.append(("no companion reaches an entry-point name as a bare global",
                   not _stray, f"{_stray}" if _stray else "(clean)"))
    # The scanner needs its own positive. Over a clean tree it returns [] whether
    # or not it descends into function scopes, so narrowing the walk to module
    # level is invisible -- measured: deleting the `get_children()` recursion
    # left the suite green. A planted source pins both arms, and the second
    # plant is nested two deep because that is the arm the narrowing removes.
    # The names live in a string literal, so scanning THIS file does not find
    # them (a needle written as code would make the probe its own subject).
    _PLANT = ("import os\n"
              "TOP = 1\n"
              "def f():\n"
              "    return TOP + os.sep + OUTER_STRAY\n"
              "def g():\n"
              "    def h():\n"
              "        return NESTED_STRAY\n"
              "    return h\n")
    _found = _free_globals(_PLANT, "<plant>")
    probes.append(("…and the scan reaches strays at module and nested scope",
                   _found == ["NESTED_STRAY", "OUTER_STRAY"], f"{_found}"))
    # …and the population is not empty, because the sentence above is a
    # universal and a universal over nothing is true for free.
    probes.append((f"…over every companion the entry point bound "
                   f"({len(_companions)} of them)", len(_companions) >= 4,
                   ", ".join(Path(m.__file__).name for m in _companions) or "(none)"))

    # SITES' population is the PR that changes the wire set, not every document.
    # The docstring's check table is presented as the index, and an index that
    # is recalled rather than derived reads as coverage while missing members.
    # It carried ten of twelve codes for as long as TOOLING and UMBRELLA have
    # existed, and no check could see that because the subject is this program.
    _roster = set(re.findall(r"^  ([A-Z]{4,})\s{2,}",
                             (REPO / ".claude" / "tools" /
                              "claim-gate-plan-check.py").read_text(), re.M))
    _codes = set(re.findall(r'bad\("([A-Z]+)"',
                            "".join(q.read_text() for q in host.shipped_files()
                                    if q.suffix == ".py")))
    probes.append(("DOCSTRING roster names every finding code and no others",
                   _roster == _codes,
                   f"roster−codes={sorted(_roster - _codes)} "
                   f"codes−roster={sorted(_codes - _roster)}"))

    # Applicability, both directions, against the real entry point. The gate is
    # what turns "all 68 plan-memos exit 1" into "0 of 68".
    _subj = host.REPO / "docs" / "plans" / "zz-probe-subject.md"
    try:
        _subj.write_text("# probe\n\nA memo about something else entirely.\n")
        _rc, _out = sh([sys.executable, str(REPO / ".claude" / "tools" /
                        "claim-gate-plan-check.py"), str(_subj.relative_to(REPO))])
        probes.append(("APPLICABILITY a memo that does not name this tool is declined",
                       _rc == 0 and "check(s) not run" in _out, f"rc={_rc}"))
        # A decline is non-coverage, and this program's rule for non-coverage is
        # that it goes through `skip()` and is counted on the closing line --
        # `0 finding(s)` printed by a run in which nothing executed is the shape
        # `skip()` exists for. The decline printed a sentence a human reads and
        # left the machinery saying nothing, so a declined run and a clean one
        # produced the same closing line.
        #
        # In-process as well as out, and not for redundancy: the subprocess arm
        # can only read rc and stdout, so it cannot say WHICH checks declined,
        # and `--arm-coverage` cannot see a `skip()` taken in a child. Both were
        # true of the first version of this probe, which left the new site
        # unfired by anything the tracer could observe.
        # `main` prints its summary; a probe is not a place to print one, and
        # the suite's own output is compared line-for-line elsewhere.
        with contextlib.redirect_stdout(io.StringIO()):
            _declined = capture_skipped(lambda: host.main(["x", str(_subj)]))
        _want = sorted(host.finding_codes())
        _got = sorted({d.split(" — ")[0] for d in _declined})
        probes.append(("…declining names every code the program can report",
                       _got == _want and len(_want) >= 12,
                       f"{len(_got)} of {len(_want)}"
                       + (f"; missing {sorted(set(_want) - set(_got))}"
                          if set(_want) - set(_got) else "")))
        _subj.write_text("# probe\n\nRun `claim-gate-plan-check` on it.\n\n"
                         "## §4 x\n\nIt cites §9.\n")
        _rc2, _out2 = sh([sys.executable, str(REPO / ".claude" / "tools" /
                          "claim-gate-plan-check.py"), str(_subj.relative_to(REPO))])
        probes.append(("APPLICABILITY a memo that does name it is checked",
                       _rc2 == 1 and "[SECT]" in _out2, f"rc={_rc2}"))
    finally:
        _subj.unlink(missing_ok=True)

    probe("SITES positive (a memo that ships no wire is not its subject)", False,
          capture(check_sites, synth("## §4 x\nA memo about something else entirely.\n"), False))
    # The discriminating arm the two probes below never had between them: a memo
    # that NAMES a wire and ships none. Under the old name-gate this collected a
    # finding per governing line; the gate is the branch's diff now, so naming
    # one changes nothing.
    probe("SITES positive (naming a wire this PR does not ship is not shipping one)", False,
          capture(check_sites, synth(
              "## §4 x\nFollow the precedent of "
              "`.claude/tools/layout-box-reader-trip-wire.sh`, untouched here.\n"), False))
    probe("  …and the same memo owes every disposal once the PR does ship one", True,
          capture(check_sites, synth(
              "## §4 x\nFollow the precedent of "
              "`.claude/tools/layout-box-reader-trip-wire.sh`, untouched here.\n"), True),
          "SITES")
    probe("  …and it says so through skip()", True,
          capture_skipped(check_sites, synth("## §4 x\nA memo about something else.\n"), False),
          "does not change the wire set")
    probe("SITES negative (a memo that ships a wire owes the disposal)", True,
          capture(check_sites, synth(
              "## §4 x\nS1 adds `.claude/tools/claim-provenance-trip-wire.sh`.\n"), True),
          "SITES")

    probe("FENCE ban ignores fence prose with no sameness claim", False,
          capture(check_fence, synth(
              "## §6 x\nThe fence arm is defined per surface; only a matching "
              "marker closes the block, in both implementations.")))


    # ------------------------------------------------------------------
    # The thirteen arms `--arm-coverage` reported as reached by no probe.
    # Each is a `ban_control` by this program's own definition, and the file's
    # opening sentence asserted the opposite of their existence.
    # ------------------------------------------------------------------
    probe("PAIR negative (a memo with no §2 at all)", True,
          capture(check_pairs, synth("## §4 x\n\nNothing pairwise here.\n")),
          "no §2")
    probe("PAIR negative (§2 with a single invariant has no pairs to enumerate)", True,
          capture(check_pairs, synth("## §2 Coupled invariants\n\n"
                                     "| **I1** | invariant 1 | forces something |\n")),
          "nothing to pair")
    probe("PAIR negative (a pair naming an invariant §2 does not define)", True,
          capture(check_pairs, synth(SYNTH_PAIRS + "\n| I1 × I9 | intersection |")),
          "I9")
    probe("SECT negative (a cited umbrella section the umbrella has no heading for)", True,
          capture(check_section_refs, synth("## §4 x\n\nSee umbrella §9.\n"),
                  synth("## §1 D\n", "umbrella.md")),
          "umbrella §9")
    probe("SECT positive (a cited umbrella section the umbrella does define)", False,
          capture(check_section_refs, synth("## §4 x\n\nSee umbrella §1.\n"),
                  synth("## §1 D\n", "umbrella.md")))
    probe("CITE negative (a basename naming several tracked files)", True,
          capture(check_citations, synth("## §4 x\n\nSee `mod.rs:5`.\n")),
          "tracked files")
    probe("CITE negative (an inverted range)", True,
          capture(check_citations, synth(
              "## §4 x\n\nSee `claim_gate_ledgers.py:10-5`.\n")),
          "inverted range")
    # ANNO's absent-wire arm is the one that actually runs on the prerequisite
    # branch, and `with_wire()` always materialised a stub, so it was the least
    # exercised path in the file and the most travelled in the tree.
    _saved_wire = host.WIRE
    try:
        host.WIRE = REPO / ".claude" / "tools" / "zz-no-such-wire.sh"
        probe("ANNO reports rather than passing when the wire is not in the tree", False,
              capture(host.check_annotations))
        probe("  …and it says so through skip()", True,
              capture_skipped(host.check_annotations), "is not in this tree")
    finally:
        host.WIRE = _saved_wire
    _saved_range = registries.changes_the_wire_set
    try:
        registries.changes_the_wire_set = lambda: None
        probe("SITES reports rather than deciding when the range cannot be formed", True,
              capture_skipped(check_sites, synth("## §4 x\n"), None),
              "could not be formed")
    finally:
        registries.changes_the_wire_set = _saved_range
    _saved_gov = registries.GOVERNING
    try:
        registries.GOVERNING = ["zz-no-such-governing-file.toml"]
        probe("SITES reports a governing file that has left the tree", True,
              capture_skipped(check_sites, synth(
                  "## §4 x\nS1 ships a wire.\n"), True),
              "is not in the tree")
    finally:
        registries.GOVERNING = _saved_gov
    probe("PREFLIGHT negative (a pasted exit code the tool does not return)", True,
          capture(check_preflight, write_probe(
              PREFLIGHT_FIXTURE + real_pf.replace("EXIT=0", "EXIT=9"))),
          "EXIT=9")
    _saved_skill = host.SKILL
    try:
        host.SKILL = REPO / "zz-no-such-skill-dir"
        probe("FENCE reports when the two trackers cannot be loaded at all", True,
              capture(check_fence, synth("## §4 x\n")),
              "cannot run the two in-repo fence trackers")
    finally:
        host.SKILL = _saved_skill
    # The needle is BUILT, not written: a literal here is a string in the tree,
    # so `git grep` finds this file and the command stops printing nothing --
    # the probe would then measure its own source rather than the arm.
    _absent = "zz" + "zznotpresent"
    probe("TRANSCRIPT arrow form, a command that now prints nothing", True,
          capture(check_transcripts, synth(
              f"## §4 x\n\n```\ngit grep -l {_absent} -- .claude   → 3\n```\n")),
          "prints nothing at all")
    probe("TRANSCRIPT $-form, pasted non-listing output that does not reproduce", True,
          capture(check_transcripts, synth(
              "## §4 x\n\n```\n$ git ls-files -- '.claude/tools/claim_gate_ledgers.py'\n"
              "some other answer entirely\n```\n")),
          "does not reproduce")

    # The file's opening sentence, decided by running the probes rather than by
    # asserting it. Shelled out because the measurement traces `selftest()` and
    # cannot trace itself; the child sees the guard and does not shell back.
    if os.environ.get(host.ARM_COVERAGE_ENV):
        # NOT omitted. A probe that removes itself under a condition takes the
        # count down with it and the run still closes `N/N ok`, which is the
        # `ban_control` shape this block was added to abolish -- measured:
        # under the guard the suite printed 136/136 with no ARMS row and no
        # notice. It stays in the list, and it stays RED, because under the
        # guard the universal is not being measured by anything.
        probes.append(("ARMS every bad()/skip() site is fired by some probe",
                       False, f"not measured: {host.ARM_COVERAGE_ENV} is set, so "
                              f"this run is the measurement's own child"))
    else:
        _env = dict(os.environ, **{host.ARM_COVERAGE_ENV: "1"})
        _r = subprocess.run(
            [sys.executable, str(REPO / ".claude" / "tools" / "claim-gate-plan-check.py"),
             "--arm-coverage"], cwd=REPO, capture_output=True, text=True, env=_env)
        # A crashed child prints nothing on stdout, which is byte-identical to a
        # clean measurement. Reading only `stdout` made a planted
        # `raise RuntimeError` in the child close the suite green -- and
        # `arm_sites()` parses every shipped file while `unreached_arms()` runs
        # the whole probe set, so a syntax error or any raising probe produces
        # exactly that shape.
        if _r.returncode != 0 or _r.stderr.strip():
            probes.append(("ARMS every bad()/skip() site is fired by some probe",
                           False, f"the measurement did not complete: rc={_r.returncode} "
                                  f"{_r.stderr.strip().splitlines()[-1:] }"))
        else:
            _unreached = [x for x in _r.stdout.split("\n") if x.strip()]
            _declared = sorted(host.UNPROBED)
            _extra = [x for x in _unreached if x.split()[0] not in _declared]
            _stale = [d for d in _declared
                      if d not in [x.split()[0] for x in _unreached]]
            probes.append(("ARMS every bad()/skip() site is fired by some probe",
                           not _extra and not _stale,
                           f"unfired: {_extra}" if _extra else
                           f"UNPROBED declares {_stale}, which the probes do fire"
                           if _stale else f"{len(_unreached)} unfired, all declared"))

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
