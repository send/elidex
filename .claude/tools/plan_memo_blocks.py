#!/usr/bin/env python3
"""Phase 1 of CommonMark 0.31.2 "Appendix: A parsing strategy" -- BLOCK
structure -- for `plan-memo-umbrella-check.py`: the line grammar of the
block types -- raw extents (fenced code blocks §4.5, HTML blocks §4.6:
`raw_opener` / `raw_extent`), the block starts that interrupt a paragraph
(§4.1 / §4.2 / §4.3 / §5.1 / §5.2), the ONE block-boundary predicate
`block_end` and the run it bounds (`run_end`), GFM §4.10 table rows, and
link reference definitions (§4.7).  Driven line by line, in ONE forward
pass, by `plan_memo_tables.py::Memo._phase1`, which owns the block state
(what is open: a raw extent, a table, a run) -- this module holds no state
and looks at no previous line.  The inline grammar a definition's label,
destination and title reuse (`link_label`, `link_destination`,
`link_title`, `_skip_ws`, `_escaped`) is Phase 2's, in
`plan_memo_lexer.py`, which this module imports and never the reverse.

Every block type of the spec's closed list (§4 leaf blocks, §5 container
blocks, GFM tables) has a disposition in the plan's §3.0 table: LEXED here,
or PROSE-AS-WRITTEN with a `[LEX-UNSUPPORTED?]` seed (`unsupported_block`).
"""

import bisect
import re

from plan_memo_lexer import (
    Lexed, _escaped, _skip_ws, link_destination, link_label, link_title,
)

# --------------------------------------------------------------------------
# CommonMark §4.5 fenced code blocks
# --------------------------------------------------------------------------

_FENCE_OPEN = re.compile(r"^ {0,3}(`{3,}|~{3,})(.*)$")


def fence_opener(line):
    """The closer pattern of the fenced code block `line` opens (§4.5), or
    None.  Opener: <=3 spaces of indent, >=3 backticks or tildes (not
    mixed); a backtick fence's info string may not contain a backtick.
    Closer: same character, at least as long, <=3 spaces of indent, nothing
    but spaces and tabs after it.  An unclosed fence runs to "the end of the
    containing block (or document)" (§4.5); this lexer has no container
    blocks (see the header), so that is the end of the document
    (`raw_extent`)."""
    m = _FENCE_OPEN.match(line)
    if not m or (m.group(1)[0] == "`" and "`" in m.group(2)):
        return None
    ch, k = m.group(1)[0], len(m.group(1))
    return re.compile(r"^ {0,3}" + re.escape(ch) + "{%d,}" % k + r"[ \t]*$")


# --------------------------------------------------------------------------
# Block starts that end a paragraph or a table (GFM §4.10: "the table is
# broken at the first empty line, or beginning of another block-level
# structure").  ATX headings (CommonMark §4.2) and thematic breaks (§4.1) are
# one-line leaf blocks; a list-item line (§5.2) or a block-quote line (§5.1)
# starts a new paragraph.  ⚠ LOCAL POLICY, stricter than CommonMark: here ANY
# list-item-shaped or `>` line interrupts a paragraph, whereas under §5.2 an
# empty list item cannot interrupt a paragraph, an ordered list item can only
# when its number is 1, and under §5.1 / §5.2 a line without the marker can
# be lazy continuation text of the container.  The policy is the safe side
# for a scanner: a span is never read across such a line, so a backtick
# opened in one item and closed in the next is literal (control).
# --------------------------------------------------------------------------

