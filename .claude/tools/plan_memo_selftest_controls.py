#!/usr/bin/env python3
"""The function-shaped controls for `plan-memo-umbrella-check.py --self-test`,
and the registry that names every control.

The record-shaped controls (`Case`: one fixture, one measure, one exact
value) live in `plan_memo_selftest_cases.py` / `_cases_pr510.py` / `_cases_inline.py`;
this module
holds the controls a record cannot express -- a property over the module set
(the spelling sweep, the row-kind coverage), an injected fault (a raising
`resolve()`, a parser `RuntimeError`), a comparison of two runs (the pipe
shape, the attribution), the CommonMark conformance corpus -- and `registry()`,
the ONE name -> (kind, control) table both the runner and the mutation proof
read.  Every control here is a function of the freshly loaded checker module
returning `(ok, detail)`, and every one is reachable by name through the
registry, because the mutant runner re-runs controls BY NAME against a
patched set.

The controls whose measure is WORK rather than text -- the linearity
witnesses, every one of them written against `_count_calls` / `_count_lines`
/ `_CountedList` -- are `plan_memo_selftest_work.py`, whose `registry()`
fragment this module's `registry()` merges (PR #510 R24 touch-time split).
This module imports no work witness, and that import list is the seam's
statement.

`empty_registry_fails`, the runner's emptiness guard, lives here beside its
proof (`empty_registry_control`) so the one mutant against it patches one
file: a mutant row whose file is a SELF-TEST module (this one, or the work
module) is exec'd from patched text
(`plan_memo_selftest_harness.patched_module`) and its controls are taken from
the PATCHED module's registry, merged over the unpatched rest
(`plan_memo_selftest_mutants.SELFTEST`).

Import direction, one way: the runner imports this module; this module
imports the harness (`plan_memo_selftest_harness.py`), the case registry and
the work registry.
"""

import pathlib
import tempfile

from plan_memo_selftest_cases import CASES, VIOLATION, build
import plan_memo_selftest_cases_pr510  # noqa: F401 -- appends the review-round controls to CASES
import plan_memo_selftest_cases_inline  # noqa: F401 -- appends the Phase-2 inline rounds to the same CASES
from plan_memo_selftest_harness import GRAMMAR, MODULES, SOURCES, control, run_on
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


def row_kind_coverage_control(M):
    """PROPERTY, the KIND half of the R14 spelling sweep: every composer that
    reads "a row id in this position" admits EVERY row kind the grammar
    enumerates (`plan_memo_ids.ROW_KINDS`) -- the marker's appositive
    subject (`attributed_to_other`, over `ROW_NOUN_ID`), the two-owner clause
    (`OWNS_TWO`) and the row-noun-anchored reading (`_anchored`, through
    `check()`: the mention is `anchored`).  The kinds are the GRAMMAR's
    tuple; the sample id per kind is looked up here, and a kind without a
    sample is red, so a fourth row kind added to the grammar reaches this
    control before it reaches any composer.  The spelling sweep cannot see
    this class: a composer built on `SHORT_ID` alone spells nothing twice,
    and passed it while `Slice `#11-zz-alpha` — **UMBRELLA, …**` attributed
    nothing (PR #510 R20)."""
    import plan_memo_ids as ids          # the FRESHLY loaded set, not the import-time one
    import plan_memo_roles as roles
    import plan_memo_tables as tables
    samples = {"short": "9z", "slug": "#11-zz-alpha"}
    missing = [k for k in ids.ROW_KINDS if k not in samples]
    if missing:
        return False, "no sample id for row kind(s) %s -- add one before any composer reads the kind" % missing
    fails, probes = [], 0
    for kind in ids.ROW_KINDS:
        rid = samples[kind]
        for d in ("**%s**" % rid, "`%s`" % rid):
            probes += 3
            if tables.attributed_to_other("Slice %s — **%s**" % (d, tables.MARKER), "7z") != rid:
                fails.append("appositive/%s %r" % (kind, d))
            m = roles.OWNS_TWO.search("owned by %s and **Qx**" % d)
            if m is None or m.group("aid") != rid:
                fails.append("OWNS_TWO/%s %r" % (kind, d))
            res, _ = run_on(M, build(), "Slice %s lands first." % d)
            if not any(x.id == rid and x.anchored for x in res.mentions):
                fails.append("anchored/%s %r" % (kind, d))
    return not fails, "%d probes over row kinds %s%s" % (
        probes, list(ids.ROW_KINDS), (": FAIL " + ", ".join(fails)) if fails else ", every composer admits every kind")


