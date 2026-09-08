#!/usr/bin/env python3
"""The WORK controls for `plan-memo-umbrella-check.py --self-test`: every
control whose measure is a COST, not a text.

The seam is mechanical, not a taste: a control here states how much work a
clause is allowed to do and measures it with one of the harness's
deterministic witnesses (`_count_calls`, `_count_lines`, `_count_line_sites`,
`_CountedList` -- never a wall clock, which has turned a control red in both
directions on a contended host).  The WORK MODULES -- this one and
`plan_memo_selftest_growth.py`, which it imports -- are the only importers of
those four, so "is this a work control?" is answered by the import graph
rather than by a reader's judgement.  Everything a control can say about the
checker's OUTPUT -- what it reports, what it refuses to read, what it
declares -- stays in `plan_memo_selftest_controls.py`; `deep_nesting_control`
stays there too, because a depth of 1,000 that parses is a correctness claim
about the frame stack, not a cost bound.

EVERY CONTROL HERE IS WRITTEN AGAINST A SHAPE, and that is the module's
limit as well as its point.  Each names a construct, a bound and a
discriminating partner, and each was written after a review round reported
that construct -- which is a population defined by the symptoms already seen,
and it had been wrong nine times in five rounds (R23 two non-linear sites,
R26 four, R27 two more, one of them not even in `inline_pass`).  The tenth
per-shape control was not the fix.  `plan_memo_selftest_growth.py` is: one
rule over a corpus GENERATED from the grammar.  These stay because each says
something sharper about its own clause than a growth bound can, and because
three of them measure costs the generated sweep cannot see at all (work
inside the C `re` engine, and Phase 1, which a block-level probe never
reaches).

`registry()` returns the WORK fragment of the one name -> (kind, control)
table -- this module's controls merged with the growth module's, exactly as
`plan_memo_selftest_controls.registry()` then merges that, so the runner and
the mutation proof still read ONE table.  A mutant row whose file is a work
module patches the self-test, not the checker set: the module is exec'd from
the patched text (`plan_memo_selftest_harness.patched_module`) and the row's
controls are taken from the patched module's `registry()` merged over the
unpatched rest (`plan_memo_selftest_mutants.SELFTEST`).

Import direction, one way: the controls module imports this one, this one
imports the growth module, and both import the harness and nothing of the
controls.
"""

import pathlib
import tempfile

from plan_memo_selftest_growth import registry as growth_registry
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
            p.write_text(text, encoding="utf-8")
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
            p.write_text(text, encoding="utf-8")
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
            p.write_text(text, encoding="utf-8")
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
        p.write_text(text, encoding="utf-8")
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

def linear_file_token_control(M):
    """The linearity witness for the bare file-name token: N parenthesis groups
    (or N names) are ONE pass over the text, not a scan from every start.

    THE DEFECT THIS REPLACED WAS ALREADY THERE (PR #510 R26-2, found while
    fixing the token's paren rule and reported by nobody).  `_TOKEN` was an
    `re` pattern whose file arm re-entered at every start position, so a run
    with no `.md` in it was rescanned from each of its own characters: measured
    against the retired pattern, `(a)`xN cost 3.95x and `a`xN 4.00x per
    doubling, where this scan costs 2.00x and 1.98x.  The paren fix and the
    cost fix are one edit, because "balanced at any depth" is not a regular
    language and leaving `re` is what made the reading expressible at all.

    Two probes, and they fail differently: `(a)`xN has N groups and NO
    candidate end (the retired pattern's worst case, a run it had to reject),
    while `x.md `xN has N candidate ends and N segments (this scan's own worst
    case, and the shape the mutant re-injects a per-candidate walk into).  A
    control on either alone would leave the other's clause unwatched.

    Counted by `_count_lines`: the work is a `for` inside one function and
    passes through no module binding `_count_calls` could watch."""
    import plan_memo_tokens     # the freshly loaded module

    ok, detail = True, []
    for label, mk, per in (("(a) groups", lambda n: "(a)" * n, 40),
                           ("x.md names", lambda n: "x.md " * n, 60)):
        seen = {}
        for n in (100, 400):
            try:
                with _count_lines(plan_memo_tokens, limit=per * n) as c:
                    plan_memo_tokens.file_and_cite_spans(mk(n))
            except _WorkExceeded:
                return False, ("%d %s cost more than %d source lines: not linear"
                               % (n, label, per * n))
            seen[n] = c.lines
        ok = ok and seen[400] <= 4 * seen[100] + 400
        detail.append("%s %d / %d (<= %dN)" % (label, seen[100], seen[400], per))
    return ok, ("source lines for 100 / 400, at most 4x the work for 4x the input: %s"
                % "; ".join(detail))


