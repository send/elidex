#!/usr/bin/env python3
"""The DOCUMENT work controls for `plan-memo-umbrella-check.py --self-test`:
every cost control whose probe is a MEMO rather than a string.

The seam against `plan_memo_selftest_work.py` is the SUBJECT, and it is
mechanical rather than a taste, in the same three ways the other seams of this
suite are stated.  A control THERE hands one function of the checker set a
string it builds in memory -- `inline_pass`, `process`, `file_and_cite_spans`,
`split_row`, the id scan -- and counts the work that one call does.  A control
HERE writes a memo to a scratch directory and costs the PIPELINE over it:
Phase 1 parsing it (`Memo`), or `check()` running end to end.  In the imports:
this module is the only importer of `tempfile` among the modules whose probes
are WRITTEN shapes, because writing a file is exactly what makes a probe a
document one.  ⚠ Not among all three work modules: `plan_memo_selftest_growth.
py` writes memos as well, and it is separated from both of these by its
POPULATION being generated rather than by how it probes -- two seams, two
questions, and reading them as one would put this module's next control in
whichever file a reader guessed.

WHY THE SPLIT (touch-time, PR #510 R31).  The work module reached 991 lines
when this round's two pipeline controls landed in it, and the seam above was
already there to be read off the imports -- five of its controls wrote a memo
and ten did not.  Splitting it is `CLAUDE.md`'s touch-time rule applied at the
point where a file is still bounded rather than at the point a reviewer counts
it; ⚠ the round's controls were written first and this carve is the commit
after them, which is the rule's shape late rather than its shape on time.

TWO PIPELINE SHAPES live here and the difference matters to what a probe can
witness.  `Memo(path)` runs PHASE 1 ALONE, which is what the three oldest
controls cost (orphan detection, the unresolved walk, the sibling set) -- no
disposition, no scan, no population.  `_on_pipeline` runs `check()`, the one
entry point, which is the only way to reach a scan that the seeds and the
licensing rule live in; both of PR #510 R31's cost findings were there, and
neither is reachable from a block-level probe at all.

`registry()` returns this module's fragment of the one name -> (kind, control)
table; `plan_memo_selftest_work.registry()` merges it exactly as it already
merges the growth module's.  A mutant row whose file is this module patches
the self-test, not the checker set (`plan_memo_selftest_mutants.SELFTEST`).

Import direction, one way: the work module imports this one, this one imports
the harness, and neither imports the controls.
"""

import pathlib
import tempfile

from plan_memo_selftest_harness import (
    _CountedList, _WorkExceeded, _count_calls, _count_lines, _count_pattern_spans,
)


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


# The memo every control below runs the WHOLE pipeline over: the four schemas,
# so the run is not a schema miss, and one declared umbrella id, so the scans
# these controls measure have something to find.  `%s` is the body each shapes
# for itself.  Written here rather than taken from `plan_memo_selftest_cases.
# build` on purpose: the cases module's builder is the INVARIANTS module's
# import (that is the seam between the two property modules), and a work
# control asks nothing about the verdict this memo produces.
_PIPELINE_MEMO = """# fixture

| ID | Citation | Anchor | Used by |
|---|---|---|---|
| [C1] | ECMA-262 §1 X | `#a` | — |

| Site | Syntax | Emits | Observable | Tier | Slice |
|---|---|---|---|---|---|
| `x.rs` | `a` | nothing | none | T1 | 9z |

| # | Slice | Primary module(s) | Slot | Tier | Deps |
|---|---|---|---|---|---|
| **9z** | **UMBRELLA, not a terminal unit.** charter. | `a.rs` | — | T1 | — |

| Slot | Why deferred | Trigger | Re-eval |
|---|---|---|---|
| `#11-zz-alpha` | **UMBRELLA, not a terminal unit.** why. | now | 2026-12-31 |

%s
"""


def _on_pipeline(M, body, witness):
    """Run `check()` over `_PIPELINE_MEMO` carrying `body`, inside `witness`
    (a harness work witness already constructed).  Returns the `Result`."""
    with tempfile.TemporaryDirectory() as d:
        p = pathlib.Path(d) / "fixture.md"
        p.write_text(_PIPELINE_MEMO % body, encoding="utf-8")
        with witness:
            return M.check(str(p))


