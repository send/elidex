#!/usr/bin/env python3
"""The BEHAVIOURAL property controls for `plan-memo-umbrella-check.py
--self-test`: every control that enumerates its own population, sweeps it, and
asks the question by RUNNING the checker.

The seam against `plan_memo_selftest_properties.py` is the SUBJECT, and it is
mechanical rather than a taste.  A control there reads the checker AS WRITTEN
-- its source text, its AST, the docstring of its entry point, the code object
of one of its methods -- and calls nothing of it; a control here CALLS it, and
reads what comes back.  In the imports: that module is the only importer of
`ast` and of the harness's module-set handles (`MODULES` / `SOURCES` /
`GRAMMAR` / `HERE`), and this one is the only importer of the fixture builder
and the fixture runner (`build` / `run_on`), exactly as
`plan_memo_selftest_work.py` is the only importer of the three work witnesses.
Carved at PR #510 R29, at 998 lines, before the round's own controls were
written into it.

TWO SHAPES LIVE HERE, and neither is a fixture record.  The first is an
EQUIVALENCE: one document written two ways that render the same must get the
same verdict (a §2.5 numeric reference for any one character, each of
CommonMark's three spellings of any one line break, each of §2.1's three line
endings), and the population is every position of one prose rather than the
positions a defect was once found at.  The second is an ORACLE: a reader must
answer its own DEFINITION over an enumerated family (`_straddles` over every
blank layout of eight positions), or two readers that the code claims decide
one question must agree over a generated corpus (the lexer's file token against
the sibling resolver; every row-id composer against every row kind the grammar
declares).  What is NOT here: a control that runs one fixture and reads the
verdict, which is a `Case` in `plan_memo_selftest_cases*.py` or a function in
`plan_memo_selftest_controls.py`, and a control whose measure is WORK, which is
`plan_memo_selftest_work.py`'s.

`registry()` returns this module's fragment of the one name -> (kind, control)
table; `plan_memo_selftest_properties.registry()` merges it and
`plan_memo_selftest_controls.registry()` merges that, so the runner and the
mutation proof still read ONE table, and every entry of it named
`PROPERTY: ...` still comes from the two property modules.  A mutant row whose
file is THIS module patches the self-test, not the checker set
(`plan_memo_selftest_mutants.SELFTEST`).

Import direction, one way: the properties module imports this one; this one
imports the harness and the fixture builder, and nothing of the controls.
"""

import itertools
import pathlib
import re
import tempfile

from plan_memo_selftest_cases import build
from plan_memo_selftest_harness import run_on


# One document SPELLING per id kind `plan_memo_ids.KINDS` declares, for the
# atom half of the id-scan corpus below.  Checked against `KINDS` and against
# `kind_of` when the control runs, so a kind added to the grammar without a
# spelling here turns the control RED instead of quietly leaving that kind out
# of the sweep -- the same guard `plan_memo_selftest_growth.ID_SPELLINGS`
# carries, for the same reason.
_SCAN_SAMPLES = {"short": "9z", "slug": "#11-a", "cite": "[C1]"}

# The two corpora, both EXHAUSTIVE up to their length.  The first is over
# characters, because the class this control was written for lives BETWEEN
# characters of a decoration run; the second is over ATOMS, because a slug and
# a citation id are five and four characters long and no character-level corpus
# of a workable size reaches one with decoration on both sides.  `.` is an atom
# because the short kind's boundary has a dotted-number clause; the decoration
# marks and the id spellings are read off the grammar rather than typed.
_SCAN_CHAR_LEN, _SCAN_ATOM_LEN = 6, 5
# The windows: the whole text, one that starts inside it (so a token's left
# decoration is cut by `pos`) and one that ends inside it (so its right
# decoration is cut by `endpos`) -- the two clamps a code span's delimiters
# impose, and the two the walks read.
_SCAN_WINDOWS = 3