def linear_inline_tail_control(M):
    """The linearity witness for the §6.3 inline-link tail: `[`xN followed by
    `](`xN costs O(N), not O(N^2).

    THE SECOND FALSIFICATION OF `inline_pass`'s LINEAR CONTRACT (PR #510
    R26-3; R23 fixed the first, in the image demotion, and the docstring went
    on claiming linearity through both).  Every active `]` whose next character
    is `(` runs `_inline_tail`, whose destination scan walks the remaining
    suffix -- `]` is an ordinary destination character and each `](` deepens
    the parentheses -- and on failure the pass advances ONE character.
    Measured before the fix: 3.95x per doubling of the input.

    What bounds it is `link_destination`'s nesting limit, which §6.3's own
    parenthetical exists to permit ("Implementations may impose limits on
    parentheses nesting to avoid performance issues") -- the scan can only run
    long by opening parentheses it never closes, because a close that would
    take the run below its start ENDS it and any group with a net close
    RESOLVES the link instead of failing it.

    Two probes: the reported shape, and the same with a trailing `)` -- the
    obvious "fix" of refusing the tail when no `)` stands anywhere ahead passes
    the first and fails the second."""
    import plan_memo_lexer     # the freshly loaded module

    ok, detail = True, []
    for label, mk in (("[^N ](^N", lambda n: "[" * n + "](" * n),
                       ("[^N ](^N )", lambda n: "[" * n + "](" * n + ")")):
        seen = {}
        for n in (100, 400):
            try:
                with _count_lines(plan_memo_lexer, limit=1200 * n) as c:
                    plan_memo_lexer.inline_pass(mk(n), {})
            except _WorkExceeded:
                return False, "%d of %s cost more than %d source lines: not linear" % (n, label, 1200 * n)
            seen[n] = c.lines
        # 6x, not 4x, and the number is argued rather than tuned: the cost is
        # N brackets plus at most `DESTINATION_NESTING_LIMIT` scanned characters
        # per `]`, and the second term's constant is still settling at these
        # sizes (2.17x, then 2.08x, then 2.04x per doubling, measured).  What
        # the bound has to separate is LINEAR from QUADRATIC, and quadratic
        # growth over 4x the input is 16x.
        ok = ok and seen[400] <= 6 * seen[100]
        detail.append("%s %d / %d" % (label, seen[100], seen[400]))
    return ok, ("source lines for 100 / 400 -- linear is ~4x, quadratic would be ~16x: %s"
                % "; ".join(detail))


