#!/usr/bin/env python3
"""Row grammar for `plan-memo-umbrella-check.py` -- how a row is named and
keyed (the id-token grammar itself is `plan_memo_ids.py`'s), the table
schemas, `Row` / `Table` and the ONE admission site `admit_table`, the kind
markers, and the mask disposition every scanner reads through.

Everything here answers "what is a row, what is its id, and what kind does
its declaring field declare?".  The lexical substrate (Phase 1 blocks:
`plan_memo_blocks.py`; Phase 2 inline: `plan_memo_lexer.py`) is the lexer's;
the document driver is `plan_memo_memo.py`'s `Memo` and the transitive memo
set `plan_memo_population.py`'s `Population`, both of which import this
module and never the reverse;
what the prose says about the rows, and whether that is allowed, is the
checker and `plan_memo_roles.py`.

Two rules decided here, once:
  * a table is admitted in `admit_table` and nowhere else: header and
    delimiter rows of equal width (GFM §4.10), and -- local policy over GFM's
    "may vary" clause -- a body row of a SCHEMA table whose width differs from
    the header is a schema miss (exit 2), never a silent skip or a shifted read;
  * row ids are read from the raw id cell (`bare_id` reads the one decorated-id
    grammar at the cell's START; a non-empty cell that does not start with an
    id is an UNKEYED row and a schema miss), kind markers from the disposed
    declaring field (a quoted marker declares nothing).

Every block is lexed ONCE, where it is minted (a cell in `split_row`, a
paragraph in `plan_memo_memo.Paragraph`); the disposition step (`dispose`,
run by `Population` once the ids are known) tags each block's mask with the
kind of every span -- `code` / `html` / `autolink` / `link` / `image` /
`cite` / `file` / `mark` -- minus the id-only code spans and the `**` pairs
that decorate one, and `stream(lexed)` -- the block AS THE DOCUMENT RENDERS
IT: the spans that render nothing dropped, the spans that render text the
checker refuses to read blanked in place, the §2.5 references substituted --
is the ONE text each predicate over a block reads.
"""

import bisect
import re

from plan_memo_blocks import block_end, delimiter_width, split_row
from plan_memo_lexer import file_and_cite_spans
from plan_memo_ids import (
    CITE_ID, DECOR, DECOR_CHARS, ROW_ID, ROW_KINDS, SHORT_ID, SLUG_ID, bounded, decorated_id,
    tokens,
)

# A cell that carries nothing: the one predicate every reader of an optional
# cell (an id cell, a `Deps` cell) decides emptiness by.  Emptiness is decided
# by SHAPE -- after decoration is stripped, a cell with no alphanumeric
# character carries nothing (`—`, `-`, `–`, `--`, `…`, `` ` ``) -- plus the
# ONLY lexical exceptions, `EMPTY_WORDS`, matched exactly (case-insensitive)
# after the strip.  A closed word list on a gating predicate leaves the next
# spelling authoritative, so the list is the exception and the shape is the
# rule; the polarity of a word outside the list (`nil`, `(none)`) is a
# FALSE rc 1 -- the cell is read as a `Deps` edge and reported by assertion
# (b) -- never a silent skip.
EMPTY_WORDS = frozenset({"n/a", "none"})


def is_empty(cell_stream):
    """Whether a cell carries nothing, read from the cell's disposed STREAM
    (`stream`): decoration does not fill a cell -- `**—**` renders `—` and is
    as empty as it -- and neither does an HTML comment, which renders nothing
    at all.  Reading the raw text here spelled the decoration strip a second
    time (`.strip("*`")`), and that spelling knew about `**` and backticks
    only; the stream knows every construct, because it is what the reader
    reads (PR #510 design re-gate 4)."""
    bare = cell_stream.strip(" \t")
    # `isalnum` is DELIBERATELY Unicode: a letter in any script fills a cell
    # (`—あ` is not empty); the shape rule is "no letter or digit at all"
    return not any(ch.isalnum() for ch in bare) or bare.casefold() in EMPTY_WORDS


# The ID cell is the one place the shape rule is WRONG: an id cell is either
# an id or a deliberate blank, and anything else (`?`, `…`, `**?**`, `--`) is
# an UNKEYED row -- the silent-skip class the schema miss exists for.  So the
# blanks are LITERAL here: exactly these four after decoration strip.
ID_CELL_BLANKS = frozenset({"", "\u2014", "-", "\u2013"})


def is_blank_id_cell(cell_text):
    """Whether an id cell is a deliberate non-row (a literal blank), as
    opposed to unkeyed content.  The RAW cell, not its stream: the id cell is
    the one text `bare_id` reads raw as well, because the decoration IS part
    of the id grammar there (`**9z**`), so the two readings of that cell agree
    by being the same reading."""
    return cell_text.strip(" \t").strip("*`").strip(" \t") in ID_CELL_BLANKS


# THE KIND PHRASES, and the ONE rule the three share: each is bounded on both
# sides by the grammar's ASCII alphanumeric class (`plan_memo_ids.bounded`),
# because a phrase matcher with no edges matches INSIDE a longer word.
# Measured at PR #510 R22, when none of the three had them: `SUBUMBRELLA, not
# a terminal unit` and `UMBRELLA, not a terminal unitary claim` both read as
# the marker (a bare `in` test); `KIND UNDETERMINEDNESS` and `MANKIND
# UNDETERMINED` both made a row kind-undetermined, which with a nonempty
# `Deps` cell is an `UMBRELLA-CELL` finding and exit 1; `is a pointer rather
# than a slicer` made a row a pointer.  Fixed together and from one spelling,
# because an enumerated fix leaves the next member of the class authoritative.
MARKER = "UMBRELLA, not a terminal unit"
"""The marker's PHRASE, for reporting it (`split_units` names it in a
finding) and for composing the matcher.  Every match goes through
`MARKER_RE`; a bare `MARKER in text` is the unbounded reading R22 removed."""

