#!/usr/bin/env python3
"""The function-shaped controls for `plan-memo-umbrella-check.py --self-test`,
and the registry that names every control.

The record-shaped controls (`Case`: one fixture, one measure, one exact
value) live in `plan_memo_selftest_cases.py` / `_cases_pr510.py` /
`_cases_inline.py` / `_cases_sibling.py`;
this module
holds the controls a record cannot express -- an injected fault (a raising
`resolve()`, a parser `RuntimeError`), a comparison of two runs (the pipe
shape, the attribution), the CommonMark conformance corpus -- and `registry()`,
the ONE name -> (kind, control) table both the runner and the mutation proof
read.  Every control here is a function of the freshly loaded checker module
returning `(ok, detail)`, and every one is reachable by name through the
registry, because the mutant runner re-runs controls BY NAME against a
patched set.

TWO fragments this module's `registry()` merges, each carved out at touch
time and each with a MECHANICAL seam rather than a prose one.  The controls
whose measure is WORK rather than text -- the linearity witnesses, every one
of them written against `_count_calls` / `_count_lines` / `_CountedList` --
are `plan_memo_selftest_work.py` (PR #510 R24); the PROPERTY controls, every
one of which enumerates its own population and sweeps it, reach this module
through `plan_memo_selftest_records.py` (the written record held against the
tree), which merges `plan_memo_selftest_properties.py` (PR #510 R25), which
merges `plan_memo_selftest_invariants.py` (R29) -- the sentence, the checker as
written and the checker run -- and, beside them, `plan_memo_selftest_ratchets.py`
(R49); those fragments are exactly the entries named `PROPERTY: ...`, which
`property_family_control` checks (⚠ this sentence omitted the ratchets from R49
until the fifth attestation's concept sweep).  This module imports no work witness and reads
no module source, AST or code object -- it imports neither `ast` nor the
harness's `MODULES` / `SOURCES` / `GRAMMAR` -- and those two import lists are
the two seams' statement.

`empty_registry_fails`, the runner's emptiness guard, lives here beside its
proof (`empty_registry_control`) so the one mutant against it patches one
file: a mutant row whose file is a SELF-TEST module (this one, or the work
module) is exec'd from patched text
(`plan_memo_selftest_harness.patched_module`) and its controls are taken from
the PATCHED module's registry, merged over the unpatched rest
(`plan_memo_selftest_mutants.SELFTEST`).

Import direction, one way: the runner imports this module; this module
imports the harness (`plan_memo_selftest_harness.py`), the case registry, the
property registry and the work registry.
"""

import pathlib
import tempfile

from plan_memo_selftest_cases import CASES, VIOLATION, build
import plan_memo_selftest_cases_pr510  # noqa: F401 -- appends the review-round controls to CASES
import plan_memo_selftest_cases_inline  # noqa: F401 -- appends the Phase-2 inline rounds to the same CASES
import plan_memo_selftest_cases_sibling  # noqa: F401 -- appends the sibling-resolver family to the same CASES
import plan_memo_selftest_cases_r26  # noqa: F401 -- appends R26's rounds to the same CASES
import plan_memo_selftest_cases_r42  # noqa: F401 -- appends R42's round to the same CASES
from plan_memo_selftest_harness import control, run_on
from plan_memo_selftest_records import registry as property_registry
from plan_memo_selftest_work import registry as work_registry


def attribution_control(M):
    """A pointer slot whose cell opens `Slice **9z** -- **UMBRELLA, ...**`, or
    `Slice `#11-zz-alpha` -- **UMBRELLA, ...**` (PR #510 R20: the slug form
    of the same position), is declaring the NAMED row's kind, not its own.
    §5: a pointer slot "carries no marker of its own".  The count must not
    move when such a row is added, and the row's kind is `pointer`."""
    base = len(run_on(M, build())[0].population.no_owner_ids())
    out = []
    for d in ("**9z**", "`#11-zz-alpha`"):
        pop = run_on(M, build(wb="**(carved at PR-B)** Slice %s — **UMBRELLA, not a terminal unit** — "
                                 "points into §5." % d))[0].population
        out.append((d, len(pop.no_owner_ids()), pop.ids["#11-zz-beta"].kind))
    ok = all(n == base and kind == "pointer" for _, n, kind in out)
    return ok, "a marker naming another row does not enter the count (%d -> %s)" % (
        base, ", ".join("%s: %d %s" % x for x in out))


def degenerate_control(M):
    """A whole-line grep for the marker CANNOT disagree with the marker count;
    the declaring-field parse can.  This proves the two are different programs
    rather than one program written twice."""
    import plan_memo_stream   # the FRESHLY loaded module, not the import-time one
    text = build(d7z="**UMBRELLA, not a terminal unit** stray")
    umb = run_on(M, text)[0].population.no_owner_ids()
    by_grep = sum(1 for l in text.split("\n") if l.startswith("|") and plan_memo_stream.MARKER in l)
    return len(umb) != by_grep, "declaring-field parse=%d vs whole-line marker grep=%d (must differ)" % (
        len(umb), by_grep)


def pipe_shape_control(M):
    """The same table written with and without leading/trailing pipes yields
    the same ids and the same reported sites (I-C cell shape)."""
    piped = ("| Slot | Why deferred | Trigger | Re-eval |\n|---|---|---|---|\n"
             "| `#11-zz-gamma` | **UMBRELLA, not a terminal unit.** Slice 9z lands first. | now | 2026-12-31 |")
    bare = "\n".join(l.strip("|") for l in piped.split("\n"))
    out = []
    for extra in (piped, bare):
        res, reported = run_on(M, build(extra=extra))
        out.append((res.rc, sorted(res.population.no_owner_ids()), sorted((m.id, m.source, m.text[m.start:m.end]) for m in reported)))
    return out[0] == out[1] and out[0][0] != 2, "ids %s, sites %s" % (out[0][1], out[0][2])


