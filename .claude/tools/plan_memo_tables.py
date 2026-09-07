#!/usr/bin/env python3
"""Row inventory for `plan-memo-umbrella-check.py` -- tables, rows, ids, kinds,
and the one transitive `Population` every scan and assertion reads.

Everything here answers "what rows does this document set have, what is each
one's id, and what kind does its declaring field declare?".  The lexical
substrate (Phase 1 blocks: `plan_memo_blocks.py`; Phase 2 inline: `plan_memo_lexer.py`) is the lexer's;
what the prose says about the rows, and whether that is allowed, is the
checker and `plan_memo_roles.py`.

Three rules decided here, once:
  * a table is admitted in `admit_table` and nowhere else: header and
    delimiter rows of equal width (GFM §4.10), and -- local policy over GFM's
    "may vary" clause -- a body row of a SCHEMA table whose width differs from
    the header is a schema miss (exit 2), never a silent skip or a shifted read;
  * row ids are read from the raw id cell (`bare_id` reads the one decorated-id
    grammar at the cell's START; a non-empty cell that does not start with an
    id is an UNKEYED row and a schema miss), kind markers from the disposed
    declaring field (a quoted marker declares nothing);
  * the population is transitive over the memos a memo links; the same id
    declared twice, and a reference no definition answers (the memo it meant
    to link is outside the population), are schema misses.

Every block is lexed ONCE, where it is minted (a cell in `split_row`, a
paragraph in `Paragraph`); `Memo` resolves the links; the disposition step
(`Population`) then tags each block's mask with the kind of every span --
`code` / `def` / `link` / `cite` / `file` -- minus the id-only code spans,
and `stream(lexed)` (every masked span blanked) is the ONE text each
predicate over a block reads.
"""

import bisect
import pathlib
import re
from urllib.parse import unquote

from plan_memo_blocks import (
    block_end, container_text, definition_block, delimiter_width, is_blank, is_setext_underline,
    list_item_line, one_line_block, quote_content, raw_extent, raw_opener, run_end, split_row,
    starts_block, table_header_at,
)
from plan_memo_lexer import Lexed, blank_spans, normalize_label

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
# Id grammar, spelled ONCE.  An id is a short alphanumeric token, a `#11-`
# slug, or a `[C19]`-style citation id (the citation table's id column) --
# nothing else -- and may be decorated with bold, backticks, or both, in
# either order.  `decorated_id(core, tag)` is the one spelling every reader
# uses (the id cell, the prose anchor, the bare cell token, the owner
# reference); it binds groups `<tag>l` / `<tag>id` / `<tag>r`, and
# `balanced(m, tag)` says whether the decoration closes what it opened.
# --------------------------------------------------------------------------

SHORT_ID = r"[0-9A-Za-z]{1,4}"
SLUG_ID = r"#11-[a-z0-9-]+"
CITE_ID = r"\[[A-Z][0-9]+\]"
DECOR = r"(?:\*\*|`)*"


def decorated_id(core, tag=""):
    return r"(?P<%sl>%s)(?P<%sid>%s)(?P<%sr>%s)" % (tag, DECOR, tag, core, tag, DECOR)


_DECOR_TOKENS = re.compile(r"\*\*|`")


def balanced(m, tag=""):
    """The decoration around `<tag>id` closes, in reverse order, exactly what
    it opened -- `**x**`, `` `x` ``, `` **`x`** `` -- and is not empty."""
    left = _DECOR_TOKENS.findall(m.group(tag + "l"))
    return bool(left) and _DECOR_TOKENS.findall(m.group(tag + "r")) == left[::-1]


ROW_NOUN = r"(?:Slices?|slices?|Rows?|rows?|Umbrellas?|umbrellas?)"
"""How this document names a row when it refers to one.  Lives here because it
is a fact about row IDENTITY -- `attributed_to_other` needs it to decide whose
kind a marker declares."""

# `Slice-M` / `Slice-4a` are the same anchor with a hyphen.  Requiring `\s+`
# left them invisible to both passes; measured, four of five such sites in this
# memo are real violations.
ROW_NOUN_ID = ROW_NOUN + r"[ \t\n-]+" + decorated_id(SHORT_ID)   # ASCII separators (`\s` is Unicode)

