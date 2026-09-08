#!/usr/bin/env python3
"""The PROPERTY controls for `plan-memo-umbrella-check.py --self-test`: every
control that enumerates its own population and SWEEPS it.

The seam is mechanical, not a taste, and it is stated twice.  In the registry:
every control this module contributes is named `PROPERTY: ...` and every
`PROPERTY: ...` entry of the one table comes from here, so "is this a
property?" is answered by the name a reader already sees.  In the imports:
this module is the ONLY importer of `ast` and of the harness's module-set
handles (`MODULES` / `SOURCES` / `GRAMMAR` / `HERE`) -- reading the checker's own source
text, AST or code objects is what a sweep does and what a fixture control never
does -- exactly as `plan_memo_selftest_work.py` is the only importer of the
three work witnesses.

What is here, and the population each one sweeps: the id-grammar spelling sweep
(every string constant of every module but the grammar's), the row-kind
coverage sweep (every row kind the grammar enumerates, against every composer),
the kind-phrase gate (every name `Population._kind`'s code object reads), the
render-equivalence sweep (every position of one prose, re-spelled as a §2.5
reference), the break-equivalence sweep (every line break of one prose, in
each of CommonMark's three spellings of one), the line-ending property (§2.1's
three endings, written as bytes),
the anchored-matcher width sweep (every pattern-method call site of the
module set), and the encoding sweep (every text-I/O call site of every source
of this checker, the self-test's own included -- the one sweep whose population
is GLOBBED rather than taken from `MODULES`, because the defect it was written
against was in the self-test).  What is NOT: a control that runs one fixture and reads the
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
from plan_memo_selftest_harness import GRAMMAR, HERE, MODULES, SOURCES, run_on


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


def row_kind_coverage_control(M):
    """PROPERTY, the KIND half of the R14 spelling sweep: every composer that
    reads "a row id in this position" admits EVERY row kind the grammar
    enumerates (`plan_memo_ids.ROW_KINDS`) -- the marker's appositive
    subject (`attributed_to_other`, over `ROW_NOUN_ID`), the two-owner clause
    (`OWNS_TWO`), the row-noun-anchored reading (`_anchored`, through
    `check()`: the mention is `anchored`) and -- since PR #510 R24 -- the BARE
    reading (`_bare`: the same id with NO row noun before it, so the mention
    comes back not `anchored`).  The kinds are the GRAMMAR's
    tuple; the sample id per kind is looked up here, and a kind without a
    sample is red, so a fourth row kind added to the grammar reaches this
    control before it reaches any composer.  The spelling sweep cannot see
    this class: a composer built on `SHORT_ID` alone spells nothing twice,
    and passed it while `Slice `#11-zz-alpha` — **UMBRELLA, …**` attributed
    nothing (PR #510 R20).

    ⚠ The bare pass was NOT probed here before R24, and could not honestly
    have been: it asked the COMPLEMENT (`t.kind == "cite"`) where `_anchored`
    asked `ROW_KINDS`, so a kind added to `KINDS` and not to `ROW_KINDS` would
    have been admitted by one pass and refused by the other -- and this
    control, which asks only "does every composer ADMIT every row kind", would
    have stayed green straight through that disagreement, because admitting is
    all the complement ever does.  Collapsing the two spellings is what made
    the bare reading answerable by this question at all; the control grew to
    cover it in the same round, since a collapse whose result nothing probes is
    an assertion."""
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
            probes += 4
            if tables.attributed_to_other("Slice %s — **%s**" % (d, tables.MARKER), "7z") != rid:
                fails.append("appositive/%s %r" % (kind, d))
            m = roles.OWNS_TWO.search("owned by %s and **Qx**" % d)
            if m is None or m.group("aid") != rid:
                fails.append("OWNS_TWO/%s %r" % (kind, d))
            res, _ = run_on(M, build(), "Slice %s lands first." % d)
            if not any(x.id == rid and x.anchored for x in res.mentions):
                fails.append("anchored/%s %r" % (kind, d))
            # The BARE reading: no row noun, so the mention must come back
            # UNanchored -- the half `_anchored` cannot answer for.
            res, _ = run_on(M, build(), "%s lands first." % d)
            if not any(x.id == rid and not x.anchored for x in res.mentions):
                fails.append("bare/%s %r" % (kind, d))
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
    `plan_memo_emphasis.DELIMS`, plus the THREE the branch table cannot yield
    because they are tested inside a helper, each named with its reason:
    `\\` (`_is_escape`, `_is_hard_break`), `!` (`_is_image`) and, since PR #510
    R25-2 made a line break a lexed construct, the line ending itself
    (`_is_hard_break`: `&#10;` renders a line ending but opens no §6.7 break,
    so `x\\` + `&#10;` and `x\\` + a real ending do NOT render the same).  The
    last one excludes no position `_RENDER_PROSE` holds today -- that prose is
    one line -- and is listed because the hand-added set is the sweep's one
    unmechanical part, so a helper added below `inline_pass` has to arrive
    here rather than be noticed when a newline is first written into the
    prose.  The set is reported with the run.

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
    active = set(plan_memo_emphasis.DELIMS) | {"\\", "!", "\n"}
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
                % (swept, len(_RENDER_PROSE), "".join(repr(c)[1:-1] for c in sorted(active)), len(bad),
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


# The prose the BREAK-equivalence sweep re-spells, one line ending at a time.
# Every ending sits in an adjacency that makes the answer observable -- a row
# noun then an id (the R25 shape), a licensing phrase's two halves, a slot noun
# then a slug, a row noun then a TERMINAL id -- because an ending between two
# words that decide nothing reads the same under every spelling and is no probe
# (the lesson `_RENDER_PROSE` above carries).
_BREAK_PROSE = ("Slice\nC owns it, and umbrella\n9z's children carry the obligation, and Slot\n"
                "#11-zz-alpha lands before Slice\n7z.")

# The three source spellings of ONE line break, CommonMark 0.31.2: the §6.8
# soft break (the ending alone), and §6.7's two hard forms -- two or more
# spaces before the ending, and a backslash before it.  A hard break renders
# `<br />` where a soft one renders nothing extra, but this checker's model of
# both is the plan's §3.0b row: a break renders a LINE ENDING, and a line
# ending bounds every unit the scanners read.  That model is what makes the
# three interchangeable HERE, and it is the claim this control states.
_BREAK_SPELLINGS = (("§6.8 soft", "\n"), ("§6.7 two spaces", "  \n"), ("§6.7 backslash", "\\\n"))


def break_equivalence_control(M):
    """PROPERTY, the SECOND member of R24's render-equivalence family: the
    checker's verdict is INVARIANT under a re-spelling of any ONE line break in
    the source as each of the three spellings CommonMark gives it.

    THE FAMILY'S FIRST GUARD COULD NOT REACH THIS CLASS, and said so in
    advance.  `render_equivalence_control` re-spells ONE CHARACTER as its §2.5
    numeric reference, and excludes every position holding a character
    `inline_pass` branches on -- `\\` by name.  The exclusion is not an
    oversight to be repaired: `&#92;` before a line ending renders a LITERAL
    backslash and a soft break, so at a `\\` the re-spelling is genuinely not
    rendering-equivalent and sweeping it would be a false alarm.  What extends
    is the family's PROPERTY (one rendering, several source spellings, one
    verdict), not that function's mechanism; the unit here is a line BREAK
    rather than a character, because no substitution of one character can
    spell one.  R25 landed in the declared blind spot one round after it was
    declared: `Slice\\` + a line ending + `C` reads `Slice C` to a reader and
    named no row, because the backslash stood in the stream as a character
    `NOUN_ANCHOR` cannot cross.

    HONESTLY, what it cannot see.  A break in a CELL (a table row is one
    line, so a cell holds no line ending at all -- the whole class is a
    paragraph's); a disagreement needing TWO breaks re-spelled at once; an
    adjacency no shape in `_BREAK_PROSE` reaches; and a line ending inside a
    construct where the spelling is NOT equivalent, which is why the sweep
    re-spells the paragraph's own endings and never one inside a code span or
    a raw HTML span (§6.7: a hard break is neither of those).  ⚠ It also says
    nothing about the two-space form's SPACES: they render as whitespace,
    which is what the ending contributes anyway, so this control cannot tell
    a checker that reads them from one that strips them."""
    want = _census(run_on(M, build(), _BREAK_PROSE)[0])   # the base: every break soft
    swept, bad = 0, []
    for label, spelling in _BREAK_SPELLINGS:
        for i, ch in enumerate(_BREAK_PROSE):
            if ch != "\n":
                continue
            swept += 1
            variant = _BREAK_PROSE[:i] + spelling + _BREAK_PROSE[i + 1:]
            got = _census(run_on(M, build(), variant)[0])
            if got != want:
                bad.append("%s at offset %d: %s" % (label, i, _first_difference(want, got)))
    # a base carrying no site cannot disagree about one, so the census is
    # asserted non-trivial rather than assumed to be
    ok = not bad and swept == 3 * _BREAK_PROSE.count("\n") >= 12 and len(want[2]) >= 2
    return ok, ("%d re-spelling(s) of %d line break(s) in %d spellings against a base of %d site(s) "
                "and rc %d, %d disagreement(s)%s"
                % (swept, _BREAK_PROSE.count("\n"), len(_BREAK_SPELLINGS), len(want[2]), want[0],
                   len(bad), (": " + "; ".join(bad[:2])) if bad else ""))


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
# (`line_ending_control` above writes its three fixtures that way ON PURPOSE, so
# that the endings under test survive); `json.load` / `json.dump` take a file
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

    THE POPULATION IS DISCOVERED, NOT LISTED: every `plan_memo*.py` beside this
    file plus the entry point, globbed -- the checker set and the self-test
    both, so a module a later touch-time split carves out is swept the day it
    lands and not the day somebody remembers it.  Text comes from `SOURCES`
    when the current set holds it (`load` records the checker set,
    `patched_module` a patched self-test module) and from disk otherwise, so a
    MUTANT is seen on either half.

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
    files = sorted(p.name for p in HERE.glob("plan_memo*.py")) + ["plan-memo-umbrella-check.py"]
    for file in files:
        src = SOURCES.get(file)
        if src is None:
            src = (HERE / file).read_text(encoding="utf-8")
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


def _paren_shapes(depth):
    """Every balanced parenthesis shape of nesting depth 0..`depth`, as a
    (prefix, suffix) pair wrapping a stem -- generated from the definition of
    balance, so depth 3 is present because the generator reaches it and not
    because anybody typed it."""
    out = [("", "")]
    for _ in range(depth):
        out += [("(" + a, b + ")") for a, b in out] + [(a + "()", b) for a, b in out]
    return sorted(set(out))


def file_token_resolver_agreement_control(M):
    """PROPERTY: a name the SIBLING RESOLVER accepts, standing alone in prose, is
    ONE file token to the LEXER -- the correspondence `plan_memo_lexer`'s
    `FILE_SUFFIX` comment asserts, and the one PR #510 R26-2 falsified.

    That comment says `sibling_path` stage (d) "CONSUMES this constant for the
    same test on a link destination", i.e. that the two readers decide "is this
    a file name" once.  They did not: the lexer's token arm admitted a FLAT
    parenthesised chunk while the resolver accepts nested-parenthesis `.md`
    paths, so over the prose `foo((9z)).md` the lexer could reach no further
    left than the bare suffix and reported the declared id `9z` as a naming
    site.

    THE POPULATION IS GENERATED FROM THE PROPERTY: every balanced parenthesis
    shape up to depth 3, wrapped round a stem holding an id, before and after
    it, plus the empty shape -- so depth 3 is covered because balance generates
    it.  Each name is kept only if `sibling_path` resolves it; the lexer must
    then read it as exactly one span covering the whole name.  A run that
    yielded no name at all would report the same "no disagreement" a clean one
    does, so the corpus is required to be non-empty AND to hold members of
    depth >= 2 -- the class the flat arm could not read.

    HONESTLY, the correspondence is ONE-directional and only that direction is
    a claim: the resolver refuses names the lexer tokenises quite happily
    (`NUL.md`, `a:b.md`, `/abs/x.md` -- stage (c)'s standing polarity), because
    the resolver answers "is there a memo beside this one" and the lexer answers
    "where does this name end".  What must never happen is the other way round:
    a string the resolver would follow to a file, which the lexer breaks into
    pieces and reads an id out of."""
    import pathlib as _p
    import plan_memo_lexer, plan_memo_sibling      # the freshly loaded set

    names, deep = [], 0
    for pre, post in _paren_shapes(3):
        for stem in ("9z", "m9z", "9z.notes", "a" + pre + "9z" + post + "b"):
            name = pre + stem + post + plan_memo_lexer.FILE_SUFFIX
            if plan_memo_sibling.sibling_path(_p.Path("/nonexistent-fixture-root"), name) is None:
                continue
            names.append(name)
            deep = max(deep, max(_depth_profile(name)))
    hits = [n for n in names
            if plan_memo_lexer.file_and_cite_spans(n) != [(0, len(n), "file")]]
    return (not hits and len(names) >= 20 and deep >= 2,
            "%d resolver-accepted name(s) swept, deepest nesting %d, %d not read as one token%s"
            % (len(names), deep, len(hits), (": " + "; ".join(hits[:3])) if hits else ""))


def _depth_profile(s):
    d, out = 0, [0]
    for c in s:
        d += (c == "(") - (c == ")")
        out.append(d)
    return out


class _RecordingStream:
    """A stand-in for `sys.stdout` that records what `reconfigure` was asked
    for.  Not a mock of a stream: `stream_encoding_control` never writes to
    it."""

    def __init__(self):
        self.asked = []

    def reconfigure(self, **kw):
        self.asked.append(kw)


def stream_encoding_control(M):
    """PROPERTY, the ABSENCE half of the encoding rule: the entry point sets BOTH
    of its output streams to UTF-8 (`plan-memo-umbrella-check._utf8_streams`).

    WHY THIS EXISTS BESIDE THE SWEEP (PR #510 R26-4).  Fixing the fifteen text-I/O
    call sites made `PYTHONUTF8=0 LC_ALL=C ... --self-test` get further and then
    die anyway, with a `UnicodeEncodeError` on the `§` in a control's name: the
    OUTPUT stream had the same locale dependence, and the sweep beside this
    control cannot ever report it, because a sweep of call sites looks for a
    missing ARGUMENT and this defect was a missing CALL.  A check whose
    population is "the places that already do I/O" is defined by the symptom's
    vocabulary; this one is defined by the property (the streams the checker
    prints on) and so has the absence in range.

    The subject is the function, exercised: both streams are replaced by
    recorders, `_utf8_streams()` is called, and each must have been asked for
    `encoding="utf-8"` exactly once.  `getattr`-guarded in production, so the
    recorders need only offer `reconfigure`.

    HONESTLY: this says the entry point configures the streams, not that every
    `print` in the tool then survives; a caller who replaces `sys.stdout` after
    `main` has run is outside it, and so is a stream with no `reconfigure`,
    which the function deliberately tolerates rather than dies on."""
    import sys as _sys
    out, err = _RecordingStream(), _RecordingStream()
    keep = _sys.stdout, _sys.stderr
    try:
        _sys.stdout, _sys.stderr = out, err
        M._utf8_streams()
    finally:
        _sys.stdout, _sys.stderr = keep
    want = [{"encoding": "utf-8"}]
    return (out.asked == want and err.asked == want,
            "stdout asked %s, stderr asked %s (each must be exactly %s)" % (out.asked, err.asked, want))


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
        "PROPERTY: the verdict is invariant under re-spelling any ONE line break as each of CommonMark's three (the §6.8 soft break, §6.7's two-space and backslash hard breaks) -- the render-equivalence family's second guard, for the class its first one excludes by construction":
            ("CONTROL", break_equivalence_control),
        "PROPERTY: no ANCHORED pattern in the module set is handed a subject truncated by a number (a width window is a second statement of what the anchor already says)":
            ("CONTROL", anchored_matcher_width_control),
        "PROPERTY: every name the sibling resolver accepts, standing alone in prose, is ONE file token to the lexer (the correspondence FILE_SUFFIX's comment asserts)":
            ("CONTROL", file_token_resolver_agreement_control),
        "PROPERTY: no source of this checker performs text I/O without naming its encoding (the checker set and the self-test both, globbed)":
            ("CONTROL", encoding_sweep_control),
        "PROPERTY: the entry point sets BOTH output streams to UTF-8 -- the absence a call-site sweep cannot report":
            ("CONTROL", stream_encoding_control),
    }