def raw_offset_control(M):
    """A site after a `\\|` in its cell is reported at its RAW column."""
    _, reported = run_on(M, build(c1=r"x \| Slice 9z owns it"))
    ok = len(reported) == 1 and reported[0].line[reported[0].col:reported[0].col + 2] == "9z"
    return ok, "col lands on %r" % (reported[0].line[reported[0].col:reported[0].col + 2] if reported else None)


def split_locator_control(M):
    """A site read ACROSS a construct that renders nothing is reported at its
    RAW column (design re-gate 4).  The stream is no longer the raw text --
    `Slice 9**z** owns it` renders `Slice 9z`, four characters shorter -- so
    every reporting coordinate goes through `Stream.at` first.  Without that
    map the column is off by the dropped delimiters and points into the middle
    of the token, which is `raw_offset_control`'s failure mode for the other
    direction (an escaped pipe).  The probe drops four characters BEFORE the
    site (`**Note.**`): a site with nothing dropped in front of it sits at the
    same offset in both texts and cannot tell the readings apart."""
    _, reported = run_on(M, build(), "**Note.**  Slice 9**z** owns it.")
    ok = len(reported) == 1 and reported[0].line[reported[0].col:reported[0].col + 5] == "9**z*"
    at = reported[0].line[reported[0].col:reported[0].col + 5] if reported else None
    return ok, "one site, col lands on %r (the raw `9`, not the stream's)" % at


def empty_registry_fails(n_controls, n_mutants):
    """The emptiness guard: a run over ZERO controls or (when mutants are
    requested) ZERO mutants proves nothing and must be a FAIL, not a green
    `0 mutant(s), 0 survived` -- the trip-wire checks the same two counts
    from the outside, so neither guard is the only one.  `n_mutants` is None
    when mutants were not requested.  Returns the FAIL strings."""
    out = []
    if n_controls == 0:
        out.append("EMPTY REGISTRY: 0 controls ran; a self-test over nothing is not green")
    if n_mutants == 0:
        out.append("EMPTY REGISTRY: 0 mutants ran; a mutation proof over nothing is not green")
    return out


def empty_registry_control(M):
    """The runner-level proof that an empty registry is a FAIL: the guard
    fires on (0, 0), on (0, None), on (n, 0), and is silent on (n, m)."""
    fired = (bool(empty_registry_fails(0, 0)), bool(empty_registry_fails(0, None)),
             bool(empty_registry_fails(5, 0)), bool(empty_registry_fails(5, 7)))
    return fired == (True, True, True, False), "guard fires %s (want True, True, True, False)" % (fired,)


def printable_output_control(M):
    """The runner's report channel escapes EVERY C0 character and DEL, over the
    whole class rather than the one codepoint that was found.  Population: all
    33 of them, not a sample; and a printable character must survive untouched,
    which is the half that would let an over-eager escape pass."""
    bad = []
    for n in list(range(0x20)) + [0x7F]:
        got = M.printable("a%sb" % chr(n))
        if got != "a<U+%04X>b" % n:
            bad.append("U+%04X -> %r" % (n, got))
    kept = M.printable("ok [POSITIVE] `[x](child.md)` -- 3 site(s)")
    if kept != "ok [POSITIVE] `[x](child.md)` -- 3 site(s)":
        bad.append("a printable line was altered: %r" % kept)
    return not bad, ("33 control character(s) checked, %d not escaped%s"
                     % (len(bad), ("; " + "; ".join(bad[:3])) if bad else ""))


def control_char_destination_control(M):
    """A decoded destination holding a C0 control (`child%00.md`) is not a
    sibling: rejected in the validation line, so `Path.resolve()` never
    sees it.  An exception from `check()` is caught HERE and is red -- a
    crash is the defect this control exists for, not a harness accident."""
    try:
        res, _ = run_on(M, build(), "See [x](child%00.md).")
    except Exception as e:       # noqa: BLE001 -- the defect under test
        return False, "check() raised %s: %s" % (type(e).__name__, str(e)[:60])
    return res.rc == 0, "rc %d (must be 0, no exception)" % res.rc


def unavailable_sibling_control(M):
    """(e) of `sibling_path`: an `OSError` (an over-long name) OR, on Python
    3.9-3.12, a `RuntimeError` (a symlink loop) from `resolve()` is the
    unavailable-sibling schema miss, rc 2 -- not an exception.  Neither is
    raised for these names on every platform / version, so both are
    injected: `Path.resolve` raises for the named file while the control
    runs.  An exception from `check()` is red here."""
    import plan_memo_sibling
    orig = plan_memo_sibling.pathlib.Path.resolve
    report = []
    for name, exc in (("a" * 4000 + ".md", OSError(36, "File name too long")),
                      ("loop.md", RuntimeError("Symlink loop from 'loop.md'"))):
        def resolve(self, *a, _name=name, _exc=exc, **kw):
            if self.name == _name:
                raise _exc
            return orig(self, *a, **kw)
        plan_memo_sibling.pathlib.Path.resolve = resolve
        try:
            res, _ = run_on(M, build(), "See [x](%s)." % name)
        except Exception as e:       # noqa: BLE001 -- the defect under test
            return False, "check() raised %s: %s" % (type(e).__name__, str(e)[:60])
        finally:
            plan_memo_sibling.pathlib.Path.resolve = orig
        miss = any(f[0] == "SCHEMA" and "linked memo unavailable" in f[3] for f in res.findings)
        if res.rc != 2 or not miss:
            return False, "%s: rc %d, miss %s (must be rc 2 with the miss)" % (type(exc).__name__, res.rc, miss)
        report.append("%s -> rc 2 + miss" % type(exc).__name__)
    return True, "; ".join(report)