_ATX = re.compile(r"^ {0,3}#{1,6}(?:[ \t]|$)")
_THEMATIC = re.compile(r"^ {0,3}(?:(?:-[ \t]*){3,}|(?:\*[ \t]*){3,}|(?:_[ \t]*){3,})$")
_LIST_ITEM = re.compile(r"^ {0,3}(?:[-+*]|[0-9]{1,9}[.)])(?:[ \t]|$)")   # §5.2: ASCII digits
_QUOTE = re.compile(r"^ {0,3}>")
# §4.3 setext heading underline: `=` or `-` characters, <=3 spaces of indent,
# trailing spaces/tabs.  Precedence (§4.1 / §4.3): a `-` line after paragraph
# text is the underline, not a thematic break (Example 59); after a list item
# or `>` line it is NOT an underline (Examples 92-94: "cannot be a lazy
# continuation line in a list item or block quote") and stays a thematic
# break / text; with no paragraph before it, `---` is a thematic break and
# `===` is text.
_SETEXT = re.compile(r"^ {0,3}(?:=+|-+)[ \t]*$")
# §4.4 indented code: >=4 spaces at a block start (it cannot interrupt a
# paragraph); §4.6 HTML block start conditions 1-7 (after <=3 spaces).
_INDENTED = re.compile(r"^ {4,}[^ \t]")
_HTML_TAG_NAMES = (
    "address|article|aside|base|basefont|blockquote|body|caption|center|col|colgroup|dd|"
    "details|dialog|dir|div|dl|dt|fieldset|figcaption|figure|footer|form|frame|frameset|h[1-6]|"
    "head|header|hr|html|iframe|legend|li|link|main|menu|menuitem|nav|noframes|ol|optgroup|"
    "option|p|param|search|section|summary|table|tbody|td|tfoot|th|thead|title|tr|track|ul")
# §4.6 start conditions 1-7, each its own group so the END condition can be
# chosen: 1 `</pre>` etc., 2 `-->`, 3 `?>`, 4 `>`, 5 `]]>`, 6 and 7 a blank
# line.  "All types of HTML blocks except type 7 may interrupt a paragraph."
_HTML_BLOCK = re.compile(
    r"^ {0,3}<(?:"
    r"(?P<t1>(?:pre|script|style|textarea)(?:[ \t>]|$))"
    r"|(?P<t2>!--)"
    r"|(?P<t3>\?)"
    r"|(?P<t4>![A-Za-z])"
    r"|(?P<t5>!\[CDATA\[)"
    r"|(?P<t6>/?(?:" + _HTML_TAG_NAMES + r")(?:[ \t>]|/>|$))"
    r"|(?P<t7>(?:[A-Za-z][A-Za-z0-9-]*(?:[ \t]+[A-Za-z_:][A-Za-z0-9_.:-]*(?:[ \t]*=[ \t]*"
    r"(?:[^ \t\"'=<>`]+|'[^']*'|\"[^\"]*\"))?)*[ \t]*/?>|/[A-Za-z][A-Za-z0-9-]*[ \t]*>)[ \t]*$)"
    r")", re.IGNORECASE | re.ASCII)
_HTML_END = {
    "t1": re.compile(r"</(?:pre|script|style|textarea)>", re.IGNORECASE | re.ASCII),
    "t2": re.compile(r"-->"), "t3": re.compile(r"\?>"), "t4": re.compile(r">"),
    "t5": re.compile(r"\]\]>"),
}


def is_blank(line):
    """CommonMark §2.1: "A line containing no characters, or a line
    containing only spaces (U+0020) or tabs (U+0009), is called a blank
    line" -- the ASCII class, not `str.strip()`'s Unicode whitespace (an
    NBSP-only line is paragraph text)."""
    return not line.strip(" \t")


def one_line_block(line):
    return bool(_ATX.match(line) or _THEMATIC.match(line))


def is_setext_underline(line):
    """§4.3: a setext heading underline (the caller supplies the paragraph
    it closes and the precedence above)."""
    return bool(_SETEXT.match(line))


def html_block_type(line):
    """The §4.6 start condition (`"t1"`..`"t7"`) `line` meets, or None."""
    m = _HTML_BLOCK.match(line)
    return m.lastgroup if m else None


def html_block_ends(kind, line):
    """Whether `line` meets the §4.6 END condition of an HTML block of
    `kind` (types 1-5 by content; 6 and 7 end only at a blank line, which
    the caller handles as every block's end)."""
    pat = _HTML_END.get(kind)
    return pat is not None and pat.search(line) is not None


def unsupported_block(line, at_block_start):
    """The PROSE-AS-WRITTEN block type a paragraph line would open under
    CommonMark §5.1 / §4.4 -- "quote" (§5.1), "indented-code" (§4.4, at a
    block start only: it cannot interrupt a paragraph) -- or None.  Phase 1
    reads such a line as paragraph text; the checker reports it as a
    `[LEX-UNSUPPORTED?]` SEED when it holds a `|` or a declared id, so the
    bound of the lexer is printed rather than assumed.  (HTML blocks are
    RAW extents, `raw_opener` / `raw_extent`, seeded by the driver.)"""
    if _QUOTE.match(line):
        return "quote"
    if at_block_start and _INDENTED.match(line):
        return "indented-code"
    return None


