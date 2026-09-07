#!/usr/bin/env python3
"""Firing proofs and controls for `plan-memo-umbrella-check.py`.

A checker that has never been shown to fire is a checker that reports `0`
for a class it cannot see.  Every check in the companion has at least one
control here, and the controls come in four kinds:

  POSITIVE          a wrong site the checker MUST report.
  POSITIVE-NOVEL    a wrong site written in a spelling that appears NOWHERE in
                    the memo and nowhere in the licensing tables.  This is the
                    control that decides whether the predicate is a rule or a
                    transcription of the sites it was written against.  If a
                    novel wrong spelling passes, the checker is a grep for
                    yesterday's mistakes.
  NEGATIVE          a licensed site the checker must NOT report.
  KNOWN-MISS        a wrong site the checker is KNOWN not to report.  These are
                    RED and stay red.  They are printed with the run so a
                    reader never reads the finding count as a coverage
                    statement.  Turning one green by widening the claim -- for
                    instance by redefining the population so the missed site is
                    out of scope -- is the failure
                    `feedback_control-rewritten-to-bless-the-defect` names.

Every control runs `check()` -- the SAME pipeline `main()` runs, not a copy of
it.  The registry lives in `plan_memo_selftest_cases.py` (builder + pre-converge
controls) and `plan_memo_selftest_cases_pr510.py` (the PR #510 review-round
controls, appended to the same `CASES`); the mutants (a re-executable proof
that each control can go red) in `plan_memo_selftest_mutants.py`.

Run:  python3 .claude/tools/plan-memo-umbrella-check.py --self-test [--mutants]
"""

import importlib.util
import pathlib
import sys
import tempfile

from plan_memo_selftest_cases import CASES, build
import plan_memo_selftest_cases_pr510  # noqa: F401 -- appends the review-round controls to CASES

HERE = pathlib.Path(__file__).resolve().parent

# Import name -> file, in dependency order.  The checker's file name is not an
# import name, so it is loaded under a fixed one.
MODULES = [
    ("plan_memo_lexer", "plan_memo_lexer.py"),
    ("plan_memo_blocks", "plan_memo_blocks.py"),
    ("plan_memo_tables", "plan_memo_tables.py"),
    ("plan_memo_memo", "plan_memo_memo.py"),
    ("plan_memo_roles", "plan_memo_roles.py"),
    ("plan_memo_umbrella_check", "plan-memo-umbrella-check.py"),
]


_CODE = {}      # file name -> code object of the UNPATCHED source (immutable)


def load(patches=None):
    """A FRESH module set, exec'd from source text (`patches` = {file name:
    source} overrides), installed in `sys.modules` in dependency order so the
    checker's own imports resolve to the patched modules.  Returns the checker
    module.  `unload()` removes the set again.  Unpatched sources are compiled
    once; every call still execs into fresh module dicts."""
    patches = patches or {}
    unload()
    mod = None
    for name, file in MODULES:
        src = patches.get(file)
        if src is not None:
            code = compile(src, str(HERE / file), "exec")
        elif file in _CODE:
            code = _CODE[file]
        else:
            code = _CODE[file] = compile((HERE / file).read_text(), str(HERE / file), "exec")
        spec = importlib.util.spec_from_loader(name, loader=None, origin=str(HERE / file))
        mod = importlib.util.module_from_spec(spec)
        mod.__file__ = str(HERE / file)
        sys.modules[name] = mod
        exec(code, mod.__dict__)
    return mod


def unload():
    for name, _ in MODULES:
        sys.modules.pop(name, None)


def patched_runner(src):
    """This runner itself, exec'd from patched source text into a fresh
    module (not installed in `sys.modules`), so a mutant against the RUNNER's
    own guards can take its control from the patched registry."""
    spec = importlib.util.spec_from_loader("plan_memo_umbrella_selftest_patched", loader=None,
                                           origin=str(HERE / "plan_memo_umbrella_selftest.py"))
    mod = importlib.util.module_from_spec(spec)
    mod.__file__ = str(HERE / "plan_memo_umbrella_selftest.py")
    exec(compile(src, mod.__file__, "exec"), mod.__dict__)
    return mod