def undecodable_sibling_control(M):
    """The ONE I/O chokepoint: a sibling whose bytes are not UTF-8 is an
    UNAVAILABLE linked memo (rc 2 + the miss), never an exception out of
    `check()`.  An exception here is red."""
    try:
        with tempfile.TemporaryDirectory() as d:
            p = pathlib.Path(d) / "fixture.md"
            p.write_text(build() + "\nSee [bad](bad.md).\n", encoding="utf-8")
            (pathlib.Path(d) / "bad.md").write_bytes(b"\xff\xfe not utf-8 \x80\n")
            res = M.check(str(p))
    except Exception as e:       # noqa: BLE001 -- the defect under test
        return False, "check() raised %s: %s" % (type(e).__name__, str(e)[:60])
    miss = any(f[0] == "SCHEMA" and "linked memo unavailable (UnicodeDecodeError)" in f[3]
               for f in res.findings)
    return res.rc == 2 and miss, "rc %d, unreadable-sibling miss %s (must be rc 2 with the miss)" % (res.rc, miss)


def spec_examples_control(M):
    """The CommonMark 0.31.2 spec's own block examples through Phase 1
    (`plan_memo_selftest_conformance`): every vendored example aligned with
    its expected html or excluded by a stated §3.0 disposition; the multi-
    line detail is printed whole because the exclusion list IS the report."""
    import plan_memo_memo       # the freshly loaded module
    import plan_memo_selftest_conformance as conf
    ok, detail = conf.run(plan_memo_memo)
    # The newlines are LAYOUT and the lines are CONTENT, so the escape goes
    # inside the join -- the same shape the worklist uses, and the one
    # `report_channel_control` names structurally (PR #510 R42-8).
    print("\n       ".join(M.printable(l) for l in ("       " + detail).split("\n")))
    return ok, detail.split("\n")[0]


def inline_examples_control(M):
    """The CommonMark 0.31.2 spec's own example lists for every INLINE section
    the plan's §3.0b calls LEXED or MASKED -- §2.4 backslash escapes, §2.5
    character references, §6.1 code spans, §6.3 links, §6.4 images, §6.5
    autolinks, §6.6 raw HTML -- through Phase 1 and Phase 2
    (`plan_memo_selftest_conformance.run_inline`, the SAME aligner the block
    half runs): each masked raw HTML span verbatim, then one `<a href=` per
    link and autolink, one `<code>` per code span, one `<img src=` per
    resolved image.  The detail is printed whole, like the block half's."""
    import plan_memo_memo       # the freshly loaded module
    import plan_memo_selftest_conformance as conf
    ok, detail = conf.run_inline(plan_memo_memo)
    # The newlines are LAYOUT and the lines are CONTENT, so the escape goes
    # inside the join -- the same shape the worklist uses, and the one
    # `report_channel_control` names structurally (PR #510 R42-8).
    print("\n       ".join(M.printable(l) for l in ("       " + detail).split("\n")))
    return ok, detail.split("\n")[0]


def refused_destination_silence_control(M):
    """PROPERTY: a destination the sibling resolver REFUSES leaves no finding,
    no seed and no note naming it -- and that silence is PINNED, not merely
    current.

    ⚠ WRITTEN BECAUSE THE SILENCE WAS A DECLARED POLICY WITH NO MECHANISM
    (PR #510 R47, blind-spot audit).  `plan_memo_sibling`'s stage (c) refuses a
    reserved character, a DOS device and an anchored path, and says so three
    times in prose -- "dropped without a report … which is this stage's
    standing polarity".  The existing controls pin that such a memo is NOT
    WALKED; nothing pinned that the run says NOTHING.  Measured: inserting a
    `[DEST-REFUSED?]` seed for exactly this case left all 701 controls green,
    so the polarity could be reversed -- in either direction -- in silence.

    ⚠ THE DISCRIMINATING PARTNER IS IN THE SAME RUN: an ordinary sibling in the
    same memo IS walked and DOES appear in the population line.  Without it
    "nothing names these" is also true of a checker that reads no destinations
    at all.

    ⚠ This control states the policy; it does not endorse it.  §8 carries the
    open question -- a could-not-scan that exits 0 sits against §1's own rule --
    and if that is ever decided the other way, THIS control is what turns red
    and makes the decision explicit."""
    import tempfile, pathlib
    with tempfile.TemporaryDirectory() as d:
        root = pathlib.Path(d)
        (root / "sib.md").write_text("# an ordinary sibling\n", encoding="utf-8")
        try:
            (root / "notes:child.md").write_text("# reachable on this fs\n", encoding="utf-8")
        except OSError:
            pass
        memo = root / "m.md"
        # ⚠ The FULL fixture, not the slot table alone: with three schemas
        # missing the run exits 2 on schema misses before it prints the
        # population line, and the discriminating half would read False for a
        # reason that has nothing to do with the destinations.
        from plan_memo_selftest_cases import build
        memo.write_text(build() + "\nSee [a](notes%3Achild.md), [b](NUL.md) and [c](sib.md).\n",
                        encoding="utf-8")
        # ⚠ Read the RESULT, not stdout: `check()` returns and the report is
        # printed by `main()`, so a `redirect_stdout` here captures nothing and
        # BOTH halves read as "not named" -- the refused half would have passed
        # vacuously.
        res = M.check(str(memo))
        out = "\n".join(
            [str(f) for f in res.findings] + [str(n) for n in res.notes]
            + [res.population.display(m.path) for m in res.population.memos])
    refused = [n for n in ("notes%3Achild.md", "notes:child.md", "NUL.md") if n in out]
    walked = "sib.md" in out
    ok = not refused and walked
    return ok, ("refused destinations named in the report: %s (must be none); the ordinary sibling "
                "IS named: %s (must be True)" % (refused or "none", walked))


