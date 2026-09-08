#!/usr/bin/env python3
"""Harness for `plan-memo-umbrella-check.py --self-test`: the module loader,
the fixture runner and the work witnesses the controls are written against.

Nothing here decides what is right or wrong about the checker; that is the
controls' business (`plan_memo_selftest_controls.py`).  This module holds the
MECHANISM the controls share:

  * `load` / `unload` -- a FRESH checker module set exec'd from source text
    (optionally patched: the mutation proof's one lever), installed in
    `sys.modules` in dependency order; `SOURCES` records the text each module
    was exec'd from, so a control over the SOURCE reads the set a mutant
    patched;
  * `patched_module` -- a module of the SELF-TEST set itself exec'd from
    patched text into a fresh module, for a mutant against a control;
  * `run_on` -- write a fixture and its siblings to a scratch directory and
    run `check()`, the checker's ONE pipeline;
  * `measure` / `control` -- the one factory that turns a `Case` record into
    a control (exact comparison, never `>=`);
  * `_count_calls` / `_count_lines` / `_count_line_sites` / `_CountedList` --
    deterministic work witnesses for the linearity controls (a wall-clock
    bound flaked on contended hosts in both directions; a work count does
    not).

EVERY TEXT I/O CALL HERE NAMES ITS ENCODING (PR #510 R26-4).  The checker
advertises "Python 3.9+, standard library only" and reads production memos as
explicit UTF-8 (`plan_memo_memo.Memo.read`); the self-test read its own module
sources and wrote its fixtures with the LOCALE default instead, so on a host
whose preferred encoding is not UTF-8 the suite raised `UnicodeDecodeError` in
`load()` -- before a single control ran -- because these sources hold non-ASCII
text (`PYTHONUTF8=0 LC_ALL=C python3 plan-memo-umbrella-check.py --self-test`
reproduced it).  A proof that cannot start is worse than a red one: it is
indistinguishable from a broken interpreter.  The rule is a CLASS, not these
four call sites, and `plan_memo_selftest_properties.encoding_sweep_control`
enforces it over every module of this checker -- the checker set and the
self-test both -- so the next module's first `write_text` arrives already
covered.

Import direction, one way: the runner (`plan_memo_umbrella_selftest.py`)
imports the controls, the controls import this module, and this module
imports nothing of either.
"""

import importlib.util
import pathlib
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent

# Import name -> file, in dependency order.  The checker's file name is not an
# import name, so it is loaded under a fixed one.
MODULES = [
    ("plan_memo_ids", "plan_memo_ids.py"),
    ("plan_memo_emphasis", "plan_memo_emphasis.py"),
    ("plan_memo_tokens", "plan_memo_tokens.py"),
    ("plan_memo_html", "plan_memo_html.py"),
    ("plan_memo_lexer", "plan_memo_lexer.py"),
    ("plan_memo_blocks", "plan_memo_blocks.py"),
    ("plan_memo_tables", "plan_memo_tables.py"),
    ("plan_memo_sibling", "plan_memo_sibling.py"),
    ("plan_memo_memo", "plan_memo_memo.py"),
    ("plan_memo_population", "plan_memo_population.py"),
    ("plan_memo_roles", "plan_memo_roles.py"),
    ("plan_memo_umbrella_check", "plan-memo-umbrella-check.py"),
]

# The id grammar module: the ONE file that may spell an id character class;
# the spelling sweep (`id_spelling_sweep_control`) reads every other module
# of the set for a second spelling.
GRAMMAR = "plan_memo_ids.py"


_TEXT = {}      # file name -> the UNPATCHED source text (immutable)
_CODE = {}      # file name -> code object of the UNPATCHED source (immutable)
SOURCES = {}    # file name -> the source text the CURRENT module set was exec'd from