MARKER_RE = re.compile(bounded(re.escape(MARKER)))
"""The ONE matcher for the marker, read over a block's disposed STREAM (a
declaring field is one).  It must still read a marker the document SPLITS
with a construct that renders nothing -- `**UMBRELLA, not a *terminal*
unit.**` is the marker then a `.` once the emphasis delimiters are dropped
(design re-gate 4) -- so the boundary is a lookaround AROUND the phrase and
never a change to the phrase."""

# §5's fifth row kind.  A kind-undetermined row carries the split and NOTHING
# else -- no ordering, no owner, no acceptance -- which is the same obligation
# the naming rule enforces against umbrellas.  Two spellings are in use; both
# are tolerated and the divergence is reported (a kind with two spellings is a
# kind no program can enumerate).
UNDETERMINED = re.compile(bounded(r"KIND\s*[—-]?\s*UNDETERMINED"), re.IGNORECASE | re.ASCII)

# A row that is a POINTER into a slot rather than a slice of its own (§1.0's
# "SCHEDULED FROM ITS OWN SLOT" rows).  ⚠ Keyed on one spelling, and the safe
# polarity: a differently-spelled pointer row is terminal, and so REPORTED by
# the acceptance seed, never missed.
POINTER = re.compile(bounded(r"is a pointer rather than a slice"))

KIND_PHRASES = (("marker", MARKER_RE), ("undetermined", UNDETERMINED), ("pointer", POINTER))
"""EVERY phrase whose presence or absence in a declaring field changes the
row's kind, as (name, matcher) -- the ONE enumeration, read by BOTH sides of
the kind question:

  * `Population._kind` takes its matches from here and never from a matcher
    of its own, so a phrase that decides a kind is necessarily a member (the
    self-test's `kind_phrase_gate_control` reads `_kind`'s own code object
    for a second matcher and fails on one);
  * `split_units` scans for each member, so the residue gate
    (`Population._kind_residue`) covers every member BY DEFAULT.

The construction, not the list, is the fix.  At PR #510 R22 each phrase grew
its own word boundary; at design re-gate 4 the residue gate was written for
the MARKER alone, and `KIND UNDETER`MINED`` -- which a reader reads as the
undetermined kind, since a code span contributes its content as plain text
(§6.1) -- silently reclassified the row as terminal at exit 0 (R23).  Both
are the same mistake: gating the phrase in front of you leaves every other
member of the class authoritative.  A phrase added below is gated by
arriving in this tuple, and cannot decide a kind without arriving here."""

# --------------------------------------------------------------------------
# Row identity.  The id grammar itself -- the three kinds, the decoration,
# and the ONE boundary every reader consumes (`tokens`) -- is
# `plan_memo_ids.py`'s; what is here is how a ROW is named and keyed.
# --------------------------------------------------------------------------

ROW_NOUN = r"(?ai:slices?|rows?|umbrellas?)"
"""How this document names a row when it refers to one.  Lives here because it
is a fact about row IDENTITY -- `attributed_to_other` needs it to decide whose
kind a marker declares.  ASCII case-insensitive, in ONE place (the scoped
`(?ai:…)`: `SLICE C`, `ROW 9`, `UMBRELLA C` name a row as `Slice C` does, and
under `a` a long s never folds to `s`) -- every composer (`ROW_NOUN_SEP`,
`ROW_NOUN_ID`, the roles' `NOUN_ANCHOR` and `LICENSE_BEFORE`'s trailing-noun
clause) inherits it; an
enumeration of Title-case and lower-case spellings left `SLICE C owns it`
naming no row (PR #510 R15)."""

# `Slice-M` / `Slice-4a` are the same anchor with a hyphen.  Requiring `\s+`
# left them invisible to both passes; measured, four of five such sites in this
# memo are real violations.
ROW_NOUN_SEP = ROW_NOUN + r"[ \t\n-]+"       # ASCII separators (`\s` is Unicode)
# A row noun then a row id of EVERY row kind (`ROW_ID`: slug or short, the
# grammar's alternation) -- `Slice **E**`, `Slice `#11-zz-alpha``.  Built on
# `SHORT_ID` alone until PR #510 R20, so a marker attributed to a slug row
# (`Slice `#11-zz-alpha` — **UMBRELLA, …**`) named nobody and the pointer row
# was counted as an umbrella.
ROW_NOUN_ID = ROW_NOUN_SEP + decorated_id(ROW_ID)

# An id-only code span is tokenised by the declared-id GRAMMAR, longest
# alternative first (a `#11-` slug is atomic -- its internal hyphens are not
# separators), with the separators whitespace, list punctuation, `|` and `-`
# (a `Deps`-shaped edge, `9z | 7z` / `0a-0b`) between tokens.  ALL THREE
# kinds, not `ROW_ID`: a citation id is declared (the citation table keys
# its rows by it) and `` `[C1]` `` is the document spelling one.  A bare id
# in a cell or in prose is NOT tokenised on a list -- it is bounded by the
# grammar's continuation rule (`plan_memo_ids.tokens`); a hyphen bounds a
# short id, and `slice-9z-sib.md` is safe because a file name is a lexer
# `file` token, masked before the scan.
_ID_RUN_TOKEN = re.compile(r"(?P<id>%s|%s|%s)|(?P<sep>[\s,;/→>+&|-]+)" % (SLUG_ID, CITE_ID, SHORT_ID),
                           re.ASCII)