def demoted_agreement_control(M):
    """Every §3.0b family demoted into a resolved image description, against
    the spec's OWN html -- the cross-product the vendored corpus cannot reach
    (`plan_memo_selftest_conformance.demoted_agreement_control`; 23 of its 335
    inline examples render an `<img>` -- "22" is the Images SECTION's size --
    and none carries a backtick or a `<` in a DESCRIPTION)."""
    import plan_memo_selftest_conformance as conf
    return conf.demoted_agreement_control(M)


def code_span_reading_control(M):
    """The spec's §6.1 examples against the READER'S rendering of a code span
    (`plan_memo_selftest_conformance.run_code_reading`) -- the half the
    conformance charter declined to look at until PR #510 R32, and the half
    the round's finding landed in."""
    import plan_memo_selftest_conformance as conf
    return conf.run_code_reading(M)


# the slot table whose one row is an UMBRELLA, for the linked-memo controls
UMBRELLA_SLOT = ("| Slot | Why deferred | Trigger | Re-eval |\n|---|---|---|---|\n"
                 "| `#11-zz-gamma` | **UMBRELLA, not a terminal unit.** carved. | now | 2026-12-31 |")


def lazy_header_after_definition_control(M):
    """PR #510 R17 #1, the reviewer's consequence end to end: a LINKED memo
    holds a block quote whose first line is a reference definition and
    whose next line is an UNQUOTED schema-table header over a QUOTED
    delimiter row (`> [a]: /u\\n| Slot | … |\\n> |---|…|\\n> | row |`).
    cmark-gfm forms the table inside the quote (measured: the definition is
    paragraph text until the paragraph ends, and the table extension reads
    the header out of the paragraph's last line), so the checker must admit
    it: the row's id is declared, its kind is `umbrella`, the no-owner
    census is one larger than without the sibling's table, and rc is not 2.
    Until R17 the consumed definition left `cur` empty and the quote ended
    before `table_header_at` was asked: the table was dropped, the id never
    declared and -- schema presence being required of the MAIN memo only --
    rc 0, the silent class."""
    link = "See [the walk](slice-9z-sib.md)."
    quoted = "> [a]: /u\n" + "\n".join(("> " if k else "") + l for k, l in enumerate(UMBRELLA_SLOT.split("\n")))
    base, _ = run_on(M, build(), link, sibling="nothing here.")
    res, _ = run_on(M, build(), link, sibling=quoted)
    row = res.population.ids.get("#11-zz-gamma")
    n0, n1 = len(base.population.no_owner_ids()), len(res.population.no_owner_ids())
    ok = row is not None and row.kind == "umbrella" and n1 == n0 + 1 and res.rc != 2
    return ok, "id declared %s, kind %s, no-owner census %d -> %d (must be +1), rc %d (must not be 2)" % (
        row is not None, row.kind if row else None, n0, n1, res.rc)