def load(patches=None):
    """A FRESH module set, exec'd from source text (`patches` = {file name:
    source} overrides), installed in `sys.modules` in dependency order so the
    checker's own imports resolve to the patched modules.  Returns the checker
    module.  `unload()` removes the set again.  Unpatched sources are compiled
    once; every call still execs into fresh module dicts.  `SOURCES` records
    the text each module of the set was exec'd from (patched or not), so a
    control over the SOURCE (the spelling sweep) reads the same set a mutant
    patched.

    `SOURCES` is CLEARED first, not merely overwritten: `patched_module` records
    a patched SELF-TEST module's text there too (PR #510 R26-4) and that file is
    not one of `MODULES`, so without the clear a previous mutant's patched
    self-test source would still be standing when the next row's sweep read
    it."""
    patches = patches or {}
    unload()
    SOURCES.clear()
    mod = None
    for name, file in MODULES:
        src = patches.get(file)
        if file not in _TEXT:
            _TEXT[file] = (HERE / file).read_text(encoding="utf-8")
        SOURCES[file] = src if src is not None else _TEXT[file]
        if src is not None:
            code = compile(src, str(HERE / file), "exec")
        elif file in _CODE:
            code = _CODE[file]
        else:
            code = _CODE[file] = compile(_TEXT[file], str(HERE / file), "exec")
        spec = importlib.util.spec_from_loader(name, loader=None, origin=str(HERE / file))
        mod = importlib.util.module_from_spec(spec)
        mod.__file__ = str(HERE / file)
        sys.modules[name] = mod
        exec(code, mod.__dict__)
    return mod


def unload():
    for name, _ in MODULES:
        sys.modules.pop(name, None)


def patched_module(file, src):
    """A module of the SELF-TEST set (`file`, e.g. the controls module),
    exec'd from patched source text into a fresh module that is NOT installed
    in `sys.modules`, so a mutant against a control -- or against the
    runner's emptiness guard, which lives beside its proof -- takes its
    controls from the PATCHED module's `registry()` while the checker set
    stays unpatched.  The patched module's own imports (this harness, the
    case registry) resolve to the installed, unpatched modules.

    The patched text is recorded in `SOURCES` (PR #510 R26-4) for the same
    reason `load` records the checker set's: a control that sweeps SOURCE TEXT
    must see what a mutant did, and a mutant against a self-test module is the
    only way to prove such a sweep covers the self-test half at all.  `load`
    clears `SOURCES`, so the entry lasts exactly one mutant row."""
    spec = importlib.util.spec_from_loader(pathlib.Path(file).stem + "_patched", loader=None,
                                           origin=str(HERE / file))
    mod = importlib.util.module_from_spec(spec)
    mod.__file__ = str(HERE / file)
    SOURCES[file] = src
    exec(compile(src, mod.__file__, "exec"), mod.__dict__)
    return mod


def run_on(M, text, prose="", sibling=None, files=None):
    """Write the fixture and its siblings to a scratch dir and run `check()`.
    Returns (Result, unlicensed mentions)."""
    with tempfile.TemporaryDirectory() as d:
        p = pathlib.Path(d) / "fixture.md"
        p.write_text(text + "\n" + prose + "\n", encoding="utf-8")
        # Every fixture link resolves to this file (an absent target is rc 2);
        # its name carries an id so the destination-masking control keeps its
        # subject.
        (pathlib.Path(d) / "slice-9z-sib.md").write_text((sibling or "") + "\n", encoding="utf-8")
        for name, content in (files or {}).items():
            f = pathlib.Path(d) / name
            f.parent.mkdir(parents=True, exist_ok=True)
            f.write_text(content, encoding="utf-8")
        res = M.check(str(p))
        return res, [m for m in res.mentions if not m.licensed]


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


class _WorkExceeded(Exception):
    pass