# --------------------------------------------------------------------------
# Table schemas, identified by HEADER ROW (a line range is a figure a later
# edit silently invalidates).  Column indexes are BODY columns: the optional
# leading pipe is stripped at admission, so column 0 is the first cell.
# --------------------------------------------------------------------------


class Schema:
    """A table family: its exact header cells (trimmed, in order), the column
    whose text DECLARES the row's kind, the column holding the row id (both
    named by header cell and resolved to body-column indexes here), and
    `kinds` -- the id KINDS that column may declare.

    A table's id column is keyed by one kind set, not by "whatever the id
    grammar reads": a §5 slice or a §8 slot is keyed by a `ROW_KINDS` id
    (short or `#11-` slug) and a citation table by a `[C19]` citation id, and
    the two sets are disjoint by the grammar.  The set is the schema's, and
    `bare_id` is the ONE place it is applied, so an id of a foreign kind is
    not admitted anywhere: before PR #510 R21 the column read the grammar
    bare, and a citation-shaped id in a slice or slot table entered `ids` and
    the census as an umbrella while BOTH mention passes ignored it (a
    citation id is masked in prose and exempt from the reference walk), so
    its ownership text could never be checked and the run still exited 0 --
    while a short id in the citation table polluted the keep-set, un-masking
    a `[C1]`-shaped span nothing declares."""

    __slots__ = ("name", "header", "decl", "idc", "kinds")

    def __init__(self, name, header, decl=None, idc=None, kinds=()):
        self.name, self.header = name, header
        self.decl = header.index(decl) if decl is not None else None
        self.idc = header.index(idc) if idc is not None else None
        self.kinds = tuple(kinds)
        assert (self.idc is None) == (not self.kinds), \
            "a schema with an id column declares the kinds that column may key, and only such a schema does"


SCHEMAS = [
    Schema("citation", ["ID", "Citation", "Anchor", "Used by"], idc="ID", kinds=("cite",)),
    Schema("stub", ["Site", "Syntax", "Emits", "Observable", "Tier", "Slice"]),
    Schema("slice", ["#", "Slice", "Primary module(s)", "Slot", "Tier", "Deps"],
           decl="Slice", idc="#", kinds=ROW_KINDS),
    Schema("slot", ["Slot", "Why deferred", "Trigger", "Re-eval"],
           decl="Why deferred", idc="Slot", kinds=ROW_KINDS),
]


class Row:
    """One table row, minted once at admission: its cells, the CONTENT line
    they were split from (`line`: the raw line inside a block quote has the
    marker in front, so a cell's raw column is a column of this text), its
    schema (None for a non-schema table or a header row), its own id
    (`self_id`, from the raw id cell), and -- set by `Population` -- `field`,
    the masked declaring field, and `kind` ("umbrella" / "undetermined" /
    "pointer" / "terminal")."""

    __slots__ = ("memo", "lineno", "line", "cells", "schema", "self_id", "field", "kind")

    def __init__(self, memo, lineno, line, cells, schema):
        self.memo, self.lineno, self.line, self.cells, self.schema = memo, lineno, line, cells, schema
        self.self_id = (bare_id(cells[schema.idc].text, schema.kinds)
                        if schema and schema.idc is not None else None)
        self.field, self.kind = None, None

    def id_cell(self):
        """The raw text of this row's id cell (None for a row without one)."""
        return self.cells[self.schema.idc].text if self.schema and self.schema.idc is not None else None

    def name(self):
        """How a finding names this row -- the ONE spelling, so no printer
        composes `row %r` of `self_id` itself: the repr of its id when the id
        cell declares one (`'9z'`, `'#11-x'`), else its declaring LOCATOR,
        `<no id> at :LINE (TOKEN)` -- the row's line and the first
        whitespace-delimited token of its declaring field with the inline
        decoration (backticks, asterisks) stripped, e.g. `<no id> at :1985
        (Function/eval)` for the umbrella memo's `Function`/`eval` row, whose
        id cell is the literal blank `**—**` (a deliberate non-row: unkeyed,
        outside `ids`, still a data row the seeds and the declaring-field
        printers read).  `row None` named nothing a reader could find (PR
        #510 R20); a row with no declaring field is named by its line alone."""
        if self.self_id is not None:
            return repr(self.self_id)
        token = ""
        if self.schema is not None and self.schema.decl is not None:
            words = re.sub(r"[`*]", "", self.cells[self.schema.decl].text).split()
            token = words[0] if words else ""
        return "<no id> at :%d%s" % (self.lineno, " (%s)" % token if token else "")

    def col(self, header_cell):
        """The cell under the schema's header cell named `header_cell`."""
        return self.cells[self.schema.header.index(header_cell)]


class Table:
    __slots__ = ("schema", "header", "rows", "misses")

    def __init__(self, schema, header):
        self.schema = schema            # Schema or None
        self.header = header            # Row (schema None)
        self.rows = []                  # [Row], body rows only
        self.misses = []                # [(lineno, message)] width policy