def _scan_corpus(ids):
    """(units, length) -> every string over `units` of at most that length,
    both corpora, or (None, why) when the grammar has outgrown the samples."""
    kinds = {k for k, _ in ids.KINDS}
    if set(_SCAN_SAMPLES) != kinds:
        return None, "_SCAN_SAMPLES covers %s; plan_memo_ids.KINDS declares %s" % (
            sorted(_SCAN_SAMPLES), sorted(kinds))
    for kind, sample in sorted(_SCAN_SAMPLES.items()):
        if ids.kind_of(sample) != kind:
            return None, "the %r sample %r is a %r id" % (kind, sample, ids.kind_of(sample))
    chars = tuple(sorted(ids.DECOR_CHARS)) + ("9", "z")
    if ids.kind_of("9z") != "short":
        return None, "the character corpus's id characters do not spell a short id"
    atoms = tuple(ids.DECOR_MARKS) + tuple(v for _k, v in sorted(_SCAN_SAMPLES.items())) + (".",)
    out = []
    for units, length in ((chars, _SCAN_CHAR_LEN), (atoms, _SCAN_ATOM_LEN)):
        for k in range(length + 1):
            out += ["".join(t) for t in itertools.product(units, repeat=k)]
    return out, ""


def _reference_scan(ids, rx, text, pos, hi):
    """The token stream `decorated_id`'s OWN composition yields over
    `text[pos:hi]` -- the grammar searched for as one pattern, which is what
    the scan did until PR #510 R29-2 -- with the same boundary clause applied
    (`_glued`, which that round did not touch)."""
    out = []
    for m in rx.finditer(text, pos, hi):
        kind = next(k for k, _rx in ids.KINDS if m.group(k) is not None)
        l, r = m.group("l"), m.group("r")
        if not l and ids._glued(text, m.start() - 1, -1, kind, pos, hi):
            continue
        if not r and ids._glued(text, m.end(), +1, kind, pos, hi):
            continue
        out.append((kind, m.group("id"), m.start(), m.end(),
                    m.start("id"), m.end("id"), l, r))
    return out


def id_scan_grammar_agreement_control(M):
    """PROPERTY: `plan_memo_ids.tokens` reads exactly the language
    `plan_memo_ids.decorated_id` spells -- over an EXHAUSTIVE corpus of
    decoration and id characters, every token identical in kind, id text, both
    extents and both decorations.

    WHY THE CLAIM HAD TO BE STATED (PR #510 R29-2).  The scan used to search
    for `decorated_id`'s composition directly, and that pattern begins with
    `DECOR`, an unbounded repeat: inside a run of decoration marks the engine
    consumed the rest of the run at EVERY position, failed to find an id after
    it and unwound, so `!` followed by N backticks cost 0.016 / 0.063 / 0.259 s
    for N = 1000 / 2000 / 4000 -- quadratic.  The scan now searches for the id
    CORE, whose alternatives each begin with a fixed character, and WALKS the
    decoration outward from it.  That moves the reading out of one pattern into
    a pattern plus two walks, so "does it still read the same documents?" stops
    being obvious, and this control is the answer: exhaustively, not by sample.

    THE ORACLE IS THE GRAMMAR, NOT THE RETIRED CODE.  `decorated_id` is still
    live -- `plan_memo_roles` and `plan_memo_tables` compose their own matchers
    from it -- so the reference here is the module's own spelling of "a
    decorated id", composed over the module's own `KINDS`, with the module's
    own `_glued` for the boundary clause that round did not touch.  What it
    therefore CANNOT say: that the grammar is right.  A mutant that changes
    `DECOR_MARKS` or a kind's pattern changes the reference too and this
    control stays green -- the id-grammar controls elsewhere in the suite are
    what hold that, and this one holds only that the two readings agree.

    THE POPULATION IS EXHAUSTIVE, in two corpora and three windows each
    (`_scan_corpus`): every string of at most six characters over the
    decoration characters and two short-id characters, and every string of at
    most five ATOMS over the decoration marks, one spelling per id kind and a
    `.`.  Both are required to be non-empty and the sweep is required to have
    found tokens carrying decoration on both sides, since a corpus that yields
    no decorated token would report the same agreement a broken scan does."""
    import plan_memo_ids as ids       # the FRESHLY loaded set

    corpus, why = _scan_corpus(ids)
    if corpus is None:
        return False, "the corpus could not be generated: %s" % why
    rx = re.compile(ids.decorated_id("|".join("(?P<%s>%s)" % kv for kv in ids.KINDS)))
    bad, probes, decorated = [], 0, 0
    for text in corpus:
        for pos, endpos in ((0, None), (1, None), (0, max(0, len(text) - 1))):
            hi = len(text) if endpos is None else endpos
            probes += 1
            want = _reference_scan(ids, rx, text, pos, hi)
            got = [(t.kind, t.id, t.start, t.end, t.idstart, t.idend, t.l, t.r)
                   for t in ids.tokens(text, pos, endpos)]
            decorated += sum(1 for t in want if t[6] and t[7])
            if got != want and len(bad) < 3:
                bad.append("%r [%d:%d]: the grammar reads %s, the scan reads %s" % (text, pos, hi, want, got))
    ok = not bad and len(corpus) > 0 and probes == _SCAN_WINDOWS * len(corpus) and decorated > 0
    return ok, ("%d strings x %d windows = %d probes, %d of them yielding a token decorated on both "
                "sides, %d disagreement(s)%s"
                % (len(corpus), _SCAN_WINDOWS, probes, decorated, len(bad),
                   (": " + "; ".join(bad)) if bad else ""))


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
    `plan_memo_tokens.file_and_cite_spans` states that reading and its declared
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
    `plan_memo_tokens`' `FILE_SUFFIX` comment asserts, and the one PR #510 R26-2
    falsified.

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
    import plan_memo_sibling, plan_memo_tokens     # the freshly loaded set

    names, deep = [], 0
    for pre, post in _paren_shapes(3):
        for stem in ("9z", "m9z", "9z.notes", "a" + pre + "9z" + post + "b"):
            name = pre + stem + post + plan_memo_tokens.FILE_SUFFIX
            if plan_memo_sibling.sibling_path(_p.Path("/nonexistent-fixture-root"), name) is None:
                continue
            names.append(name)
            deep = max(deep, max(_depth_profile(name)))
    hits = [n for n in names
            if plan_memo_tokens.file_and_cite_spans(n) != [(0, len(n), "file")]]
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
    call sites `plan_memo_selftest_properties.encoding_sweep_control` enumerates made `PYTHONUTF8=0 LC_ALL=C ... --self-test` get further and then
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