class _count_calls:
    """Count calls of `module.<name>` while the block runs (the callee is
    looked up through the module global at call time, so a re-injected
    re-parse is counted), and stop the block the moment `limit` is passed --
    a non-timing linearity witness.  A host-speed wall-clock cutoff turned
    the registered trip-wire red on contended runners (81-107 ms measured
    against a 50 ms bound); a work count does not depend on the host.
    `module` may be a class (`pathlib.PurePath`): a dunder set on it is
    reached through the type slot, so `list.__contains__` and
    `set.__contains__` count too.  `limit=None` counts without a stop (for
    a LOWER bound: a counter watching nothing is red)."""

    def __init__(self, module, name, limit):
        self.module, self.name, self.limit, self.calls = module, name, limit, 0

    def __enter__(self):
        orig = getattr(self.module, self.name)

        def counted(*a, **kw):
            self.calls += 1
            if self.limit is not None and self.calls > self.limit:
                raise _WorkExceeded()
            return orig(*a, **kw)
        self._orig = orig
        setattr(self.module, self.name, counted)
        return self

    def __exit__(self, *exc):
        setattr(self.module, self.name, self._orig)
        return False


class _count_lines:
    """Count the source lines executed in ONE module's frames while the block
    runs (`sys.settrace` on this thread; a frame of any other file is not
    traced), and stop the block the moment `limit` is passed.  The work
    witness where the work passes through no module binding `_count_calls`
    could watch -- a comprehension or a `while` re-doing work inside one
    function.  Every loop iteration is a line event (a backward jump reports
    its line again -- measured on CPython 3.9 and 3.14: a 200 x 200 nested
    comprehension is 80,403 / 80,603 lines), so a re-scan shows as lines,
    deterministically, on any host."""

    def __init__(self, module, limit):
        self.file, self.limit, self.lines = module.__file__, limit, 0

    def __enter__(self):
        self._prev = sys.gettrace()

        def tracer(frame, event, arg):
            if frame.f_code.co_filename != self.file:
                return None
            if event == "line":
                self.lines += 1
                if self.lines > self.limit:
                    raise _WorkExceeded()
            return tracer
        sys.settrace(tracer)
        return self

    def __exit__(self, *exc):
        sys.settrace(self._prev)
        return False


class _count_line_sites:
    """Count the executions of EACH source line of `modules` separately --
    {(file, lineno): executions} in `counts` -- rather than their total.

    `_count_lines` answers "how much work in all", which is the right witness
    for ONE shape whose cost a control already understands.  It is the wrong
    one for a SWEEP: a total is dominated by the linear pass over the text
    (measured over the generated corpus: ~48 source lines per character), so a
    re-scan whose inner loop is two lines is invisible inside it until the
    input is thousands of characters long.  Per SITE it is not: a line the
    scan runs once per character doubles when the input doubles, and a line
    inside a re-scan quadruples, whatever the constants around it are.  That
    is what `generated_growth_control` compares, and it is why the sweep can
    run at 6 and 12 repetitions instead of hundreds.

    No limit and no early stop: both runs are wanted in full, since the claim
    is about the RATIO of two tallies rather than about either one."""

    def __init__(self, modules):
        self.files = {m.__file__ for m in modules}
        self.counts = {}

    def __enter__(self):
        self._prev = sys.gettrace()
        counts, files = self.counts, self.files

        def tracer(frame, event, arg):
            if frame.f_code.co_filename not in files:
                return None
            if event == "line":
                key = (frame.f_code.co_filename, frame.f_lineno)
                counts[key] = counts.get(key, 0) + 1
            return tracer
        sys.settrace(tracer)
        return self

    def __exit__(self, *exc):
        sys.settrace(self._prev)
        return False


class _CountedList(list):
    """A list that counts its reads (`__getitem__` / `__len__`) and stops
    past `limit`: the work witness for a table a production method searches.
    `bisect` consults a list SUBCLASS through these (its exact-list fast
    path does not apply), so a binary search shows ceil(log2(N+1)) reads per
    lookup and a linear scan ~N/2."""

    __slots__ = ("reads", "limit")

    def __init__(self, items, limit):
        super().__init__(items)
        self.reads, self.limit = 0, limit

    def _read(self):
        self.reads += 1
        if self.reads > self.limit:
            raise _WorkExceeded()

    def __getitem__(self, i):
        self._read()
        return list.__getitem__(self, i)

    def __len__(self):
        self._read()
        return list.__len__(self)