def admit_table(memo, lines, linenos, i, lazy):
    """Admit the GFM table whose header is content line `lines[i]` (raw line
    `linenos[i]`; `lazy` = the container's lazy-candidate list, None at the
    document level) -- the ONE admission site; the driver (`Memo._parse`)
    found `table_header_at` there, off a raw line, a blank and a block
    start.  Returns (Table, end): the table runs from the header to the
    first `block_end` with NO PARAGRAPH OPEN -- a blank line or the
    beginning of another block-level structure (GFM §4.10), a type-7 HTML
    opener and an indented line included (a table is not a paragraph, so
    `<span>` right after the rows opens an HTML block and `    x` an
    indented code block; the lines after are raw, not one-cell rows), and a
    lazy candidate of the enclosing quote (§5.1: continuation text only of
    a paragraph, and a table is none); every line in between is a body
    row, pipes or not (GFM Example 202: a pipe-less line after the rows is
    a row -- so a reference definition written right after a table is a
    row of it, never a definition).

    Width: GFM §4.10 "The remainder of the table's rows may vary in the
    number of cells.  If there are a number of cells fewer than the number
    of cells in the header row, empty cells are inserted.  If there are
    greater, the excess is ignored" (verbatim, GFM 0.29 §4.10 after Example
    203) -- a NON-schema row is cut to the header's width here, before
    its cells are lexed, so an ignored cell's `[x](absent.md)` is never a
    link and its id never a site (PR #510 R13); a SCHEMA row of any other
    width is the schema miss (local policy over "may vary": a shifted read
    fabricates findings).  A short row is not padded: an empty cell would
    hold nothing a scanner reads.
    """
    n = len(lines)
    header, width = split_row(lines[i]), delimiter_width(lines[i + 1])
    hdr_text = [c.text for c in header]
    schema = next((s for s in SCHEMAS if hdr_text == s.header), None)
    t = Table(schema, Row(memo, linenos[i], lines[i], header, None))
    j = i + 2
    while j < n and not block_end(lines, j, False, lazy):
        body = split_row(lines[j])
        if schema is not None and len(body) != width:
            t.misses.append((linenos[j], "row has %d cell(s); the %r header has %d -- "
                             "a shifted read fabricates findings, so this row is "
                             "unscanned" % (len(body), schema.name, width)))
        else:
            t.rows.append(Row(memo, linenos[j], lines[j], body[:width], schema))
        j += 1
    return t, j


# --------------------------------------------------------------------------
# Row identity
# --------------------------------------------------------------------------

def bare_id(cell_text, kinds):
    """The row id an id cell declares: the grammar's first bounded token when
    it stands at the cell's START (so `**7z** — MERGED` and `` `#11-x`
    (carved from #483) `` read `7z` / `#11-x`, and `xxxxC` declares nothing
    -- the boundary is the grammar's, `plan_memo_ids.tokens`), decoration
    stripped, trailing prose ignored, AND of a kind the schema's id column
    keys (`Schema.kinds`) -- or None when the cell does not start with such
    an id (then the row declares nothing and, unless the cell is a literal
    blank, is the unkeyed-row schema miss).

    The kind test is HERE and nowhere else: it is the same question as "does
    this cell start with an id" -- an id of a foreign kind is no id of this
    table's -- so there is one predicate, not a caller-side re-test that the
    next reader of the column can forget (PR #510 R21)."""
    t = next(tokens(cell_text.strip(" \t")), None)
    return t.id if t is not None and t.start == 0 and t.kind in kinds else None


_APPOSITIVE = re.compile(ROW_NOUN_ID + r"\s*[—–-]\s*" + DECOR + r"\s*$", re.ASCII)


def attributed_to_other(field, rid):
    """The row id a marker names, when it is not this row's own: the marker's
    APPOSITIVE subject -- `Slice **E** — **UMBRELLA, …**`, or the slug form
    `Slice `#11-zz-alpha` — **UMBRELLA, …**` (`ROW_NOUN_ID` reads every row
    kind) -- with nothing between them but dash punctuation and emphasis.
    Until PR #510 R20 the slug form did not match, so the field was read as
    the row's OWN declaration: kind umbrella, no `UMBRELLA-MARK` attribution
    finding, a pointer row inside the census, exit 0.  The self-test carries
    both directions -- a field that merely MENTIONS a sibling ("Unlike Slice
    7z, **UMBRELLA, not a terminal unit.**") is self-declaring and must not
    attribute.  The FIRST marker occurrence decides: a field that declares
    itself and then says a sibling "is not it" is self-declaring, and a later
    occurrence never overrides the first.

    "IMMEDIATELY BEFORE" IS A GRAMMAR FACT, NOT A CHARACTER COUNT (PR #510
    R24).  `_APPOSITIVE` ends in `\\s*$`, so it already says "ending where the
    marker begins" -- the search is bounded by `endpos`, which is where `$`
    matches, and the appositive is read over the whole field before that.  It
    was a 70-character SLICE, and a slice that starts mid-phrase truncates the
    match rather than the context: a declared 76-character `#11-…` slug (the
    `ROW_ID` grammar puts no length bound on one) followed by ``Slice `<slug>`
    — **UMBRELLA, not a terminal unit.**`` fell outside the window, the field
    was read as the row's own declaration, no `UMBRELLA-MARK` was emitted, and
    the census carried a corrupted row at rc 0.  The mention-only direction
    was never the window's to hold: the discrimination is the DASH -- `\\s*[—–-]\\s*`
    between the id and the marker -- and "Unlike Slice 7z, **UMBRELLA, …**"
    fails on the comma, at any width."""
    m = MARKER_RE.search(field)
    if m is None:
        return None
    g = _APPOSITIVE.search(field, 0, m.start())
    if g and g.group("id") != rid:
        return g.group("id")
    return None


