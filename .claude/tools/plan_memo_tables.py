#!/usr/bin/env python3
"""Row grammar for `plan-memo-umbrella-check.py` -- how a row is named and
keyed (the id-token grammar itself is `plan_memo_ids.py`'s), the table
schemas, and `Row` / `Table` with the ONE admission site `admit_table`.

Everything here answers "what is a row, and what is its id?".  What a block
SAYS -- the mask disposition, the two readings, the kind phrases read out of
them -- is `plan_memo_stream.py`'s, carved out of this module at PR #510 R31
when it reached 1,006 lines (the seam, and the four references that crossed
it, are stated in that module's docstring).  The direction is one way: this
module imports it, for `Table.bind`'s one question about what a header
RENDERS and for the marker its rows are read for.  The lexical substrate
(Phase 1 blocks: `plan_memo_blocks.py`; Phase 2 inline: `plan_memo_lexer.py`)
is the lexer's; the document driver is `plan_memo_memo.py`'s `Memo` and the
transitive memo set `plan_memo_population.py`'s `Population`, both of which
import this module and never the reverse; what the prose says about the rows,
and whether that is allowed, is the checker and `plan_memo_roles.py`.

Two rules decided here, once:
  * a table is admitted in `admit_table` and nowhere else: header and
    delimiter rows of equal width (GFM §4.10), and -- local policy over GFM's
    "may vary" clause -- a body row of a SCHEMA table whose width differs from
    the header is a schema miss (exit 2), never a silent skip or a shifted read;
  * row ids are read from the raw id cell (`bare_id` reads the one decorated-id
    grammar at the cell's START -- raw because it runs before the keep-set it
    declares exists); a cell that starts with no such id is a deliberate blank
    or an UNKEYED row and a schema miss, and THAT half is decided over what a
    reader sees in the cell (`is_blank_id_cell`), because it declares nothing
    and so waits for the disposition; kind markers come from the disposed
    declaring field (a quoted marker declares nothing).

A cell is lexed ONCE, where it is minted (`split_row`), and `Table.bind`
then asks `plan_memo_stream.rendered` which schema the header spells; every
other predicate over a cell reads `plan_memo_stream.stream`, the block as the
document renders it.
"""

import re

from plan_memo_blocks import block_end, delimiter_width, split_row
from plan_memo_ids import BEFORE, DASH, DASH_CLASS, DECOR, ROW_ID, ROW_KINDS, decorated_id, tokens
from plan_memo_stream import MARKER_RE, rendered

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
# blanks are LITERAL here: exactly these four, as a READER sees the cell.
ID_CELL_BLANKS = frozenset({""} | set(DASH))


def is_blank_id_cell(cell_reading):
    """Whether an id cell is a deliberate non-row (a literal blank), as
    opposed to unkeyed content -- decided over WHAT A READER SEES in the cell
    (`stream(reader=True)`), which is the reading its one caller composes
    (`plan_memo_population.Population._unkeyed`), exactly as `is_empty`'s
    caller composes the disposed stream.

    THE READING, and why it is neither of the other two (PR #510 R30).

      * NOT THE RAW CELL.  This predicate used to strip `*` and backticks off
        the raw text UNCONDITIONALLY, justified by "the id cell is the one text
        `bare_id` reads raw as well, because the decoration IS part of the id
        grammar there (`**9z**`), so the two readings of that cell agree by
        being the same reading".  That is true of `**9z**` and false of a
        decoration run the inline grammar pairs NOTHING with: `**`, `*`,
        `` ` ``, ``` `` ``` and `***` each render literally (§6.1 wants a
        closing backtick string of EQUAL length, §6.2 a closing delimiter
        run), so each is a cell carrying content -- and the strip turned every
        one of them into the empty-string blank.  Measured: a slice row whose
        id cell is `**`, with the umbrella marker in its declaring field and a
        nonempty `Deps` edge, exited 0 -- the row entered neither `ids` nor
        assertion (b), which is §1's forbidden clean exit for content that was
        not scanned.  A hand-rolled "peel matched pairs" is the same mistake
        one layer down: ``` `` ``` peels to nothing under the ID grammar's
        decoration walk (two marks, `plan_memo_ids.DECOR_MARKS`) and renders
        literally under CommonMark (one backtick string that closes nothing),
        so the authority for "does this decoration pair" must be the inline
        lexer and not that walk.
      * NOT THE DISPOSED STREAM either, though every other predicate over a
        block reads that one: the stream BLANKS a code span, so `` `?` `` would
        come out empty and read as a deliberate blank -- the silent-skip class
        above, re-opened by the other reading.  A reader sees `?` there, and
        `?` is content.

    `bare_id` still reads the cell RAW, and must: it runs before the keep-set
    exists, because the keep-set is what it declares.  The two never read one
    cell -- this predicate is asked ONLY of a cell `bare_id` found no id at the
    START of -- so what survives of the agreement claim is one direction, and
    it holds: a cell the raw grammar keys never arrives here, and a cell whose
    RENDERING exposes an id the raw grammar could not see (`&#57;z` renders
    `9z`) is reported as unkeyed rather than skipped, which is the polarity §1
    asks for."""
    return cell_reading.strip(" \t") in ID_CELL_BLANKS



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