def linear_html_attempts_control(M):
    """The linearity witness for §6.6: a `<` whose closer stands nowhere ahead
    does not RUN the tag grammar.

    THE THIRD FALSIFICATION, and the one no existing witness could have seen
    (PR #510 R26-3).  Four of `_HTML_TAG`'s six alternatives reach their closer
    by an unbounded lazy scan, so over `<!--`xN every `<` scanned to the end of
    the text and failed, and the pass advanced one character: 3.46x / 3.30x /
    3.93x / 3.91x per doubling for a comment, a declaration, an instruction and
    a CDATA section -- all inside the C `re` engine, where `_count_lines`
    traces nothing and no binding the lexer holds is called.

    So the measure is ATTEMPTS, which is what the fix claims: `_match_tag` is
    the one site that runs the grammar, and it must be entered at most once per
    text here, not once per opener.  Every shape has the opener repeated N
    times with NO closer of its kind anywhere.  The last two are the
    DISCRIMINATING half: they end in a bare `>`, so a gate that asked only "is
    there a `>` ahead" -- the one closer every alternative shares, and the
    tempting way to write this in one line -- would pass them and re-run the
    lazy scan N times.  (`<!--`xN + `>` is NOT such a shape and is deliberately
    absent: its last four characters spell a real `-->`, so the comment arm
    MATCHES, consumes the text and makes progress, and it would witness
    nothing.)"""
    import plan_memo_lexer     # the freshly loaded module

    detail = []
    for label, mk in (("<!--", lambda n: "<!--" * n), ("<?x", lambda n: "<?x" * n),
                      ("<![CDATA[", lambda n: "<![CDATA[" * n), ("<!a", lambda n: "<!a" * n),
                      ("<?x + a bare >", lambda n: "<?x" * n + ">"),
                      ("<![CDATA[ + a bare >", lambda n: "<![CDATA[" * n + ">")):
        n = 200
        try:
            with _count_calls(plan_memo_lexer, "_match_tag", limit=1) as c:
                plan_memo_lexer.inline_pass(mk(n), {})
        except _WorkExceeded:
            return False, ("%d x %r ran the §6.6 grammar more than once: the closer index is not "
                           "gating the attempt" % (n, label))
        detail.append("%s %d" % (label, c.calls))
    # AND THE LOWER BOUND, which the mutation proof demanded: an index that
    # answers "no" to everything makes all six counts 0 and is the cheapest
    # possible gate, so a control with only the upper bound reports a refusal
    # of every §6.6 span as a success (measured -- the `return False` mutant
    # SURVIVED this control until this probe was added).
    with _count_calls(plan_memo_lexer, "_match_tag", limit=None) as c:
        plan_memo_lexer.inline_pass("a <b>x</b> b", {})
    if c.calls < 2:
        return False, ("a text holding two real tags ran the §6.6 grammar %d time(s): the closer index "
                       "is refusing what it should admit" % c.calls)
    return True, ("grammar attempts over 200 openers with no closer of their kind: %s; two real tags "
                  "%d" % ("; ".join(detail), c.calls))


def linear_closer_index_control(M):
    """The linearity witness for the closer index ITSELF: over a text with N
    openers and no closer of any kind, at most one search per literal is
    STARTED, not one per opener.

    The gate above would still be a quadratic if it asked its question
    quadratically -- `str.find` from an opener scans forward to the end of the
    text, so re-asking at every `<` costs exactly what the regex used to.
    `_Closers` carries a CURSOR for that reason; the two are one fix and two
    claims, so they are two controls.  The search runs in C, so what is
    countable is how many are STARTED (`_find_from`), which is the claim.

    The bound is four, one per distinct literal (`-->`, `?>`, `]]>`, `>`), over
    600 openers of the three lazy kinds -- where a per-opener search is 600.
    A count of ZERO is red too: it would mean the index was never asked."""
    import plan_memo_lexer     # the freshly loaded module

    text = "<!--" * 200 + "<?x" * 200 + "<![CDATA[" * 200
    try:
        with _count_calls(plan_memo_lexer, "_find_from", limit=4) as c:
            plan_memo_lexer.inline_pass(text, {})
    except _WorkExceeded:
        return False, ("600 openers with no closer started more than 4 searches: the closer index is "
                       "re-asking `find` per opener rather than carrying a cursor")
    return c.calls > 0, ("%d search(es) started over 600 openers (<= 4, one per literal; 0 would mean "
                         "the index is never consulted)" % c.calls)