def sequence_control(M):
    """Phase 1's block SEQUENCE (`Memo.sequence`) over the §4.4 chunk and
    the §5.1 / §5.2 container shapes the vendored spec examples do not reach
    -- which block a line lands in, and (a list entry's last two fields) a
    list's first marker and tightness, facts no naming site can discriminate
    -- and,
    for the raw shapes, the CONTENT of the raw extent (`Memo.raw`: the
    lines after the marker, re-spelt in line columns), since the sequence
    alone never read it: `c0 = col` in `quote_content` survived 285
    controls with `>\\t\\tfoo` holding seven spaces (design re-gate 3,
    MIN-5).  Every expected sequence was read off commonmark.js 0.31.2
    (`node cm.js`) before being written, except the table shapes (no
    tables there), which are cmark-gfm's (measured, `gh api -X POST
    /markdown -f mode=gfm`): a lazy delimiter row or body row is paragraph
    text and a table is not a paragraph, while a lazy HEADER row is the
    header when the delimiter carries the marker and a RUN is open --
    paragraph text or a definition (PR #510 R17).  A crash on a shape is
    red."""
    import plan_memo_memo         # the freshly loaded module
    shapes = [
        # (markdown, expected sequence[, expected raw content lines])
        # a candidate where no paragraph is open ends the quote
        ("> # h\nlazy", [["quote", 1, 1], ["h1", 1, 1], ["p", 2, 2]]),
        ("> ```\nlazy", [["quote", 1, 1], ["fence", 1, 1], ["p", 2, 2]]),
        (">     foo\n    bar", [["quote", 1, 1], ["indented", 1, 1], ["indented", 2, 2]], ["    foo", "    bar"]),
        ("> | a |\n> |---|\n| 1 |", [["quote", 1, 2], ["table", 1, 2], ["p", 3, 3]]),
        # lazy continuation, and the underline that cannot be lazy
        ("> a\nb\n> ===", [["quote", 1, 3], ["h1", 1, 3]]),
        ("> foo\nbar\n===", [["quote", 1, 3], ["p", 1, 3]]),
        ("text\n> q\nlazy\n---", [["p", 1, 1], ["quote", 2, 3], ["p", 2, 3], ["hr", 4, 4]]),
        ("> > a\nb\n> c", [["quote", 1, 3], ["quote", 1, 3], ["p", 1, 3]]),
        ("> [a]: /u\nlazy", [["quote", 1, 2], ["def", 1, 1], ["p", 2, 2]]),
        ("> | a |\n|---|", [["quote", 1, 2], ["p", 1, 2]]),
        # a lazy HEADER is the table's header where a paragraph is open (cmark-gfm); after a heading it is not
        ("> a\n| h |\n> |---|\n> | 1 |", [["quote", 1, 4], ["p", 1, 1], ["table", 2, 4]]),
        ("> # h\n| h |\n> |---|", [["quote", 1, 1], ["h1", 1, 1], ["p", 2, 2], ["quote", 3, 3], ["p", 3, 3]]),
        ("> a\n\n> b", [["quote", 1, 1], ["p", 1, 1], ["quote", 3, 3], ["p", 3, 3]]),
        # PR #510 R17: a definition keeps the run open, so the lazy header after it is the table's (cmark-gfm,
        # measured on each shape -- `Memo._parse`'s docstring is the table); after a blank quote line, a
        # fence or a table no run is open and the quote ends; a lazy delimiter / body row is unchanged
        ("> [a]: /u\n| h |\n> |---|\n> | 1 |", [["quote", 1, 4], ["def", 1, 1], ["table", 2, 4]]),
        ("> [a]: /u\n> [b]: /v\n| h |\n> |---|", [["quote", 1, 4], ["def", 1, 1], ["def", 2, 2], ["table", 3, 4]]),
        ("> a\n> [b]: /v\n| h |\n> |---|", [["quote", 1, 4], ["p", 1, 2], ["table", 3, 4]]),
        ("- [a]: /u\n| h |\n  |---|\n  | 1 |", [["list", 1, 4, "-", True], ["item", 1, 4], ["def", 1, 1], ["table", 2, 4]]),
        ("> [a]: /u\n| h |\n> |---|\n| 1 |", [["quote", 1, 3], ["def", 1, 1], ["table", 2, 3], ["p", 4, 4]]),
        ("> [a]: /u\n| h |\n|---|", [["quote", 1, 3], ["def", 1, 1], ["p", 2, 3]]),
        ("> [a]: /u\n>\n| h |\n> |---|", [["quote", 1, 2], ["def", 1, 1], ["p", 3, 3], ["quote", 4, 4], ["p", 4, 4]]),
        ("> a\n>\n| h |\n> |---|", [["quote", 1, 2], ["p", 1, 1], ["p", 3, 3], ["quote", 4, 4], ["p", 4, 4]]),
        ("> ```\n> x\n> ```\n| h |\n> |---|", [["quote", 1, 3], ["fence", 1, 3], ["p", 4, 4], ["quote", 5, 5], ["p", 5, 5]]),
        ("> | a |\n> |---|\n| h |\n> |---|", [["quote", 1, 2], ["table", 1, 2], ["p", 3, 3], ["quote", 4, 4], ["p", 4, 4]]),
        ("> > [a]: /u\n| h |\n> |---|\n> | 1 |", [["quote", 1, 4], ["quote", 1, 4], ["def", 1, 1], ["p", 2, 4]]),
        # the marker and §2.2 tab stops (Example 6 exactly, and its neighbours)
        (">\t\tfoo", [["quote", 1, 1], ["indented", 1, 1]], ["      foo"]),
        (">\t\ta", [["quote", 1, 1], ["indented", 1, 1]], ["      a"]),
        (">\ta", [["quote", 1, 1], ["p", 1, 1]]),
        (">  \ta", [["quote", 1, 1], ["p", 1, 1]]),
        ("    > a", [["indented", 1, 1]], ["    > a"]),
        # the §4.4 chunk: blank lines inside stay, trailing ones do not
        ("    a\n\n    b\n\nc", [["indented", 1, 3], ["p", 5, 5]], ["    a", "", "    b"]),
        ("    a\n  \n    b", [["indented", 1, 3]], ["    a", "  ", "    b"]),
        # §5.2 / §5.3 (PR #510 R15): the item's content indentation, its lazy candidates, the
        # §5.2 interruption rule on a lazy line, sibling types, tightness, the tab rule
        ("- item\n\n    [child](child.md)", [["list", 1, 3, "-", False], ["item", 1, 3], ["p", 1, 1], ["p", 3, 3]]),
        ("- item\n para", [["list", 1, 2, "-", True], ["item", 1, 2], ["p", 1, 2]]),
        ("- a\n2. b", [["list", 1, 1, "-", True], ["item", 1, 1], ["p", 1, 1],
                       ["list", 2, 2, "2.", True], ["item", 2, 2], ["p", 2, 2]]),
        ("> a\n2. b", [["quote", 1, 1], ["p", 1, 1], ["list", 2, 2, "2.", True], ["item", 2, 2], ["p", 2, 2]]),
        ("foo\n-", [["h2", 1, 2]]),
        ("- a\n  -", [["list", 1, 2, "-", True], ["item", 1, 2], ["h2", 1, 2]]),
        ("* a\n* * *\n* b", [["list", 1, 1, "*", True], ["item", 1, 1], ["p", 1, 1], ["hr", 2, 2],
                             ["list", 3, 3, "*", True], ["item", 3, 3], ["p", 3, 3]]),
        ("-\n\n  foo", [["list", 1, 1, "-", True], ["item", 1, 1], ["p", 3, 3]]),
        ("-\n  foo\n- b", [["list", 1, 3, "-", True], ["item", 1, 2], ["p", 2, 2], ["item", 3, 3], ["p", 3, 3]]),
        ("- a\n\n- b", [["list", 1, 3, "-", False], ["item", 1, 2], ["p", 1, 1], ["item", 3, 3], ["p", 3, 3]]),
        ("- a\n-\n\n- b", [["list", 1, 4, "-", False], ["item", 1, 1], ["p", 1, 1], ["item", 2, 2],
                           ["item", 4, 4], ["p", 4, 4]]),
        ("- a\n  - b\n\n    c\n- d", [["list", 1, 5, "-", True], ["item", 1, 4], ["p", 1, 1],
                                      ["list", 2, 4, "-", False], ["item", 2, 4], ["p", 2, 2], ["p", 4, 4],
                                      ["item", 5, 5], ["p", 5, 5]]),
        ("- > a\n  >\n- b", [["list", 1, 3, "-", True], ["item", 1, 2], ["quote", 1, 2], ["p", 1, 1],
                             ["item", 3, 3], ["p", 3, 3]]),
        ("> - a\n    ---", [["quote", 1, 2], ["list", 1, 2, "-", True], ["item", 1, 2], ["p", 1, 2]]),
        ("- a\n\n\t  b", [["list", 1, 3, "-", False], ["item", 1, 3], ["p", 1, 1], ["indented", 3, 3]], ["    b"]),
        ("-\t\tfoo", [["list", 1, 1, "-", True], ["item", 1, 1], ["indented", 1, 1]], ["      foo"]),
        ("1. a\n\n   b\n\n   c", [["list", 1, 5, "1.", False], ["item", 1, 5], ["p", 1, 1], ["p", 3, 3], ["p", 5, 5]]),
    ]
    with tempfile.TemporaryDirectory() as d:
        p = pathlib.Path(d) / "shape.md"
        for md, want, *raw in shapes:
            p.write_text(md + "\n", encoding="utf-8")
            try:
                memo = plan_memo_memo.Memo(p)
            except Exception as e:       # noqa: BLE001 -- the defect under test
                return False, "%r raised %s: %s" % (md, type(e).__name__, str(e)[:60])
            if memo.sequence != want:
                return False, "%r: sequence %s, expected %s" % (md, memo.sequence, want)
            got = [t for _, t, _ in memo.raw]
            if raw and got != raw[0]:
                return False, "%r: raw content %s, expected %s" % (md, got, raw[0])
    return True, "%d shapes, each sequence (and each raw extent's content) as commonmark.js reads it" % len(shapes)