def run_on(M, text, prose="", sibling=None, files=None):
    """Write the fixture and its siblings to a scratch dir and run `check()`.
    Returns (Result, unlicensed mentions)."""
    with tempfile.TemporaryDirectory() as d:
        p = pathlib.Path(d) / "fixture.md"
        p.write_text(text + "\n" + prose + "\n")
        # Every fixture link resolves to this file (an absent target is rc 2);
        # its name carries an id so the destination-masking control keeps its
        # subject.
        (pathlib.Path(d) / "slice-9z-sib.md").write_text((sibling or "") + "\n")
        for name, content in (files or {}).items():
            f = pathlib.Path(d) / name
            f.parent.mkdir(parents=True, exist_ok=True)
            f.write_text(content)
        res = M.check(str(p))
        return res, [m for m in res.mentions if not m.licensed]


# ----------------------------------------------------------------------------
# Each control is a function of the checker module returning (ok, detail).
# The mutant runner re-runs controls BY NAME against a patched module set, so
# the registry is name -> callable and every control is reachable that way.
# ----------------------------------------------------------------------------


def measure(res, reported, m):
    """The value a `Case.measure` takes on one run -> (got, detail)."""
    if m == "sites":
        return len(reported), "reported :: %s" % [x.context()[:80] for x in reported]
    if m == "rc":
        return res.rc, "rc :: %s" % [f[0] + " " + f[3][:60] for f in res.mechanical][:3]
    what, arg = m
    if what == "finding":
        return sum(1 for f in res.findings if f[0] == arg), arg
    if what == "note":
        return sum(1 for n in res.notes if arg in n), "note %r" % arg
    if what == "schema":
        # the one measure that reads a schema-miss run: SCHEMA findings carrying `arg`
        return (sum(1 for f in res.findings if f[0] == "SCHEMA" and arg in f[3]),
                "SCHEMA %r (rc must be 2 iff any)" % arg)
    if what == "id":
        return int(arg in res.population.ids), "id %r declared" % arg
    if what == "seed":
        # the one measure that reads a seed's READING: LEX-UNSUPPORTED? findings carrying `arg`
        return (sum(1 for f in res.findings if f[0] == "LEX-UNSUPPORTED?" and arg in f[3]),
                "LEX-UNSUPPORTED? carrying %r" % arg)
    raise ValueError(m)


def control(c):
    """The one factory: run the fixture, take the measure, compare EXACTLY.
    Exact, not `>=`: every fixture carries exactly one intended site, so a
    scanner that reports one site twice must turn a control red rather than
    inflate the production census behind a green self-test.  Every measure
    but `rc` and `schema` also requires the run not to be a schema miss;
    `schema` requires rc 2 exactly when it counts one."""
    def run(M):
        res, reported = run_on(M, c.text, c.prose, c.sibling, c.files)
        got, detail = measure(res, reported, c.measure)
        if c.measure == "rc":
            ok = got == c.expect
        elif c.measure[0] == "schema":
            ok = got == c.expect and (res.rc == 2) == (got > 0)
        else:
            ok = got == c.expect and res.rc != 2
        return ok, "%s = %d (expected %d), rc %d" % (detail, got, c.expect, res.rc)
    return run


def attribution_control(M):
    """A pointer slot whose cell opens `Slice **9z** -- **UMBRELLA, ...**` is
    declaring 9z's kind, not its own.  §5: a pointer slot "carries no marker of
    its own".  The count must not move when such a row is added."""
    base = build()
    ptr = build(wb="**(carved at PR-B)** Slice **9z** — **UMBRELLA, not a terminal unit** — points into §5.")
    n = [len(run_on(M, t)[0].population.no_owner_ids()) for t in (base, ptr)]
    return n[0] == n[1], "a marker naming another row does not enter the count (%d -> %d)" % tuple(n)


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


class _WorkExceeded(Exception):
    pass


