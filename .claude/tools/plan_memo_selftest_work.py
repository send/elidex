#!/usr/bin/env python3
"""The WORK controls for `plan-memo-umbrella-check.py --self-test`: every
control whose measure is a COST, not a text.

The seam is mechanical, not a taste: a control here states how much work a
clause is allowed to do and measures it with one of the harness's
deterministic witnesses (`_count_calls`, `_count_lines`, `_count_line_sites`,
`_CountedList`, `_count_pattern_spans` -- never a wall clock, which has turned
a control red in both directions on a contended host).  The WORK MODULES --
this one and the two it imports, `plan_memo_selftest_pipeline.py` and
`plan_memo_selftest_growth.py` -- are the only importers of those five, so "is
this a work control?" is answered by the import graph rather than by a
reader's judgement.  Between the three the seams are TWO and they are
not the same question.  `plan_memo_selftest_growth.py` is separated by its
POPULATION: its probes are generated from the grammar or from the definition
of a memo family, where this module's and the pipeline module's are shapes
somebody wrote after a round reported one.  This module and
`plan_memo_selftest_pipeline.py` are separated by the PROBE: here one call is
handed a string built in memory, there a memo is written to a scratch
directory and Phase 1 or `check()` is run over it -- which is why the pipeline
module is the only importer of `tempfile` among the two written-shape modules.
⚠ Not among all three: the growth module writes memos too (its digraph
corpus), which is the seam being the population and not the probe.  Everything a control can say about the
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

from plan_memo_selftest_growth import registry as growth_registry
from plan_memo_selftest_harness import _WorkExceeded, _count_calls, _count_lines
from plan_memo_selftest_pipeline import registry as document_registry


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
    alive (measured).

    TWO SHAPES, because one of them was not enough and a round proved it.
    (a) UNMATCHED: `a* ` x N, the closers-that-find-nothing above.
    (b) NESTED: `*a ` x N then `b* ` x N -- N openers, then their N closers,
    so every closer PAIRS and each successful match had to clear the
    delimiters between the two.  Until PR #510 R31-2 that clearing re-walked
    the growing range of already-dead inner delimiters, which (a) cannot see:
    in (a) nothing ever matches, so there is no range to clear.
    ⚠ (b) IS A STOPGAP, NOT A DESIGN CHOICE, and the difference was measured at
    PR #510 R32.  With the pre-R31 scan re-injected the generated growth
    property is GREEN over all 3,577 of its probes, because a delimiter atom
    repeated merges into ONE long run (`***bbb`) rather than the N separate
    runs this shape needs.  Until R32 that was written up as a limit of the
    generator's VOCABULARY.  It is not: three atoms derived from
    `plan_memo_emphasis.DELIMS` turn the defect red there, and what actually
    keeps them out is the ~+8 s they cost on an always-run wire.  Cost is a
    reason; "the property cannot reach this" was not one, and stating the wrong
    one is how a stopgap becomes the tenth per-shape control this module's own
    header argues against.  Carried in the plan's §8."""
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

    nest = {}
    for n in (100, 400):
        text = "*a " * n + "b* " * n
        runs = ([plan_memo_emphasis.run_at(text, 3 * k) for k in range(n)]
                + [plan_memo_emphasis.run_at(text, 3 * n + 3 * k + 1) for k in range(n)])
        try:
            with _count_lines(plan_memo_emphasis, limit=200 * n) as c:
                plan_memo_emphasis.process(runs)
        except _WorkExceeded:
            return False, ("%d nested pairs cost more than %d source lines: not linear -- every "
                           "match re-walked the delimiters it had already cleared" % (n, 200 * n))
        nest[n] = c.lines
    if nest[400] > 4 * nest[100] + 200:
        return False, ("%d / %d source lines for 100 / 400 NESTED pairs: %.1fx the work for 4x the "
                       "input" % (nest[100], nest[400], nest[400] / max(nest[100], 1)))
    return ok, ("%d / %d source lines for 100 / 400 unmatched runs, %d / %d for 100 / 400 nested "
                "pairs (both <= 4x the work for 4x the input)" % (out[100], out[400], nest[100], nest[400]))


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
# The stops for those two: the correct scan spends three source lines and no
# walk at all on a run with no id after it, whatever its length.
_SCAN_RUN_LINES, _SCAN_RUN_WALKS = 1000, 8


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

    # The two ceilings are STOPS, not the claim -- every claim below is an
    # equality or an exact count, and a bound nobody tuned.  They are here so
    # that a run which is going to be red is red QUICKLY: the per-position
    # mutant this control is written against walks the whole run at every
    # position, which is 32 million traced line events over the long run and
    # 23 s of wall clock before the equality can even be compared (measured).
    # Both are two orders of magnitude above what the correct scan spends.
    def lines(text, limit):
        try:
            with _count_lines(ids, limit=limit) as c:
                list(ids.tokens(text))
        except _WorkExceeded:
            return None
        return c.lines

    def calls(text, limit):
        try:
            with _count_calls(ids, "_decor_start", limit=limit) as a, \
                    _count_calls(ids, "_decor_end", limit=limit) as b:
                found = len(list(ids.tokens(text)))
        except _WorkExceeded:
            return None, limit + 1, limit + 1
        return found, a.calls, b.calls

    run, long_run = "!" + "`" * _SCAN_RUN, "!" + "`" * _SCAN_RUN_LONG
    bad = []
    short_lines, long_lines = lines(run, _SCAN_RUN_LINES), lines(long_run, _SCAN_RUN_LINES)
    if short_lines is None or long_lines is None:
        bad.append("a run of marks with no id after it ran more than %d source lines: the scan is "
                   "entering the run" % _SCAN_RUN_LINES)
        short_lines = long_lines = _SCAN_RUN_LINES + 1
    _found, run_starts, run_ends = calls(long_run, _SCAN_RUN_WALKS)
    if short_lines != long_lines:
        bad.append("a run of %d marks costs %d source lines and one of %d costs %d: the scan is "
                   "entering the run" % (_SCAN_RUN, short_lines, _SCAN_RUN_LONG, long_lines))
    if run_starts or run_ends:
        bad.append("a run with no id after it walked the decoration %d + %d time(s)"
                   % (run_starts, run_ends))
    seen, widths = {}, []
    for d in _SCAN_WIDTHS:
        text = ("`" * d + "9z" + "`" * d) * _SCAN_IDS
        found, starts, ends = calls(text, _SCAN_IDS + 1)
        widths.append((d, found, starts, ends))
        if (found, starts, ends) != (_SCAN_IDS, _SCAN_IDS, _SCAN_IDS):
            bad.append("width %d: %s id(s), %d left and %d right walk(s) (each must be %d)"
                       % (d, found, starts, ends, _SCAN_IDS))
        seen[d] = lines(text, 100 * len(text))
        if seen[d] is None:
            bad.append("width %d ran more than %d source lines over %d characters"
                       % (d, 100 * len(text), len(text)))
            seen[d] = 100 * len(text) + d
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




def registry():
    """name -> (kind, control), the WORK fragment of the one table: this
    module's per-shape witnesses merged with the growth module's generated
    sweep."""
    reg = dict(growth_registry())
    reg.update(document_registry())
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
        "split_row is linear: <= 64 source lines per character and per cell (breaks partitioned in the one scan)":
            ("CONTROL", scaling_split_row_control),
        "the id scan walks a decoration run ONCE per id and never at all where no id follows it (an exact count, not a ratio -- the half of R29-2 a Python witness can see)":
            ("CONTROL", linear_id_scan_control),
        "a resolved image's demotion is linear: N nested images demote their descendants once, not once per enclosing image":
            ("CONTROL", linear_image_demotion_control),
    })
    return reg