def degenerate_control(M):
    """A whole-line grep for the marker CANNOT disagree with the marker count;
    the declaring-field parse can.  This proves the two are different programs
    rather than one program written twice."""
    import plan_memo_tables   # the FRESHLY loaded module, not the import-time one
    text = build(d7z="**UMBRELLA, not a terminal unit** stray")
    umb = run_on(M, text)[0].population.no_owner_ids()
    by_grep = sum(1 for l in text.split("\n") if l.startswith("|") and plan_memo_tables.MARKER in l)
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
    import plan_memo_memo
    orig = plan_memo_memo.pathlib.Path.resolve
    report = []
    for name, exc in (("a" * 4000 + ".md", OSError(36, "File name too long")),
                      ("loop.md", RuntimeError("Symlink loop from 'loop.md'"))):
        def resolve(self, *a, _name=name, _exc=exc, **kw):
            if self.name == _name:
                raise _exc
            return orig(self, *a, **kw)
        plan_memo_memo.pathlib.Path.resolve = resolve
        try:
            res, _ = run_on(M, build(), "See [x](%s)." % name)
        except Exception as e:       # noqa: BLE001 -- the defect under test
            return False, "check() raised %s: %s" % (type(e).__name__, str(e)[:60])
        finally:
            plan_memo_memo.pathlib.Path.resolve = orig
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
            p.write_text(build() + "\nSee [bad](bad.md).\n")
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
    print("       " + detail.replace("\n", "\n       "))
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
    print("       " + detail.replace("\n", "\n       "))
    return ok, detail.split("\n")[0]


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
            p.write_text(md + "\n")
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
            p.write_text(md + "\n")
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
        (root / "fixture.md").write_text(build() + "\nSee [up](../child.md).\n")
        (pathlib.Path(d) / "child.md").write_text(twin)
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


# The spellings the grammar module owns.  A SOURCE-TEXT sweep over string
# constants: it reads these exact spellings and nothing about purpose.
_ID_SPELLINGS = (
    "[0-9A-Za-z",           # the ASCII alphanumeric class (`ALNUM`; `[0-9A-Za-z-]`, `[0-9A-Za-z_-]` too)
    "[a-z0-9-]",            # the slug body
    "[A-Za-z][0-9]",        # the citation label, and its two case-halves
    "[A-Z][0-9]", "[a-z][0-9]",
    "#11-",                 # the slug prefix as a literal (`startswith("#11-")` is a kind test)
    "\\w", "\\d",           # Unicode classes in a str pattern: never an ASCII boundary
)


def _string_constants(src, file):
    """(lineno, value) of every string constant of `src` that is not a
    docstring (the first statement of a module / class / function body)."""
    import ast
    tree = ast.parse(src, filename=file)
    docs = set()
    for node in ast.walk(tree):
        if isinstance(node, (ast.Module, ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)):
            body = node.body
            if body and isinstance(body[0], ast.Expr) and isinstance(body[0].value, ast.Constant) \
                    and isinstance(body[0].value.value, str):
                docs.add(id(body[0].value))
    return [(node.lineno, node.value) for node in ast.walk(tree)
            if isinstance(node, ast.Constant) and isinstance(node.value, str) and id(node) not in docs]


