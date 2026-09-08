#!/usr/bin/env python3
"""The SOURCE property controls for `plan-memo-umbrella-check.py --self-test`:
every control that enumerates its own population, sweeps it, and asks the
question of the checker AS WRITTEN.

The seam is mechanical, not a taste, and it is stated three ways.  In the
SUBJECT: a control here reads this checker's source text, its AST, the
docstring of its entry point or the code object of one of its methods, and
CALLS nothing of it -- so no fixture is written, no document is parsed and no
verdict is read.  A control whose question needs the checker RUN is
`plan_memo_selftest_invariants.py`'s (carved at PR #510 R29, at 998 lines), and
one whose measure is a COST is `plan_memo_selftest_work.py`'s.  In the imports:
this module is the ONLY importer of `ast` and of the harness's module-set
handles (`MODULES` / `SOURCES` / `GRAMMAR` / `HERE`), and the invariants module
the only importer of `build` / `run_on`, exactly as the work module is the only
importer of the three work witnesses.  In the registry: every control the two
property modules contribute is named `PROPERTY: ...` and every `PROPERTY: ...`
entry of the one table comes from one of them.

WHY THE SOURCE IS EVER THE SUBJECT.  Three of the defects this checker has had
run no Python line and pass through no module binding a witness can watch --
`list.pop(0)`'s shift, a `str` slice's copy, a locale-defaulted `open()` that
only fails on another host -- so what can be stated about them is the
STRUCTURE, over every source rather than at the one site a reviewer named.
Each such control carries its own "HONESTLY, what it cannot see", because a
shape sweep over an AST is exactly as narrow as the shapes it spells.

`registry()` returns this module's fragment of the one name -> (kind, control)
table, merged with the invariants module's;
`plan_memo_selftest_controls.registry()` merges that, so the runner and the
mutation proof still read ONE table.  A mutant row whose file is THIS module
patches the self-test, not the checker set
(`plan_memo_selftest_mutants.SELFTEST`).

Import direction, one way: the controls module imports this one; this one
imports the invariants module and the harness, and nothing of the controls.
"""

import ast
import re

from plan_memo_selftest_harness import GRAMMAR, HERE, MODULES, SOURCES
from plan_memo_selftest_invariants import registry as invariant_registry

# The entry point's file name, which is not an import name.
ENTRY = "plan-memo-umbrella-check.py"


def _swept_sources():
    """THE POPULATION EVERY SOURCE SWEEP OF THIS MODULE READS, as (file, text):
    every `plan_memo*.py` beside this file plus the entry point, GLOBBED -- the
    checker set and the self-test both, so a module a later touch-time split
    carves out is swept the day it lands and not the day somebody remembers it.

    Text comes from `SOURCES` when the current set holds it (`load` records the
    checker set, `patched_module` a patched self-test module) and from disk
    otherwise, so a MUTANT is seen on either half.  One function, because three
    sweeps ask the same question and a second spelling of "every source of this
    checker" is a second answer waiting to drift."""
    out = []
    for file in sorted(p.name for p in HERE.glob("plan_memo*.py")) + [ENTRY]:
        src = SOURCES.get(file)
        if src is None:
            src = (HERE / file).read_text(encoding="utf-8")
        out.append((file, src))
    return out


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
    HTML tag-name grammar in `plan_memo_html.py`, `[a-zA-Z0-9+.-]` the URL
    scheme grammar in `plan_memo_sibling.py` -- neither is an id class, and the
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


def _slice_bounds(node, ints):
    """Every SLICE bound in `node`'s subtree that is fixed by a NUMBER: an
    integer literal, or a module-global name bound to one (`ints`).  A bound
    computed from a match position (`m.start`, `len(x)`) is not one -- that is
    a position in the text, not a width."""
    out = []
    for sub in ast.walk(node):
        if not (isinstance(sub, ast.Subscript) and isinstance(sub.slice, ast.Slice)):
            continue
        for bound in (sub.slice.lower, sub.slice.upper):
            if bound is None:
                continue
            for leaf in ast.walk(bound):
                if isinstance(leaf, ast.Constant) and isinstance(leaf.value, int):
                    out.append(str(leaf.value))
                elif isinstance(leaf, ast.Name) and isinstance(ints.get(leaf.id), int):
                    out.append("%s=%d" % (leaf.id, ints[leaf.id]))
    return out


