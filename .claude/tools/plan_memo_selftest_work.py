#!/usr/bin/env python3
"""The WORK controls for `plan-memo-umbrella-check.py --self-test`: every
control whose measure is a COST, not a text.

The seam is mechanical, not a taste: a control here states how much work a
clause is allowed to do and measures it with one of the harness's
deterministic witnesses (`_count_calls`, `_count_lines`, `_CountedList` --
never a wall clock, which has turned a control red in both directions on a
contended host).  This module is the ONLY importer of those three, so
"is this a work control?" is answered by the import list rather than by a
reader's judgement.  Everything a control can say about the checker's OUTPUT
-- what it reports, what it refuses to read, what it declares -- stays in
`plan_memo_selftest_controls.py`; `deep_nesting_control` stays there too,
because a depth of 1,000 that parses is a correctness claim about the
frame stack, not a cost bound.

`registry()` returns this module's fragment of the one name -> (kind,
control) table; `plan_memo_selftest_controls.registry()` merges it, so the
runner and the mutation proof still read ONE table.  A mutant row whose file
is THIS module patches the self-test, not the checker set: the module is
exec'd from the patched text (`plan_memo_selftest_harness.patched_module`)
and the row's controls are taken from the patched module's `registry()`
merged over the unpatched rest (`plan_memo_selftest_mutants.SELFTEST`).

Import direction, one way: the controls module imports this one; this one
imports the harness and nothing of the controls.
"""

import pathlib
import tempfile

from plan_memo_selftest_harness import _CountedList, _WorkExceeded, _count_calls, _count_lines


def linear_emphasis_control(M):
    """The linearity witness for §6.2's delimiter walk: N delimiter runs that
    pair with NOTHING cost O(N) work, not O(N^2).  `process_emphasis` searches
    back from each closer for an opener, and the Appendix's `openers_bottom`
    memo is what keeps a failed search from being repeated by every later
    closer of the same key; without it a paragraph of `*` runs re-walks the
    whole stack each time.  Source lines executed in the emphasis module are
    counted (the work passes through no module binding a call counter could
    watch), with the bound stated per run: quadratic growth blows it long
    before the wall clock would say so on any host.  The probe is `a* ` x N --
    runs that can only CLOSE (preceded by a letter, followed by a space), so
    every one of them searches back and finds nothing; a probe of runs that
    can only OPEN never enters the search at all and leaves the mutant
    alive (measured)."""
    import plan_memo_emphasis     # the freshly loaded module
    out = {}
    for n in (100, 400):
        runs = [plan_memo_emphasis.run_at("a* " * n, 3 * k + 1) for k in range(n)]
        try:
            with _count_lines(plan_memo_emphasis, limit=60 * n) as c:
                plan_memo_emphasis.process(runs)
        except _WorkExceeded:
            return False, "%d unmatched delimiter runs cost more than %d source lines: not linear" % (n, 60 * n)
        out[n] = c.lines
    ok = out[400] <= 4 * out[100] + 200
    return ok, "%d / %d source lines for 100 / 400 unmatched runs (<= 60N, and 4x the work for 4x the input)" % (
        out[100], out[400])


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


def linear_image_demotion_control(M):
    """The linearity witness for §6.4's demotion: N nested resolved images
    demote their descendants ONCE, not once per enclosing image.

    `![`x N is properly nested, so every enclosing image covers every image
    that closed inside it; re-tagging them where it closes costs 1+2+...+N.
    Measured at PR #510 R23 before the fix: 24 KB of that shape took 0.4 s
    against 0.1 s for 12 KB -- four times the work for twice the input, on
    input a plan memo could plausibly hold.  The fix records each demotion as
    an index RANGE and applies the union once (`plan_memo_lexer._demote`).

    Counted by `_count_lines`, not by the clock: a wall-clock ratio is not
    admissible here (a contended host has turned one red in both directions),
    and the demotion passes through no module binding `_count_calls` could
    watch.  Two probes, because the ranges are two: `images` (the nested
    images themselves) and `pairs` (the emphasis inside their descriptions),
    and a per-close walk over EITHER is the same defect.  The correctness
    half needs no control here -- the 335 inline conformance examples were
    green before this fix and are green after it, so what this control adds
    is only the work."""
    import plan_memo_lexer     # the freshly loaded module

    ok, detail = True, []
    for label, mk, per in (("images", lambda n: "![" * n + "x" + "](i)" * n, 160),
                           ("images + emphasis", lambda n: "![*" * n + "x" + "*](i)" * n, 200)):
        seen = {}
        for n in (100, 400):
            try:
                with _count_lines(plan_memo_lexer, limit=per * n) as c:
                    plan_memo_lexer.inline_pass(mk(n), {})
            except _WorkExceeded:
                return False, ("%d nested %s cost more than %d source lines: not linear"
                               % (n, label, per * n))
            seen[n] = c.lines
        ok = ok and seen[400] <= 4 * seen[100] + 400
        detail.append("%s %d / %d (<= %dN)" % (label, seen[100], seen[400], per))
    return ok, ("source lines for 100 / 400 nested, at most 4x the work for 4x the input: %s"
                % "; ".join(detail))

def registry():
    """name -> (kind, control), this module's fragment of the one table."""
    return {
        "emphasis matching is linear: N unmatched delimiter runs cost O(N) work (the Appendix's openers_bottom)":
            ("CONTROL", linear_emphasis_control),
        "links() is linear: 30 nested brackets are one inline_pass call":
            ("CONTROL", linear_links_control),
        "Phase-1 orphan detection is linear: <= 4 link_label calls per line":
            ("CONTROL", linear_orphans_control),
        "unresolved_references is linear: <= N*(log2 N + 2) reads of the line-offset table (a bisect per site, not a scan)":
            ("CONTROL", scaling_unresolved_control),
        "linked_files is linear: <= N Path.__eq__ calls over N distinct siblings (a set dedup hashes, a list compares)":
            ("CONTROL", scaling_linked_files_control),
        "split_row is linear: <= 64 source lines per character and per cell (breaks partitioned in the one scan)":
            ("CONTROL", scaling_split_row_control),
        "block quotes are linear: N quotes cost <= 4N quote_content calls":
            ("CONTROL", scaling_quotes_control),
        "a resolved image's demotion is linear: N nested images demote their descendants once, not once per enclosing image":
            ("CONTROL", linear_image_demotion_control),
    }