def id_spelling_sweep_control(M):
    """PROPERTY: the id-token grammar is spelled ONCE.  Every string constant
    (docstrings excepted; comments are not code) of every module of the set
    other than `plan_memo_ids.py` is read for a second spelling of a class
    the grammar owns (`_ID_SPELLINGS`); one hit is red.  Read over
    `SOURCES` -- the text the CURRENT set was exec'd from -- so a mutant that
    re-introduces a spelling is seen.

    HONESTLY: this is a source-TEXT sweep.  What it cannot see: a class
    spelled in another order or with other ranges (`[A-Za-z0-9-]` is the
    HTML tag-name grammar in `plan_memo_blocks.py`, `[a-zA-Z0-9+.-]` the URL
    scheme grammar in `plan_memo_memo.py` -- neither is an id class, and the
    sweep reads no purpose, so it must not read those spellings either); a
    class built by concatenation or escaped at runtime; a hand-written
    character test (`ch.isalnum()`, `in string.ascii_letters`); `\\b`
    (roles' vocabulary patterns use it under `re.ASCII`, which is not an
    id boundary); and anything in a comment.  The R8 controls (`次のSlice
    C`, `次は#11-zz-alpha`, `9z.次の`) are the BEHAVIOURAL half; this is the
    textual half, and neither bounds the other."""
    hits = []
    for _, file in MODULES:
        if file == GRAMMAR:
            continue
        src = SOURCES.get(file)
        if src is None:
            return False, "no loaded source for %s (load() before the sweep)" % file
        for lineno, value in _string_constants(src, file):
            for needle in _ID_SPELLINGS:
                if needle in value:
                    hits.append("%s:%d %r spells %r" % (file, lineno, value[:40], needle))
    n = sum(len(_string_constants(SOURCES[f], f)) for _, f in MODULES if f != GRAMMAR)
    return not hits, ("%d string constants in %d modules swept, %d second spelling(s)%s"
                      % (n, len(MODULES) - 1, len(hits), (": " + "; ".join(hits[:3])) if hits else ""))


# ------------------------------------------------ PR #510 Codex R24 controls --

# The prose the render-equivalence sweep re-spells, one character at a time.
# It carries, in one paragraph, every shape a stage-2 disposition question is
# asked about, and each in the ADJACENCY that makes the answer observable: a
# `**`-decorated id against a second id (so keeping or dropping the delimiters
# is the difference between two ids and one token), a bare id against a `.md`
# file name (so where the name BEGINS is the difference between a site and
# none), a row noun, a licensing phrase and a role word.  A shape whose two
# answers agree is no probe: `**9z** owns` reads `9z` either way, and the sweep
# stayed green over both R24 defects until the adjacencies were written in.
_RENDER_PROSE = "Slice **9z**7z owns it and 9z notes.md lands, the child of Qx."


def _census(res):
    """The checker's verdict on one run, with every RAW coordinate and every
    quoted excerpt dropped: the exit status, the finding CODES with the memo
    and line they were reported against, and each naming site as (id, line,
    source, anchored, licensed).  A re-spelling moves columns and changes the
    excerpts a report quotes, and neither is a claim about the document."""
    return (res.rc,
            sorted((f[0], f[1], f[2]) for f in res.findings),
            sorted((m.id, m.lineno, m.source, m.anchored, m.licensed) for m in res.mentions))