_EDGE_ANCHORS = ("^", "\\A", "(?<", "$", "\\Z", "(?=", "(?!")


def anchored_matcher_width_control(M):
    """PROPERTY, the STRUCTURAL guard for R24's third family: an ANCHORED
    pattern is never handed a subject that a NUMBER truncated.

    A width window in a matcher's input is a second, silent statement of what
    "immediately before" means, and it disagrees with the pattern: `$` then
    matches where the slice ended rather than where the marker began, and a
    lookbehind at index 0 of a slice succeeds against nothing at all.  Both
    R24 members are that: `_APPOSITIVE` (`\\s*$`) read a 70-character slice, so
    a 76-character slug pushed the appositive out of the window and a pointer
    row was counted as an umbrella at rc 0; `LICENSE_BEFORE` (a lookbehind and
    `$`) read a 40-character one, so a 40-character licensing phrase licensed
    a mention the document does not license.  Neither number was a claim about
    the document; both were about the copy.

    THE SWEEP, over `SOURCES` -- the text the CURRENT set was exec'd from, so
    a mutant is seen.  Every call whose receiver is a module-global name bound
    to a compiled pattern is a pattern-method call; if that pattern's text
    carries an EDGE anchor and any argument's expression holds a slice whose
    bound is fixed by a number (an integer literal, or a global bound to one),
    that is the finding.  A local name assigned from such a slice in the same
    function counts as the slice, so hiding the width in a variable does not
    hide it here.  Neither the method names nor the argument positions are
    enumerated -- the receiver being a compiled pattern is the predicate -- so
    a pattern method this suite has never used is in scope too.

    HONESTLY, what it cannot see.  The anchor test is TEXTUAL, over
    `rx.pattern`.  A receiver that is not a module-global name -- a pattern
    passed in as an argument, held in a list, or reached through another
    module's attribute -- is not resolved.  The taint is one level and
    intra-function.  And a truncation performed INSIDE a helper is invisible:
    `Block.window` slices `self.stream` by a numeric `w`, and
    `plan_memo_roles.roles` hands that window to `ROLE_PATTERNS`.  That is not
    a finding, and would not be one if it were seen: those patterns carry no
    edge anchor, and they are a RANKING over the reported set, never a filter
    on it, so a narrower window ranks differently and reports the same sites.
    This control says nothing about such a window; what it says is that no
    ANCHORED pattern is given one."""
    import re as _re
    hits, anchored_calls, calls = [], 0, 0
    for name, file in MODULES:
        src = SOURCES.get(file)
        if src is None:
            return False, "no loaded source for %s (load() before the sweep)" % file
        mod = __import__(name)
        ints = {k: v for k, v in vars(mod).items() if isinstance(v, int)}
        tree = ast.parse(src, filename=file)
        # name -> the scopes it is width-tainted in, unioned over every
        # ENCLOSING scope of a node (an inner function sees the outer's names)
        tainted = {}
        for scope in ast.walk(tree):
            if not isinstance(scope, (ast.Module, ast.FunctionDef, ast.AsyncFunctionDef)):
                continue
            names = {t.id for node in ast.walk(scope) if isinstance(node, ast.Assign)
                     for t in node.targets if isinstance(t, ast.Name)
                     and _slice_bounds(node.value, ints)}
            if names:
                for node in ast.walk(scope):
                    tainted.setdefault(id(node), set()).update(names)
        for node in ast.walk(tree):          # ONE walk: a call is counted once
            if not (isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute)
                    and isinstance(node.func.value, ast.Name)):
                continue
            rx = getattr(mod, node.func.value.id, None)
            if not isinstance(rx, _re.Pattern):
                continue
            calls += 1
            if not any(a in rx.pattern for a in _EDGE_ANCHORS):
                continue
            anchored_calls += 1
            for arg in node.args:
                widths = _slice_bounds(arg, ints)
                if isinstance(arg, ast.Name) and arg.id in tainted.get(id(arg), ()):
                    widths = widths or ["via " + arg.id]
                if widths:
                    hits.append("%s:%d %s.%s given %s" % (file, node.lineno, node.func.value.id,
                                                          node.func.attr, ",".join(sorted(set(widths)))))
    ok = not hits and anchored_calls >= 10
    return ok, ("%d pattern-method call site(s) swept, %d on an anchored pattern, %d given a "
                "number-bounded subject%s" % (calls, anchored_calls, len(hits),
                                              (": " + "; ".join(sorted(set(hits))[:3])) if hits else ""))