# --------------------------------------------------------------------------
# Disposition: the one place a lexical span meets the row ids
# --------------------------------------------------------------------------


def id_only(inner, keep):
    """The disposition exception: a code span whose content is only row ids
    (and separators) is the document SPELLING an id, and is a mention.  The
    run must be covered end to end by id tokens and separators."""
    if inner in keep:
        return True
    pos, ids = 0, []
    for m in _ID_RUN_TOKEN.finditer(inner):
        if m.start() != pos:
            return False
        pos = m.end()
        if m.group("id") is not None:
            ids.append(m.group("id"))
    return pos == len(inner) and bool(ids) and all(x in keep for x in ids)


def code_mask(lx, keep):
    """Code spans of `lx` minus the id-only ones, and minus the `#11-` slugs
    of `keep` INSIDE the remaining spans (a backticked slug, alone or in a
    command line, is the document spelling an id: the same exception; the
    slug is read by the grammar's tokeniser inside the span, so
    `#11-zz-alpha_extra` is not `#11-zz-alpha`) -- the spans a reader of
    prose must skip."""
    out = []
    for a, b in lx.code:
        if id_only(lx.text[a:b].strip("`"), keep):    # whitespace is a separator token
            continue
        cut = a
        for t in tokens(lx.text, a, b):
            if t.kind == "slug" and t.id in keep:
                out.append((cut, t.idstart))
                cut = t.idend
        out.append((cut, b))
    return out


# WHAT A DISPOSED SPAN CONTRIBUTES TO THE STREAM, per kind -- the column
# §3.0b's table carries, read off the spec's rendering of each construct and
# nothing else:
#   True  = the construct renders TEXT the checker refuses to read, so its
#           span stands as blanks: it can hide an id but never JOIN what sits
#           on either side of it, because a reader does not read across it;
#   False = the construct renders NOTHING at all -- a raw HTML tag or comment
#           (§6.6: markup, not text), a link's tail (§6.3: `](dest)` prints
#           nothing) or a `mark` (the backslash of a §6.7 hard line break, a
#           link's `[`, a matched §6.2 / GFM delimiter run; a §2.4 escape's
#           backslash is NOT one -- an escape SUBSTITUTES, below) -- so its
#           span contributes no character
#           and the text on either side of it is ONE run, exactly as the
#           rendered document reads it.
# An image renders no text either, but its own tail is BLANK: `![alt](i.png)`
# puts a picture in the flow, not the letters of `alt`, so the two sides of
# it are not one word (the description is scanned as prose all the same --
# the stated deviation, §2 B×D).
RENDERS_TEXT = {"code": True, "autolink": True, "image": True, "cite": True, "file": True,
                "html": False, "link": False, "mark": False}


