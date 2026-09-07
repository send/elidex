#!/usr/bin/env python3
"""Row grammar for `plan-memo-umbrella-check.py` -- how a row is named and
keyed (the id-token grammar itself is `plan_memo_ids.py`'s), the table
schemas, `Row` / `Table` and the ONE admission site `admit_table`, the kind
markers, and the mask disposition every scanner reads through.

Everything here answers "what is a row, what is its id, and what kind does
its declaring field declare?".  The lexical substrate (Phase 1 blocks:
`plan_memo_blocks.py`; Phase 2 inline: `plan_memo_lexer.py`) is the lexer's;
the document driver and the transitive memo set (`Memo` / `Population`) are
`plan_memo_memo.py`'s, which imports this module and never the reverse;
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
kind of every span -- `code` / `def` / `link` / `cite` / `file` -- minus the
id-only code spans, and `stream(lexed)` (every masked span blanked) is the
ONE text each predicate over a block reads.
"""

import re

from plan_memo_blocks import block_end, delimiter_width, split_row
from plan_memo_ids import CITE_ID, DECOR, SHORT_ID, SLUG_ID, decorated_id, tokens
from plan_memo_lexer import blank_spans

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


def is_empty(cell_text):
    """Decoration does not fill a cell: `**—**` is as empty as `—`."""
    bare = cell_text.strip(" \t").strip("*`").strip(" \t")
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
    opposed to unkeyed content."""
    return cell_text.strip(" \t").strip("*`").strip(" \t") in ID_CELL_BLANKS


MARKER = "UMBRELLA, not a terminal unit"

# §5's fifth row kind.  A kind-undetermined row carries the split and NOTHING
# else -- no ordering, no owner, no acceptance -- which is the same obligation
# the naming rule enforces against umbrellas.  Two spellings are in use; both
# are tolerated and the divergence is reported (a kind with two spellings is a
# kind no program can enumerate).
UNDETERMINED = re.compile(r"KIND\s*[—-]?\s*UNDETERMINED", re.IGNORECASE | re.ASCII)

# A row that is a POINTER into a slot rather than a slice of its own (§1.0's
# "SCHEDULED FROM ITS OWN SLOT" rows).  ⚠ Keyed on one spelling, and the safe
# polarity: a differently-spelled pointer row is terminal, and so REPORTED by
# the acceptance seed, never missed.
POINTER = re.compile(r"is a pointer rather than a slice")

# --------------------------------------------------------------------------
# Row identity.  The id grammar itself -- the three kinds, the decoration,
# and the ONE boundary every reader consumes (`tokens`) -- is
# `plan_memo_ids.py`'s; what is here is how a ROW is named and keyed.
# --------------------------------------------------------------------------

ROW_NOUN = r"(?:Slices?|slices?|Rows?|rows?|Umbrellas?|umbrellas?)"
"""How this document names a row when it refers to one.  Lives here because it
is a fact about row IDENTITY -- `attributed_to_other` needs it to decide whose
kind a marker declares."""

# `Slice-M` / `Slice-4a` are the same anchor with a hyphen.  Requiring `\s+`
# left them invisible to both passes; measured, four of five such sites in this
# memo are real violations.
ROW_NOUN_SEP = ROW_NOUN + r"[ \t\n-]+"       # ASCII separators (`\s` is Unicode)
ROW_NOUN_ID = ROW_NOUN_SEP + decorated_id(SHORT_ID)

# An id-only code span is tokenised by the declared-id GRAMMAR, longest
# alternative first (a `#11-` slug is atomic -- its internal hyphens are not
# separators), with the separators whitespace, list punctuation, `|` and `-`
# (a `Deps`-shaped edge, `9z | 7z` / `0a-0b`) between tokens.  A bare id in
# a cell or in prose is NOT tokenised on a list -- it is bounded by the
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
    whose text DECLARES the row's kind, and the column holding the row id --
    both named by header cell and resolved to body-column indexes here."""

    __slots__ = ("name", "header", "decl", "idc")

    def __init__(self, name, header, decl=None, idc=None):
        self.name, self.header = name, header
        self.decl = header.index(decl) if decl is not None else None
        self.idc = header.index(idc) if idc is not None else None