def starts_block(line):
    """The NON-RAW block starts that INTERRUPT a paragraph (and so end a run, a
    paragraph and a GFM table) -- CommonMark §4 / §5, each verified against
    commonmark.js 0.31.2: an ATX heading (§4.2), a thematic break (§4.1), a
    list item (§5.2; ⚠ local policy, stricter: any marker line, where the
    spec lets only a non-empty item, an ordered one starting at 1,
    interrupt), a `>` line (§5.1), a setext underline (§4.3: the paragraph
    before it becomes a heading -- `[foo]:\n---` is a heading, not a
    definition).  The RAW block starts -- a fence (§4.5), an HTML block
    opener (§4.6: types 1-6 interrupt, type 7 only where no paragraph is
    open) -- are `raw_opener`'s, the other arm of `block_end`.  NOT in the
    set, by the same oracle: an indented line (§4.4 "cannot interrupt a
    paragraph": `[foo]:\n    code` is a definition with destination `code`)
    and a reference definition (§4.7 "cannot interrupt a paragraph")."""
    return bool(one_line_block(line) or _LIST_ITEM.match(line) or _QUOTE.match(line) or _SETEXT.match(line))


def raw_opener(line, para_open):
    """THE one rule for where a RAW extent begins -- lines never
    inline-parsed, always a run / paragraph / table end: ("fence", closer)
    for a fenced code block opener (§4.5), ("html", type) for a line meeting
    an HTML block start condition (§4.6), else None.  `para_open` is Phase
    1's one context bit, the driver's block state: "all types of HTML blocks
    except type 7 may interrupt a paragraph", so a type-7 opener counts only
    where no paragraph is open -- the document start, after a blank line, a
    raw extent, a TABLE, a one-line block or a setext heading (commonmark.js
    0.31.2: `text\n<span>`, `[foo]: /url\n<span>` and `- item\n<span>` stay
    one paragraph; `# h\n<span>` and `text\n===\n<span>` are a heading and
    a raw block; GFM §4.10: a table "is broken at the first empty line, or
    beginning of another block-level structure", and a table is not a
    paragraph, so `<span>` right after a table's rows opens a block)."""
    closer = fence_opener(line)
    if closer is not None:
        return "fence", closer
    t = html_block_type(line)
    if t is None or (t == "t7" and para_open):
        return None
    return "html", t


def raw_extent(lines, i, opener):
    """End (exclusive) of the raw extent `opener` (from `raw_opener`) opens
    at `lines[i]`: a fence runs through its closer, or to the end of the
    document; an HTML block from the opener to the line meeting its §4.6 end
    condition (types 1-5 by content, possibly the opener itself) or, for
    types 6 and 7, up to the next blank line (excluded) or the end."""
    kind, arg = opener
    n = len(lines)
    if kind == "fence":
        j = i + 1
        while j < n and not arg.match(lines[j]):
            j += 1
        return min(j + 1, n)
    if html_block_ends(arg, lines[i]):      # the opener may meet the end condition itself
        return i + 1
    j = i + 1
    while j < n:
        if arg in ("t6", "t7") and is_blank(lines[j]):
            return j
        j += 1
        if html_block_ends(arg, lines[j - 1]):
            return j
    return n


def table_header_at(lines, i):
    """GFM §4.10: `lines[i]` is a table header iff the next line is a
    delimiter row of the same width.  ⚠ Local policy over pure CommonMark
    (where a table is not a block): a header ends the run and the paragraph
    before it, so `[foo]:\n|9z|7z|\n|--|--|` is a paragraph `[foo]:` and a
    table, not a definition whose destination is the header row."""
    if i + 1 >= len(lines):
        return False
    width = delimiter_width(lines[i + 1])
    return width is not None and len(split_row(lines[i])) == width


def block_end(lines, i, para_open):
    """THE ONE block-boundary predicate (plan §2 I-A): whether line `i`
    begins a block, ending the run / paragraph / table before it -- a blank
    line (§2.1), a raw-extent opener (`raw_opener`: a fenced code block §4.5
    or an HTML block §4.6, where `para_open` -- the ONE context bit, the
    driver's block state -- decides the type-7 case), a paragraph-
    interrupting block start (`starts_block`) or a GFM table header
    (`table_header_at`).  `run_end` (a run: paragraph text, `para_open`
    True) and `admit_table` (a table body: no paragraph is open, False) read
    this and nothing else, and the driver `Memo._phase1` classifies a line
    outside a run by the same four arms; no caller looks back at a previous
    line."""
    return (is_blank(lines[i]) or raw_opener(lines[i], para_open) is not None
            or starts_block(lines[i]) or table_header_at(lines, i))