def row_noun_schema_control(M):
    """PROPERTY: the row nouns are DERIVED from the schemas -- every row-keyed
    schema's name reads as a row noun, and no other schema's does.

    The population is `plan_memo_tables.SCHEMAS`, walked here rather than
    transcribed, so the next schema arrives in this control with itself.  What
    R27-2 reported was one missing spelling (`Slot`, the §8 table's own id
    column header, absent from a hand-written `slices?|rows?|umbrellas?` since
    the alternation was written): a slot row attributing a marker to another
    row read as declaring ITSELF an umbrella -- no `UMBRELLA-MARK`, a pointer
    inside the census, exit 0.  A control that only added `Slot` would state
    the same closed list one entry longer, which is the shape
    `feedback_checks-must-not-be-defined-by-the-symptom-vocabulary` names.

    BOTH DIRECTIONS, because the derivation has to be a FILTER and not just a
    union: a schema whose id column keys `ROW_KINDS` names rows, and one that
    does not (the citation table, keyed by `cite`; the stub table, keyed by
    nothing) names none -- ``Citation `#11-zz-alpha` — **UMBRELLA, …**`` is a
    category error, not an attribution.  Deriving from every schema would pass
    the first half and fail the second.

    The two GENERIC nouns are swept as well: they belong to no schema (a row
    of any table is a `row`; an umbrella row is an `umbrella`), so nothing but
    this half says the derivation kept them.  Three spellings each, because
    `ROW_NOUN`'s case-insensitivity is scoped rather than global (PR #510
    R15)."""
    import plan_memo_ids
    import plan_memo_tables

    field = "%s `#11-zz-alpha` — **UMBRELLA, not a terminal unit.**"
    row_kinds = set(plan_memo_ids.ROW_KINDS)
    want = {s.name: bool(s.kinds) and set(s.kinds) <= row_kinds for s in plan_memo_tables.SCHEMAS}
    want.update({n: True for n in plan_memo_tables.GENERIC_ROW_NOUNS})
    bad = []
    for noun, names_a_row in sorted(want.items()):
        for spelling in (noun, noun.capitalize(), noun.upper()):
            got = plan_memo_tables.attributed_to_other(field % spelling, "9z")
            if (got == "#11-zz-alpha") != names_a_row:
                bad.append("%r -> %r (must %sname a row)"
                           % (spelling, got, "" if names_a_row else "NOT "))
    return not bad, ("%d schema name(s) + %d generic noun(s), three spellings each: %s"
                     % (len(plan_memo_tables.SCHEMAS), len(plan_memo_tables.GENERIC_ROW_NOUNS),
                        "; ".join(bad) if bad
                        else "every row-keyed schema's name names a row and no other schema's does"))


