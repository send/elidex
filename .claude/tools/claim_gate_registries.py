#!/usr/bin/env python3
"""The two checks whose coverage is a hand-declared SEED, and which therefore
print their own complement.

Split out of `claim-gate-plan-check.py` at 1154 lines (CLAUDE.md's touch-time
discipline, which fires at 1000 and is not deferrable). The seam is a shared
mechanism rather than a line budget, and it is the one property no other check
in this program has:

  * `SITES` reads `GOVERNING`, a hand-written list of five paths;
  * `UMBRELLA` reads `BINDING`, a hand-written list of decision tuples.

Every other check derives its population -- `TOOLING` from `shipped_files()`,
`CITE` from `git ls-files`, `CONTROLS` by RUNNING the self-test, `PATHS` from
git. A derived population is wrong when the derivation is wrong, which a probe
can catch. A hand-written one is wrong the moment the world gains a member
nobody added, which no probe can catch -- so each of these two owes a
COMPLEMENT REPORT, printing what its registry does not cover on every run.
"The registry says what it covers; the report says what it does not."
`report_governing_complement` and `report_umbrella_coverage` are that, and
neither is optional: `GOVERNING` declared itself a seed for several rounds
while nothing printed the rest, which reads to a reader as coverage.

Measured before the split, so the pair is the whole population rather than the
two that were noticed: `GOVERNING` and `BINDING` are the only module-level
registry constants in the program. `claim_gate_reruns.report_transcripts` is a
third reporter and is deliberately NOT here -- it takes its list as an
argument, derived per run from what the check could not re-execute, and owns no
registry. Its docstring names itself the precedent this shape follows, which is
a different relation from being an instance of it.

`host` is the entry point, bound by it at import time exactly as the other
companions are. `bad`, `skip` and the memo model are resolved through it on
every call and never imported by name: the probe file REBINDS `host.FAILS` and
`host.SKIPPED` for the duration of one check, so a name bound once here would
append to the list that was swapped out, `capture()` would return empty, and
every negative probe would silently invert.
"""
from __future__ import annotations

import re
from types import ModuleType

import claim_gate_git

host: ModuleType  # bound by the entry point at import time
# Annotations spell `host.Memo`, not `Memo`: the class lives in a file whose
# name has hyphens and cannot be imported, so a bare `Memo` here resolves to
# nothing for any tool that evaluates annotations.


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
        f = host.REPO / rel
        if not f.exists():
            # A silent `continue` here loses a fifth of the population to a
            # rename, with no finding and no notice.
            host.skip("SITES", f"{rel} is named as a governing file and is not in the "
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
    rc, out = host.sh(["git", "ls-files"])
    if rc != 0:
        return
    rows: list[tuple[str, int]] = []
    for rel in out.split("\n"):
        if not rel or rel in GOVERNING or WIRE_FILE.match(rel):
            continue
        f = host.REPO / rel
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


def check_sites(memo: host.Memo, ships: bool | None = None) -> None:
    # Round 10: this fired on every memo in the tree, so the prerequisite PR --
    # which lands one PR BEFORE the memo this tool is for -- put a checker on
    # `main` that exits 1 on all 68 of them, for an obligation none of them
    # has. The obligation arises from changing the wire set, not from being a
    # plan-memo, and a check whose population is "every document handed to me"
    # reports a defect that is really a category error.
    ships = changes_the_wire_set() if ships is None else ships
    if ships is None:
        host.skip("SITES", "`origin/main...HEAD` could not be formed in this clone, so "
                      "whether this PR changes the wire set was not decided")
        return
    if not ships:
        host.skip("SITES", f"`git diff --name-only origin/main...HEAD` adds no "
                      f"`*-trip-wire.sh`, so this PR does not change the wire set "
                      f"and no line describing that set needs disposing of here "
                      f"({memo.path.name} may still name a wire it does not ship)")
        return
    # Both spellings count as citing a line: the backticked `path:12-14` form,
    # and the bare `path:12:` form a pasted `git grep -n` listing produces --
    # §4.6's edit set is entirely of the second kind, and reading only the first
    # reported eleven sites as undisposed when §4.6 pastes six of them verbatim.
    cited = set()
    for m in host.CITE_RE.finditer(memo.text):
        rel, a, b = m.group(1), int(m.group(2)), m.group(3)
        for h in host.resolve(rel):
            for k in range(a, (int(b) if b else a) + 1):
                cited.add(f"{h}:{k}")
    for m in re.finditer(r"^([\w./-]+):(\d+):", memo.text, re.M):
        for h in host.resolve(m.group(1)):
            cited.add(f"{h}:{m.group(2)}")
    loose = set(re.findall(r"`?([\w./-]+\.(?:yml|toml|md|sh))`?[: ]", memo.text))

    for rel, i, line in wire_set_lines():
        if f"{rel}:{i}" in cited:
            continue
        star = (" (the memo names the file but not this line)"
                if rel.split("/")[-1] in loose else "")
        host.bad("SITES", f"{rel}:{i} speaks about the wire set and the memo does not "
                     f"cite it{star} -- edit it or cite it as deliberately left "
                     f"alone: {line[:60]}")

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


def report_umbrella_coverage(umbrella: host.Memo | None) -> None:
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
    for c in host.data_rows(sect3):
        joined = " | ".join(c)
        if NAMES_S1.search(joined) and not amended_here:
            rows.append(f"§3 row — {c[2].strip()[:64] if len(c) > 2 else joined[:64]}")
    if rows:
        print(f"  umbrella: {len(rows)} binding item(s) with no BINDING entry "
              f"(the registry is a seed; a withdrawal here is caught by review, "
              f"not by this tool):")
        for r in rows:
            print(f"    {r}")


def check_umbrella(memo: host.Memo, umbrella: host.Memo | None) -> None:
    # Invoked with one argv path, this check has nothing to compare against and
    # returned quietly -- and so did the umbrella's own citations, its
    # transcripts, and every `umbrella §N` reference in `check_section_refs`.
    # The whole registry below, whose subject is a withdrawal swept through the
    # sites that STATE it and not the document that OBLIGES it, was disabled by
    # dropping an argument.
    if umbrella is None:
        host.skip("UMBRELLA", "no umbrella memo on argv: no binding decision was "
                         "compared, and the umbrella's own citations, transcripts "
                         "and `umbrella §N` targets went unchecked with it")
        return
    for name, mandates, withdraws, amended in BINDING:
        if re.search(amended, umbrella.text, re.S):
            continue
        if re.search(mandates, umbrella.text, re.S) and re.search(withdraws, memo.text, re.S):
            host.bad("UMBRELLA", f"the slice withdraws {name} while the umbrella still "
                            f"mandates it — a binding decision cannot be changed by a "
                            f"slice edit; amend the umbrella in the same commit or "
                            f"withdraw the slice's change")