def deep_nesting_control(M):
    """Container nesting is off the call stack (PR #510 R16): 1,000 nested
    block quotes and 1,000 nested list items -- commonmark.js 0.31.2 renders
    1,000 `<blockquote>` / 1,000 `<ul>` for them (measured) -- parse to the
    sequence a container per marker gives (each nested container is stated
    before its content, then the one paragraph), and `check()` over a
    1,000-deep quote around a site reports that site.  With the Phase-1
    passes as plain calls, ~500 markers raised `RecursionError` inside
    `_parse` and the population's chokepoint reported the memo as
    "unavailable" (rc 2, no census); an exception here -- from `Memo` or from
    `check()` -- is red, never a harness accident: it is the defect."""
    import plan_memo_memo
    n = 1000
    shapes = [
        ("> " * n + "a", [["quote", 1, 1]] * n + [["p", 1, 1]]),
        ("- " * n + "a", [b for _ in range(n) for b in (["list", 1, 1, "-", True], ["item", 1, 1])] + [["p", 1, 1]]),
    ]
    with tempfile.TemporaryDirectory() as d:
        p = pathlib.Path(d) / "deep.md"
        for md, want in shapes:
            p.write_text(md + "\n", encoding="utf-8")
            try:
                got = plan_memo_memo.Memo(p).sequence
            except Exception as e:       # noqa: BLE001 -- the defect under test
                return False, "%d nested `%s` raised %s: %s" % (n, md[0], type(e).__name__, str(e)[:60])
            if got != want:
                return False, "%d nested `%s`: %d sequence entries, expected %d" % (n, md[0], len(got), len(want))
    try:
        res, reported = run_on(M, build(), "> " * n + "Slice 9z owns it")
    except Exception as e:       # noqa: BLE001 -- the defect under test
        return False, "check() over a %d-deep quote raised %s: %s" % (n, type(e).__name__, str(e)[:60])
    ok = res.rc != 2 and len(reported) == 1
    return ok, "%d nested quotes / items: %d + %d sequence entries as commonmark.js nests them; check() rc %d, %d site" % (
        n, n + 1, 2 * n + 1, res.rc, len(reported))


def parse_runtime_error_control(M):
    """The I/O chokepoint is I/O ONLY: a `RuntimeError` raised while PARSING a
    memo (injected: `quote_content` raises on a sentinel line) is a crash out
    of `check()` -- crash = FAIL -- never the "linked memo unavailable" miss.
    `RecursionError` is a `RuntimeError`, and until PR #510 R16 the chokepoint
    caught `RuntimeError` (for the py<=3.12 symlink-loop case `_resolve`
    alone guards), so a parser defect was reported as an unavailable memo,
    rc 2, with no census.  Red when `check()` returns instead of raising."""
    import plan_memo_memo
    orig = plan_memo_memo.quote_content

    def injected(line):
        if line.startswith("> BOOM"):
            raise RuntimeError("injected parser defect")
        return orig(line)
    plan_memo_memo.quote_content = injected
    try:
        try:
            res, _ = run_on(M, build(), "> BOOM")
        except RuntimeError as e:
            return "injected parser defect" in str(e), "RuntimeError propagated out of check(): %s" % e
        except Exception as e:       # noqa: BLE001 -- not the injected exception: red, not a harness crash
            return False, "check() raised %s, not the injected RuntimeError: %s" % (type(e).__name__, str(e)[:60])
    finally:
        plan_memo_memo.quote_content = orig
    miss = [f[3][:50] for f in res.findings if f[0] == "SCHEMA" and "linked memo unavailable" in f[3]]
    return False, "check() returned rc %d with %s -- a parser exception was swallowed as an I/O miss" % (res.rc, miss)


def orphan_offset_control(M):
    """The orphan-definition exemption is by the orphan's exact BRACKET, not
    its line.  `paragraph\\n[sib]: child.md "[sib]"` -- commonmark.js
    0.31.2 renders it `<p>paragraph\\n[sib]: child.md &quot;[sib]&quot;</p>`:
    a definition cannot interrupt a paragraph, so the line is literal text,
    and both `[sib]` are shortcuts with no definition, literal too.  The
    label bracket (line 2, column 0) is the orphan's own and exempt; the
    `[sib]` inside the title is a shortcut whose label has an orphan
    definition -- the documented unresolved-reference miss (rc 2), and
    `child.md` is NOT walked (it is linked by nothing).  A line-number
    exemption exempted the title's bracket too: rc 0, `child.md` silently
    outside the population (PR #510 R14 #3)."""
    res, _ = run_on(M, build(), 'paragraph\n[sib]: child.md "[sib]"', files={"child.md": VIOLATION + "\n"})
    miss = sum(1 for f in res.findings if f[0] == "SCHEMA" and "unresolved reference 'sib'" in f[3])
    walked = [m.path.name for m in res.population.memos]
    ok = res.rc == 2 and miss == 1 and walked == ["fixture.md"]
    return ok, "rc %d (must be 2), unresolved 'sib' x%d (must be 1), population %s (child.md must not be walked)" % (
        res.rc, miss, walked)