def dispose(lx, keep):
    """Tag `lx.mask`: every span the scanners must not read an id out of, as
    (start, end, kind) -- `code` (minus id-only spans and kept slugs), `html`
    (a §6.6 raw HTML span, whole: an id inside an attribute or a comment is
    no naming site, exactly as on a raw HTML-block line -- the memo seeds
    it instead; PR #510 R17), `autolink` (a §6.5 autolink, whole: its text
    IS its destination, so an id inside it is no more a naming site than an
    id in a link's destination is -- and unlike a raw HTML span it is NOT
    seeded, because the construct is fully lexed and hides nothing; PR #510
    R21), `link` (the tail; the visible text stays, it
    is prose), `image` (the same, for an image -- its own tail and every
    construct demoted into its description), `cite`, `file`, and `mark`
    (every span that renders no character: the backslash of a §6.7 hard line
    break, a link's `[`, a matched §6.2 emphasis or GFM strikethrough
    delimiter run).  A
    reference definition is a Phase-1 block of its own, never inline
    content, so no block holds one to mask.

    THE DECORATION EXCEPTION (PR #510 design re-gate 4).  A `**`-pair whose
    content is only declared ids is not markup around prose: it is the
    document DECORATING an id, the way `` `9a` `` spells one, so its
    delimiters STAND in the stream and the id grammar reads them as the
    boundary they are (`plan_memo_ids`: "a decorated side is bounded by the
    decoration itself") -- `**9z**7z` is the id `9z` and then the id `7z`,
    not the token `9z7z`.  It is `id_only`, the SAME predicate the code-span
    disposition has always used for `` `9z` ``, asked of the emphasis span:
    one exception, spelled once, over both constructs.  Everywhere else the
    delimiters render nothing and are dropped, so `Slice 9**z**` names the
    row `9z` a reader sees, not the row `9` the asterisks used to bound
    (`*9z*` is not decoration -- `DECOR` is `**` and a backtick -- so a
    single-`*` pair around an id is dropped like any other emphasis).

    TWO STAGES, and the boundary is which TEXT the question is asked of (PR
    #510 R24).  Stage 1 is LEXICAL: the spans the inline parse alone decides
    (a code span, a raw HTML span, an autolink, a link's or an image's tail,
    a §6.7 hard break's backslash, a link's `[`, every §6.2 / GFM delimiter
    run) -- no
    rendered text is needed to place any of them.  Stage 2 asks the two
    questions that are about what the document RENDERS, and asks both of ONE
    text, `rd` (the stage-1 disposition with each blank filled in by what a
    reader sees there -- `stream(reader=True)`, the same rendering the residue
    detector compares against):

      * is a bare `.md` file name or a `[C19]` citation id standing here?
        A file name's boundaries are WHITESPACE boundaries and §2.5 can put
        whitespace where the source has none, so the source is the wrong text
        to ask: `9z&#32;notes.md owns it` renders `9z notes.md owns it`, where
        `9z` is a naming site, and a raw-text scan masked the whole run and
        lost the ownership claim (PR #510 R24 -- the reading introduced at R22
        was the last raw one left).
      * is a `**` pair's content only declared ids -- the decoration exception
        above?  `**&#57;z**7z` renders exactly what `**9z**7z` renders, and a
        raw-text `id_only` said no to the first and yes to the second, so the
        two documents disagreed about a site a reader cannot tell apart (found
        by enumerating the class, not reported).  The question is asked of the
        READER's text and not of the disposed stream, because ``**`x` 9z**7z``
        is the document bolding prose: a reader sees `x 9z`, while the disposed
        stream would show `9z` beside blanks, which whitespace-separates into
        an id-only run.

    The two answers are independent -- a `**` pair's content decides nothing
    about where a file name stands, and the reverse -- so ONE reading serves
    both and there is no third stage.  The file/cite spans are recorded in
    SOURCE coordinates (`Stream.at`), which is what a mask is in."""
    base = [(a, b, "code") for a, b in code_mask(lx, keep)]
    base += [(a, b, "html") for a, b in lx.html]
    base += [(a, b, "autolink") for a, b in lx.autolinks]
    base += [(a, b, "link") for a, b, _ in lx.links]
    base += [(a, b, "image") for a, b, _ in lx.images]
    base += [(a, b, "mark") for a, b in lx.marks]
    delims = [((oa, ob), (ca, cb), ch, use, ob, ca) for oa, ob, ca, cb, ch, use, _k in lx.emphasis]
    lx.mask = base + [(a, b, "mark") for pair in delims for a, b in pair[:2]]
    rd = stream(lx, reader=True)
    lx.mask = base + [(a, b, "mark") for op, cl, ch, use, ob, ca in delims
                      if not (ch == "*" and use == 2 and id_only(_reading(rd, ob, ca), keep))
                      for a, b in (op, cl)]
    lx.tokens = [(rd.at(a), rd.at(b), kind) for a, b, kind in file_and_cite_spans(rd)]
    lx.mask += lx.tokens


def _reading(rd, a, b):
    """The reader's text (`stream(reader=True)`) of the SOURCE range `[a, b)`.

    `Stream.src` maps a stream offset to the source offset it came from and is
    non-decreasing (a drop skips source offsets, a substitution repeats one),
    so the first stream offset whose source is at or past `a` is where that
    source range begins to render -- a bisect, the inverse of `Stream.at`.  A
    source range that renders nothing gives an empty reading, which is what it
    reads as."""
    return rd[bisect.bisect_left(rd.src, a):bisect.bisect_left(rd.src, b)]


def _inner(kind, text):
    """The text a READER sees where this checker blanks: a code span's content
    without its backtick strings (§6.1, minus the one space each side the spec
    strips when both are there and the content is not all spaces), an
    autolink's URL without its angle brackets, and -- for the spans that are
    already their own text -- the span as written."""
    if kind == "code":
        k = len(text) - len(text.lstrip("`"))
        body = text[k:len(text) - k]
        if len(body) > 1 and body[0] == " " and body[-1] == " " and body.strip(" "):
            body = body[1:-1]
        return body
    if kind == "autolink":
        return text[1:-1]
    return text


class Stream(str):
    """A block's rendered text, carrying the map back to the source offsets
    the report and the id grammar are written in (`at`) and, in this stream's
    OWN coordinates, the spans of it the checker refuses to read as prose
    (`blanks`).  A `str`, so every predicate reads it as before; the map
    exists because a stream character no longer sits at its own source offset
    once a construct that renders nothing has been dropped."""

    def __new__(cls, text, src, blanks):
        o = str.__new__(cls, text)
        o.src, o.blanks = src, blanks
        return o

    def at(self, i):
        """The offset in the block's raw text that stream offset `i` came
        from (the end sentinel for `i` at or past the stream's end, so a
        match's `end` maps as its `start` does)."""
        return self.src[min(i, len(self.src) - 1)]


