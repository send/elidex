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
  * `_count_calls` / `_count_lines` / `_count_line_sites` / `_CountedList` /
    `_count_pattern_spans` -- deterministic work witnesses for the linearity
    controls (a wall-clock bound flaked on contended hosts in both directions;
    a work count does not).

EVERY TEXT I/O CALL HERE NAMES ITS ENCODING (PR #510 R26-4).  The checker
advertises "Python 3.9+, standard library only" and reads production memos as
explicit UTF-8 (`plan_memo_memo.Memo.__init__`); the self-test read its own module
sources and wrote its fixtures with the LOCALE default instead, so on a host
whose preferred encoding is not UTF-8 the suite raised `UnicodeDecodeError` in
`load()` -- before a single control ran -- because these sources hold non-ASCII
text (`PYTHONUTF8=0 LC_ALL=C python3 plan-memo-umbrella-check.py --self-test`
reproduced it).  A proof that cannot start is worse than a red one: it is
indistinguishable from a broken interpreter.  The rule is a CLASS, not the
call sites that were first fixed, and `plan_memo_selftest_properties.encoding_sweep_control`
enforces it over every module of this checker -- the checker set and the
self-test both -- so the next module's first `write_text` arrives already
covered.

Import direction, one way: the runner (`plan_memo_umbrella_selftest.py`)
imports the controls, the controls import this module, and this module
imports nothing of either.

WORKFLOW RULE (the golden manifest): adding, removing or changing a mutation row, a control or a case -- INCLUDING editing a
control's body, its docstring or a comment inside it, and a row's find/replace, all of
which are digested -- requires regenerating the golden manifest
(`python3 .claude/tools/plan-memo-umbrella-check.py --write-manifest`) and committing its
diff.  ONE HOME: `plan_memo_selftest_manifest`'s docstring; every other mention points there.
"""

import importlib.util
import pathlib
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent

# -- THE MODULE POPULATION: one derivation, read by every reader ------------
#
# ⚠ IT WAS SPELLED IN THREE PLACES (the fifth attestation, over `79309486`):
# a hand-written `MODULES` list here that nothing compared with the disk, the
# runner's list of mutant modules to import, and a glob inside the loop
# ratchet.  The runner's list was INERT -- removing two of its imports still
# gave "430 mutants, 0 survived", because the ratchet's glob had filled the
# registry as a side effect of a control running earlier -- and the hand list
# left a new checker module outside both the load set and the kind-question
# ratchet (a `plan_memo_extra.py` calling `pop._claims` was reported by
# nothing).  So the population is DERIVED here, once, from the directory, and
# every reader consumes it: `files()` is the set, `is_selftest` the self-test
# partition (a FILE-NAME rule), `registry_modules` the row-registry partition
# (by CONTENT: a module holding its own list), `MODULES` the checker half in
# import order.
# The second and third spellings are deleted, not cross-checked.

# THE THREAT MODEL of the population and the registries has one home: the
# docstring of `plan_memo_selftest_registry`.

ENTRY = "plan-memo-umbrella-check.py"
ENTRY_NAME = "plan_memo_umbrella_check"


GLOB = "plan_memo*.py"
"""THE glob, spelled once: every docstring that describes the population
points here rather than restating it (⚠ it was narrowed to `plan_memo_*.py` at
`b325c668`, and `plan_memo.py` / `plan_memoize.py` left the population in
silence -- the sixth attestation)."""


def files(here=None):
    """THE POPULATION: every `GLOB` file beside this one, plus the entry point
    (whose file name is not an import name) -- so a module a touch-time split
    carves out is in it the day it lands."""
    here = HERE if here is None else here
    return sorted(p.name for p in here.glob(GLOB)) + [ENTRY]


def is_selftest(file):
    """The partition, by file name: a self-test module says so in its name."""
    return "selftest" in file


_ASSIGNS = {}


def _assigns(here, file, name):
    """Does `file` bind `name` at module level?  The registry partition is
    decided by CONTENT, not by file name.  Answered once per (file, size,
    mtime, name): the collection asks it for every file on every collect."""
    import ast
    path = here / file
    try:
        stat = path.stat()
        key = (str(path), stat.st_size, stat.st_mtime, name)
    except OSError:
        key = None
    if key is not None and key in _ASSIGNS:
        return _ASSIGNS[key]
    tree = ast.parse(path.read_text(encoding="utf-8"), filename=file)
    for node in tree.body:
        targets = (node.targets if isinstance(node, ast.Assign)
                   else [node.target] if isinstance(node, ast.AnnAssign) else [])
        if any(isinstance(t, ast.Name) and t.id == name for t in targets):
            if key is not None:
                _ASSIGNS[key] = True
            return True
    if key is not None:
        _ASSIGNS[key] = False
    return False


def registry_modules(name, here=None):
    """The import names of every self-test module that holds its OWN `name`
    list (`"MUTANTS"` / `"CASES"`), read off each module's top level.

    ⚠ BY CONTENT, NOT BY FILE NAME (the sixth attestation): the first rule was
    a name prefix, and renaming a mutants module out of it dropped 38 rows at
    rc 0 -- a row can leave a registry only by its module ceasing to hold the
    list, and that is what this reads.  `registry_membership_control` holds the
    other direction (a file outside the population that the population imports
    or that imports it)."""
    here = HERE if here is None else here
    return [import_name(f) for f in files(here) if is_selftest(f) and _assigns(here, f, name)]


def import_name(file):
    return ENTRY_NAME if file == ENTRY else file[:-len(".py")]


def checker_files(here=None):
    """The CHECKER half of the population: every file that is not self-test."""
    return [f for f in files(here) if not is_selftest(f)]


def _import_order(here=None):
    """(import name, file) for the checker half, in DEPENDENCY order derived
    from each module's top-level imports (Kahn, ties by name).  The order was
    part of the hand list ("this list is exec'd in order, so a module that
    arrives after its importer is a NameError"); it is now a property of the
    import graph, and a cycle is an error rather than a silent misorder."""
    import ast
    here = HERE if here is None else here
    by_name = {import_name(f): f for f in checker_files(here)}
    deps = {}
    for name, file in by_name.items():
        tree = ast.parse((here / file).read_text(encoding="utf-8"), filename=file)
        got = set()
        for node in tree.body:
            if isinstance(node, ast.ImportFrom) and node.module in by_name:
                got.add(node.module)
            elif isinstance(node, ast.Import):
                got |= {a.name for a in node.names if a.name in by_name}
        deps[name] = got - {name}
    order, done = [], set()
    while len(order) < len(by_name):
        ready = [n for n in sorted(by_name) if n not in done and deps[n] <= done]
        if not ready:
            raise RuntimeError("the checker modules' imports form a cycle: %s"
                               % sorted(set(by_name) - done))
        order.append(ready[0])
        done.add(ready[0])
    return [(n, by_name[n]) for n in order]


MODULES = _import_order()


class Registry(dict):
    """A control table that REFUSES a name it already holds -- by item and by
    `update` alike -- so one fragment's control cannot silently shadow
    another's (the sixth-pass attestation's N-1: a duplicated key left 747
    controls and rc 0, the shadowed one never run).  `merge` builds one.  It
    is the generation-time "duplicate control name" validation of the golden
    manifest (`plan_memo_selftest_manifest`); a merge that bypasses it is a
    removed or changed manifest line, whatever its spelling."""

    def __setitem__(self, key, value):
        if key in self:
            raise KeyError("duplicate control name %r" % (key,))
        dict.__setitem__(self, key, value)

    def update(self, *args, **kw):
        for key, value in dict(*args, **kw).items():
            self[key] = value


def merge(*fragments):
    """ONE table from registry fragments, refusing a duplicated name."""
    reg = Registry()
    for fragment in fragments:
        reg.update(fragment)
    return reg

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
    return sys.modules[ENTRY_NAME]


_INSTALLED_LEAVES = []
"""Self-test LEAF modules a mutant row installed under their real name; see
`patched_module`.  Cleared by `unload()`, so an install lasts one row."""


def unload():
    for name, _ in MODULES:
        sys.modules.pop(name, None)
    while _INSTALLED_LEAVES:
        # RESTORE, never merely delete: one of the leaf installs in a full run
        # replaces a module that was already there, and `load()` calls
        # `unload()`, so a nested load inside a control would otherwise EVICT
        # the patch mid-row and silently turn that mutant into a no-op.
        name, prev = _INSTALLED_LEAVES.pop()
        if prev is None:
            sys.modules.pop(name, None)
        else:
            sys.modules[name] = prev


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
    # ⚠ A LEAF of the self-test set owns no controls: it is imported BY NAME,
    # at call time, from the module that does (PR #510 R45 --
    # `plan_memo_selftest_conformance` holds the spec falsifier, and every
    # control over it lives in `plan_memo_selftest_controls`).  Merging a
    # `registry()` it does not have is an AttributeError, and leaving it
    # uninstalled makes the mutant a no-op -- the silent direction.  So a leaf
    # is installed under its REAL name for the row and `unload()` drops it,
    # which is the same one-row lifetime `SOURCES` already has.
    if not hasattr(mod, "registry"):
        name = pathlib.Path(file).stem
        _INSTALLED_LEAVES.append((name, sys.modules.get(name)))
        sys.modules[name] = mod
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
        for name, content in dict(files or ()).items():
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
    deterministically, on any host.

    ⚠ ONE module OR SEVERAL (PR #510 R42-7).  A touch-time split moves work out
    of the module a witness names, and the witness then counts a shrinking part
    of it while the contract it guards is about the whole: carving §6.3's
    grammar out of the lexer left `linear_inline_tail_control` watching the
    lexer alone, and its mutant -- lifting the destination nesting bound --
    SURVIVED, because the quadratic scan it re-introduces now runs in the other
    file.  A witness whose subject can be split takes the SET."""

    def __init__(self, module, limit):
        mods = module if isinstance(module, (list, tuple)) else (module,)
        self.files = {m.__file__ for m in mods}
        self.file, self.limit, self.lines = next(iter(self.files)), limit, 0

    def __enter__(self):
        self._prev = sys.gettrace()

        def tracer(frame, event, arg):
            if frame.f_code.co_filename not in self.files:
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


class _count_pattern_spans:
    """Tally the CHARACTERS a compiled pattern held by `module.<name>` is
    handed -- `(endpos or len(subject)) - pos` per application, in `.chars`,
    with the applications in `.calls` -- and stop the block past `limit`.

    THE FIFTH WITNESS, AND THE FIRST WHOSE SUBJECT IS THE `re` ENGINE (PR #510
    R31-4).  The other four count Python: calls of a module binding, source
    lines executed in a file, executions of one source line, reads of a list.
    A pattern applied once per site over a span that grows with the block is
    quadratic without moving any of those numbers -- one call, one source
    line, one list read, every time -- because the growth is inside the C
    engine, which executes no traced frame.  What DOES grow is the span, so
    that is what this counts.

    ⚠ IT IS AN UPPER BOUND ON WORK, NOT A MEASURE OF IT, and the two methods
    differ: `search` scans the span, so its work really is proportional to
    what it is handed, while `match` is ANCHORED at `pos` and its work is
    bounded by the pattern however much text stands ahead.  A control that
    tallies both is therefore conservative in the safe direction -- it can
    only over-count an anchored call -- and a bound it passes is a bound the
    engine's real work passes too.

    The proxy delegates to the real pattern and returns exactly what it
    returns, so every verdict under the witness is the production verdict: the
    subject reads its own input, and nothing of its guard is stubbed."""

    def __init__(self, module, name, limit):
        self.module, self.name, self.limit = module, name, limit
        self.chars, self.calls = 0, 0

    def _tally(self, subject, pos, endpos):
        self.calls += 1
        self.chars += (len(subject) if endpos is None else endpos) - pos
        if self.limit is not None and self.chars > self.limit:
            raise _WorkExceeded()

    def __enter__(self):
        self._orig = orig = getattr(self.module, self.name)
        tally = self._tally

        class _Proxy:
            def __getattr__(_self, attr):
                return getattr(orig, attr)

        def _make(method):
            def call(_self, subject, pos=0, endpos=None):
                tally(subject, pos, endpos)
                fn = getattr(orig, method)
                return fn(subject, pos) if endpos is None else fn(subject, pos, endpos)
            return call

        # The three APPLICATION methods are counted; everything else a caller
        # might reach for (`finditer`, `pattern`, `flags`) falls through
        # `__getattr__` to the real pattern, so there is ONE delegation path
        # and no list of method names to keep.
        for method in ("match", "search", "fullmatch"):
            setattr(_Proxy, method, _make(method))
        setattr(self.module, self.name, _Proxy())
        return self

    def __exit__(self, *exc):
        setattr(self.module, self.name, self._orig)
        return False