SCHEMAS = [
    Schema("citation", ["ID", "Citation", "Anchor", "Used by"], idc="ID"),
    Schema("stub", ["Site", "Syntax", "Emits", "Observable", "Tier", "Slice"]),
    Schema("slice", ["#", "Slice", "Primary module(s)", "Slot", "Tier", "Deps"], decl="Slice", idc="#"),
    Schema("slot", ["Slot", "Why deferred", "Trigger", "Re-eval"], decl="Why deferred", idc="Slot"),
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
        self.self_id = bare_id(cells[schema.idc].text) if schema and schema.idc is not None else None
        self.field, self.kind = None, None

    def id_cell(self):
        """The raw text of this row's id cell (None for a row without one)."""
        return self.cells[self.schema.idc].text if self.schema and self.schema.idc is not None else None

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

def bare_id(cell_text):
    """The row id an id cell declares: the grammar's first bounded token when
    it stands at the cell's START (so `**7z** — MERGED` and `` `#11-x`
    (carved from #483) `` read `7z` / `#11-x`, and `xxxxC` declares nothing
    -- the boundary is the grammar's, `plan_memo_ids.tokens`), decoration
    stripped, trailing prose ignored -- or None when the cell does not start
    with an id (then the row declares nothing)."""
    t = next(tokens(cell_text.strip(" \t")), None)
    return t.id if t is not None and t.start == 0 else None


_APPOSITIVE = re.compile(ROW_NOUN_ID + r"\s*[—–-]\s*" + DECOR + r"\s*$", re.ASCII)


def attributed_to_other(field, rid):
    """The row id a marker names, when it is not this row's own: the marker's
    APPOSITIVE subject -- `Slice **E** — **UMBRELLA, …**` -- with nothing
    between them but dash punctuation and emphasis.  A proximity window
    instead excluded a genuine self-declaration that merely MENTIONED a sibling
    ("Unlike Slice 7z, **UMBRELLA, not a terminal unit.**"); the self-test
    carries both directions.  The FIRST marker occurrence decides: a field
    that declares itself and then says a sibling "is not it" is
    self-declaring, and a later occurrence never overrides the first."""
    m = re.search(re.escape(MARKER), field)
    if m is None:
        return None
    g = _APPOSITIVE.search(field[max(0, m.start() - 70): m.start()])
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


def dispose(lx, keep):
    """Tag `lx.mask`: every span the scanners must not read an id out of, as
    (start, end, kind) -- `code` (minus id-only spans and kept slugs), `link`
    (the tail; the visible text stays, it is prose), `image` (the same, for
    an image), `cite`, `file`.  A reference definition is a Phase-1 block of
    its own, never inline content, so no block holds one to mask."""
    out = [(a, b, "code") for a, b in code_mask(lx, keep)]
    out += [(a, b, "link") for a, b, _ in lx.links]
    out += [(a, b, "image") for a, b in lx.images]
    out += lx.tokens
    lx.mask = out


def stream(lx):
    """`lx.text` with EVERY span of its disposed mask blanked -- code spans
    (id-only spans and kept slugs were excepted there), link tails, citation
    ids, file names.  This is the ONE stream every predicate
    over a block reads: the kind-marker reader (a quoted marker is not a
    declaration), the seeds' vocabularies (a `gates` inside a code span is not
    ordering prose; a `MERGED` inside one is not a retirement), the licensing
    rule's context.  `dispose` first -- the mask is None before it."""
    assert lx.mask is not None, "stream() before dispose(): the Population has not run yet"
    return blank_spans(lx.text, [(a, b) for a, b, _ in lx.mask])