def stream(lx, reader=False):
    """`lx.text` AS THE DOCUMENT RENDERS IT, under the disposition above:
    every §2.5 character reference substituted by the character it stands for,
    every span that renders no character dropped, and every span that renders
    text the checker refuses to read blanked in place (code spans -- id-only
    spans and kept slugs were excepted in `dispose` -- autolinks, image tails,
    citation ids, file names).  This is the ONE text every predicate over a
    block reads: the id scanners (`plan_memo_ids.tokens` over this stream),
    the kind-marker reader (a quoted marker is not a declaration; a marker
    split by `*terminal*` or `&#44;` still IS one, since the reader reads
    one phrase), the seeds' vocabularies (a `gates` inside a code span is not
    ordering prose), the licensing rule's context.  `dispose` first -- the
    mask is None before it.

    Blanks keep their length and their line endings, so a masked construct
    stays a boundary and a report coordinate stays findable; a drop does not,
    which is why the result carries `Stream.at`.  Where a span of each kind
    overlaps, the DROP wins: that a construct renders nothing is a fact of
    the spec, while a blank is this checker's policy about text that IS
    rendered.  ⚠ The case that USED to settle it -- a link tail holding a
    `.md` file token, where blanking the token inside the dropped tail would
    leave the tail's two sides apart though the document reads them as one --
    CAN NO LONGER ARISE: since R24 the file and citation tokens are read off
    the RENDERING, where the tail is already gone, so no `file` span is
    emitted inside one.  The ordering therefore stands with no witness, and
    the mutant that proved it is retired (that row in
    `plan_memo_selftest_mutants_inline.py` carries the re-runnable
    measurement).  It is kept because `disp` is a max over 0 < 1 < 2 and the
    ordering is what makes that max total -- not because a case exercises it.
    If you find a shape where a blank span overlaps a drop span it belongs
    here as a control; the probe is `lx = Lexed(t); lx.resolve({});
    dispose(lx, keep)`, then intersect the positions of the spans whose
    `RENDERS_TEXT[kind]` is true with those of the rest.  Four shapes were
    tried when this was written -- a link with a `.md` destination, a numeric
    reference inside a destination, a code span in link text, an image with a
    `.md` destination -- and none overlapped.  Four is a sample, not a proof.

    `reader=True` is the SAME rendering with the blanks filled in by what a
    reader sees there (`_inner`): not a text any predicate reads -- I-A is
    the disposition, and a quoted marker declares nothing -- but the one the
    residue detector compares against, since the difference between the two
    IS everything this checker refuses to read (`split_units`)."""
    assert lx.mask is not None, "stream() before dispose(): the Population has not run yet"
    text = lx.text
    disp = bytearray(len(text))         # 0 = text, 1 = blank, 2 = drop
    for a, b, kind in lx.mask:
        v = 1 if RENDERS_TEXT[kind] else 2
        for k in range(a, b):
            if v > disp[k]:
                disp[k] = v
    # A §2.4 escape and a §2.5 reference are the two spellings of "this text
    # renders as that character", and both substitute -- EXCEPT where the
    # character would spell a DECORATION the document does not have: the
    # stream carries exactly one kind of markup, the id decoration a kept
    # span stands for (`dispose`), and `\*\*C\*\*` / `&#42;&#42;C&#42;&#42;`
    # are text, not the bold `**C**` no reader sees (measured: the umbrella
    # memo quotes a `grep` pattern in that shape).
    #
    # Those spans are BLANKED, which is the third disposition and the only
    # one that is true of them: the construct renders TEXT (one punctuation
    # character) that this checker refuses to read (reading it would spell
    # markup the document does not have), which is exactly what a blank
    # says.  Standing as WRITTEN was the fourth, and it is not a disposition
    # at all -- it leaves the entity's SOURCE letters where the id scanner
    # reads them, so a row `ast` was named by every `&ast;` in the document
    # and a row `42` by every `&#42;` (PR #510 R23; the mirror of the `Slice
    # 9**z**` fabrication design re-gate 4 closed, and the same rule closes
    # both: what stands in the stream is what the document RENDERS).
    # Dropping them is the other wrong answer: `9&ast;z` renders `9*z`, and
    # a drop would join the two sides into the id `9z` a reader does not
    # read.  A blank keeps the span's length and its coordinates, and no id
    # can straddle one of these: the character it stands for is a `*` or a
    # backtick, which is in no id token's character class, so the residue
    # (`_units`) cannot fire on it -- proved by the class, not by luck.
    subst, decor = {}, {}
    for a, b, ch in lx.subst:
        (decor if ch in DECOR_CHARS else subst)[a] = (b, ch)
    for a, (b, _ch) in decor.items():
        if disp[a] == 0:                # inside a dropped or blanked span, that span wins
            for k in range(a, b):
                disp[k] = 1
    # start -> (end, the text a READER sees there): the outermost blank span
    # at each start (an inner span of a blanked one is inside its text, not
    # beside it), read by the reader's rendering alone
    blank_at = {}
    if reader:
        for a, b, kind in lx.mask:
            if RENDERS_TEXT[kind] and disp[a] == 1 and (a not in blank_at or blank_at[a][0] < b):
                blank_at[a] = (b, _inner(kind, text[a:b]))
        for a, (b, ch) in decor.items():
            # what a reader sees where a decoration spelling is blanked is
            # the one character it renders; a span of `lx.mask` that starts
            # here instead (a `.md` token can) is the outer construct and
            # keeps it
            if a not in blank_at and disp[a] == 1:
                blank_at[a] = (b, ch)
    buf, src, blanks, i, n = [], [], [], 0, len(text)
    while i < n:
        if disp[i] == 0 and i in subst:
            end, ch = subst[i]
            buf.append(ch)
            src.extend([i] * len(ch))
            i = end
            continue
        if disp[i] == 0:
            buf.append(text[i])
            src.append(i)
        elif disp[i] == 1:
            if reader and i in blank_at:
                end, body = blank_at[i]
                blanks.append((len(buf), len(buf) + len(body)))
                buf.extend(body)
                src.extend([i] * len(body))
                i = end
                continue
            if not reader and (not blanks or blanks[-1][1] != len(buf)):
                blanks.append((len(buf), len(buf) + 1))
            elif not reader:
                blanks[-1] = (blanks[-1][0], len(buf) + 1)
            buf.append("\n" if text[i] == "\n" else " ")
            src.append(i)
        i += 1
    src.append(n)
    return Stream("".join(buf), src, blanks)