def linear_raw_seed_control(M):
    """The linearity witness for the always-run raw-content seed: one raw line
    holding N declared-id tokens and N file names costs O(N), not O(N*N).

    THE DEFECT (PR #510 R31-3).  `lex_unsupported_seed` asked of every token
    whether it stood inside a file name or a citation, and asked it by summing
    the WHOLE span list -- `any(a < t.idend and t.idstart < b for a, b, _ in
    spans)` -- so N tokens tested N spans.  The reviewer measured 0.09 / 0.32 /
    1.28 / 4.59 s over `9z note.md` repeated 1,000 / 2,000 / 4,000 / 8,000
    times, for 11-88 KB of input.  `plan_memo_tokens.covers` is the reading
    now: the list is ordered by start and non-overlapping by construction, so
    one bisect finds the only span that can reach the token.

    THE SUBJECT IS THE CALLER, not `covers`, and that is why this runs the
    whole pipeline instead of calling the new function N times.  A probe that
    called `covers` directly would prove `covers` linear and leave the summing
    comprehension -- the thing the reviewer found, and the thing a later
    caller could write again -- entirely unwatched.  Measured: with the
    comprehension re-injected the ENTIRE suite stayed green before this
    control existed, the generated growth property included, since that
    property's corpus is a block-level text and never reaches the seed.

    Counted by `_count_lines` over the entry-point module, where both the
    comprehension and the `covers` call site live; `covers` itself is in
    another file and traces nothing here, which is the point -- the fixed path
    executes one line per token and the defect executes one per token per
    span.  Measured 1,658 / 3,458 source lines fixed (x2.09 for 4x the input)
    against 81,858 / 1,284,258 re-injected (x15.7).

    The LOWER bound is the seed itself: the run must report exactly one
    LEX-UNSUPPORTED? finding naming the declared id, or the scan measured
    above walked a keep-set that was empty and this control would be green
    over nothing."""
    def run(n, limit):
        c = _count_lines(M, limit=limit)
        res = _on_pipeline(M, "<div>\n" + ("9z note.md " * n) + "\n</div>", c)
        seeds = [f for f in res.findings if f[0] == "LEX-UNSUPPORTED?"]
        return c.lines, res.rc, seeds

    try:
        small, rc_s, seeds_s = run(200, 10 ** 7)
        big, rc_b, seeds_b = run(800, 4 * small + 2000)
    except _WorkExceeded:
        return False, ("a raw line of 800 id tokens and 800 file names cost more than 4x the 200 "
                       "line's work: the seed tests every span per token, not the one that can reach it")
    if rc_s or rc_b or len(seeds_s) != 1 or len(seeds_b) != 1:
        return False, ("the seed did not fire over the probe (rc %d / %d, %d / %d finding(s)): the "
                       "keep-set was empty and the scan above walked nothing"
                       % (rc_s, rc_b, len(seeds_s), len(seeds_b)))
    if "9z" not in seeds_b[0][3]:
        return False, "the seed fired without naming the declared id: %r" % (seeds_b[0][3][:80],)
    return True, ("%d / %d source lines of the entry point for 200 / 800 id-and-file-name pairs on one "
                  "raw line (<= 4x the work for 4x the input), one seed finding naming `9z` each time"
                  % (small, big))


