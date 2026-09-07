#!/usr/bin/env python3
"""The function-shaped controls for `plan-memo-umbrella-check.py --self-test`,
and the registry that names every control.

The record-shaped controls (`Case`: one fixture, one measure, one exact
value) live in `plan_memo_selftest_cases.py` / `_cases_pr510.py`; this module
holds the controls a record cannot express -- a property over the module set
(the spelling sweep, the row-kind coverage), a linearity witness (a counted
call, a counted line, a counted read), an injected fault (a raising
`resolve()`, a parser `RuntimeError`), a comparison of two runs (the pipe
shape, the attribution), the CommonMark conformance corpus -- and `registry()`,
the ONE name -> (kind, control) table both the runner and the mutation proof
read.  Every control here is a function of the freshly loaded checker module
returning `(ok, detail)`, and every one is reachable by name through the
registry, because the mutant runner re-runs controls BY NAME against a
patched set.

`empty_registry_fails`, the runner's emptiness guard, lives here beside its
proof (`empty_registry_control`) so the one mutant against it patches one
file: a mutant row whose file is this module is exec'd from patched text
(`plan_memo_selftest_harness.patched_module`) and its controls are taken from
the PATCHED module's registry.

Import direction, one way: the runner imports this module; this module
imports the harness (`plan_memo_selftest_harness.py`) and the case registry.
"""

import pathlib
import tempfile

from plan_memo_selftest_cases import CASES, VIOLATION, build
import plan_memo_selftest_cases_pr510  # noqa: F401 -- appends the review-round controls to CASES
from plan_memo_selftest_harness import (
    GRAMMAR, MODULES, SOURCES, _count_calls, _count_lines, _CountedList, _WorkExceeded, control, run_on,
)


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


def linear_links_control(M):
    """The linearity witness for the "Appendix: A parsing strategy" bracket
    stack: 30 nested brackets are parsed by EXACTLY ONE `inline_pass` call
    (no substring is re-parsed).  A recursive inner re-parse (the per-clause
    patch this replaced) re-enters `inline_pass` once per `]` and is
    exponential in the depth; the counter stops it at the second call.  The
    timing is reported for information only."""
    import time
    import plan_memo_lexer      # the freshly loaded module
    s = "[" * 30 + "x" + "]" * 30
    t0 = time.perf_counter()
    try:
        with _count_calls(plan_memo_lexer, "inline_pass", limit=1) as c:
            plan_memo_lexer.inline_pass(s, {})
    except _WorkExceeded:
        return False, "30-deep nested brackets re-entered inline_pass (a re-parse): not linear"
    ms = (time.perf_counter() - t0) * 1000
    return c.calls == 1, "30-deep nested brackets in %d inline_pass call (%.3f ms, informative)" % (c.calls, ms)


def linear_orphans_control(M):
    """The linearity witness for Phase-1 orphan detection, two-fold: a
    paragraph of N definition-shaped lines (all orphans -- `text` heads the
    paragraph) is read by `Memo` with EXACTLY one `link_label` call per line
    -- one shape parse per line, counted where Phase 1 calls it: the
    `plan_memo_blocks` binding `reference_definitions` reads (⚠ counting the
    lexer's own binding saw Phase 2's bracket parse, one call per `[l..]`,
    and nothing of Phase 1: the quadratic mutant RG-1 was then killed only by
    48 s of wall clock, the whole of the trip-wire's runtime -- the exact
    count is what makes a mis-bound counter red: fewer than N calls means
    the counter is not watching the parser) -- and the counter stops the
    block at 4 calls per line, over N = 1000 and 4N = 4000.  The per-line
    re-walk this replaced parsed every remaining definition again per line
    (~4.5 million `link_label` calls, 7.95 s).  The wall-clock ratio
    t(4N)/t(N) is printed for INFORMATION only: as a gate (`< 8`) it went
    red on the PRODUCTION code under bursty host load (9.3-10.6 measured,
    min of 3), and a host-dependent gate is a CI flake in both directions.
    What the count does not see and the ratio would have: a per-line
    re-JOIN of the rest of the run (`"\\n".join(lines[i:])` per line) that
    parses nothing -- C-level work, no call, no Python line.  No mutant of
    that class exists; if one is written it needs a witness of its own."""
    import time
    import plan_memo_blocks     # the freshly loaded module
    import plan_memo_memo

    def run(n):
        text = "text\n" + "".join("[l%d]: f%d.md\n" % (i, i) for i in range(n - 1))
        with tempfile.TemporaryDirectory() as d:
            p = pathlib.Path(d) / "orphans.md"
            p.write_text(text)
            t0 = time.perf_counter()
            with _count_calls(plan_memo_blocks, "link_label", limit=4 * n) as c:
                memo = plan_memo_memo.Memo(p)
            t = time.perf_counter() - t0
            orphans = sum(len(v) for v in memo.orphans.values())
        return orphans, c.calls, t

    try:
        o1, c1, t1 = run(1000)
        o4, c4, t4 = run(4000)
    except _WorkExceeded:
        return False, "Memo exceeded 4 link_label calls per line: not linear"
    ratio = t4 / t1 if t1 else float("inf")
    ok = o1 == 999 and o4 == 3999 and c1 == 1000 and c4 == 4000
    return ok, ("%d/%d orphans, %d/%d Phase-1 link_label calls (must be exactly 1000/4000); "
                "t(1000)=%.1f ms t(4000)=%.1f ms ratio %.1f (informative)" % (o1, o4, c1, c4, t1 * 1000, t4 * 1000, ratio))