def _straddles(blanks, a, b):
    """Whether `[a, b)` holds a character inside one of the `blanks` AND one
    outside every one of them -- the unit is read ACROSS a span, as opposed
    to sitting wholly inside one (a quoted marker: I-A's disposition, on
    purpose) or wholly outside every one (the ordinary reading)."""
    inside = sum(max(0, min(b, y) - max(a, x)) for x, y in blanks)
    return 0 < inside < b - a


def _readings(lx):
    """THE TWO READINGS of one block, in the order every reader of them takes
    them: what a READER sees (`reader=True`, the blanks filled in with the
    text the constructs render), and the disposed STREAM this checker's
    predicates read.  The residue is exactly their disagreement, so both
    consumers -- the seed (`split_units`) and the census gate
    (`kind_disagreements`) -- take the pair from here."""
    return stream(lx, reader=True), stream(lx)


def _units(st, keep):
    """The units of ONE rendering `st` that are read ACROSS one of its blanks
    -- an id token of `keep`, or a member of `KIND_PHRASES` -- as (kind,
    text, offset in the block's RAW text)."""
    if not st.blanks:
        return []
    # `ROW_KINDS`, the grammar's closed set, and NOT its complement -- the same
    # spelling the bare and anchored naming passes read (PR #510 R24; this said
    # `t.kind != "cite"`, which agrees exactly today and diverges the moment a
    # kind is added to `KINDS` without being a row kind).
    out = [("id", t.id, st.at(t.idstart)) for t in tokens(st)
           if t.id in keep and t.kind in ROW_KINDS and _straddles(st.blanks, t.idstart, t.idend)]
    for name, rx in KIND_PHRASES:
        out += [(name, m.group(0), st.at(m.start())) for m in rx.finditer(st)
                if _straddles(st.blanks, m.start(), m.end())]
    return out


def split_units(lx, keep):
    """THE RESIDUE, reported rather than decided (the plan's §3.0b): every
    lexical unit read across a span this checker refuses to read as prose, as
    (kind, text, offset in the block's RAW text) -- an id token of `keep`, or
    a `KIND_PHRASES` member -- in EITHER of the two readings.

    Both directions, because the disagreement is symmetric and a rule stated
    over one of them leaves the other authoritative (PR #510 R23).  The
    reader reads a unit the disposed stream does not: `KIND UNDETER`MINED``
    is the undetermined kind to a reader (§6.1: the code span contributes
    `MINED` as plain text) and nothing to the stream.  And the stream reads
    one the READER does not: a blank stands as spaces, so `KIND `x`
    UNDETERMINED` is the undetermined kind to `UNDETERMINED`'s `\\s*` and
    `KIND x UNDETERMINED` -- no kind at all -- to a reader.  An id cannot
    make that second shape (a blank's filler is a space, which bounds every
    id token), so it is the phrases that need the second scan; scanning both
    renderings for both is one rule rather than that carve-out.

    The mechanism that fixes the rest of this class -- the stream IS the
    rendered text -- cannot reach here by construction: inside a code span,
    an autolink, a citation id or a file name the checker does not read the
    rendered text, deliberately (I-A: a quoted marker declares nothing, a
    `9z` in a shell command names no row), so where a unit STRADDLES such a
    span the two readings disagree and neither is the checker's to pick.
    §1 forbids a clean exit for a could-not-scan, so the disagreement is
    printed; and where it decides a gating census -- a kind phrase in a row's
    declaring field -- `Population._kind_residue` raises it as a schema miss
    instead (`plan_memo_population.py`).

    The comparison is exact and needs no threshold: both readings come from
    the ONE builder, and a unit is in the residue exactly when its extent
    covers characters on both sides of a blank's edge.  A unit that straddles
    in BOTH readings is one disagreement and is reported once: the two
    renderings map it back to the same raw offset, so the record is the
    same, and the reader's spelling of it is the one reported."""
    seen = {}
    for st in _readings(lx):
        for kind, text, off in _units(st, keep):
            seen.setdefault((kind, off), (kind, text, off))
    return list(seen.values())


def kind_disagreements(lx):
    """THE CENSUS QUESTION the residue answers: the `KIND_PHRASES` the two
    readings of `lx` disagree about, BECAUSE one of them reads the phrase
    across a blank -- the gating half of `split_units`, asked per phrase
    (`Population._kind_residue`).

    Both conjuncts are load-bearing, and each is a measured case:

      * the readings must DISAGREE about the phrase.  A field that spells a
        phrase cleanly somewhere reads the same kind under both renderings
        even if it straddles a blank elsewhere, and the census is not in
        doubt;
      * and the disagreement must come from a STRADDLE.  A phrase quoted
        WHOLE (`` `UMBRELLA, not a terminal unit` ``) also makes the two
        readings differ -- and is I-A's deliberate disposition, not a
        could-not-scan: a quoted phrase declares nothing, and that is a
        decision, not a doubt."""
    rd, st = _readings(lx)
    out = []
    for name, rx in KIND_PHRASES:
        hit = [(rd.blanks, m) for m in rx.finditer(rd)]
        other = list(rx.finditer(st))
        if bool(hit) == bool(other):
            continue
        hit = hit or [(st.blanks, m) for m in other]
        if any(_straddles(blanks, m.start(), m.end()) for blanks, m in hit):
            out.append(name)
    return out