def straddle_definition_control(M):
    """PROPERTY: `_straddles` answers its own DEFINITION -- "some character of
    `[a, b)` inside a blank AND some character of it outside every blank" --
    over an enumerated family of blank layouts and every extent inside each.

    The predicate stopped being a one-line sum at R27-3 (it summed the WHOLE
    blank list per candidate token, which made the always-run seed quadratic
    in the block) and became a bisect to the overlapping window with an early
    exit.  The oracle here is the definition read one CHARACTER at a time --
    not the retired implementation, which would only say the two agree and
    could not tell a shared misreading from a correct one.

    The family is enumerated, not sampled: every subset of eight positions,
    each in two layouts -- the maximal runs (what `stream()` builds for the
    disposed reading, where a run of blanked characters is merged into one
    entry) and the same runs cut into single characters (what it builds for
    the READER's, where two blanked constructs can stand adjacent without
    merging).  Both satisfy the ordered, non-overlapping invariant `Stream`
    states and the bisect reads, and the second is the one that would catch a
    search that assumed merging."""
    import plan_memo_tables

    width = 8
    layouts = []
    for mask in range(1 << width):
        runs, i = [], 0
        while i < width:
            if mask >> i & 1:
                j = i
                while j < width and mask >> j & 1:
                    j += 1
                runs.append((i, j))
                i = j
            else:
                i += 1
        layouts.append(runs)
        layouts.append([(k, k + 1) for x, y in runs for k in range(x, y)])
    bad, n = [], 0
    for blanks in layouts:
        covered = {k for x, y in blanks for k in range(x, y)}
        for a in range(width + 1):
            for b in range(a + 1, width + 2):
                n += 1
                cells = range(a, b)
                want = (any(k in covered for k in cells) and any(k not in covered for k in cells))
                if plan_memo_tables._straddles(blanks, a, b) != want and len(bad) < 4:
                    bad.append("%r over [%d, %d) said %s, the definition says %s"
                               % (blanks, a, b, not want, want))
    return not bad, ("%d layouts x every extent = %d questions: %s"
                     % (len(layouts), n, "; ".join(bad) if bad
                        else "the bisect answers what the definition answers"))


def registry():
    """name -> (kind, control), this module's fragment of the one table."""
    return {
        "PROPERTY: plan_memo_ids.tokens reads exactly the language plan_memo_ids.decorated_id spells, over an EXHAUSTIVE corpus of decoration and id characters (the core-first scan against the grammar's own composition)":
            ("CONTROL", id_scan_grammar_agreement_control),
        "PROPERTY: every row-id composer admits every row kind of plan_memo_ids.ROW_KINDS (the kind half of the spelling sweep)":
            ("CONTROL", row_kind_coverage_control),
        "PROPERTY: the verdict is invariant under a §2.5 re-spelling of any prose character the document renders the same (the rendered-text rule, swept position by position)":
            ("CONTROL", render_equivalence_control),
        "PROPERTY: the census is the same under each of the three line endings CommonMark §2.1 recognises (LF, CRLF, a bare CR), written as bytes":
            ("CONTROL", line_ending_control),
        "PROPERTY: the verdict is invariant under re-spelling any ONE line break as each of CommonMark's three (the §6.8 soft break, §6.7's two-space and backslash hard breaks) -- the render-equivalence family's second guard, for the class its first one excludes by construction":
            ("CONTROL", break_equivalence_control),
        "PROPERTY: every name the sibling resolver accepts, standing alone in prose, is ONE file token to the lexer (the correspondence FILE_SUFFIX's comment asserts)":
            ("CONTROL", file_token_resolver_agreement_control),
        "PROPERTY: the entry point sets BOTH output streams to UTF-8 -- the absence a call-site sweep cannot report":
            ("CONTROL", stream_encoding_control),
        "PROPERTY: every row-keyed schema's NAME is a row noun and no other schema's is (the nouns are derived from SCHEMAS, so the next schema's is covered by default)":
            ("CONTROL", row_noun_schema_control),
        "PROPERTY: _straddles answers its own definition (a character inside a blank and a character outside every blank), over every blank layout of eight positions and every extent inside it":
            ("CONTROL", straddle_definition_control),
    }