# --------------------------------------------------------------------------
# Row identity, part two: how a row is NAMED.
# --------------------------------------------------------------------------

# The nouns that name a row without naming its table: a row of ANY schema is a
# `row`, and one whose kind is umbrella is an `umbrella`.  Closed, and closed
# for a reason -- neither is a schema's name, so no schema can supply them.
GENERIC_ROW_NOUNS = ("row", "umbrella")


def _row_nouns():
    """The nouns this document names a row with: the two generic ones and the
    NAME OF EVERY ROW-KEYED SCHEMA.

    DERIVED, not enumerated (PR #510 R27-2).  `ROW_NOUN` was a hand-written
    `slices?|rows?|umbrellas?`, and the §8 slot table -- a schema of this very
    list since before the checker was reviewed -- was not in it, so
    ``Slot `#11-zz-alpha` — **UMBRELLA, not a terminal unit.**`` matched no
    appositive, the marker was read as the containing row's OWN declaration,
    no `UMBRELLA-MARK` was emitted and the census carried a pointer row as an
    umbrella at rc 0.  Adding `slots?` to the literal would have fixed that
    sentence and left the next schema's noun in the same place, which is the
    mistake `KIND_PHRASES`' comment names one screen up: gating the phrase in
    front of you leaves every other member of the class authoritative.

    A schema's `name` IS the noun: it is what the schema-miss finding already
    calls its rows (`the %r header`), and the row-keyed ones -- the schemas
    whose id column keys `ROW_KINDS` ids, which is exactly the set whose rows
    `ROW_NOUN_ID` can name -- are `slice` and `slot` today.  A citation table
    is excluded by the same test that excludes it from `ids`: its column keys
    `cite`, and `Citation `[C1]` — …` names no row.  The plural is `s?` on
    each, and the alternation is longest-first so no noun is read as a prefix
    of another.
    """
    nouns = set(GENERIC_ROW_NOUNS)
    row_kinds = set(ROW_KINDS)
    nouns |= {s.name for s in SCHEMAS if s.kinds and set(s.kinds) <= row_kinds}
    return "(?ai:%s)" % "|".join(re.escape(n) + "s?" for n in sorted(nouns, key=lambda n: (-len(n), n)))