def render_equivalence_control(M):
    """PROPERTY, the STRUCTURAL guard for design re-gate 4's rule ("the stream
    IS the rendered text"): the checker's verdict is INVARIANT under a
    re-spelling of the source that the document renders identically.

    The re-spelling is mechanical and covers every position, so the control is
    not defined by the vocabulary of the defects that produced it: for each
    character of `_RENDER_PROSE` in turn, that ONE character is replaced by its
    §2.5 numeric character reference (`z` -> `&#122;`), which renders the same
    character and nothing else, and the census of the two runs must be equal.
    A reader that consults `lx.text` or the raw source where the pipeline
    declares the rendered stream disagrees with itself at whichever position
    it reads, and the sweep visits them all -- it is how the two R24 members
    of this family were separated from each other (`file_and_cite_spans` over
    raw text; the decoration exception's `id_only` over raw content), and the
    next member needs no new control.

    THE POPULATION, and its edge.  A numeric reference renders a character but
    creates no STRUCTURE: `&#42;` opens no emphasis and `&#91;` no link, so a
    position holding a character the inline pass BRANCHES on is not a
    rendering-equivalent re-spelling and is excluded.  The excluded set is read
    off `inline_pass`'s own code object (its one-character constants) and
    `plan_memo_emphasis.DELIMS`, plus the two the branch table cannot yield
    because they are tested inside a helper, each named with its reason:
    `\\` (`_is_escape`) and `!` (`_is_image`).  It is reported with the run.

    HONESTLY, the two directions of that edge are NOT symmetric, and only one
    of them is safe.  A character wrongly LEFT IN the excluded set is a
    position not swept -- the sweep is weaker and says nothing about it, which
    is why the set and the swept count are printed rather than assumed.  A
    character wrongly LEFT OUT is re-spelled although it is active, the two
    documents then really do render differently, and the control goes RED: a
    false alarm, never a silent pass.  What the sweep cannot see at all: a
    disagreement that needs TWO positions re-spelled at once; a raw reading in
    a CELL (the sweep re-spells prose, where a `|` is an ordinary character --
    in a cell it is the row grammar's separator and its numeric spelling is
    not equivalent); a raw reading that no shape in `_RENDER_PROSE` reaches;
    and anything about a raw line the inline parser never enters (the
    LEX-UNSUPPORTED? seed reads such a line AS WRITTEN, on purpose --
    `plan_memo_lexer.file_and_cite_spans` states that reading and its declared
    miss)."""
    import plan_memo_emphasis, plan_memo_lexer
    active = set(plan_memo_emphasis.DELIMS) | {"\\", "!"}
    active |= {c for c in plan_memo_lexer.inline_pass.__code__.co_consts
               if isinstance(c, str) and len(c) == 1}
    want = _census(run_on(M, build(), _RENDER_PROSE)[0])
    swept, bad = 0, []
    for i, ch in enumerate(_RENDER_PROSE):
        if ch in active:
            continue
        swept += 1
        variant = _RENDER_PROSE[:i] + "&#%d;" % ord(ch) + _RENDER_PROSE[i + 1:]
        got = _census(run_on(M, build(), variant)[0])
        if got != want:
            bad.append("position %d (%r): %s" % (i, ch, _first_difference(want, got)))
    ok = not bad and swept >= 40
    return ok, ("%d of %d positions re-spelled as a §2.5 reference (excluded, the inline pass "
                "branches on them: %s), %d disagreement(s)%s"
                % (swept, len(_RENDER_PROSE), "".join(sorted(active)), len(bad),
                   (": " + "; ".join(bad[:2])) if bad else ""))


def _first_difference(want, got):
    """The first field of two censuses that differs, as a short string -- the
    sweep reports WHICH claim moved, not two whole censuses."""
    for name, a, b in zip(("rc", "findings", "sites"), want, got):
        if a != b:
            if name == "rc":
                return "rc %s -> %s" % (a, b)
            gone, new = sorted(set(map(str, a)) - set(map(str, b))), sorted(set(map(str, b)) - set(map(str, a)))
            return "%s -%s +%s" % (name, gone, new)
    return "equal"


def kind_phrase_gate_control(M):
    """PROPERTY: `Population._kind` reads NO phrase matcher of its own -- every
    one of them comes from `plan_memo_tables.KIND_PHRASES`, which is also the
    tuple the residue gate (`_kind_residue` / `kind_disagreements`) iterates.

    This is the construction R23 #2 asked for and the behavioural controls
    cannot state.  Those controls say the gate covers the three phrases that
    exist today; this one says a FOURTH cannot decide a row's kind without
    arriving in the tuple that gates it -- the failure mode was not "the
    undetermined phrase was forgotten" but "the phrase I was looking at was
    gated and every other one stayed authoritative by default".

    The subject is `_kind`'s own code object, not its source text: every
    global and attribute name it reads is in `co_names` (a nested
    comprehension's too), so a phrase read through an import inside the
    function, or through the tables module, is seen as well.  A name that
    resolves -- in either module's globals -- to a compiled pattern outside
    `KIND_PHRASES` is the finding."""
    import plan_memo_population, plan_memo_tables
    import re as _re

    code = plan_memo_population.Population._kind.__code__
    names = set(code.co_names)
    for const in code.co_consts:            # a comprehension is its own code object
        names |= set(getattr(const, "co_names", ()))
    member = {id(rx) for _, rx in plan_memo_tables.KIND_PHRASES}
    scopes = (vars(plan_memo_population), vars(plan_memo_tables))
    stray = sorted(n for n in names
                   for g in scopes
                   if isinstance(g.get(n), _re.Pattern) and id(g[n]) not in member)
    return not stray, ("%d name(s) read by _kind, %d phrase(s) in KIND_PHRASES, %d read outside it%s"
                       % (len(names), len(member), len(stray),
                          (": " + ", ".join(sorted(set(stray)))) if stray else ""))