def display_path_control(M):
    """PR #510 R19 #2: every printer names a memo by the population's ONE
    display name (`Population.display`) -- its path RELATIVE to the root
    memo's directory, or the resolved absolute path when it lies outside
    that directory -- never the basename, under which `a/child.md:1` and
    `b/child.md:1` were one `child.md:1` in the worklist, the findings and
    the population summary.  Two same-named siblings each carry a naming
    site and a raw `|` line (a LEX-UNSUPPORTED? seed): the sites' and the
    seeds' file columns must both tell the two files apart; then a sibling
    one directory UP from the root is named absolutely."""
    twin = VIOLATION + "\n\n<div>|</div>\n"
    res, reported = run_on(M, build(), "See [a](a/child.md) and [b](b/child.md).",
                           files={"a/child.md": twin, "b/child.md": twin})
    sites = sorted({m.file for m in reported if m.memo is not res.population.main})
    seeds = sorted({f[1] for f in res.findings if f[0] == "LEX-UNSUPPORTED?"})
    want = ["a/child.md", "b/child.md"]
    if res.rc == 2 or sites != want or seeds != want:
        return False, "rc %d, site files %s, seed files %s (each must be %s)" % (res.rc, sites, seeds, want)
    with tempfile.TemporaryDirectory() as d:
        root = pathlib.Path(d) / "root"
        root.mkdir()
        (root / "fixture.md").write_text(build() + "\nSee [up](../child.md).\n", encoding="utf-8")
        (pathlib.Path(d) / "child.md").write_text(twin, encoding="utf-8")
        res = M.check(str(root / "fixture.md"))
        outside = str((pathlib.Path(d) / "child.md").resolve())
        seeds = sorted({f[1] for f in res.findings if f[0] == "LEX-UNSUPPORTED?"})
    ok = res.rc != 2 and seeds == [outside]
    return ok, "same-basename siblings named %s; a memo outside the root's directory named %s (must be its absolute path)" % (
        want, seeds)


def empty_id_row_name_control(M):
    """PR #510 R20: a finding names a row whose id cell declares no id by its
    declaring LOCATOR -- `row <no id> at :LINE (TOKEN)`, the row's line and
    the first token of its declaring field (`Row.name`, the one spelling every
    printer composes) -- never `row None`, which named nothing a reader could
    find.  The fixture is the umbrella memo's shape at its line 1985: a §5 row
    whose id cell is the literal blank `**—**` (a deliberate non-row: unkeyed,
    outside `ids`, still a data row) and whose Slice cell opens
    `` `Function`/`eval` `` and orders itself after a slug its Deps cell does
    not carry, so the ORDER-PROSE? seed fires on it.  WHAT the seed compares
    (Deps vs prose) is a documented seed class and is not under test; the NAME
    the finding carries is.  Every finding of the run is read for `row None`:
    the empty-id row also reaches the declaring-field printers through
    `declaring_rows()`."""
    table = ("| # | Slice | Primary module(s) | Slot | Tier | Deps |\n|---|---|---|---|---|---|\n"
             "| **—** | `Function`/`eval` — Terminal.  Acceptance: the probe must return 3.  "
             "Lands after Slice `#11-zz-alpha`. | `g.rs` | — | T1 | **9z** |")
    text = build(extra=table)
    lineno = next(i for i, l in enumerate(text.split("\n"), 1) if l.startswith("| **—** |"))
    res, _ = run_on(M, text)
    hits = [f[3] for f in res.findings if f[0] == "ORDER-PROSE?" and f[2] == lineno]
    want = "row <no id> at :%d (Function/eval)" % lineno
    none = sorted({f[0] for f in res.findings if "row None" in f[3]})
    ok = res.rc != 2 and len(hits) == 1 and want in hits[0] and not none
    return ok, "rc %d (not 2), ORDER-PROSE? on line %d x%d (must be 1) carrying %r: %s; findings spelling `row None`: %s" % (
        res.rc, lineno, len(hits), want, bool(hits) and want in hits[0], none or "none")


# ------------------------------------------------ PR #510 Codex R24 controls --

def multiline_span_locator_control(M):
    """A §6.6 raw HTML span that CROSSES a line ending seeds each of its lines
    at the line it is on, not all of them at the line the span opened on.

    The seed is the only diagnostic there is for the content of such a span --
    the ordinary naming scan masks it deliberately (an id inside an attribute
    or a comment is no naming site) -- so its line number is the whole of what
    it hands the reader.  `foo <!-- begin` / `Slice 9z owns it` / `end -->`
    named `9z` against the FIRST of those lines (PR #510 R24-3).

    The expected line is COMPUTED from the fixture, never written here as a
    number: the fixture's own text says which line holds the id, and a
    constant would be a claim about `HEADER`'s current length rather than
    about the locator.  The opener's line is asserted to carry no seed of its
    own, so the control cannot pass by reporting both lines."""
    prose = "foo <!-- begin\nSlice 9z owns it\nend -->"
    text = build() + "\n" + prose + "\n"
    lines = text.split("\n")
    want = lines.index("Slice 9z owns it") + 1          # 1-based, as a finding's is
    opener = lines.index("foo <!-- begin") + 1
    res, _ = run_on(M, build(), prose)
    seeds = [(f[2], f[3]) for f in res.findings if f[0] == "LEX-UNSUPPORTED?"]
    naming = [ln for ln, msg in seeds if "'9z'" in msg]
    ok = naming == [want] and not any(ln == opener for ln, _ in seeds) and res.rc != 2
    return ok, ("the id `9z` is seeded at line(s) %s (want [%d], the line it is ON; the span opens on "
                "%d, which carries no seed of its own); %d seed(s) in all, rc %d"
                % (naming, want, opener, len(seeds), res.rc))