# The pattern methods that try the pattern at EVERY position of their subject.
# `match` and `fullmatch` are deliberately absent: they are applied at one
# position, so what the pattern costs there is paid once and not once per
# character.
_SCANNING_METHODS = frozenset(("finditer", "search", "findall", "sub", "subn", "split"))


def _leading_unbounded_repeat(rx):
    """Whether `rx` BEGINS with an unbounded repeat that something else
    follows -- the shape a scanning method pays for at every position -> (bool,
    "") or (None, why the question could not be asked).

    The predicate is `re`'s OWN parser, not a reading of the pattern's text: a
    pattern is a program, and "what does it try first" is a question about the
    parse rather than about the spelling (`(?P<l>(?:\\*\\*|`)*)` and
    `(?:\\*\\*|`)*` are the same first move under different text).  The module is
    private and was renamed in 3.11, so both names are tried and a version that
    has neither makes the control RED rather than silently green -- a predicate
    that cannot run is not a predicate that passed.

    "SOMETHING ELSE FOLLOWS" is the whole of the difference between a cost and
    a shape.  `re.compile("`+").finditer` also starts with an unbounded repeat
    and is linear, because a repeat that IS the pattern either matches where it
    starts or fails there in one step; the cost appears when the repeat is
    consumed and the match then fails AFTER it, because the engine unwinds the
    run and re-enters it one character along.  So the flag needs both: a
    leading unbounded repeat, and a sibling after it at some enclosing level.
    Measured over the module set: dropping that second half reports two linear
    sites and nothing else -- `plan_memo_lexer.py:365 _LABEL_WS.sub` over
    `[ \\t\\r\\n]+`, and `:535 _BACKTICKS.finditer` over `` `+ ``."""
    try:
        try:
            from re import _parser as parser        # CPython 3.11+
        except ImportError:
            import sre_parse as parser              # CPython 3.9 / 3.10
        sub = parser.parse(rx.pattern, rx.flags)
        maxrepeat = parser.MAXREPEAT
    except Exception as exc:                        # any failure is "cannot ask", which is red
        return None, "re's own parser is not reachable (%s: %s)" % (type(exc).__name__, exc)
    more = False
    while True:
        if not len(sub):
            return False, ""
        more = more or len(sub) > 1
        op, av = sub[0]
        name = str(op)
        if name.endswith("SUBPATTERN"):             # a group: its body is what runs first
            sub = av[-1]
            continue
        if name.endswith("ATOMIC_GROUP"):
            sub = av
            continue
        if name.endswith("MAX_REPEAT") or name.endswith("MIN_REPEAT"):
            return more and av[1] is maxrepeat, ""
        return False, ""