def scaling_unresolved_control(M):
    """The linearity witness for the unresolved-reference walk, deterministic:
    N reference lines are ONE paragraph with N sites, and `Paragraph.locate`
    reads the line-offset table at most N * (ceil(log2(N+1)) + 2) times --
    a bisect probes ceil(log2(N+1)) entries per site (`N.bit_length()` IS
    that number for N >= 1), plus one `len` and the final `offsets[k]` --
    and at least N times (one read per site; a counter watching nothing is
    red).  A linear scan per site (the R6-2 mutant, ~N^2/2 reads) is
    stopped at the bound.  The table is wrapped in a counting list AFTER the
    memo is parsed (`Paragraph.offsets` is a slot), so the walk is the
    production walk over the production table; `bisect` consults a list
    subclass through `__getitem__`.  Over N = 1000 and 4N = 4000.  The
    wall-clock ratio t(4N)/t(N) is printed for information only: as a gate
    (`< 8`) it depended on the host -- under bursty load the production
    ratios of this control's siblings measured 8.4-10.6, red, and a
    quadratic mutant's could fall under 8 the same way."""
    import time
    import plan_memo_memo

    def bound(n):
        return n * (n.bit_length() + 2)     # N * (ceil(log2(N+1)) + 2)

    def run(n):
        text = "".join("[x][missing]\n" for _ in range(n))
        with tempfile.TemporaryDirectory() as d:
            p = pathlib.Path(d) / "u.md"
            p.write_text(text)
            memo = plan_memo_memo.Memo(p)
            tables = [_CountedList(para.offsets, limit=bound(n)) for para in memo.paragraphs]
            for para, table in zip(memo.paragraphs, tables):
                para.offsets = table
            t0 = time.perf_counter()
            sites = len(memo.unresolved_references())
            t = time.perf_counter() - t0
        return sites, len(tables), sum(x.reads for x in tables), t

    try:
        n1, p1, r1, t1 = run(1000)
        n4, p4, r4, t4 = run(4000)
    except _WorkExceeded:
        return False, "locate read the offset table more than N * (log2 N + 2) times: a scan per site, not a bisect"
    ratio = t4 / t1 if t1 else float("inf")
    ok = (n1, p1, n4, p4) == (1000, 1, 4000, 1) and 1000 <= r1 <= bound(1000) and 4000 <= r4 <= bound(4000)
    return ok, ("%d/%d sites in %d/%d paragraph(s), %d/%d offset-table reads (N <= reads <= N*(log2 N + 2) "
                "= %d/%d); t(1000)=%.2f ms t(4000)=%.2f ms ratio %.1f (informative)"
                % (n1, n4, p1, p4, r1, r4, bound(1000), bound(4000), t1 * 1000, t4 * 1000, ratio))


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