def property_family_control(M):
    """PROPERTY: the entries named `PROPERTY: ...` are exactly the property
    family's fragments -- `records.registry()` (which merges the properties and
    invariants modules') and `ratchets.registry()` -- in both directions.

    Written for the fifth attestation's concept sweep: three docstrings said
    "the three property modules", and the ratchets module, carved at R49, was a
    fourth that none of them named.  A claim about which modules a family is
    goes stale under every split, so it is a control and not a sentence."""
    from plan_memo_selftest_ratchets import registry as ratchet_registry
    family = set(property_registry()) | set(ratchet_registry())
    named = {n for n in registry() if n.startswith("PROPERTY:")}
    unnamed = sorted(family - named)
    stray = sorted(named - family)
    return not unnamed and not stray, ("%d family entr(ies), %d named PROPERTY:; %d not so named, "
                                       "%d named but outside the family%s"
                                       % (len(family), len(named), len(unnamed), len(stray),
                                          ("; " + "; ".join((unnamed + stray)[:3])) if unnamed or stray
                                          else ""))


def registry():
    """name -> (kind, control): the ONE table the runner and the mutation
    proof read, this module's controls MERGED with the work module's fragment
    (`plan_memo_selftest_work.registry`) -- the same "one list, filled by
    several modules" the cases and the mutants already use."""
    reg = dict(work_registry())
    reg.update(property_registry())
    from plan_memo_selftest_ratchets import registry as ratchet_registry
    reg.update(ratchet_registry())
    for c in CASES:
        assert c.name not in reg, "duplicate control name %r" % c.name
        reg[c.name] = (c.kind, control(c))
    reg["CommonMark 0.31.2 spec examples (Tabs, §4.1-§4.9, §5.1-§5.3): Phase 1's block sequence aligns with the html"] = ("CONTROL", spec_examples_control)
    reg["CommonMark 0.31.2 spec examples (§2.4, §2.5, §6.1-§6.6): Phase 2's inline claim aligns "
        "with the html"] = ("CONTROL", inline_examples_control)
    reg["Phase 1's block sequence over the §4.4 chunk and the §5.1 / §5.2 container shapes matches commonmark.js"] = ("CONTROL", sequence_control)
    reg["a destination the sibling resolver REFUSES leaves no finding, seed or note naming it -- the stated policy, pinned rather than merely current"] = ("CONTROL", refused_destination_silence_control)
    reg["CommonMark 0.31.2 §6.4: Phase 2's inline claim agrees with the spec's own html for every "
        "§3.0b family DEMOTED into a resolved image description (the cross-product the corpus cannot reach)"] = ("CONTROL", demoted_agreement_control)
    reg["CommonMark 0.31.2 §6.1: a code span READS as the text the spec's own html puts inside `<code>` (line endings converted, then the one-space trim)"] = ("CONTROL", code_span_reading_control)
    reg["a lazy schema header after a definition in a linked memo's quote is a table: id declared, kind umbrella, census +1"] = ("CONTROL", lazy_header_after_definition_control)
    reg["a marker naming another row does not enter the count"] = ("CONTROL", attribution_control)
    reg["the entries named `PROPERTY: ...` are exactly the property family's fragments (records and ratchets), both directions"] = ("CONTROL", property_family_control)
    reg["declaring-field parse and whole-line marker grep differ"] = ("CONTROL", degenerate_control)
    reg["a table with and without edge pipes reads the same"] = ("CONTROL", pipe_shape_control)
    reg["a site after an escaped pipe is reported at its raw column"] = ("CONTROL", raw_offset_control)
    reg["a site read across a construct that renders nothing is reported at its raw column (the stream map)"] = ("CONTROL", split_locator_control)
    reg["an empty control or mutant registry is a FAIL, never green"] = ("CONTROL", empty_registry_control)
    reg["the runner's report channel escapes every C0 control character and DEL, so a run line a control names with one is still greppable"] = ("CONTROL", printable_output_control)
    reg["a decoded destination with a C0 control character is rejected, never resolved"] = ("CONTROL", control_char_destination_control)
    reg["an OSError from resolve() is the unavailable-sibling schema miss, never an exception"] = ("CONTROL", unavailable_sibling_control)
    reg["an undecodable sibling is the unavailable-linked-memo schema miss, never an exception"] = ("CONTROL", undecodable_sibling_control)
    reg["an orphan definition exempts its OWN bracket only: `[sib]: child.md \"[sib]\"` is the documented miss, rc 2, child.md not walked"] = ("CONTROL", orphan_offset_control)
    reg["container nesting is off the call stack: 1,000 nested quotes / items parse as commonmark.js nests them"] = ("CONTROL", deep_nesting_control)
    reg["a RuntimeError raised while PARSING a memo is a crash out of check(), never the unavailable-memo miss"] = ("CONTROL", parse_runtime_error_control)
    reg["diagnostics name a memo relative to the root memo's directory: `a/child.md` and `b/child.md` are two files, and a memo outside that directory is named by its absolute path"] = ("CONTROL", display_path_control)
    reg["a row whose id cell declares no id is named by its declaring locator (`row <no id> at :LINE (token)`), never `row None`"] = ("CONTROL", empty_id_row_name_control)
    reg["a §6.6 span crossing a line ending seeds each of its lines at ITS line, not all of them at the opener's"] = ("CONTROL", multiline_span_locator_control)
    return reg