def leading_run_scan_control(M):
    """PROPERTY: no SCANNING pattern-method call in the module set applies a
    pattern that begins with an unbounded repeat -- the shape whose cost is
    quadratic in a run its subject happens to hold.

    WHY A SWEEP AND NOT A WITNESS (PR #510 R29-2).  `plan_memo_ids._TOKEN` was
    `decorated_id`'s composition handed to `finditer`, and that composition
    begins with `DECOR`.  Over `!` followed by N backticks the engine consumed
    the run at every position inside it, failed to find an id after it and
    unwound: 0.016 / 0.063 / 0.259 s for N = 1000 / 2000 / 4000, four times the
    work for twice the input.  NOTHING in this suite could have measured that.
    The work is inside the C `re` engine -- no Python line runs, so
    `_count_lines` sees nothing and `generated_growth_control` declares exactly
    this blind spot -- and it passes through no module binding, so
    `_count_calls` sees one `finditer` call whatever it costs.  That is the
    position `front_drain_sweep_control` was written from and it has the same
    answer: what cannot be counted can be STATED, over every call site rather
    than at the one a reviewer named.  Run against the module set as it stood
    before the fix, this sweep reports that site and no other: 39 pattern-method
    calls, 23 of them scanning, one hit, at `plan_memo_ids.py`'s
    `_CORE.finditer` (measured; the same set after the fix reports 39, 23 and
    none).

    THE POPULATION IS THE ONE `anchored_matcher_width_control` ALREADY WALKS --
    every call whose receiver is a module-global name bound to a compiled
    pattern, read over `SOURCES` so a mutant is seen -- because the two ask
    different questions of one set of sites: that one asks what the pattern is
    GIVEN, this one asks what the pattern IS.  A lower bound is part of the
    verdict: a sweep that finds no scanning call at all reports what a broken
    walk reports.

    HONESTLY, what it cannot see.  It reads the FIRST element only, so a
    pattern that scans a run in the MIDDLE (`x[ab]*y` over
    `xaaaa...xaaaa...`) is the same class and is invisible here; the cost of
    the leading repeat's own body is not read either, so a repeat over an
    expensive alternation counts the same as one over a character.  A receiver
    that is not a module-global name -- a pattern held in a list, passed in as
    an argument, or reached through another module -- is not resolved, exactly
    as in the width sweep beside it.  And it says nothing about a pattern
    applied by `match` / `fullmatch`: those run at one position, and the walk
    that chooses the position is then the caller's own and countable in
    Python."""
    import re as _re
    hits, scanning, calls = [], 0, 0
    for name, file in MODULES:
        src = SOURCES.get(file)
        if src is None:
            return False, "no loaded source for %s (load() before the sweep)" % file
        mod = __import__(name)
        for node in ast.walk(ast.parse(src, filename=file)):
            if not (isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute)
                    and isinstance(node.func.value, ast.Name)):
                continue
            rx = getattr(mod, node.func.value.id, None)
            if not isinstance(rx, _re.Pattern):
                continue
            calls += 1
            if node.func.attr not in _SCANNING_METHODS:
                continue
            scanning += 1
            leading, why = _leading_unbounded_repeat(rx)
            if leading is None:
                return False, why
            if leading:
                hits.append("%s:%d %s.%s applies %r, which starts with an unbounded repeat"
                            % (file, node.lineno, node.func.value.id, node.func.attr, rx.pattern[:44]))
    ok = not hits and scanning >= 10
    return ok, ("%d pattern-method call site(s) swept, %d of them scanning, %d applying a pattern "
                "that starts with an unbounded repeat%s"
                % (calls, scanning, len(hits), (": " + "; ".join(sorted(set(hits))[:3])) if hits else ""))


# The standard-library calls that OPEN OR MOVE TEXT and take an `encoding`
# argument which, left out, is the LOCALE's.  Read as the class it is: every
# spelling below defaults to `locale.getencoding()`, so leaving the argument
# out makes what the call reads or writes a property of the HOST rather than of
# the file.  `open` covers the builtin and `io` / `codecs` / `gzip` / `bz2` /
# `lzma` / `pathlib.Path`'s methods of that name at once, because the sweep
# reads the CALLEE NAME and not what it is bound to.
#
# ⚠ WHAT IS NOT HERE, and why neither is a hole: `read_bytes` / `write_bytes`
# and any `"rb"` / `"wb"` call move BYTES and have no encoding to name
# (`plan_memo_selftest_invariants.line_ending_control` writes its three fixtures
# that way ON PURPOSE, so that the endings under test survive); `json.load` / `json.dump` take a file
# object somebody else opened, and that opener is in this list.
_ENCODED_IO = frozenset((
    "open",             # builtin / io / codecs / gzip / bz2 / lzma / Path.open
    "read_text", "write_text",
    "fdopen",           # os.fdopen
    "NamedTemporaryFile", "TemporaryFile", "SpooledTemporaryFile",
    "reconfigure",      # TextIOWrapper.reconfigure: one that names no encoding
))                      # leaves the locale's in place, which IS the defect