def scaling_linked_files_control(M):
    """The linearity witness for `linked_files`' dedup, deterministic: over
    N links to N DISTINCT siblings the dedup makes at most N
    `PurePath.__eq__` calls -- a set compares an entry only on a hash match,
    so N distinct paths cost 0 comparisons -- and at least N
    `PurePath.__hash__` calls (each candidate is hashed to be looked up; a
    counter watching nothing is red).  A list membership test (the R8-5
    mutant) compares each candidate against every earlier one, ~N^2/2
    `__eq__` calls, and is stopped at the bound.  Both dunders are counted on
    `pathlib.PurePath` itself: `Path` inherits them, and `list.__contains__`
    / `set.__contains__` reach them through the type slot (`set` cannot be
    hooked; the comparisons it does not make can be counted).  Over N = 1000
    and 4N = 4000.  The wall-clock ratio t(4N)/t(N) is printed for
    information only: as a gate (`< 8`) the production ratio measured 7.4-7.6
    under bursty host load, and the mutant's 10.7 unloaded was the thinnest
    kill of the suite -- a host-dependent gate flakes in both directions."""
    import time
    import plan_memo_memo

    def run(n):
        # N DISTINCT siblings (none need exist: `linked_files` names, the
        # population checks), so the dedup structure actually grows
        text = "".join("See [x%d](slice-9z-sib-%d.md).\n" % (i, i) for i in range(n))
        with tempfile.TemporaryDirectory() as d:
            p = pathlib.Path(d) / "links.md"
            p.write_text(text)
            memo = plan_memo_memo.Memo(p)
            t0 = time.perf_counter()
            with _count_calls(pathlib.PurePath, "__hash__", limit=None) as h, \
                    _count_calls(pathlib.PurePath, "__eq__", limit=n) as e:
                k = len(memo.linked_files())
            t = time.perf_counter() - t0
        return k, e.calls, h.calls, t

    try:
        k1, e1, h1, t1 = run(1000)
        k4, e4, h4, t4 = run(4000)
    except _WorkExceeded:
        return False, "linked_files compared paths more than N times: a list membership test, not a set"
    ratio = t4 / t1 if t1 else float("inf")
    ok = (k1, k4) == (1000, 4000) and e1 <= 1000 and e4 <= 4000 and h1 >= 1000 and h4 >= 4000
    return ok, ("%d/%d siblings, %d/%d Path.__eq__ calls (<= N), %d/%d Path.__hash__ calls (>= N); "
                "t(1000)=%.2f ms t(4000)=%.2f ms ratio %.1f (informative)"
                % (k1, k4, e1, e4, h1, h4, t1 * 1000, t4 * 1000, ratio))


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


def scaling_split_row_control(M):
    """The linearity witness for `split_row`, deterministic: over N
    escaped-pipe cells it executes at most 64 * (len(line) + N) source lines
    of `plan_memo_blocks` -- the row scan, `_escaped`, the two trims, `_cell`
    and `Cell.__init__` are 58 source lines together, and each of them runs
    at most once per character (the scan step; `_escaped`'s backslash walk
    and a trim visit a character once) or once per cell (the assembly step)
    -- and at least len(line) (the scan visits every character; a tracer
    watching nothing is red).  A per-cell filter over the row-wide break
    list (the R9 #3 mutant) runs ~2 N^2 lines and is stopped at the bound.
    Counted by `_count_lines`, a `sys.settrace` line counter over this ONE
    module's frames: the work here passes through no module binding a
    `_count_calls` could watch (a comprehension over a local list), so the
    executed source lines ARE the count.  N = 500 and 4N = 2000.  The
    wall-clock ratio t(4N)/t(N) is printed for information only: as a gate
    (`< 8`) the production ratio measured 8.4 under bursty host load -- red
    on correct code -- where the count is the same on every host."""
    import time
    import plan_memo_blocks

    def bound(length, n):
        return 64 * (length + n)

    def run(n):
        line = "|" + " a\\|b |" * n
        t0 = time.perf_counter()
        with _count_lines(plan_memo_blocks, limit=bound(len(line), n)) as c:
            k = len(plan_memo_blocks.split_row(line))
        t = time.perf_counter() - t0
        return k, c.lines, len(line), t

    try:
        k1, l1, n1, t1 = run(500)
        k4, l4, n4, t4 = run(2000)
    except _WorkExceeded:
        return False, "split_row ran more than 64 source lines per character and cell: a per-cell filter, not one scan"
    ratio = t4 / t1 if t1 else float("inf")
    ok = (k1, k4) == (500, 2000) and n1 <= l1 <= bound(n1, 500) and n4 <= l4 <= bound(n4, 2000)
    return ok, ("%d/%d cells, %d/%d source lines over %d/%d characters (len <= lines <= 64*(len+N), %.1f per "
                "character+cell); t(500)=%.2f ms t(2000)=%.2f ms ratio %.1f (informative)"
                % (k1, k4, l1, l4, n1, n4, l1 / (n1 + 500), t1 * 1000, t4 * 1000, ratio))


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
    """The CommonMark 0.31.2 spec's own `Raw HTML` §6.6 examples (613-632)
    through Phase 1 and Phase 2 (`plan_memo_selftest_conformance.run_inline`):
    the raw HTML spans the one inline pass masks are exactly the text the
    expected html emits verbatim, paragraph by paragraph; the detail is
    printed whole, like the block half's."""
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