def registry():
    """name -> (kind, control): the ONE table the runner and the mutation
    proof read, this module's controls MERGED with the work module's fragment
    (`plan_memo_selftest_work.registry`) -- the same "one list, filled by
    several modules" the cases and the mutants already use."""
    reg = dict(work_registry())
    for c in CASES:
        assert c.name not in reg, "duplicate control name %r" % c.name
        reg[c.name] = (c.kind, control(c))
    reg["CommonMark 0.31.2 spec examples (Tabs, §4.1-§4.9, §5.1-§5.3): Phase 1's block sequence aligns with the html"] = ("CONTROL", spec_examples_control)
    reg["CommonMark 0.31.2 spec examples (§2.4, §2.5, §6.1-§6.6): Phase 2's inline claim aligns "
        "with the html"] = ("CONTROL", inline_examples_control)
    reg["Phase 1's block sequence over the §4.4 chunk and the §5.1 / §5.2 container shapes matches commonmark.js"] = ("CONTROL", sequence_control)
    reg["a lazy schema header after a definition in a linked memo's quote is a table: id declared, kind umbrella, census +1"] = ("CONTROL", lazy_header_after_definition_control)
    reg["a marker naming another row does not enter the count"] = ("CONTROL", attribution_control)
    reg["declaring-field parse and whole-line marker grep differ"] = ("CONTROL", degenerate_control)
    reg["a table with and without edge pipes reads the same"] = ("CONTROL", pipe_shape_control)
    reg["a site after an escaped pipe is reported at its raw column"] = ("CONTROL", raw_offset_control)
    reg["a site read across a construct that renders nothing is reported at its raw column (the stream map)"] = ("CONTROL", split_locator_control)
    reg["an empty control or mutant registry is a FAIL, never green"] = ("CONTROL", empty_registry_control)
    reg["a decoded destination with a C0 control character is rejected, never resolved"] = ("CONTROL", control_char_destination_control)
    reg["an OSError from resolve() is the unavailable-sibling schema miss, never an exception"] = ("CONTROL", unavailable_sibling_control)
    reg["an undecodable sibling is the unavailable-linked-memo schema miss, never an exception"] = ("CONTROL", undecodable_sibling_control)
    reg["an orphan definition exempts its OWN bracket only: `[sib]: child.md \"[sib]\"` is the documented miss, rc 2, child.md not walked"] = ("CONTROL", orphan_offset_control)
    reg["PROPERTY: the id character classes are spelled once, in plan_memo_ids.py (a source-text sweep)"] = ("CONTROL", id_spelling_sweep_control)
    reg["PROPERTY: every row-id composer admits every row kind of plan_memo_ids.ROW_KINDS (the kind half of the spelling sweep)"] = ("CONTROL", row_kind_coverage_control)
    reg["container nesting is off the call stack: 1,000 nested quotes / items parse as commonmark.js nests them"] = ("CONTROL", deep_nesting_control)
    reg["a RuntimeError raised while PARSING a memo is a crash out of check(), never the unavailable-memo miss"] = ("CONTROL", parse_runtime_error_control)
    reg["diagnostics name a memo relative to the root memo's directory: `a/child.md` and `b/child.md` are two files, and a memo outside that directory is named by its absolute path"] = ("CONTROL", display_path_control)
    reg["PROPERTY: Population._kind reads every kind phrase from plan_memo_tables.KIND_PHRASES, the tuple the residue gate iterates (a fourth phrase cannot decide a kind without being gated)"] = ("CONTROL", kind_phrase_gate_control)
    reg["a row whose id cell declares no id is named by its declaring locator (`row <no id> at :LINE (token)`), never `row None`"] = ("CONTROL", empty_id_row_name_control)
    reg["PROPERTY: the verdict is invariant under a §2.5 re-spelling of any prose character the document renders the same (the rendered-text rule, swept position by position)"] = ("CONTROL", render_equivalence_control)
    return reg