def encoding_sweep_control(M):
    """PROPERTY: no source of this checker performs TEXT I/O without naming its
    encoding.  Every call whose callee NAME is one of `_ENCODED_IO` must pass
    `encoding=`; one that does not is red.

    WHY IT IS A PROPERTY AND NOT TWO FIXES (PR #510 R26-4).  The checker
    advertises Python 3.9+ with only the standard library and reads production
    memos as explicit UTF-8 (`plan_memo_memo.Memo.read`), while the SELF-TEST
    read its own module sources and wrote its fixtures with the locale default.
    On a host whose preferred encoding is not UTF-8 that raised
    `UnicodeDecodeError` inside `load()` before a single control ran --
    `PYTHONUTF8=0 LC_ALL=C python3 plan-memo-umbrella-check.py --self-test`
    reproduces it, because these sources hold non-ASCII text.  The reviewer
    named two sites (the source read and the fixture writes); the class was
    FIFTEEN, in four modules, and the three it did not name
    (`plan_memo_selftest_mutants.run`'s source read and the fixture writes of
    the work and controls modules) are exactly what an enumerated fix would
    have left standing.  So the fix is the predicate, not the list.

    THE POPULATION IS DISCOVERED, NOT LISTED: `_swept_sources()`, which is
    every `plan_memo*.py` beside this file plus the entry point, globbed.

    A LOWER BOUND IS PART OF THE VERDICT: a sweep that finds no call at all is
    red, because "no site without an encoding" is also what a broken walk, an
    empty population or a renamed callee reports.

    HONESTLY, what it cannot see.  The predicate is the callee's NAME in an
    `ast.Call`, so a call reached through an alias (`w = p.write_text; w(t)`),
    through `getattr`, or inside a library helper that opens a file itself is
    invisible; so is an encoding that is named but wrong.  The list is an
    INCLUSION list, which is the safe direction -- a standard-library spelling
    nobody here has used yet is MISSED, never blessed -- and the environment
    half of this finding is checked outside the suite by the command above,
    because a control cannot change the preferred encoding of the interpreter
    it is already running in."""
    hits, calls = [], 0
    files = _swept_sources()
    for file, src in files:
        for node in ast.walk(ast.parse(src, filename=file)):
            if not isinstance(node, ast.Call):
                continue
            f = node.func
            name = (f.attr if isinstance(f, ast.Attribute)
                    else f.id if isinstance(f, ast.Name) else None)
            if name not in _ENCODED_IO:
                continue
            calls += 1
            if not any(k.arg == "encoding" for k in node.keywords):
                hits.append("%s:%d %s() names no encoding" % (file, node.lineno, name))
    return (not hits and calls > 0,
            "%d text-I/O call site(s) in %d source(s) swept, %d naming no encoding%s"
            % (calls, len(files), len(hits), (": " + "; ".join(hits[:3])) if hits else ""))