def linear_code_closer_control(M):
    """The linearity witness for §6.1: the closer of a backtick string is found
    by an INDEX, not by walking the runs.

    THE FOURTH member of R26-3's class, reported by nobody and measured twice
    (`plan_memo_lexer.backtick_runs` carries the arithmetic).  The walk is
    superlinear rather than quadratic -- 0.27 x L^1.5, flat over a 60x range of
    lengths -- and the witness is NOT the obvious one: runs of lengths 1, 2,
    3, ... measure 3.9x per doubling of the run count and 1.0x per doubling of
    the LENGTH, because that shape's text grows quadratically with its own
    parameter.  This probe is the shape that does degrade: D unclosable runs
    first, D^2/2 short runs after them for the walks to cross.

    Counted by `_count_lines`: the walk was a `while` inside one function."""
    import plan_memo_lexer     # the freshly loaded module

    def shape(d):
        return "".join("`" * (k + 2) + "x" for k in range(d)) + "`x" * (d * d // 2)

    seen = {}
    for d in (20, 80):
        text = shape(d)
        try:
            with _count_lines(plan_memo_lexer, limit=12 * len(text)) as c:
                plan_memo_lexer.inline_pass(text, {})
        except _WorkExceeded:
            return False, ("%d characters of unclosable backtick runs cost more than %d source "
                           "lines: the closer is being walked to, not indexed" % (len(text), 12 * len(text)))
        seen[len(text)] = c.lines
    (l1, w1), (l2, w2) = sorted(seen.items())
    return (w2 <= (l2 / l1) * w1 * 1.5,
            "source lines %d / %d for %d / %d characters (%.1fx the work for %.1fx the length)"
            % (w1, w2, l1, l2, w2 / w1, l2 / l1))


# The id-scan witness's shapes (PR #510 R29-2).  `K` ids, each wrapped in `d`
# decoration marks a side; `d` runs over `_SCAN_WIDTHS` so the cost of ONE more
# mark can be read off as a difference rather than compared against a constant
# somebody chose.  Three widths, because two give one difference and one
# difference cannot be shown to be constant.
_SCAN_IDS, _SCAN_WIDTHS = 50, (1, 2, 3)
# The run with no id after it: the shape the retired pattern re-entered at
# every position.  Two lengths, eight times apart, and the claim is EQUALITY.
_SCAN_RUN, _SCAN_RUN_LONG = 1000, 8000


def linear_id_scan_control(M):
    """The linearity witness for the id scan, stated EXACTLY rather than as a
    ratio: the decoration around an id is walked ONCE per id, and a run of
    decoration marks that no id follows is not walked at all.

    THE HALF THIS CAN SEE, AND THE HALF IT CANNOT (PR #510 R29-2).  The defect
    was `finditer` over a pattern beginning with `DECOR`, so the engine consumed
    a decoration run at every position inside it and unwound -- 0.016 / 0.063 /
    0.259 s over `!` + N backticks for N = 1000 / 2000 / 4000.  That cost was
    inside the C `re` engine: no Python line ran and no module binding was
    called, so NO witness in this suite could have gone red on it, and the sweep
    that states it instead is
    `plan_memo_selftest_properties.leading_run_scan_control`.  What this control
    holds is the fix's OWN clause.  The scan now finds the id core with a
    pattern and walks the decoration outward in PYTHON, which is countable --
    and which is a re-scan waiting to be written, since the two walks are the
    only place a run can be crossed twice.

    THREE CLAIMS, EACH AN EQUALITY OR AN EXACT COUNT, so none of them needs a
    doubling family or a bound anybody tuned:

      * a run of marks with NO id after it costs the scan the same number of
        source lines at 1,000 marks and at 8,000 -- the walks are never entered,
        because there is no core to enter them from;
      * `_decor_start` and `_decor_end` are called EXACTLY once per id the scan
        yields, at every decoration width: one walk each side, per id, never per
        position;
      * one more mark per side costs the SAME number of source lines at every
        width, so a mark is walked a constant number of times rather than a
        number that grows with the run it sits in.  A walk that re-crossed the
        marks it had already crossed would make that difference grow.

    The counts are also required to be non-zero where the claim is about
    something happening (the ids are found, the walks are entered), since a
    counter watching nothing reports what a clean run reports."""
    import plan_memo_ids as ids     # the freshly loaded module

    def lines(text):
        with _count_lines(ids, limit=10 ** 9) as c:
            list(ids.tokens(text))
        return c.lines

    def calls(text):
        with _count_calls(ids, "_decor_start", limit=None) as a, \
                _count_calls(ids, "_decor_end", limit=None) as b:
            found = len(list(ids.tokens(text)))
        return found, a.calls, b.calls

    run, long_run = "!" + "`" * _SCAN_RUN, "!" + "`" * _SCAN_RUN_LONG
    short_lines, long_lines = lines(run), lines(long_run)
    _found, run_starts, run_ends = calls(long_run)
    bad = []
    if short_lines != long_lines:
        bad.append("a run of %d marks costs %d source lines and one of %d costs %d: the scan is "
                   "entering the run" % (_SCAN_RUN, short_lines, _SCAN_RUN_LONG, long_lines))
    if run_starts or run_ends:
        bad.append("a run with no id after it walked the decoration %d + %d time(s)"
                   % (run_starts, run_ends))
    seen, widths = {}, []
    for d in _SCAN_WIDTHS:
        text = ("`" * d + "9z" + "`" * d) * _SCAN_IDS
        found, starts, ends = calls(text)
        widths.append((d, found, starts, ends))
        if (found, starts, ends) != (_SCAN_IDS, _SCAN_IDS, _SCAN_IDS):
            bad.append("width %d: %d id(s), %d left and %d right walk(s) (each must be %d)"
                       % (d, found, starts, ends, _SCAN_IDS))
        seen[d] = lines(text)
    steps = [seen[b] - seen[a] for a, b in zip(_SCAN_WIDTHS, _SCAN_WIDTHS[1:])]
    if len(set(steps)) != 1 or steps[0] <= 0:
        bad.append("one more mark a side costs %s source lines at the successive widths: not constant"
                   % steps)
    ok = not bad and short_lines > 0
    return ok, ("%d source lines over a run of %d and of %d marks with no id (equal, and the walks "
                "entered %d + %d times); %d id(s) and one walk a side each at widths %s; %s source "
                "lines more per extra mark%s"
                % (short_lines, _SCAN_RUN, _SCAN_RUN_LONG, run_starts, run_ends, _SCAN_IDS,
                   list(_SCAN_WIDTHS), steps, (": FAIL " + "; ".join(bad)) if bad else ""))


# The three quote shapes and the number of lines ONE quote's gather examines in
# each: its own marker line; that plus the blank line that ends it; that plus a
# lazy continuation candidate between them.  The numbers are read off the
# fixtures, not off the driver.
_QUOTE_SHAPES = (("nested", lambda n: ">" * n + " x\n", 1),
                 ("blank-ended", lambda n: "> q\n\n" * n, 2),
                 ("lazy candidate", lambda n: "> a\nb\n\n" * n, 3))
_QUOTE_N = 400


def quote_build_once_control(M):
    """The §5.1 marker's cost claim, stated EXACTLY: `quote_content` builds a
    line's content once per line a quote's gather examines, and never to answer
    WHETHER a line carries a marker.

    WHAT WAS WRONG (PR #510 R29-1).  `quote_content` was the marker test as
    well as the content build, and its content is a fresh copy of the rest of
    the line.  Two of its three callers wanted only the test -- `starts_block`,
    and the `_parse` arm that opens the container -- so inside N nested quotes
    the same suffix was copied twice per level and one copy was thrown away
    after a comparison against `None`.  Measured over `">"*2000 + " x"` before
    the split: 4,002 calls copying 4,005,998 characters, against 2,000 quotes.
    After it: 2,000 calls copying 2,002,999.

    WHAT IS STILL TRUE, AND THIS CONTROL DOES NOT SAY OTHERWISE.  The
    remaining copy is one per line per ENCLOSING quote, and Phase 1 hands each
    container's content to itself as a list of STRINGS, so the characters it
    materialises are the sum of the content lengths over the nesting levels --
    quadratic in the depth, and not removable without giving a "line" an
    OFFSET instead of being a copy, which is a change to every `blocks.py`
    predicate.  This control therefore counts BUILDS and never characters: a
    control that asserted the character total would be asserting that
    quadratic is correct.

    AND THE COPY IS NOT WHAT THAT SHAPE COSTS, measured rather than assumed.
    Over `">"*n + " x"` the Python lines executed in `plan_memo_memo` and
    `plan_memo_blocks` are EXACTLY linear (572,256 -> 1,144,256 for n = 4,000
    -> 8,000, 2.00x), so every superlinear term is C-level; and `gc.disable()`
    takes n = 64,000 from 0.581 s to 0.224 s and its growth from 2.9x to 2.2x
    per doubling, which makes the cyclic collector -- walking the N suspended
    frames and their content lists at every collection -- the larger share, not
    the memcpy.  Holding a tail of 128,000 characters at a fixed depth of 8,000
    multiplies the copied volume 16-fold and the time by 2.4.

    THE THREE PROBES are the three ways a gather ends, because the rule is
    about the lines a gather EXAMINES and each shape gives it a different
    number: the nested shape (its own marker line, and nothing after it), the
    blank-ended shape (the marker line and the blank that closes the quote) and
    the lazy shape (a continuation candidate between the two).  A control on
    the nested shape alone would leave `starts_block`'s test -- which only runs
    where a gather reaches a candidate -- unwatched (measured: reverting it is
    401 builds against 400 on that shape, and 1,200 against 600 on this one).
    The marker test is required to have been called as well, since a driver
    that asked nothing at all would report the same equality."""
    import plan_memo_blocks     # the freshly loaded modules
    import plan_memo_memo

    bad, detail = [], []
    for label, make, per in _QUOTE_SHAPES:
        text = make(_QUOTE_N)
        with tempfile.TemporaryDirectory() as d:
            p = pathlib.Path(d) / "quotes.md"
            p.write_text(text, encoding="utf-8")
            # BOTH bindings of each name, because `Memo` imports them at
            # import time and `starts_block` calls its own module's: a counter
            # on the driver's binding alone leaves the `blocks.py` caller
            # unwatched, and its mutant SURVIVED against exactly that (PR #510
            # R29-1) -- the probe had a different subject from the claim.
            with _count_calls(plan_memo_memo, "quote_content", limit=None) as b1, \
                    _count_calls(plan_memo_blocks, "quote_content", limit=None) as b2, \
                    _count_calls(plan_memo_memo, "quote_marker", limit=None) as t1, \
                    _count_calls(plan_memo_blocks, "quote_marker", limit=None) as t2:
                memo = plan_memo_memo.Memo(p)
        quotes = sum(1 for b in memo.sequence if b[0] == "quote")
        builds, tests = b1.calls + b2.calls, t1.calls + t2.calls
        detail.append("%s: %d quote(s), %d build(s), %d test(s)" % (label, quotes, builds, tests))
        if quotes != _QUOTE_N:
            bad.append("%s parsed %d quotes, not %d" % (label, quotes, _QUOTE_N))
        elif builds != per * quotes:
            bad.append("%s built %d contents over %d quotes examining %d line(s) each (must be %d)"
                       % (label, builds, quotes, per, per * quotes))
        if tests <= builds:
            bad.append("%s asked the marker test %d time(s) against %d build(s): the callers that "
                       "want only the test are building again" % (label, tests, builds))
    return not bad, ("; ".join(detail) + ("; FAIL " + "; ".join(bad) if bad else
                                          " -- one build per line examined, never one per question"))


def registry():
    """name -> (kind, control), the WORK fragment of the one table: this
    module's per-shape witnesses merged with the growth module's generated
    sweep."""
    reg = dict(growth_registry())
    reg.update({
        "file_and_cite_spans is linear: N parenthesis groups are one pass, not a re-scan from every start position":
            ("CONTROL", linear_file_token_control),
        "inline_pass is linear over a malformed inline-link tail: `[`xN + `](`xN is O(N), bounded by §6.3's permitted nesting limit":
            ("CONTROL", linear_inline_tail_control),
        "inline_pass does not RUN the §6.6 grammar at a `<` whose closer stands nowhere ahead (the one cost the Python-level witnesses cannot see)":
            ("CONTROL", linear_html_attempts_control),
        "the closer index carries a cursor: 600 openers with no closer start at most one search per literal, not one per opener":
            ("CONTROL", linear_closer_index_control),
        "a §6.1 code span's closer is found by an index of the runs BY LENGTH, not by walking them (0.27 x L^1.5 before)":
            ("CONTROL", linear_code_closer_control),
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
        "the §5.1 marker test builds no content: quote_content is called once per line a quote's gather examines and never to answer whether a line carries a marker":
            ("CONTROL", quote_build_once_control),
        "the id scan walks a decoration run ONCE per id and never at all where no id follows it (an exact count, not a ratio -- the half of R29-2 a Python witness can see)":
            ("CONTROL", linear_id_scan_control),
        "a resolved image's demotion is linear: N nested images demote their descendants once, not once per enclosing image":
            ("CONTROL", linear_image_demotion_control),
    })
    return reg