ROW_NOUN = _row_nouns()
"""How this document names a row when it refers to one.  Lives here because it
is a fact about row IDENTITY -- `attributed_to_other` needs it to decide whose
kind a marker declares -- and BELOW `SCHEMAS` because it is derived from them
(`_row_nouns`).  ASCII case-insensitive, in ONE place (the scoped
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
    """One GFM table.  ADMITTED IN PHASE 1 AND BOUND IN PHASE 2 (PR #510
    R31-1): `admit_table` finds its extent and splits its lines into cells,
    all of which is decided over RAW text; `bind` asks which schema it is,
    which is a question about what the header RENDERS and so cannot be asked
    until the cells are lexed and `Memo.defs` is complete.  `pending` holds
    the split body lines in between, and is empty afterwards."""

    __slots__ = ("schema", "header", "width", "rows", "misses", "pending")

    def __init__(self, header, width):
        self.schema = None              # Schema or None; set by `bind`
        self.header = header            # Row (schema None)
        self.width = width              # the delimiter row's cell count
        self.rows = []                  # [Row], body rows only; minted by `bind`
        self.misses = []                # [(lineno, message)] width policy
        self.pending = []               # [(lineno, line, [Cell])] until `bind`

    def bind(self):
        """Decide the schema and mint the rows.  Phase 2 must have resolved
        the header's cells (`Memo.__init__` does, ahead of every other block,
        for this)."""
        rendered_header = [rendered(c.lexed) for c in self.header.cells]
        self.schema = next((s for s in SCHEMAS if rendered_header == s.header), None)
        for lineno, line, body in self.pending:
            if self.schema is not None and len(body) != self.width:
                self.misses.append((lineno, "row has %d cell(s); the %r header has %d -- "
                                    "a shifted read fabricates findings, so this row is "
                                    "unscanned" % (len(body), self.schema.name, self.width)))
            else:
                self.rows.append(Row(self.header.memo, lineno, line, body[:self.width], self.schema))
        del self.pending[:]


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
    203) -- a NON-schema row is cut to the header's width in `bind`, before
    its cells are ever resolved, so an ignored cell's `[x](absent.md)` is
    never a link and its id never a site (PR #510 R13); a SCHEMA row of any
    other width is the schema miss (local policy over "may vary": a shifted
    read fabricates findings).  A short row is not padded: an empty cell
    would hold nothing a scanner reads.

    EVERYTHING DECIDED HERE IS DECIDED OVER RAW TEXT, and that is the right
    text for every one of these questions, not a residue of the reading `bind`
    moved (PR #510 R31-1).  Phase 1 runs over the source lines and Phase 2
    over what a block holds, so a table's SHAPE -- that this line is a header
    (`table_header_at`), that the next one is a delimiter row of N cells
    (`delimiter_width`: "cells whose only content are hyphens"), where each
    cell ends (`split_row`, on unescaped `|`), and how many cells a row has --
    is settled before any inline construct exists.  `&#45;` is a §2.5
    character reference, an INLINE construct, so a delimiter row spelled with
    one is no delimiter row at all and the table never forms; the same goes
    for a `|` written as `&#124;`, which does not split a cell.  What `bind`
    asks is the one question here that was never block-level: which SCHEMA a
    table is, which is this checker's own policy about the names a reader
    sees in the header row.
    """
    n = len(lines)
    t = Table(Row(memo, linenos[i], lines[i], split_row(lines[i]), None), delimiter_width(lines[i + 1]))
    j = i + 2
    while j < n and not block_end(lines, j, False, lazy):
        t.pending.append((linenos[j], lines[j], split_row(lines[j])))
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


# ⚠ `BEFORE` IS LOAD-BEARING AND WAS MISSING UNTIL PR #510 R33-1.  This is
# `.search`ed over the field, and `ROW_NOUN_ID` opens with the row noun, so
# without a left boundary the search could start INSIDE a longer word:
# `Subslice 9z — **UMBRELLA, …**` (and even `xSlice 9z — …`) attributed the
# marker to `9z`, the containing row was read as a POINTER, and a false
# `UMBRELLA-MARK` mechanical failure was emitted.  `NOUN_ANCHOR` had carried
# the same boundary since R24; this composer did not, which is the "spelled
# twice, disagreeing" shape again.
_APPOSITIVE = re.compile(BEFORE + ROW_NOUN_ID + r"\s*" + DASH_CLASS + r"\s*" + DECOR + r"\s*$",
                         re.ASCII)


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