def front_drain_sweep_control(M):
    """PROPERTY: no source of this checker removes an element from the FRONT of
    a list.  Every `x.pop(0)` and every `del x[0]` over `_swept_sources()` is a
    hit, and one hit is red; at least one `popleft()` call must stand, because
    a sweep that finds neither reports what a deleted worklist reports.

    WHY A SOURCE CLAIM AND NOT A MEASUREMENT (PR #510 R28-1).  The population
    walk drained its queue with `queue.pop(0)`, which shifts every remaining
    element: quadratic in the queue's length, and the queue held one entry per
    LINK rather than one per memo.  The pending-entry half is a countable fact
    and `plan_memo_selftest_growth.population_walk_once_control` counts it.
    This half is not countable by anything in this suite: the shift is a C
    memmove inside `list.pop(0)`, so it runs no Python line, calls no module
    binding, and touches no dunder a `_CountedList` could watch -- a `deque`'s
    `popleft` and a `list`'s `pop(0)` are one call each to every witness the
    harness has.  What can be stated is the STRUCTURE, so that is what is
    stated, over every source rather than at the one site the reviewer named:
    the drain is O(1) or the sweep is red.

    HONESTLY, what it cannot see.  It is a shape sweep over the AST, so a front
    removal spelled some other way is invisible: `x[0:1] = []`, `x.remove(x[0])`,
    a slice-and-rebind (`x = x[1:]`, which is O(n) too), a `pop` whose index is
    a variable that happens to be zero, or a front insert (`x.insert(0, y)`,
    which is the same memmove but a different question -- `sys.path.insert(0,
    HERE)` in the entry point is the legitimate one and the sweep must not be
    reading it).  It also says nothing about a list a caller drains from the
    front OUTSIDE these sources, and nothing about the queue actually being a
    deque at run time -- the growth module's control records the pops through
    the module's own `deque` binding and reports if there is none."""
    hits, drains, lists = [], 0, 0
    for file, src in _swept_sources():
        for node in ast.walk(ast.parse(src, filename=file)):
            if isinstance(node, ast.Delete):
                for t in node.targets:
                    if isinstance(t, ast.Subscript) and isinstance(t.slice, ast.Constant) \
                            and t.slice.value == 0:
                        hits.append("%s:%d del x[0] shifts the whole list" % (file, node.lineno))
                continue
            if not isinstance(node, ast.Call) or not isinstance(node.func, ast.Attribute):
                continue
            if node.func.attr == "popleft":
                drains += 1
            elif node.func.attr == "pop":
                lists += 1
                if len(node.args) == 1 and isinstance(node.args[0], ast.Constant) \
                        and node.args[0].value == 0:
                    hits.append("%s:%d .pop(0) shifts the whole list" % (file, node.lineno))
    return (not hits and drains > 0,
            "%d pop() and %d popleft() call site(s) swept, %d removing from the front%s"
            % (lists, drains, len(hits), (": " + "; ".join(hits[:3])) if hits else ""))


# The module map lives in the entry point's docstring, between these two
# headings.  A name is spelled either in full or relative to the `plan_memo`
# prefix (`_selftest_controls.py`), which is the one normalisation below.
_MAP_START, _MAP_END = "\nMODULES\n", "\nWHERE THIS RUNS"
_MAP_NAME = re.compile(r"[A-Za-z0-9_.-]+\.py(?![A-Za-z0-9_])")


def _mapped_modules():
    """The module names the entry point's MODULES map spells, normalised to
    file names -> (set, "") or (None, why)."""
    src = dict(_swept_sources())[ENTRY]
    doc = ast.get_docstring(ast.parse(src, filename=ENTRY)) or ""
    doc = "\n" + doc
    a, b = doc.find(_MAP_START), doc.find(_MAP_END)
    if a < 0 or b < a:
        return None, "the docstring has no %r ... %r section" % (_MAP_START.strip(), _MAP_END.strip())
    return {n if not n.startswith("_") else "plan_memo" + n
            for n in _MAP_NAME.findall(doc[a:b])}, ""


def _map_population():
    """(mapped names, the files on disk the map is required to name).  The
    entry point is excluded from the requirement: the map names it `(this
    file)`, deliberately, and a map that had to name itself would be the one
    line nobody can get wrong."""
    return sorted(f for f, _ in _swept_sources() if f != ENTRY)