class _count_calls:
    """Count calls of `module.<name>` while the block runs (the callee is
    looked up through the module global at call time, so a re-injected
    re-parse is counted), and stop the block the moment `limit` is passed --
    a non-timing linearity witness.  A host-speed wall-clock cutoff turned
    the registered trip-wire red on contended runners (81-107 ms measured
    against a 50 ms bound); a work count does not depend on the host."""

    def __init__(self, module, name, limit):
        self.module, self.name, self.limit, self.calls = module, name, limit, 0

    def __enter__(self):
        orig = getattr(self.module, self.name)

        def counted(*a, **kw):
            self.calls += 1
            if self.calls > self.limit:
                raise _WorkExceeded()
            return orig(*a, **kw)
        self._orig = orig
        setattr(self.module, self.name, counted)
        return self

    def __exit__(self, *exc):
        setattr(self.module, self.name, self._orig)
        return False


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
    block at 4 calls per line; AND the time scales -- t(4N)/t(N) < 8 over
    N = 1000 / 4N = 4000, min of 3 runs (linear ~4, quadratic ~16), which is
    what catches a per-line re-join of the rest of the run, a cost no call
    count sees.  The per-line re-walk this replaced parsed every remaining
    definition again per line (~4.5 million `link_label` calls, 7.95 s)."""
    import time
    import plan_memo_blocks     # the freshly loaded module
    import plan_memo_memo

    def best(n):
        text = "text\n" + "".join("[l%d]: f%d.md\n" % (i, i) for i in range(n - 1))
        with tempfile.TemporaryDirectory() as d:
            p = pathlib.Path(d) / "orphans.md"
            p.write_text(text)
            t, calls, orphans = [], 0, 0
            for _ in range(3):
                t0 = time.perf_counter()
                with _count_calls(plan_memo_blocks, "link_label", limit=4 * n) as c:
                    memo = plan_memo_memo.Memo(p)
                t.append(time.perf_counter() - t0)
                calls = c.calls
                orphans = sum(len(v) for v in memo.orphans.values())
        return orphans, calls, min(t)

    try:
        o1, c1, t1 = best(1000)
        o4, c4, t4 = best(4000)
    except _WorkExceeded:
        return False, "Memo exceeded 4 link_label calls per line: not linear"
    ratio = t4 / t1 if t1 else float("inf")
    ok = o1 == 999 and o4 == 3999 and c1 == 1000 and c4 == 4000 and ratio < 8
    return ok, ("%d/%d orphans, %d/%d Phase-1 link_label calls (must be exactly 1000/4000), "
                "t(1000)=%.1f ms t(4000)=%.1f ms ratio %.1f (< 8)" % (o1, o4, c1, c4, t1 * 1000, t4 * 1000, ratio))


def scaling_unresolved_control(M):
    """The scaling witness for the unresolved-reference walk: N and 4N
    reference lines in the same process, min of 3 runs each, and the ratio
    t(4N)/t(N) must stay below 8 (linear gives ~4, quadratic ~16).  A ratio
    is host-independent where an absolute cutoff is not.  The quadratic
    terms this guards: `Paragraph.locate` (a linear scan per site) and the
    `site not in out` membership test (now a set)."""
    import time
    import plan_memo_memo

    def best(n):
        text = "".join("[x][missing]\n" for _ in range(n))
        with tempfile.TemporaryDirectory() as d:
            p = pathlib.Path(d) / "u.md"
            p.write_text(text)
            memo = plan_memo_memo.Memo(p)
            t = []
            for _ in range(3):
                t0 = time.perf_counter()
                sites = len(memo.unresolved_references())
                t.append(time.perf_counter() - t0)
        return sites, min(t)

    n1, t1 = best(1000)
    n4, t4 = best(4000)
    ratio = t4 / t1 if t1 else float("inf")
    return n1 == 1000 and n4 == 4000 and ratio < 8, (
        "t(1000)=%.2f ms, t(4000)=%.2f ms, ratio %.1f (must be < 8; linear ~4, quadratic ~16)"
        % (t1 * 1000, t4 * 1000, ratio))


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
    """`linked_files` over N and 4N links, min of 3, t(4N)/t(N) < 8 (linear
    ~4, quadratic ~16): the dedup is a set, not a list membership test."""
    import time
    import plan_memo_memo

    def best(n):
        # N DISTINCT siblings (none need exist: `linked_files` names, the
        # population checks), so the dedup structure actually grows
        text = "".join("See [x%d](slice-9z-sib-%d.md).\n" % (i, i) for i in range(n))
        with tempfile.TemporaryDirectory() as d:
            p = pathlib.Path(d) / "links.md"
            p.write_text(text)
            memo = plan_memo_memo.Memo(p)
            t = []
            for _ in range(3):
                t0 = time.perf_counter()
                k = len(memo.linked_files())
                t.append(time.perf_counter() - t0)
        return k, min(t)

    k1, t1 = best(1000)
    k4, t4 = best(4000)
    ratio = t4 / t1 if t1 else float("inf")
    return k1 == 1000 and k4 == 4000 and ratio < 8, "t(1000)=%.2f ms, t(4000)=%.2f ms, ratio %.1f (must be < 8)" % (
        t1 * 1000, t4 * 1000, ratio)


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
    """`split_row` over N and 4N escaped-pipe cells, min of 3, t(4N)/t(N) < 8
    (linear ~4, quadratic ~16): the break offsets are partitioned among the
    cells in the one row scan, not filtered per cell from a row-wide list."""
    import time
    import plan_memo_blocks

    def best(n):
        line = "|" + " a\\|b |" * n
        t = []
        for _ in range(3):
            t0 = time.perf_counter()
            k = len(plan_memo_blocks.split_row(line))
            t.append(time.perf_counter() - t0)
        return k, min(t)

    k1, t1 = best(500)
    k4, t4 = best(2000)
    ratio = t4 / t1 if t1 else float("inf")
    return k1 == 500 and k4 == 2000 and ratio < 8, "t(500)=%.2f ms, t(2000)=%.2f ms, ratio %.1f (must be < 8)" % (
        t1 * 1000, t4 * 1000, ratio)


def spec_examples_control(M):
    """The CommonMark 0.31.2 spec's own block examples through Phase 1
    (`plan_memo_selftest_conformance`): every vendored example aligned with
    its expected html or excluded by a stated §3.0 disposition; the multi-
    line detail is printed whole because the exclusion list IS the report."""
    import plan_memo_blocks     # the freshly loaded modules
    import plan_memo_memo
    import plan_memo_selftest_conformance as conf
    ok, detail = conf.run(plan_memo_blocks, plan_memo_memo)
    print("       " + detail.replace("\n", "\n       "))
    return ok, detail.split("\n")[0]


def sequence_control(M):
    """Phase 1's block SEQUENCE (`Memo.sequence`) over the §4.4 chunk and
    §5.1 container shapes the vendored spec examples do not reach -- which
    block a line lands in, a fact no naming site can discriminate -- and,
    for the raw shapes, the CONTENT of the raw extent (`Memo.raw`: the
    lines after the marker, re-spelt in line columns), since the sequence
    alone never read it: `c0 = col` in `quote_content` survived 285
    controls with `>\\t\\tfoo` holding seven spaces (design re-gate 3,
    MIN-5).  Every expected sequence was read off commonmark.js 0.31.2
    (`node cm.js`) before being written, except the table shapes (no
    tables there), which are cmark-gfm's (measured, `gh api -X POST
    /markdown -f mode=gfm`): a lazy delimiter row or body row is paragraph
    text and a table is not a paragraph, while a lazy HEADER row is the
    header when the delimiter carries the marker.  A crash on a shape is
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
        # the marker and §2.2 tab stops (Example 6 exactly, and its neighbours)
        (">\t\tfoo", [["quote", 1, 1], ["indented", 1, 1]], ["      foo"]),
        (">\t\ta", [["quote", 1, 1], ["indented", 1, 1]], ["      a"]),
        (">\ta", [["quote", 1, 1], ["p", 1, 1]]),
        (">  \ta", [["quote", 1, 1], ["p", 1, 1]]),
        ("    > a", [["indented", 1, 1]], ["    > a"]),
        # the §4.4 chunk: blank lines inside stay, trailing ones do not
        ("    a\n\n    b\n\nc", [["indented", 1, 3], ["p", 5, 5]], ["    a", "", "    b"]),
        ("    a\n  \n    b", [["indented", 1, 3]], ["    a", "  ", "    b"]),
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


def registry():
    """name -> (kind, control)."""
    reg = {}
    for c in CASES:
        assert c.name not in reg, "duplicate control name %r" % c.name
        reg[c.name] = (c.kind, control(c))
    reg["CommonMark 0.31.2 spec examples (Tabs, §4.1-§4.9, §5.1): Phase 1's block sequence aligns with the html"] = ("CONTROL", spec_examples_control)
    reg["Phase 1's block sequence over the §4.4 chunk and §5.1 container shapes matches commonmark.js"] = ("CONTROL", sequence_control)
    reg["block quotes are linear: N quotes cost <= 4N quote_content calls"] = ("CONTROL", scaling_quotes_control)
    reg["a marker naming another row does not enter the count"] = ("CONTROL", attribution_control)
    reg["declaring-field parse and whole-line marker grep differ"] = ("CONTROL", degenerate_control)
    reg["a table with and without edge pipes reads the same"] = ("CONTROL", pipe_shape_control)
    reg["a site after an escaped pipe is reported at its raw column"] = ("CONTROL", raw_offset_control)
    reg["an empty control or mutant registry is a FAIL, never green"] = ("CONTROL", empty_registry_control)
    reg["links() is linear: 30 nested brackets are one inline_pass call"] = ("CONTROL", linear_links_control)
    reg["Phase-1 orphan detection is linear: <= 4 link_label calls per line, t(4N)/t(N) < 8"] = ("CONTROL", linear_orphans_control)
    reg["unresolved_references scales linearly: t(4N)/t(N) < 8"] = ("CONTROL", scaling_unresolved_control)
    reg["a decoded destination with a C0 control character is rejected, never resolved"] = ("CONTROL", control_char_destination_control)
    reg["an OSError from resolve() is the unavailable-sibling schema miss, never an exception"] = ("CONTROL", unavailable_sibling_control)
    reg["linked_files scales linearly: t(4N)/t(N) < 8 (set dedup)"] = ("CONTROL", scaling_linked_files_control)
    reg["an undecodable sibling is the unavailable-linked-memo schema miss, never an exception"] = ("CONTROL", undecodable_sibling_control)
    reg["split_row scales linearly: t(4N)/t(N) < 8 (breaks partitioned in the scan)"] = ("CONTROL", scaling_split_row_control)
    return reg


def run(mutants=False):
    fails = []
    counts = {}
    print("=" * 74)
    print("plan-memo-umbrella-check  --  self-test")
    print("=" * 74)
    M = load()
    reg = registry()
    for name, (kind, control) in reg.items():
        counts[kind] = counts.get(kind, 0) + 1
        ok, detail = control(M)
        if kind == "KNOWN-MISS":
            # `ok` means "reported 0": the site IS wrong, so the control stays red
            print("  RED  [KNOWN-MISS] %s -- %s (this site IS wrong)" % (name, detail))
            if not ok:
                fails.append("KNOWN-MISS %s now reports; update the declared miss class" % name)
            continue
        if not ok:
            fails.append("%s %s :: %s" % (kind, name, detail))
        print("  %-4s [%s] %s (%s)" % ("ok" if ok else "FAIL", kind, name, detail[:90]))
    unload()

    print()
    print("%d control(s): %s."
          % (len(reg), ", ".join("%d %s" % (counts[k], k) for k in sorted(counts))))
    n_mutants = None
    if mutants:
        import plan_memo_selftest_mutants as mm
        fails += mm.run(reg)
        n_mutants = len(mm.MUTANTS)
    fails += empty_registry_fails(len(reg), n_mutants)
    if fails:
        print()
        for f in fails:
            print("FAIL: %s" % f)
        return 1
    print("all controls behaved as declared.")
    return 0
