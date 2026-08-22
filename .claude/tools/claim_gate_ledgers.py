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
  3. the licence is scoped BY POSITION rather than by reading intent out of a
     sentence: both checks measure mentions outside the ledger's own span, so
     the row's own text is never what answers the question about it;
  4. the ledger fails in both directions. A row whose subject is back -- a
     control the self-test ships, a path the tree has -- is a finding, or the
     section becomes somewhere to park a citation rather than a record of one.

What they do NOT share is what the outside span is measured FOR, and the first
draft of this header claimed they did -- "a row cannot satisfy the obligation it
exists to record", asserted of both. Run them and the polarities are opposite:

  * `CONTROLS` FORBIDS the outside mention. A retired id named anywhere but its
    ledger row is a finding, because the ledger licenses recording a
    retirement, not going on citing the control as live support (two sections
    did exactly that).
  * `PATHS` REQUIRES it. A row whose path no sentence outside §0.1 names is a
    finding, because the row exists to re-declare what the memo still says
    about a file, and a row with nothing to re-declare is parking.

Both directions are pinned by probes, so each is deliberate; a shared helper
for #3 would take a polarity flag and buy nothing. Only #4 -- "the row's subject
came back" -- is one rule with two populations, and that is the piece worth
collapsing if a third ledger ever arrives. Property #3's shared part is the
POSITION discipline itself, and it is stated that narrowly above.

Two further asymmetries, so the list above is not read as more than it is:
`CONTROLS` matches an id TOKEN and `PATHS` a basename, and a generic
`mentions()` would have to pick one and change the other check -- that
argument was tested rather than inherited, and it holds: `CONTROLS` does not
ask a yes/no question. It extracts ids, sub-arms and byte positions, and a
bool-returning helper cannot serve it. What did NOT survive the test is the
word this passage used to carry, "SUBSTRING": `PATHS` matched raw, so a
sentence about a sibling counted as naming the deleted file, and 332 pairs of
tracked basenames in this tree have one inside the other. Both checks match at
a BOUNDARY now, which is a shared discipline rather than a shared function --
`PATHS` adopting what `CONTROLS` already did. And `CONTROLS`
fails on a memo naming a non-member where `PATHS` deliberately does not -- the
headers on CONTROLS and PATHS below record the three populations measured before
that arm was rejected. (They were cited here as "§13/§14", which reads as a memo
section -- every other `§` in this file means one.)

`host` is the entry point, bound by it at import time, exactly as
`claim_gate_reruns` and `claim_gate_selftest` are. `bad` and `skip` are resolved
through it on every call and never imported by name: the probe file REBINDS
`host.FAILS` and `host.SKIPPED` for the duration of one check, so a name bound
once here would append to the list that was swapped out, `capture()` would
return empty, and every negative probe would silently invert.