def scaling_quotes_control(M):
    """The linearity witness for §5.1: N one-line block quotes separated by
    blank lines are read with at most 4N `quote_content` calls (counted at
    the driver's binding, `plan_memo_memo`: the branch test, the marker
    line, the blank candidate) and at least N (a counter watching nothing
    is red).  A quote that gathers every remaining line as a candidate
    (the bound dropped) makes N^2 calls and is stopped at the limit."""
    import plan_memo_memo
    n = 1000
    text = "> q\n\n" * n
    with tempfile.TemporaryDirectory() as d:
        p = pathlib.Path(d) / "quotes.md"
        p.write_text(text)
        try:
            with _count_calls(plan_memo_memo, "quote_content", limit=4 * n) as c:
                memo = plan_memo_memo.Memo(p)
        except _WorkExceeded:
            return False, "%d quotes exceeded %d quote_content calls: not linear" % (n, 4 * n)
    quotes = sum(1 for b in memo.sequence if b[0] == "quote")
    return quotes == n and n <= c.calls <= 4 * n, "%d quotes, %d quote_content calls (N <= calls <= 4N)" % (
        quotes, c.calls)


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


def registry():
    """name -> (kind, control)."""
    reg = {}
    for c in CASES:
        assert c.name not in reg, "duplicate control name %r" % c.name
        reg[c.name] = (c.kind, control(c))
    reg["CommonMark 0.31.2 spec examples (Tabs, §4.1-§4.9, §5.1-§5.3): Phase 1's block sequence aligns with the html"] = ("CONTROL", spec_examples_control)
    reg["CommonMark 0.31.2 spec examples (§6.6 Raw HTML): the spans Phase 2 masks are the html's verbatim `<` text"] = ("CONTROL", inline_examples_control)
    reg["Phase 1's block sequence over the §4.4 chunk and the §5.1 / §5.2 container shapes matches commonmark.js"] = ("CONTROL", sequence_control)
    reg["a lazy schema header after a definition in a linked memo's quote is a table: id declared, kind umbrella, census +1"] = ("CONTROL", lazy_header_after_definition_control)
    reg["block quotes are linear: N quotes cost <= 4N quote_content calls"] = ("CONTROL", scaling_quotes_control)
    reg["a marker naming another row does not enter the count"] = ("CONTROL", attribution_control)
    reg["declaring-field parse and whole-line marker grep differ"] = ("CONTROL", degenerate_control)
    reg["a table with and without edge pipes reads the same"] = ("CONTROL", pipe_shape_control)
    reg["a site after an escaped pipe is reported at its raw column"] = ("CONTROL", raw_offset_control)
    reg["an empty control or mutant registry is a FAIL, never green"] = ("CONTROL", empty_registry_control)
    reg["links() is linear: 30 nested brackets are one inline_pass call"] = ("CONTROL", linear_links_control)
    reg["Phase-1 orphan detection is linear: <= 4 link_label calls per line"] = ("CONTROL", linear_orphans_control)
    reg["unresolved_references is linear: <= N*(log2 N + 2) reads of the line-offset table (a bisect per site, not a scan)"] = ("CONTROL", scaling_unresolved_control)
    reg["a decoded destination with a C0 control character is rejected, never resolved"] = ("CONTROL", control_char_destination_control)
    reg["an OSError from resolve() is the unavailable-sibling schema miss, never an exception"] = ("CONTROL", unavailable_sibling_control)
    reg["linked_files is linear: <= N Path.__eq__ calls over N distinct siblings (a set dedup hashes, a list compares)"] = ("CONTROL", scaling_linked_files_control)
    reg["an undecodable sibling is the unavailable-linked-memo schema miss, never an exception"] = ("CONTROL", undecodable_sibling_control)
    reg["split_row is linear: <= 64 source lines per character and per cell (breaks partitioned in the one scan)"] = ("CONTROL", scaling_split_row_control)
    reg["an orphan definition exempts its OWN bracket only: `[sib]: child.md \"[sib]\"` is the documented miss, rc 2, child.md not walked"] = ("CONTROL", orphan_offset_control)
    reg["PROPERTY: the id character classes are spelled once, in plan_memo_ids.py (a source-text sweep)"] = ("CONTROL", id_spelling_sweep_control)
    reg["PROPERTY: every row-id composer admits every row kind of plan_memo_ids.ROW_KINDS (the kind half of the spelling sweep)"] = ("CONTROL", row_kind_coverage_control)
    reg["container nesting is off the call stack: 1,000 nested quotes / items parse as commonmark.js nests them"] = ("CONTROL", deep_nesting_control)
    reg["a RuntimeError raised while PARSING a memo is a crash out of check(), never the unavailable-memo miss"] = ("CONTROL", parse_runtime_error_control)
    reg["diagnostics name a memo relative to the root memo's directory: `a/child.md` and `b/child.md` are two files, and a memo outside that directory is named by its absolute path"] = ("CONTROL", display_path_control)
    return reg
