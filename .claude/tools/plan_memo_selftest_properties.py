#!/usr/bin/env python3
"""The PROPERTY controls for `plan-memo-umbrella-check.py --self-test`: every
control that enumerates its own population and SWEEPS it.

The seam is mechanical, not a taste, and it is stated twice.  In the registry:
every control this module contributes is named `PROPERTY: ...` and every
`PROPERTY: ...` entry of the one table comes from here, so "is this a
property?" is answered by the name a reader already sees.  In the imports:
this module is the ONLY importer of `ast` and of the harness's module-set
handles (`MODULES` / `SOURCES` / `GRAMMAR`) -- reading the checker's own source
text, AST or code objects is what a sweep does and what a fixture control never
does -- exactly as `plan_memo_selftest_work.py` is the only importer of the
three work witnesses.

What is here, and the population each one sweeps: the id-grammar spelling sweep
(every string constant of every module but the grammar's), the row-kind
coverage sweep (every row kind the grammar enumerates, against every composer),
the kind-phrase gate (every name `Population._kind`'s code object reads), the
render-equivalence sweep (every position of one prose, re-spelled as a §2.5
reference), the line-ending property (§2.1's three endings, written as bytes),
and the anchored-matcher width sweep (every pattern-method call site of the
module set).  What is NOT: a control that runs one fixture and reads the
verdict, which is `plan_memo_selftest_controls.py`'s, and a control whose
measure is WORK, which is `plan_memo_selftest_work.py`'s.

`registry()` returns this module's fragment of the one name -> (kind, control)
table; `plan_memo_selftest_controls.registry()` merges it, so the runner and
the mutation proof still read ONE table.  A mutant row whose file is THIS
module patches the self-test, not the checker set
(`plan_memo_selftest_mutants.SELFTEST`).

Import direction, one way: the controls module imports this one; this one
imports the harness and the fixture builder, and nothing of the controls.
"""

import ast
import pathlib
import tempfile

from plan_memo_selftest_cases import build
from plan_memo_selftest_harness import GRAMMAR, MODULES, SOURCES, run_on


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
            tuple(sorted((f[0], f[1], f[2]) for f in res.findings)),
            tuple(sorted((m.id, m.lineno, m.source, m.anchored, m.licensed) for m in res.mentions)))


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


def line_ending_control(M):
    """PROPERTY: one document written with each of the three line endings §2.1
    recognises -- LF, CRLF, and a CR not followed by an LF -- yields the SAME
    census.  §2.1 defines all three, so a checker that reads one of them is
    reading a different set of documents from the one the spec describes.

    The three fixtures are written with `write_bytes`, not `write_text`: a text
    write translates `\\n` to the platform's own ending, so on Windows the CRLF
    fixture would reach disk as `\\r\\r\\n` and the claim would be about the
    host rather than about the checker.  Bytes are what a file holds.

    This is the half a `Case` cannot state: a record writes its fixture through
    the harness, and the harness writes text.  The record-shaped CR control
    beside it (`plan_memo_selftest_cases_inline.py`, R24 §2.1) is the same
    claim from the fixture side, and covers the arm that survives translation.

    HONESTLY, what it cannot see: the census it compares is `_census`'s -- rc,
    finding codes with their memo and line, and the naming sites -- so a
    difference confined to a REPORT's quoted excerpt (which holds raw text and
    would legitimately differ) is invisible to it, as is any difference in a
    memo this fixture does not link."""
    text = build() + "\n9z owns it.\n"
    out = {}
    with tempfile.TemporaryDirectory() as d:
        (pathlib.Path(d) / "slice-9z-sib.md").write_bytes(b"\n")
        for name, ending in (("LF", "\n"), ("CRLF", "\r\n"), ("CR", "\r")):
            p = pathlib.Path(d) / ("m-%s.md" % name)
            p.write_bytes(text.replace("\n", ending).encode("utf-8"))
            res = M.check(str(p))
            out[name] = (_census(res), len(res.population.ids))
    same = len({c for c, _ in out.values()}) == 1
    ok = same and out["LF"][0][0] == 0 and out["LF"][1] > 1
    return ok, ("rc/ids per ending: %s; the three censuses %s"
                % (", ".join("%s rc %d %d id(s)" % (k, c[0], n) for k, (c, n) in out.items()),
                   "agree" if same else "DIFFER: " + _first_difference(out["LF"][0], out["CR"][0])))


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

def registry():
    """name -> (kind, control), this module's fragment of the one table."""
    return {
        "PROPERTY: the id character classes are spelled once, in plan_memo_ids.py (a source-text sweep)":
            ("CONTROL", id_spelling_sweep_control),
        "PROPERTY: every row-id composer admits every row kind of plan_memo_ids.ROW_KINDS (the kind half of the spelling sweep)":
            ("CONTROL", row_kind_coverage_control),
        "PROPERTY: Population._kind reads every kind phrase from plan_memo_tables.KIND_PHRASES, the tuple the residue gate iterates (a fourth phrase cannot decide a kind without being gated)":
            ("CONTROL", kind_phrase_gate_control),
        "PROPERTY: the verdict is invariant under a §2.5 re-spelling of any prose character the document renders the same (the rendered-text rule, swept position by position)":
            ("CONTROL", render_equivalence_control),
        "PROPERTY: the census is the same under each of the three line endings CommonMark §2.1 recognises (LF, CRLF, a bare CR), written as bytes":
            ("CONTROL", line_ending_control),
        "PROPERTY: no ANCHORED pattern in the module set is handed a subject truncated by a number (a width window is a second statement of what the anchor already says)":
            ("CONTROL", anchored_matcher_width_control),
    }