def run_end(lines, i):
    """End (exclusive) of the RUN starting at `lines[i]` -- Phase 1's text
    unit, the lines a definition is parsed over and a paragraph is grouped
    from: the consecutive lines up to the next `block_end` WITH A PARAGRAPH
    OPEN (a run is paragraph text -- a reference definition included, §4.7
    keeps the paragraph open -- so a type-7 HTML opener does not end it),
    or the one line of a one-line block (§4.1 / §4.2).  A closing setext
    underline is not a run (the driver drops it before asking)."""
    if one_line_block(lines[i]):
        return i + 1
    j = i + 1
    while j < len(lines) and not block_end(lines, j, True):
        j += 1
    return j


# --------------------------------------------------------------------------
# GFM §4.10 row split.  Raw unescaped `|` splits -- "including inside other
# inline spans" -- and `\|` becomes `|` in the cell content (the backslash is
# consumed).  Leading / trailing pipe optional.  GFM: "Spaces between pipes
# and cell content are trimmed" -- ⚠ this splitter trims TABS too (a stated
# widening: a tab-padded cell is the same cell; no memo in the population
# holds a tab).
# --------------------------------------------------------------------------


class Cell:
    """One body cell: `text` is the trimmed, unescaped content; `raw(i)` maps
    an offset into `text` back to a raw column (an unescaped `\\|` shifts
    everything after it by one); `lexed` is the cell's `Lexed`, minted with
    the cell (a cell is inline content and parses no reference definition).

    `_segments` = [(offset_in_text, raw_start)] for every maximal run of
    characters that is contiguous in the raw line -- a run breaks only at a
    consumed backslash."""

    __slots__ = ("text", "_segments", "lexed")

    def __init__(self, text, segments):
        self.text = text
        self._segments = segments
        self.lexed = Lexed(text)

    def raw(self, i):
        seg = self._segments
        k = bisect.bisect_right(seg, (i, _INF)) - 1
        if k < 0:
            return 0
        off, raw_start = seg[k]
        return raw_start + (i - off)


_INF = float("inf")


def _cell(line, a, b, breaks):
    """The cell over raw `line[a:b]` whose consumed backslashes are at the raw
    indexes in `breaks` -- THIS cell's breaks, in order, partitioned by the
    row scan (each `\\|` drops the backslash and keeps the `|`)."""
    pieces, segments, off = [], [], 0
    start = a
    for k in breaks:
        if k > start:
            segments.append((off, start))
            pieces.append(line[start:k])
            off += k - start
        start = k + 1
    if b > start:
        segments.append((off, start))
        pieces.append(line[start:b])
    return Cell("".join(pieces), segments)


def split_row(line):
    """-> [Cell, ...]: body cells of a GFM row (optional leading / trailing pipe
    stripped), split on unescaped `|` BEFORE any inline lexing."""
    bounds, breaks, start, n = [], [[]], 0, len(line)   # breaks[k] = cell k's, in order
    for i, c in enumerate(line):
        if c != "|":
            continue
        # §2.4 parity (`_escaped`): only an ODD backslash run escapes the `|`
        # (`a\\|b` has an unescaped pipe and is two cells); the odd backslash
        # is consumed.  The break is partitioned to its cell HERE, in the one
        # scan -- a per-cell filter over a row-wide list was quadratic.
        if _escaped(line, i):
            breaks[-1].append(i - 1)
        else:
            bounds.append((start, i))
            breaks.append([])
            start = i + 1
    bounds.append((start, n))
    stripped = line.strip(" \t")    # the same space/tab class as cell trimming
    if stripped.startswith("|") and bounds:
        bounds, breaks = bounds[1:], breaks[1:]
    if stripped.endswith("|") and bounds and not _escaped(stripped, len(stripped) - 1):
        bounds, breaks = bounds[:-1], breaks[:-1]   # the same parity: `\\|` at the end is a trailing pipe
    out = []
    for (a, b), cell_breaks in zip(bounds, breaks):
        while a < b and line[a] in " \t":
            a += 1
        while b > a and line[b - 1] in " \t":
            b -= 1
        out.append(_cell(line, a, b, cell_breaks))
    return out