The same call-time rule runs the other way for the globals the probe file
swaps. `SELFTEST_FILE` lives HERE, so a probe that rebinds it on `host` changes
a name nothing reads; it is reached as an attribute of this module, and for that
reason NOT re-exported by the entry point -- an alias there would publish the
very name rebinding does nothing to. `deleted_on_branch` obeyed the same rule
from this file until the git plumbing moved to `claim_gate_git`; the rule did
not change, only which module owns the name, and `check_paths` reaches it as an
attribute there so a probe swaps it there.
"""
from __future__ import annotations

import re
from pathlib import Path
from types import ModuleType

import claim_gate_git

host: ModuleType  # bound by the entry point at import time
# Annotations below spell the memo type `host.Memo`, not `Memo`: the class
# lives in a file whose name has hyphens and cannot be imported, so a bare
# `Memo` here resolves to nothing for any tool that evaluates annotations.
# `from __future__ import annotations` makes either spelling harmless at
# runtime; this one is the true one.

# `SELFTEST_FILE` below is built at IMPORT time, which is before the entry point
# can bind `host` -- so its directory is derived here rather than read off it.
# `host.REPO` is the same expression one directory up.
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
# "Every file this program ships that can make a claim about itself" is
# `host.shipped_files()`. It lives in the entry point, not here, because two
# checks need it -- this arm and `check_tooling_quantities`, which is not a
# ledger check -- and a population that one of its consumers must import the
# other module to reach is in the wrong home. Called rather than bound: the
# entry point binds `host` after this module loads.
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


def retired_controls(memo: host.Memo) -> tuple[dict[str, str], list[tuple[int, int]]]:
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


def check_controls(memo: host.Memo, have: dict[str, set[str]] | None = None,
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
    for path in (sources if sources is not None else host.shipped_files() + [memo.path]):
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
#     this memo, 2 unresolved, but the great majority of the tree's plan-memos
#     carry at least one, and nearly all of those are ELISIONS of real files
#     (`elidex-dom-api/element/tree.rs` for `crates/dom/.../src/element/tree.rs`)
#     or files outside the repository (`memory/project_open-defer-slots.md`).
#     A check that reds most of the memos in the tree for naming files that
#     exist is the category error `check_sites` was just corrected for. Stated
#     as a proportion rather than a count because the count is a property of the
#     BASE: measured 57 of 70 on the slice branch, and this file ships one PR
#     earlier, where `git ls-files 'docs/plans/*.md' | wc -l` is 68.
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


def check_paths(memo: host.Memo, gone: list[str] | None = None) -> None:
    gone = claim_gate_git.deleted_on_branch() if gone is None else gone
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

    # A row DECLARES a path however it spells it -- basename or full path -- so
    # both spellings are resolved to the same key before comparing. `named` was
    # measured on the basename and `declared` keyed on the row's literal text,
    # so a row written as `claim_gate_grammar.py` for a path git reports as
    # `.claude/tools/claim_gate_grammar.py` satisfied neither, and the memo was
    # told it owed a row it had already written.
    declared_bases = {d.rsplit("/", 1)[-1] for d in declared}

    def _declares(p: str) -> bool:
        return p in declared or p.rsplit("/", 1)[-1] in declared_bases

    for path in gone:
        base = path.rsplit("/", 1)[-1]
        # At a BOUNDARY, not as a substring. `base in elsewhere` counted a
        # sentence about a sibling file as naming this one: measured on this
        # tree, 332 pairs of tracked basenames have one inside the other
        # (`abort.rs` inside `tests_abort.rs`). `CONTROLS` matches its ids at a
        # boundary already; this is PATHS adopting the discipline its sibling
        # has, and NOT a shared `mentions()` -- a bool cannot serve CONTROLS,
        # which extracts ids, sub-arms and positions. The header's argument
        # against merging them survives; what did not survive is the substring.
        named = re.search(rf"(?<![\w.-]){re.escape(base)}(?![\w])", elsewhere) is not None
        if named and not _declares(path):
            host.bad("PATHS", f"this branch deletes `{path}` and {memo.path.name} still "
                         f"names it; §{GONE_SECTION} owes a row saying what the memo "
                         f"means by it now — the sentences around it were written "
                         f"while the file was there")
        elif not named and _declares(path):
            host.bad("PATHS", f"§{GONE_SECTION} declares `{path}`, and no sentence outside "
                         f"that section names it — the row has outlived its subject")
    # The parking-lot arm asks whether the row's SUBJECT came back, not whether
    # this branch is still the one that removed it.
    #
    # Keying it on `path not in gone` was a delayed fuse of exactly the kind
    # `check_tooling_quantities` documents and avoids. `gone` is scoped to
    # `origin/main..HEAD`, and this repository squash-merges: the moment the
    # prerequisite PR lands, the commit that deleted a file leaves the stacked
    # branch's range, `gone` empties, and a §0.1 row that was MANDATORY the day
    # before becomes a finding the author cannot discharge except by deleting a
    # true statement. Measured on this stack: `git log
    # claim-gate-plan-check..stale-claim-detector --diff-filter=D` is already
    # empty while `git log origin/main..stale-claim-detector` still names the
    # file. A row for a file some OTHER branch removed is not parking -- the
    # tree does not have it, so the memo's sentences about it are still about an
    # absent file, which is all the row asserts.
    #
    # It asks only that, though, and a row's subject can be absent for two
    # reasons: the file was removed, or the path was never a file. Both answer
    # "no" to "did it come back", so a misspelling parked in total silence --
    # measured, before the arm below existed: `claim_gate_grammer.py` declared
    # and named outside the section, zero findings. That is the parking lot this
    # module's header says a ledger must not become, opened by the very re-key
    # that closed the delayed fuse. History is what separates the two cases, and
    # `removed_in_history` reports rather than deciding when it cannot be asked.
    for path in declared:
        if host.resolve(path):
            host.bad("PATHS", f"§{GONE_SECTION} declares `{path}` as removed, and "
                              f"`git ls-files` resolves it — the row's subject is "
                              f"back in the tree, so the section is parking a "
                              f"citation rather than recording one")
            continue
        ever = claim_gate_git.removed_in_history(path)
        if ever is None:
            host.skip("PATHS", f"this clone's history cannot say whether `{path}` "
                               f"was ever removed, so §{GONE_SECTION}'s row for it "
                               f"was not checked for having a subject at all")
        elif not ever:
            host.bad("PATHS", f"§{GONE_SECTION} declares `{path}` as removed, and no "
                              f"commit reachable from HEAD ever removed it — the row "
                              f"names nothing, so a misspelling or an invented path "
                              f"reads here as a record")