# The id cell: the grammar at the cell's START, then a non-id character (so
# `**7z** — MERGED` and `` `#11-x` (carved from #483) `` read `7z` / `#11-x`,
# and `xxxxC` is not `C`).  A cell that does not start with an id is not an
# id cell.
_ID_CELL = re.compile("^" + decorated_id("(?:%s|%s|%s)" % (SLUG_ID, CITE_ID, SHORT_ID))
                      + r"(?![0-9A-Za-z-])")
_SLUG_IN_CODE = re.compile(r"(?<![0-9A-Za-z_-])" + SLUG_ID)     # an ASCII class, not `\w` (Unicode)

# An id-only code span is tokenised by the declared-id GRAMMAR, longest
# alternative first (a `#11-` slug is atomic -- its internal hyphens are not
# separators), with the separators whitespace, list punctuation, `|` and `-`
# (a `Deps`-shaped edge, `9z | 7z` / `0a-0b`) between tokens.  A bare id in
# a cell or in prose is NOT tokenised on a list -- it is bounded by the
# complement of the id-continuation class (the checker's `_ID_CONTINUES`); a
# hyphen bounds a short id, and `slice-9z-sib.md` is safe because a file
# name is a lexer `file` token, masked before the scan.
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
    number of cells.  If a number of cells fewer than the number of cells in
    the header row, empty cells are inserted.  If greater, the excess is
    ignored" -- a NON-schema row is cut to the header's width here, before
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
    """The row id an id cell declares: the decorated-id grammar at the cell's
    start, decoration stripped, trailing prose ignored -- or None when the
    cell does not start with an id (then the row declares nothing)."""
    g = _ID_CELL.match(cell_text.strip(" \t"))
    return g.group("id") if g else None


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
    command line, is the document spelling an id: the same exception) -- the
    spans a reader of prose must skip."""
    out = []
    for a, b in lx.code:
        if id_only(lx.text[a:b].strip("`"), keep):    # whitespace is a separator token
            continue
        cut = a
        for m in _SLUG_IN_CODE.finditer(lx.text, a, b):
            if m.group(0) in keep:
                out.append((cut, m.start()))
                cut = m.end()
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


class Paragraph:
    """The lines of one run no definition consumed, grouped by the driver
    (`Memo._parse`) at the block boundaries; `lexed.text` is the inline
    content code spans are lexed over, and `offsets` maps each line to its
    start offset in it (a line is a reporting coordinate only)."""

    __slots__ = ("lines", "offsets", "lexed")

    def __init__(self, numbered):
        self.lines = numbered                       # [(lineno, text)]
        self.lexed = Lexed("\n".join(t for _, t in numbered))
        self.offsets = []
        off = 0
        for _, t in numbered:
            self.offsets.append(off)
            off += len(t) + 1

    def locate(self, i):
        """Offset `i` of the content -> (lineno, line text, column).  A
        bisect over the line offsets: a linear scan per call made every
        per-site lookup quadratic in the paragraph's length (8,000 reference
        lines: 6.4 s, 16,000 calls)."""
        k = bisect.bisect_right(self.offsets, i) - 1
        lineno, text = self.lines[k]
        return lineno, text, i - self.offsets[k]


class Memo:
    """One memo, in the two phases of CommonMark's "Appendix: A parsing
    strategy".  Phase 1 (block structure, over RAW lines, `_parse`: ONE
    forward pass, the way the Appendix reads a document line by line with
    its open block in hand, re-entered once per block quote over the quote's
    content): the raw extents -- indented code (§4.4), fenced blocks (§4.5)
    and HTML blocks (§4.6), one opener rule and one extent rule, consumed in
    place -- block quotes (§5.1, the one container: marker lines stripped,
    lazy continuation lines gathered, the same pass over the content), GFM
    tables (§4.10, ending at a blank line or any block start), runs and
    their reference definitions (§4.7 -- a block of its own, recognised
    only at a block start; a definition-shaped line INSIDE a paragraph is an
    orphan, recorded in `orphans`), paragraphs.  Phase 1 STATES its result
    as a block sequence (`sequence`: `[kind, first raw line, last raw line]`
    in document order, a quote before its content) -- the claim the
    conformance control consumes against the spec's html.  Phase 2 (inline
    structure, `Lexed.resolve` / `inline_pass`) then runs over each
    paragraph's and cell's content only, with `defs` from the Phase-1
    definition blocks."""

    def __init__(self, path):
        self.path = pathlib.Path(path)
        self.text = self.path.read_text(encoding="utf-8")   # not the locale's codec
        self.lines = self.text.split("\n")
        if len(self.lines) > 1 and self.lines[-1] == "":
            self.lines.pop()        # a line ending ENDS the last line (§2.1); it begins no empty one
        self.tables = []            # [Table], in document order
        self.sequence = []          # Phase 1's block sequence: [kind, first lineno, last lineno]
        self.defs = {}              # normalised label -> destination (§4.7: the first wins)
        self.orphans = {}           # normalised label -> {1-based line numbers}
        self.raw_html = []          # [(lineno, content line)] every line of an HTML block (§4.6)
        self.item_code = []         # [(lineno, line)] indented code right after a list item's paragraph
        self.paragraphs, _ = self._parse(self.lines, list(range(1, len(self.lines) + 1)), None)
        for lx in self.lexed():
            lx.resolve(self.defs)

    def _parse(self, lines, linenos, lazy):
        """Phase 1 as ONE forward pass over one container's content `lines`
        (raw line numbers `linenos`; `lazy[i]`, None at the document level,
        marks a block quote's line that carried no marker -- a §5.1 lazy
        continuation candidate), the block STATE in hand -- which of a raw
        extent, a table or a run is open -- and every boundary decided by
        the ONE predicate `block_end` (a line inside a run was already found
        not to be one, with a paragraph open; a line outside a run is asked
        with none open, so a type-7 HTML opener or an indented line after a
        table, a one-line block or a setext heading opens a raw extent there
        and stays paragraph text after a run line).  Returns (paragraphs,
        lines consumed): the pass STOPS at a lazy candidate where no
        paragraph is open -- such a line is content only as paragraph
        continuation text, so the quote ends there (`> # h\\nlazy` is a
        heading in the quote and a paragraph after it).  Each line is
        classified once, in order:

          * a raw-extent opener (`raw_opener`; indented code, fences and HTML
            blocks share the one rule): its lines to `raw_extent` are never
            inline-parsed and are consumed here; an HTML block's lines are
            recorded in `raw_html` for the LEX-UNSUPPORTED? seed, so their
            content is printed rather than assumed -- as are the lines of an
            indented code block opened while a LIST ITEM's paragraph is the
            last block (blank lines between; `item_code`): under §5.2 that
            is the item's next paragraph (Example 108 `- foo\\n\\n    bar`),
            under LEXED-FLAT a code block, the one place the flat reading
            hides prose, so it is seeded rather than assumed;
          * a blank line ends the paragraph;
          * a `>` line opens a block quote: `_quote`, the same pass over
            its content;
          * a GFM table header off a block start: `admit_table`, the one
            admission site;
          * a setext underline after paragraph text (§4.3; not after a run
            headed by a list-item line, Example 94 -- `container_text`): the
            paragraph is the heading and the underline closes it, content of
            nothing; with no paragraph open the line is not an underline at
            all (`block_end`'s `para_open` arm) -- `---` is a thematic break
            and `===` paragraph text;
          * otherwise a RUN starts (`run_end`; joined once, each line mapped
            to its offset, the text a definition is parsed over) and its
            lines are read one by one: a definition at a block start is a
            block of its own (`defs`, first wins); a VALID definition that
            cannot take effect because a paragraph is open ("a link
            reference definition cannot interrupt a paragraph") is an
            ORPHAN -- exactly that class: a label-and-colon line that is not
            a valid definition is plain prose (commonmark.js: `[C1]:
            ECMA-262 §1 says so` is a paragraph), and a shortcut naming it is
            exempt; the rest is paragraph text, grouped so that a run start
            begins a new paragraph (a lazy setext-shaped line after a list
            item is the item's text) and a one-line block is a paragraph of
            its own.

        Linear: one lookahead per run, one parse per line."""
        out, cur, i, n = [], [], 0, len(lines)
        run_text, run_off, defs_at = {}, {}, {}
        sequence = self.sequence
        after_item = False      # the last block is a list item's paragraph (LEXED-FLAT)

        def flush(kind="p", last=None):
            nonlocal after_item
            if cur:
                sequence.append([kind, cur[0][0], cur[-1][0] if last is None else last])
                out.append(Paragraph(list(cur)))
                after_item = kind == "p" and list_item_line(cur[0][1])
                cur.clear()

        def definition_at(i):
            # the reference definition starting at content line `i`, parsed
            # over the rest of its run (`definition_block`; once per line)
            if i not in defs_at:
                defs_at[i] = definition_block(run_text[i], run_off[i])
            return defs_at[i]

        while i < n:
            line = lines[i]
            new_run = i not in run_text
            if new_run:
                if lazy is not None and lazy[i]:
                    break       # §5.1: no paragraph is open, so the quote ends here
                # outside a run no paragraph is open: the block state is the
                # run map itself, not a look at the previous line
                opener = raw_opener(line, False)
                if opener is not None:
                    flush()
                    end = raw_extent(lines, i, opener, lazy)
                    if opener[0] == "html":
                        self.raw_html.extend((linenos[k], lines[k]) for k in range(i, end))
                    elif opener[0] == "indented" and after_item:
                        self.item_code.extend((linenos[k], lines[k]) for k in range(i, end))
                    else:
                        after_item = False
                    sequence.append([opener[0], linenos[i], linenos[end - 1]])
                    i = end
                    continue
                if is_blank(line):
                    flush()
                    i += 1
                    continue
                if quote_content(line) is not None:
                    flush()
                    after_item = False
                    i += self._quote(lines, linenos, lazy, i, out)
                    continue
                if not starts_block(line) and table_header_at(lines, i, lazy):
                    flush()
                    after_item = False
                    t, end = admit_table(self, lines, linenos, i, lazy)
                    self.tables.append(t)
                    sequence.append(["table", linenos[i], linenos[end - 1]])
                    i = end
                    continue
                if cur and is_setext_underline(line) and not container_text(cur[0][1]):
                    # §4.3: the paragraph is a heading; the underline closes
                    # it and is not content
                    flush("h1" if line.strip(" \t").startswith("=") else "h2", linenos[i])
                    i += 1
                    continue
                j = run_end(lines, i, lazy)
                text, off = "\n".join(lines[i:j]), 0
                for k in range(i, j):
                    run_text[k], run_off[k] = text, off
                    off += len(lines[k]) + 1
            d = definition_at(i)
            if d is not None and not cur:
                # a block start: the definition is a block of its own
                raw, dest, stop = d
                self.defs.setdefault(normalize_label(raw), dest)
                consumed = run_text[i][run_off[i]:stop]
                k = consumed.count("\n") + (0 if consumed.endswith("\n") else 1)
                sequence.append(["def", linenos[i], linenos[i + k - 1]])
                after_item = False
                i += k
                continue
            if d is not None:
                # a valid definition that cannot take effect: the orphan
                self.orphans.setdefault(normalize_label(d[0]), set()).add(linenos[i])
            elif new_run and not (cur and is_setext_underline(line) and not one_line_block(line)):
                # a run start begins a paragraph -- unless it is the lazy
                # `==` after a list item (Example 94), which is that
                # paragraph's text
                flush()
            cur.append((linenos[i], line))
            kind = one_line_block(line)
            if kind:
                flush(kind)
            i += 1
        flush()
        return out, i

    def _quote(self, lines, linenos, lazy, i, out):
        """The block quote opening at content line `i` (a `>` line, §5.1)
        -> the number of lines it spans; its paragraphs are appended to
        `out`, its tables / definitions / raw lines / sequence entries to
        this memo's like every other block's.  The content: every marker
        line stripped (`quote_content`) and, between and after the marker
        lines, the lines without a marker as LAZY CONTINUATION CANDIDATES --
        content only as paragraph continuation text ("the result of
        deleting the initial block quote marker from one or more lines in
        which the next character after the marker is paragraph continuation
        text is a block quote with Bs as its content") -- run through the
        SAME `_parse`, so a definition inside registers (Example 218), a
        table inside is a table, a raw extent inside is raw, a paragraph
        inside is a paragraph at its real line, and a nested quote is the
        same again.  Where a candidate is a boundary even with a paragraph
        open (`block_end`: a blank line, a fence, a heading, a list item --
        never a setext underline, Example 93) no quote reaches it, so the
        candidates gathered here stop there; `_parse` stops earlier at a
        candidate where no paragraph is open (after a raw extent, a table, a
        heading: Examples 128 / 174).  Linear: each line of the document is
        gathered once per enclosing quote, never re-scanned across quotes."""
        n = len(lines)
        content, nos, inner_lazy, j = [], [], [], i
        while j < n:
            rest = quote_content(lines[j])
            if rest is None:
                content.append(lines[j])
                nos.append(linenos[j])
                inner_lazy.append(True)
                if block_end(content, len(content) - 1, True, inner_lazy):
                    content.pop()
                    nos.pop()
                    inner_lazy.pop()
                    break
            else:
                content.append(rest)
                nos.append(linenos[j])
                inner_lazy.append(False)
            j += 1
        entry = ["quote", linenos[i], None]
        self.sequence.append(entry)
        paragraphs, used = self._parse(content, nos, inner_lazy)
        out.extend(paragraphs)
        entry[2] = nos[used - 1]
        return used

    @property
    def key(self):
        """The memo's identity for every per-memo map (mention identity, the
        seeds' row maps): the RESOLVED path.  Two memos in different
        directories may share a basename, and a map keyed on the basename
        aliases their rows; the basename (`path.name`) is for display only."""
        return str(self.path)

    def lexed(self):
        """Every lexed block of this memo: each cell of each table row (header
        rows too), then each paragraph."""
        for t in self.tables:
            for row in [t.header] + t.rows:
                for cell in row.cells:
                    yield cell.lexed
        for p in self.paragraphs:
            yield p.lexed

    def sibling_path(self, dest):
        """The ONE destination -> sibling mapping: the memo on disk a link
        destination names, or None when it names none.  POLICY (CommonMark
        §6.3 / GFM say nothing about siblings on disk): a sibling is a
        RELATIVE `.md` path beside this memo.  Stages, in spec order:
          (a) the scheme test on the RAW path component -- per WHATWG URL a
              scheme is read before percent-decoding, so `notes%3Achild.md`
              has no scheme: it is the local file `notes:child.md`;
          (b) percent-decode (`slice%20sib.md` is `slice sib.md`, as
              `<slice sib.md>` is);
          (c) the DECODED name must not be absolute (`/x`, `//host/x` -- a
              site URL joined to the memo's directory would probe the host's
              filesystem root) nor hold a C0 control / DEL (`child%00.md`
              would make `resolve()` raise);
          (d) the `.md` suffix;
          (e) `resolve()` beside the memo (`_resolve`); an `OSError` or --
              on Python 3.9-3.12, for a symlink loop -- a `RuntimeError`
              there makes the sibling UNAVAILABLE: the joined, unresolved
              path is returned and the population's one I/O chokepoint
              reports it as an unavailable linked memo (exit 2), never a
              crash, never a silent drop.
        """
        raw = re.split(r"[#?]", dest, 1)[0]
        if _SCHEME.match(raw):                                       # (a)
            return None
        name = unquote(raw)                                          # (b)
        if name.startswith("/") or _CONTROL.search(name):            # (c)
            return None
        if not name.endswith(".md"):                                 # (d)
            return None
        return _resolve(self.path.parent / name)                     # (e)

    def linked_files(self):
        """Every sibling this memo links (`sibling_path`) -- from any block,
        cells included -- in first-link order, each once, the memo itself
        excluded."""
        out, seen = [], {_resolve(self.path)}
        for lx in self.lexed():
            for _, _, dest in lx.links:
                f = self.sibling_path(dest)
                if f is not None and f not in seen:
                    seen.add(f)
                    out.append(f)
        return out

    def unresolved_references(self):
        """[(lineno, label)]: every full / collapsed reference no definition
        answers, plus every shortcut whose label has a definition the grammar
        could not read (a §4.7 definition cannot interrupt a paragraph) -- the
        sites where a population the author meant to link is lost."""
        orphans, out = self.orphans, []

        def walk(lx, lineno_of):
            for off, label, form, is_image in lx.unresolved:
                if is_image:
                    continue     # §6.4: literal image syntax; an image never links a memo
                key = normalize_label(label)
                # a `[C19]`-style citation id is never a memo reference, in ANY
                # form -- shortcut, full (`[C19][C20]` adjacent citations) or
                # collapsed (`[C19][]`); a plain shortcut of any other label is
                # prose too.  An orphan definition of the label (one the grammar
                # could not read) is still reported, citation or not, except at
                # the definition's own bracket.
                # the FORM comes from the lexer's one bracket parse (escapes
                # honoured); a raw re-walk here once read `[foo\]][missing]`
                # as a shortcut and exempted it.  Each site is recorded once
                # there: the label bracket of a failed `[text][label]` is
                # re-scanned (§6.3 Example 571) but not recorded again
                exempt = _CITE_LABEL.fullmatch(key) is not None or form == "shortcut"
                lineno = lineno_of(off)
                if exempt and (key not in orphans or lineno in orphans[key]):
                    continue
                out.append((lineno, label))

        for p in self.paragraphs:
            walk(p.lexed, lambda off, p=p: p.locate(off)[0])
        for t in self.tables:
            for row in [t.header] + t.rows:
                for cell in row.cells:
                    walk(cell.lexed, lambda off, row=row: row.lineno)
        return out

    def schema_rows(self, name):
        """[Row] body rows of every table matching schema `name`."""
        return [r for t in self.tables if t.schema is not None and t.schema.name == name for r in t.rows]


_SCHEME = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")


def _resolve(path):
    """`path.resolve()`, or `path` itself when resolving raises -- `OSError`
    (an over-long name) or, on Python 3.9-3.12, `RuntimeError` for a symlink
    loop (3.13 made that an `OSError`).  The ONE site that guards it; the
    unresolved path then reaches the population's I/O chokepoint as an
    unavailable memo."""
    try:
        return path.resolve()
    except (OSError, RuntimeError):
        return path
_CONTROL = re.compile(r"[\x00-\x1f\x7f]")
# `CITE_ID` without its brackets, over a NORMALISED (casefolded) label
_CITE_LABEL = re.compile(r"[a-z][0-9]+")


class Population:
    """The memo set reachable from one memo through its links (visited set, so
    a cycle is not an error), with ONE map `ids`: id -> its declaring `Row`
    (`row.kind` = "umbrella" / "undetermined" / "pointer" / "terminal").  `misses` holds
    every schema miss (absent memo, unresolved reference, unmatched schema, row
    width, duplicate declaration, unkeyed schema row); a non-empty `misses`
    is exit 2 -- never a clean run.
    """

    def __init__(self, main_path):
        self.memos = []
        self.misses = []            # [(file, lineno, message)]
        self.spellings = set()
        self.attributed = []        # [(file, table, lineno, rid, other)]
        self.ids = {}
        queue, seen = [_resolve(pathlib.Path(main_path))], set()
        while queue:
            p = queue.pop(0)
            if p in seen:
                continue
            seen.add(p)
            # the ONE I/O chokepoint: a memo that cannot be opened, read or
            # decoded (absent, a directory, over-long, invalid UTF-8) is an
            # UNAVAILABLE linked memo -- the documented exit-2 miss, never an
            # exception out of the population
            try:
                memo = Memo(p)
            except (OSError, RuntimeError, UnicodeDecodeError) as e:
                self.misses.append((p.name, 0, "linked memo unavailable (%s) -- its population is "
                                    "unscanned" % type(e).__name__))
                continue
            self.memos.append(memo)
            queue.extend(memo.linked_files())
            # a reference no definition answers is prose under §6.3, and the
            # memo it meant to link is NOT in the population: never a clean run
            for lineno, label in memo.unresolved_references():
                self.misses.append((memo.path.name, lineno,
                                    "unresolved reference %r -- no definition answers it, so a memo "
                                    "it meant to link is NOT in the population" % label))
        self.main = self.memos[0] if self.memos else None
        if self.main is not None:
            matched = {t.schema.name for t in self.main.tables if t.schema is not None}
            for s in SCHEMAS:
                if s.name not in matched:
                    self.misses.append((self.main.path.name, 0,
                                        "no table matched schema %r -- its whole population is unscanned" % s.name))
        for memo in self.memos:
            for t in memo.tables:
                for lineno, msg in t.misses:
                    self.misses.append((memo.path.name, lineno, msg))
        # ids first (the keep-set the disposition needs), then masks, then kinds
        for memo in self.memos:
            self._declare(memo)
        keep = self.keep()
        for memo in self.memos:
            for lx in memo.lexed():
                dispose(lx, keep)
        for row in self.declaring_rows():
            row.field = stream(row.cells[row.schema.decl].lexed)
        for row in self.ids.values():
            row.kind = self._kind(row)

    # -- declarations ------------------------------------------------------

    def _declare(self, memo):
        for s in SCHEMAS:
            if s.idc is None:
                continue
            for row in memo.schema_rows(s.name):
                rid = row.self_id
                if rid is None:
                    # a LITERAL blank id cell is a deliberate non-row; anything
                    # else that is not an id is an UNKEYED row: it would be dropped
                    # from `ids`, so assertion (b) would never see its Deps
                    # edge -- the I-C silent-skip class, and a schema miss
                    if not is_blank_id_cell(row.id_cell()):
                        self.misses.append((memo.path.name, row.lineno,
                                            "the %r row's id cell does not start with an id (%r); "
                                            "the row declares nothing and is unkeyed, so its cells "
                                            "would go unasserted" % (s.name, row.id_cell()[:60])))
                    continue
                if rid in self.ids:
                    r2 = self.ids[rid]
                    self.misses.append((memo.path.name, row.lineno,
                                        "row %r is declared twice (also %s:%d in %r); a population "
                                        "with two declarations of one id cannot be scanned"
                                        % (rid, r2.memo.path.name, r2.lineno, r2.schema.name)))
                    continue
                self.ids[rid] = row

    def _kind(self, row):
        """The kind the row's masked declaring field declares.  The
        undetermined SPELLING is collected independently of the marker (a row
        can carry both; its kind stays umbrella, its spelling still joins
        `spellings`).  A row whose marker is attributed to another row is a
        POINTER (§5: a pointer slot carries no marker of its own -- assertion
        (a) reports it), as is a row that says so in words."""
        if row.field is None:
            return "terminal"
        m = UNDETERMINED.search(row.field)
        if m:
            self.spellings.add(m.group(0))
        if MARKER in row.field:
            other = attributed_to_other(row.field, row.self_id)
            if other:
                self.attributed.append((row.memo.path.name, row.schema.name, row.lineno, row.self_id, other))
                return "pointer"
            return "umbrella"
        if m:
            return "undetermined"
        if POINTER.search(row.field):
            return "pointer"
        return "terminal"

    # -- inventories -------------------------------------------------------

    def ids_of_kind(self, kind):
        return {rid: r for rid, r in self.ids.items() if r.kind == kind}

    def no_owner_ids(self):
        """Every row that carries no owner and no ordering -- the property §5's
        naming rule is stated over (umbrella + kind-undetermined)."""
        return {rid: r for rid, r in self.ids.items() if r.kind in ("umbrella", "undetermined")}

    def data_rows(self, name):
        """[Row] over every memo, for schema `name`."""
        return [r for memo in self.memos for r in memo.schema_rows(name)]

    def declaring_rows(self):
        """Every row of a schema with a declaring field and an id column, over
        every memo -- the set `field` is written for and read over (assertion
        (a)).  It is NOT `ids.values()`: a row whose id cell is EMPTY (`**—**`)
        is a deliberate non-row, unkeyed and outside `ids`, but its declaring
        field is still read (the #506 memo has one such row, the
        `Function`/`eval` row); a non-empty non-id cell is a schema miss, so
        after `misses` those two sets differ by exactly the empty-id rows."""
        return [r for s in SCHEMAS if s.decl is not None and s.idc is not None
                for r in self.data_rows(s.name)]

    def keep(self):
        """The code-span keep-set: every declared id, from every memo."""
        return set(self.ids)