# GFM §4.10: "The delimiter row consists of cells whose only content are
# hyphens (-), and optionally, a leading or trailing colon (:), or both".
_DELIM_CELL = re.compile(r":?-+:?")


def is_separator(cells):
    """GFM §4.10 delimiter row: every cell is >=1 hyphen with an optional
    leading and trailing colon."""
    return len(cells) > 0 and all(_DELIM_CELL.fullmatch(c.text) for c in cells)


def delimiter_width(line):
    """Cell count of `line` if it is a GFM delimiter row, else None.  ⚠ LOCAL
    POLICY: the row must carry at least one pipe.  GFM §4.10 does not say so
    (a one-column table's delimiter row may be `---` alone under GFM); here a
    bare `---` line is read as CommonMark does outside the extension -- a
    setext underline or a thematic break (§4.3 / §4.1) -- never a table."""
    if "|" not in line or is_blank(line):
        return None
    delim = split_row(line)
    return len(delim) if is_separator(delim) else None



def definition_block(block, off):
    """Phase 1 ("Appendix: A parsing strategy", block structure): the
    reference definition (§4.7) starting at offset `off` of `block` -- the
    RAW text of the rest of the RUN (`run_end`: every remaining line up to
    the next `block_end` -- a blank line, a raw-extent opener, a paragraph-
    interrupting block start, a table header; what the block phase hands
    over), or None.  Returns
    (label, destination, end_offset).  The label (`link_label`, §6.3: up to
    999 characters, may span lines) and the title (`link_title`) are read
    over that whole text; §4.7 "may not contain a blank line" and "a
    definition's continuation line cannot be a block start" hold by
    construction, because the run ends there (`[foo]:\n---` is a setext
    heading, `[foo]:\n#` / `>` / `***` a paragraph and a block, `[foo]:\n
    code` a definition, `[foo]: /url\n<span>` a definition and a paragraph
    `<span>` -- commonmark.js 0.31.2 agrees on each).  Read before
    any inline parsing: a backtick in the destination (`[sib]: slice`x`.md`)
    is destination text, not a code span.  The caller decides whether `off`
    is a block start (a definition cannot interrupt a paragraph); here the
    definition is only recognised."""
    defs, _ = reference_definitions(block, limit=1, start=off)
    return defs[0] if defs else None


def reference_definitions(s, limit=None, start=0):
    """CommonMark §4.7 link reference definitions at offset `start` of `s`
    (raw block text; the caller guarantees a block start).  Consumes
    consecutive definitions (at most `limit`).  Returns ([(label, dest,
    end)], rest_offset).

    Grammar: <=3 spaces, a link label, `:`, optional whitespace incl. up to one
    line ending, a destination, optionally whitespace incl. up to one line
    ending and a title, then nothing but spaces/tabs before the line ending.
    """
    out, i = [], start
    while limit is None or len(out) < limit:
        j = 0
        while j < 3 and i + j < len(s) and s[i + j] == " ":
            j += 1
        raw, k = link_label(s, i + j)
        if raw is None or k >= len(s) or s[k] != ":":
            break
        k = _skip_ws(s, k + 1)
        dest, k = link_destination(s, k)
        if dest is None:
            break
        # the definition may end at the destination; a title may follow after
        # spaces/tabs on this line or (§4.7) on the NEXT line -- ONE attempt:
        # a valid title followed by nothing but spaces/tabs extends the
        # definition, otherwise it ends at the destination (and if the
        # destination does not end its line either, there is no definition)
        eol = _line_end(s, k)
        k2 = _skip_ws(s, k)
        t = link_title(s, k2) if k2 > k else None
        eol_t = _line_end(s, t) if t is not None else None
        if eol_t is not None:
            eol = eol_t
        if eol is None:
            break
        out.append((raw, dest, eol))
        i = eol
        while i < len(s) and s[i] == "\n":
            i += 1
    return out, i


def _next_line(s, off):
    """Offset of the line after the one holding `s[off]` (or `len(s)`)."""
    nl = s.find("\n", off)
    return len(s) if nl < 0 else nl + 1


def _line_end(s, k):
    """Offset just past the line ending after only spaces/tabs from `k`, or
    the end of `s`; None if anything else intervenes."""
    while k < len(s) and s[k] in " \t":
        k += 1
    if k >= len(s):
        return k
    if s[k] == "\n":
        return k + 1
    return None