def linear_licence_scan_control(M):
    """The linearity witness for the licensing rule's BACKWARD look: a
    paragraph of N mentions hands `LICENSE_BEFORE` O(N) characters in all, not
    O(N*N).

    THE DEFECT WAS MINE AND IT WAS THE COST HALF OF A CORRECTNESS FIX (PR #510
    R31-4).  R24 deleted a 40-character slice whose index 0 let the pattern's
    lookbehind succeed against nothing, and handed the pattern the block's
    whole preceding text instead (`search(m.text, 0, m.start)`).  That was
    right about the boundary -- "immediately before" is a fact of the grammar,
    not a width -- and wrong about the cost: every mention of the block now
    scanned from the block's start, so N mentions scanned O(N*N) characters.
    The reviewer measured 0.31 / 1.40 / 4.88 / 19.09 s over `9z unrelated
    words.` repeated 1,000 / 2,000 / 4,000 / 8,000 times.  `licence_starts` is
    the bound now, and it is the same KIND of fact as the deleted width was
    not: the offsets at which a phrase may begin, read once per block.

    THE MEASURE IS THE SPAN, BECAUSE NOTHING ELSE MOVES.  Both readings make
    one pattern application per mention, execute the same handful of source
    lines and touch no list a counter could watch; what grows is the text the
    C engine is handed, which is `_count_pattern_spans`' subject and the
    reason that witness exists.  Measured: with the unbounded search
    re-injected the ENTIRE suite stayed green before this control existed.

    TWO SHAPES, and the second is the one that keeps the cheapest wrong answer
    out.  (a) `9z unrelated words.` x N -- no licensing keyword anywhere, so
    the index is empty, the pattern is applied ZERO times, and the re-injected
    search hands it 398,000 then 6,392,000 characters (x16.06).  (b) `mintage
    9z words.` x N -- a keyword literal (`mint`) that begins no phrase, so the
    index offers one candidate per mention and the pattern IS applied, over 8
    characters each: 1,600 then 6,400, exactly 4x for 4x.  Shape (a) alone
    would be passed by an index that answered "nowhere" to everything, which
    licenses nothing and is the cheapest possible index; (b)'s LOWER bound --
    one application per mention -- is what reports that."""
    import plan_memo_roles       # the freshly loaded module

    out = []
    for label, unit, per in (("no keyword", "9z unrelated words. ", 0),
                             ("keyword that begins no phrase", "mintage 9z words. ", 40)):
        seen = {}
        for n in (200, 800):
            c = _count_pattern_spans(plan_memo_roles, "LICENSE_BEFORE", limit=per * n + 400)
            try:
                res = _on_pipeline(M, unit * n, c)
            except _WorkExceeded:
                return False, ("%d mentions with %s handed LICENSE_BEFORE more than %d characters: "
                               "the backward look is scanning from the block's start"
                               % (n, label, per * n + 400))
            if res.rc:
                return False, "the probe for %s is a schema miss (rc %d), not a run" % (label, res.rc)
            seen[n] = (c.chars, c.calls, len(res.mentions))
        if seen[800][2] < 800:
            return False, ("the %s probe produced %d mention(s) for 800 repetitions: the scan measured "
                           "above walked nothing" % (label, seen[800][2]))
        if seen[800][0] > 4 * seen[200][0] + 400:
            return False, ("%d / %d characters handed to LICENSE_BEFORE for 200 / 800 mentions with %s: "
                           "%.1fx the work for 4x the input"
                           % (seen[200][0], seen[800][0], label, seen[800][0] / max(seen[200][0], 1)))
        if per and seen[800][1] < 800:
            return False, ("the %s probe applied LICENSE_BEFORE %d time(s) over 800 mentions: an index "
                           "that offers no candidate licenses nothing and is the cheapest wrong answer"
                           % (label, seen[800][1]))
        out.append("%s %d / %d characters in %d / %d application(s)"
                   % (label, seen[200][0], seen[800][0], seen[200][1], seen[800][1]))
    return True, ("for 200 / 800 mentions, at most 4x the characters for 4x the input: %s"
                  % "; ".join(out))


def registry():
    """name -> (kind, control), the DOCUMENT fragment of the work table."""
    return {
        "Phase-1 orphan detection is linear: <= 4 link_label calls per line":
            ("CONTROL", linear_orphans_control),
        "unresolved_references is linear: <= N*(log2 N + 2) reads of the line-offset table (a bisect per site, not a scan)":
            ("CONTROL", scaling_unresolved_control),
        "linked_files is linear: <= N Path.__eq__ calls over N distinct siblings (a set dedup hashes, a list compares)":
            ("CONTROL", scaling_linked_files_control),
        "block quotes are linear: N quotes cost <= 4N quote_content calls":
            ("CONTROL", scaling_quotes_control),
        "the §5.1 marker test builds no content: quote_content is called once per line a quote's gather examines and never to answer whether a line carries a marker":
            ("CONTROL", quote_build_once_control),
        "the always-run raw-content seed is linear: N id tokens and N file names on one raw line are one pass, not a span sum per token":
            ("CONTROL", linear_raw_seed_control),
        "the licensing rule's backward look is linear: N mentions hand LICENSE_BEFORE O(N) characters in all, not one growing prefix each":
            ("CONTROL", linear_licence_scan_control),
    }