def module_map_completeness_control(M):
    """PROPERTY: every module of this checker is named in the entry point's
    MODULES map.  The population is `_swept_sources()` minus the entry point --
    globbed, not listed -- and a file the map does not name is red.

    WHY A CONTROL AND NOT A WARNING (PR #510 R28-2).  The map says what each
    module OWNS, which `ls` cannot, so it earns its place; but it drifted three
    times, and the response the second time was to add the three missing names
    AND A PROSE WARNING saying the map drifts.  One round later it drifted
    again -- `plan_memo_selftest_growth.py`, carved by R27's split -- exactly as
    the warning predicted and did not prevent.  A sentence asking the author to
    be careful is not a mechanism (`memory/feedback_prose-rules-cannot-fix-unexecuted-claims.md`);
    a checker over the artefact is.  The warning is now a pointer to this
    control rather than a request.

    HONESTLY, what it cannot catch.  It checks that the map NAMES every module,
    not that what it says about one is TRUE: a description that has gone stale,
    or is attached to the wrong module, reads exactly like a fresh one here.
    It is also blind to a module outside the `plan_memo*.py` glob (a helper
    named otherwise would be swept by nothing in this file), and it takes the
    map's own `(this file)` for the entry point on trust."""
    mapped, why = _mapped_modules()
    if mapped is None:
        return False, why
    want = _map_population()
    missing = [f for f in want if f not in mapped]
    return not missing, ("%d module(s) on disk against %d name(s) in the map, %d unnamed%s"
                         % (len(want), len(mapped), len(missing),
                            (": " + ", ".join(missing)) if missing else ""))


def module_map_existence_control(M):
    """PROPERTY, the OTHER direction of the map: every name the MODULES map
    spells is a file that exists.  The completeness half cannot see a name left
    behind by a rename or a deletion -- the map would still cover every file on
    disk and simply describe one that is gone -- so the two directions are two
    controls, and each has its own mutant.

    HONESTLY: a name is anything ending in `.py` inside the section, so a name
    that appears there for another reason (a shell command quoting a glob, say)
    would be read as a claim about a file.  That is the reason the section
    holds no such text; if it ever needs to, this control is where the
    exception has to be written down rather than assumed."""
    mapped, why = _mapped_modules()
    if mapped is None:
        return False, why
    have = set(_map_population()) | {ENTRY}
    dangling = sorted(n for n in mapped if n not in have)
    return not dangling, ("%d name(s) in the map against %d file(s), %d naming nothing on disk%s"
                          % (len(mapped), len(have), len(dangling),
                             (": " + ", ".join(dangling)) if dangling else ""))

def registry():
    """name -> (kind, control), the PROPERTY fragment of the one table: this
    module's source sweeps merged with the invariants module's."""
    reg = dict(invariant_registry())
    reg.update({
        "PROPERTY: the id character classes are spelled once, in plan_memo_ids.py (a source-text sweep)":
            ("CONTROL", id_spelling_sweep_control),
        "PROPERTY: Population._kind reads every kind phrase from plan_memo_tables.KIND_PHRASES, the tuple the residue gate iterates (a fourth phrase cannot decide a kind without being gated)":
            ("CONTROL", kind_phrase_gate_control),
        "PROPERTY: no ANCHORED pattern in the module set is handed a subject truncated by a number (a width window is a second statement of what the anchor already says)":
            ("CONTROL", anchored_matcher_width_control),
        "PROPERTY: no SCANNING pattern-method call in the module set applies a pattern that BEGINS with an unbounded repeat (a run the engine re-enters at every position inside it -- a cost no witness in this suite can count)":
            ("CONTROL", leading_run_scan_control),
        "PROPERTY: no source of this checker performs text I/O without naming its encoding (the checker set and the self-test both, globbed)":
            ("CONTROL", encoding_sweep_control),
        "PROPERTY: no source of this checker removes an element from the FRONT of a list (the O(1) half of the population walk's drain, which no work witness here can measure)":
            ("CONTROL", front_drain_sweep_control),
        "PROPERTY: every module of this checker is NAMED in the entry point's MODULES map (the map is checked, not asked to be kept)":
            ("CONTROL", module_map_completeness_control),
        "PROPERTY: every name the entry point's MODULES map spells is a file that exists (the rename half the completeness direction cannot see)":
            ("CONTROL", module_map_existence_control),
    })
    return reg
