#!/usr/bin/env python3
"""Phase 1 of CommonMark 0.31.2 "Appendix: A parsing strategy" -- BLOCK
structure -- for `plan-memo-umbrella-check.py`: the line grammar of the
block types -- the ONE indentation measure (§2.2 tab stops: `indentation` /
`unindented` / `is_indented`, read by every block start), raw extents
(indented code blocks §4.4, fenced code blocks §4.5, HTML blocks §4.6: ONE
opener rule `raw_opener`, ONE extent rule `raw_extent`), the block-quote
marker of the §5.1 container (`quote_content`), the block starts that
interrupt a paragraph (§4.1 / §4.2 / §5.1 / §5.2), the setext underline that
closes one (§4.3), the ONE block-boundary predicate `block_end` -- with the
§5.1 lazy-continuation arm every container reads through it -- and the run
it bounds (`run_end`), GFM §4.10 table rows, and link reference definitions
(§4.7).  Driven line by line, in ONE forward pass, by
`plan_memo_tables.py::Memo._parse`, which owns the block state (what is
open: a raw extent, a table, a run) and recurses into a block quote's
content with the same pass -- this module holds no state and looks at no
previous line.  The inline grammar a definition's label, destination and
title reuse (`link_label`, `link_destination`, `link_title`, `_skip_ws`,
`_escaped`) is Phase 2's, in `plan_memo_lexer.py`, which this module
imports and never the reverse.

Every block type of the spec's closed list (§4 leaf blocks, §5 container
blocks, GFM tables) has a disposition in the plan's §3.0 table: LEXED (a
leaf, a raw extent, or the §5.1 container) or LEXED-FLAT (§5.2 list items:
a marker line starts a paragraph, nesting and laziness not modelled).  The
falsifier of every row is the spec's own example list, vendored in
`commonmark-0.31.2-block-examples.json` and run through this Phase 1 by
`plan_memo_selftest_conformance.py` (every control run) -- a spec-table
transcription error here turns that control red.
"""

import bisect
import re

from plan_memo_lexer import (
    Lexed, _escaped, _skip_ws, link_destination, link_label, link_title,
)

# --------------------------------------------------------------------------
# CommonMark §2.2 tabs -- THE one indentation measure.  "Tabs in lines are
# not expanded to spaces.  However, in contexts where spaces help to define
# block structure, tabs behave as if they were replaced by spaces with a tab
# stop of 4 characters."  Every block start below reads its "up to three
# spaces of indentation" through `unindented`, and §4.4's "four or more"
# through `is_indented`; no pattern spells ` {0,3}` or ` {4,}` itself
# (commonmark.js 0.31.2: `\tfoo`, ` \tfoo`, ` \t# foo` are all indented code).
# --------------------------------------------------------------------------


def indentation(s, i=0):
    """(columns, j): the indentation of the line starting at `s[i]` in
    columns -- a space is one, a tab advances to the next multiple of 4
    (§2.2) -- and the index `j` of its first other character."""
    col, j, n = 0, i, len(s)
    while j < n:
        c = s[j]
        if c == " ":
            col += 1
        elif c == "\t":
            col += 4 - col % 4
        else:
            break
        j += 1
    return col, j


def unindented(line):
    """`line` after its indentation when that is at most three columns (the
    "up to three spaces of indentation" every §4 / §5 block start allows),
    else None (four or more columns: §4.4 territory)."""
    col, j = indentation(line)
    return line[j:] if col < 4 else None


def is_indented(line):
    """§4.4: a non-blank line indented four or more columns."""
    return indentation(line)[0] >= 4 and not is_blank(line)


# --------------------------------------------------------------------------
# CommonMark §4.5 fenced code blocks
# --------------------------------------------------------------------------

_FENCE_OPEN = re.compile(r"^(`{3,}|~{3,})(.*)$")


def fence_opener(line):
    """The closer pattern of the fenced code block `line` opens (§4.5), or
    None.  Opener: <=3 columns of indent (`unindented`), >=3 backticks or
    tildes (not mixed); a backtick fence's info string may not contain a
    backtick.  Closer: same character, at least as long, <=3 columns of
    indent, nothing but spaces and tabs after it (`fence_closes`).  An
    unclosed fence runs to "the end of the containing block (or document)"
    (§4.5): the document, or the block quote holding it (`raw_extent` stops
    at the quote's first lazy candidate)."""
    rest = unindented(line)
    m = _FENCE_OPEN.match(rest) if rest is not None else None
    if not m or (m.group(1)[0] == "`" and "`" in m.group(2)):
        return None
    ch, k = m.group(1)[0], len(m.group(1))
    return re.compile("^" + re.escape(ch) + "{%d,}" % k + r"[ \t]*$")


def fence_closes(closer, line):
    """Whether `line` is the closing fence `closer` (from `fence_opener`)."""
    rest = unindented(line)
    return rest is not None and closer.match(rest) is not None


# --------------------------------------------------------------------------
# CommonMark §5.1 block quotes -- the ONE container this Phase 1 models.
# "A block quote marker, optionally preceded by up to three spaces of
# indentation, consists of (a) the character > together with a following
# space of indentation, or (b) a single character > not followed by a space
# of indentation."  The content of a marker line is what follows the marker;
# the driver (`Memo._quote`) strips every marker line, gathers the lazy
# continuation lines between and after them, and runs the SAME Phase 1 over
# the content, so a definition, a table, a raw extent or a paragraph inside
# a quote is that block at its real line.
# --------------------------------------------------------------------------


def quote_content(line):
    """The content of the block-quote line `line` -- what follows its §5.1
    marker -- or None when the line carries no marker (four or more columns
    before the `>` is §4.4 territory: `    > a` is indented code).  Block
    structure inside the quote is measured in LINE columns (§2.2: tab stops
    are the line's, not the content's), so the content's leading whitespace
    is re-spelt in spaces from its true column: after `> ` at column 2 a
    space and a tab reach column 4 -- two columns of content indentation, a
    paragraph (`>  \\ta` is `<p>a</p>`); a tab right after the `>` gives one
    column to the marker's space and the rest to the content (Example 6:
    `>\\t\\tfoo` is indented code holding `  foo`; `>\\ta` a paragraph)."""
    col, j = indentation(line)
    if col >= 4 or j >= len(line) or line[j] != ">":
        return None
    j += 1
    col += 1
    c0 = col                    # where the content begins, in line columns
    if j < len(line) and line[j] == " ":
        j += 1
        col += 1
        c0 = col
    elif j < len(line) and line[j] == "\t":
        j += 1
        c0 = col + 1            # one column of the tab is the marker's space
        col += 4 - col % 4      # the rest of the tab is content indentation
    while j < len(line) and line[j] in " \t":
        col += 1 if line[j] == " " else 4 - col % 4
        j += 1
    return " " * (col - c0) + line[j:]


# --------------------------------------------------------------------------
# Block starts that end a paragraph or a table (GFM §4.10: "the table is
# broken at the first empty line, or beginning of another block-level
# structure").  ATX headings (CommonMark §4.2) and thematic breaks (§4.1) are
# one-line leaf blocks; a block-quote line (§5.1) opens the container; a
# list-item line (§5.2) starts a new paragraph.  ⚠ LOCAL POLICY, stricter
# than CommonMark: here ANY list-item-shaped line interrupts a paragraph,
# whereas under §5.2 an empty list item cannot interrupt a paragraph, an
# ordered list item can only when its number is 1, and a line without the
# marker can be lazy continuation text of the item.  The policy is the safe
# side for a scanner: a span is never read across such a line, so a backtick
# opened in one item and closed in the next is literal (control).  (A `>`
# line interrupts a paragraph under §5.1 itself, and the quote's own lazy
# continuation IS modelled -- `block_end`'s `lazy` argument.)
# --------------------------------------------------------------------------

# Every pattern here is matched against `unindented(line)` -- the line after
# its <=3 columns of indentation (§2.2) -- never against the raw line.
_ATX = re.compile(r"^#{1,6}(?:[ \t]|$)")
_THEMATIC = re.compile(r"^(?:(?:-[ \t]*){3,}|(?:\*[ \t]*){3,}|(?:_[ \t]*){3,})$")
_LIST_ITEM = re.compile(r"^(?:[-+*]|[0-9]{1,9}[.)])(?:[ \t]|$)")   # §5.2: ASCII digits
# §4.3 setext heading underline: `=` or `-` characters, <=3 columns of
# indent, trailing spaces/tabs.  It is a block boundary ONLY where a
# paragraph is open (`block_end`): "The setext heading underline cannot be a
# lazy continuation line in a list item or block quote" (Examples 92-94), and
# with no paragraph before it there is nothing to underline -- `---` is then
# a thematic break (§4.1) and `===` paragraph text (a bare `===` IS a
# paragraph, so `===\n---` is `<h2>===</h2>`).  Precedence with a paragraph
# open: a `-` line is the underline, not a thematic break (Example 59).
_SETEXT = re.compile(r"^(?:=+|-+)[ \t]*$")
# §4.6 HTML block start conditions 1-7 (after <=3 columns).
_HTML_TAG_NAMES = (
    "address|article|aside|base|basefont|blockquote|body|caption|center|col|colgroup|dd|"
    "details|dialog|dir|div|dl|dt|fieldset|figcaption|figure|footer|form|frame|frameset|h[1-6]|"
    "head|header|hr|html|iframe|legend|li|link|main|menu|menuitem|nav|noframes|ol|optgroup|"
    "option|p|param|search|section|summary|table|tbody|td|tfoot|th|thead|title|tr|track|ul")
# §4.6 start conditions 1-7, each its own group so the END condition can be
# chosen: 1 `</pre>` etc., 2 `-->`, 3 `?>`, 4 `>`, 5 `]]>`, 6 and 7 a blank
# line.  "All types of HTML blocks except type 7 may interrupt a paragraph."
# Case is PER CONDITION, as the spec states it: 1 and 6 name their tags
# "(case-insensitive)" -- a scoped `(?i:…)`, ASCII-only under `re.ASCII` so a
# long s never folds to `s`; 7's tag and attribute names are `[A-Za-z]`
# classes by their own grammar; 4 is `<!` + an ASCII letter of either case
# (0.30+); 2 `<!--`, 3 `<?` and 5 `<![CDATA[` are exact strings -- a global
# IGNORECASE read `<![cdata[` as a CDATA opener where commonmark.js 0.31.2
# reads a paragraph (PR #510 R13).
_HTML_BLOCK = re.compile(
    r"^<(?:"
    r"(?P<t1>(?i:pre|script|style|textarea)(?:[ \t>]|$))"
    r"|(?P<t2>!--)"
    r"|(?P<t3>\?)"
    r"|(?P<t4>![A-Za-z])"
    r"|(?P<t5>!\[CDATA\[)"
    r"|(?P<t6>/?(?i:" + _HTML_TAG_NAMES + r")(?:[ \t>]|/>|$))"
    r"|(?P<t7>(?:[A-Za-z][A-Za-z0-9-]*(?:[ \t]+[A-Za-z_:][A-Za-z0-9_.:-]*(?:[ \t]*=[ \t]*"
    r"(?:[^ \t\"'=<>`]+|'[^']*'|\"[^\"]*\"))?)*[ \t]*/?>|/[A-Za-z][A-Za-z0-9-]*[ \t]*>)[ \t]*$)"
    r")", re.ASCII)
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


def _match(pat, line):
    """`pat` against `unindented(line)`: every block start reads its <=3
    columns of indentation through the one measure."""
    rest = unindented(line)
    return rest is not None and pat.match(rest) is not None


def one_line_block(line):
    """The one-line leaf block `line` is -- `"h1"`..`"h6"` for an ATX
    heading (§4.2, at its `#` count), `"hr"` for a thematic break (§4.1) --
    or None."""
    rest = unindented(line)
    if rest is None:
        return None
    m = _ATX.match(rest)
    if m:
        return "h%d" % len(m.group(0).rstrip(" \t"))
    return "hr" if _THEMATIC.match(rest) else None


def is_setext_underline(line):
    """§4.3: a setext heading underline SHAPE; whether it closes anything is
    `block_end`'s (a paragraph must be open, and not on a lazy line) and the
    driver's (the paragraph must not be `container_text`)."""
    return _match(_SETEXT, line)


def list_item_line(line):
    """§5.2: `line` opens a list item -- the ONE block type read LEXED-FLAT
    (the marker line starts a paragraph; nesting and laziness are not
    modelled), which is what the conformance control excludes by."""
    return _match(_LIST_ITEM, line)


def container_text(line):
    """A run headed by `line` is NOT a paragraph a setext underline can
    close: a list-item line (§4.3 Example 94: the text is the item's and the
    underline "cannot be a lazy continuation line" -- commonmark.js agrees).
    A `>` line heads no run (the quote is a container, `Memo._quote`) and
    neither does an indented line (§4.4: a raw extent at a block start, so
    `    foo\\n---` is a code block and a thematic break, Example 100)."""
    return list_item_line(line)


def html_block_type(line):
    """The §4.6 start condition (`"t1"`..`"t7"`) `line` meets, or None."""
    rest = unindented(line)
    m = _HTML_BLOCK.match(rest) if rest is not None else None
    return m.lastgroup if m else None


def html_block_ends(kind, line):
    """Whether `line` meets the §4.6 END condition of an HTML block of
    `kind` (types 1-5 by content; 6 and 7 end only at a blank line, which
    the caller handles as every block's end)."""
    pat = _HTML_END.get(kind)
    return pat is not None and pat.search(line) is not None


def starts_block(line):
    """The NON-RAW block starts that INTERRUPT a paragraph (and so end a run, a
    paragraph and a GFM table) -- CommonMark §4 / §5, each verified against
    commonmark.js 0.31.2: an ATX heading (§4.2), a thematic break (§4.1), a
    list item (§5.2; ⚠ local policy, stricter: any marker line, where the
    spec lets only a non-empty item, an ordered one starting at 1,
    interrupt), a `>` line (§5.1: the container opens).  The RAW block
    starts -- an indented line (§4.4), a fence (§4.5), an HTML block opener
    (§4.6: types 1-6 interrupt, type 7 only where no paragraph is open) --
    are `raw_opener`'s, and a setext underline (§4.3: the paragraph before
    it becomes a heading -- `[foo]:\n---` is a heading, not a definition --
    but with NO paragraph open there is nothing to underline, so after a
    table's rows `===` is a one-cell row and `---` a thematic break, GFM
    §4.10) is `block_end`'s third arm, the one gated on its `para_open` bit.
    NOT in the set, by the same oracle: an indented line (§4.4 "cannot
    interrupt a paragraph": `[foo]:\n    code` is a definition with
    destination `code`) and a reference definition (§4.7 "cannot interrupt
    a paragraph")."""
    return one_line_block(line) is not None or list_item_line(line) or quote_content(line) is not None


def raw_opener(line, para_open):
    """THE one rule for where a RAW extent begins -- lines never
    inline-parsed, always a run / paragraph / table end: ("fence", closer)
    for a fenced code block opener (§4.5), ("html", type) for a line meeting
    an HTML block start condition (§4.6), ("indented", None) for a line of
    four or more columns (§4.4) where no paragraph is open, else None.
    `para_open` is Phase 1's one context bit, the driver's block state:
    "all types of HTML blocks except type 7 may interrupt a paragraph" and
    "an indented code block cannot interrupt a paragraph", so a type-7
    opener and an indented line count only where no paragraph is open --
    the document start, after a blank line, a raw extent, a TABLE, a
    one-line block or a setext heading (commonmark.js 0.31.2: `text\n<span>`,
    `[foo]: /url\n<span>`, `- item\n<span>`, `a\n    b` and `[x]: /u\n
    code` stay one paragraph; `# h\n<span>`, `text\n===\n<span>` and `# h\n
    code` are a heading and a raw block; GFM §4.10: a table "is broken at
    the first empty line, or beginning of another block-level structure",
    and a table is not a paragraph, so `<span>` -- and, by the same bit, an
    indented line -- right after a table's rows opens a block; ⚠ local
    policy for the indented case: cmark-gfm's row continuation reads any
    non-blank line as a row, and commonmark.js has no tables to arbitrate)."""
    closer = fence_opener(line)
    if closer is not None:
        return "fence", closer
    t = html_block_type(line)
    if t is not None and not (t == "t7" and para_open):
        return "html", t
    if not para_open and is_indented(line):
        return "indented", None
    return None


def _is_lazy(lazy, i):
    """§5.1: whether content line `i` is a LAZY CONTINUATION CANDIDATE -- a
    line of a block quote's content that carried no marker (`lazy` is the
    driver's per-line list, None at the document level)."""
    return lazy is not None and lazy[i]


def raw_extent(lines, i, opener, lazy=None):
    """End (exclusive) of the raw extent `opener` (from `raw_opener`) opens
    at `lines[i]`: an indented code block runs over its §4.4 chunk --
    consecutive indented lines, a blank line staying inside when an indented
    line follows it (Example 111 `chunk1 … chunk3` is one block), the
    trailing blank lines not part of it; a fence runs through its closer,
    or to the end of the containing block; an HTML block from the opener to
    the line meeting its §4.6 end condition (types 1-5 by content, possibly
    the opener itself) or, for types 6 and 7, up to the next blank line
    (excluded) or the end.  A lazy candidate (`_is_lazy`) ends every raw
    extent: a line without the marker is content only as paragraph
    continuation text, and a raw extent is not a paragraph (`> ```\\nlazy`
    is an unclosed fence in the quote and a paragraph after it;
    `>     foo\\n    bar` two code blocks -- commonmark.js agrees)."""
    kind, arg = opener
    n = len(lines)
    if kind == "indented":
        j = end = i + 1
        while j < n and not _is_lazy(lazy, j):
            if is_indented(lines[j]):
                end = j + 1
            elif not is_blank(lines[j]):
                break
            j += 1
        return end
    if kind == "fence":
        j = i + 1
        while j < n and not _is_lazy(lazy, j):
            if fence_closes(arg, lines[j]):
                return j + 1
            j += 1
        return j
    if html_block_ends(arg, lines[i]):      # the opener may meet the end condition itself
        return i + 1
    j = i + 1
    while j < n and not _is_lazy(lazy, j):
        if arg in ("t6", "t7") and is_blank(lines[j]):
            return j
        j += 1
        if html_block_ends(arg, lines[j - 1]):
            return j
    return j


def table_header_at(lines, i, lazy=None):
    """GFM §4.10: `lines[i]` is a table header iff the next line is a
    delimiter row of the same width.  ⚠ Local policy over pure CommonMark
    (where a table is not a block): a header ends the run and the paragraph
    before it, so `[foo]:\n|9z|7z|\n|--|--|` is a paragraph `[foo]:` and a
    table, not a definition whose destination is the header row.  Both rows
    are read at <=3 columns of indentation, the same measure as every block
    start (cmark-gfm opens no header off an indented delimiter row):
    `\t| a | b |\n\t|---|---|` is indented code (§4.4), not a table.  Neither
    row may be a lazy candidate (§5.1): a line without the quote marker is
    paragraph continuation text only, so `> | a |\n|---|` is one paragraph
    (cmark-gfm: the delimiter row arrives in an unmatched container and
    opens nothing)."""
    if (i + 1 >= len(lines) or is_indented(lines[i]) or is_indented(lines[i + 1])
            or _is_lazy(lazy, i) or _is_lazy(lazy, i + 1)):
        return False
    width = delimiter_width(lines[i + 1])
    return width is not None and len(split_row(lines[i])) == width


def block_end(lines, i, para_open, lazy=None):
    """THE ONE block-boundary predicate (plan §2 I-A): whether line `i`
    begins a block, ending the run / paragraph / table before it -- a blank
    line (§2.1), a raw-extent opener (`raw_opener`: an indented code block
    §4.4, a fenced code block §4.5 or an HTML block §4.6, where `para_open`
    -- the ONE context bit, the driver's block state -- decides the §4.4
    and type-7 cases), a paragraph-interrupting block start
    (`starts_block`), a setext underline where a paragraph is open (§4.3:
    it closes that paragraph; with none open there is nothing to underline
    -- after a table's rows `===` is a one-cell body row, GFM §4.10 Example
    202, and `---` the thematic break `starts_block` already names), or a
    GFM table header (`table_header_at`).  `lazy` (§5.1, `_is_lazy`) marks
    a block quote's lines that carried no marker: such a line is content
    only as paragraph continuation text, so with NO paragraph open it is a
    boundary outright (the quote ends there: after a raw extent, a table, a
    one-line block or a heading), and with one open it is a boundary by
    every arm EXCEPT the setext underline -- "the setext heading underline
    cannot be a lazy continuation line", so `> foo\\nbar\\n===` is one
    paragraph (Example 93) while `> Foo\\n---` is a quote and a thematic
    break (Example 92).  `run_end` (a run: paragraph text, `para_open`
    True) and `admit_table` (a table body: no paragraph is open, False) read
    this and nothing else, and the driver `Memo._parse` classifies a line
    outside a run by the same arms; no caller looks back at a previous
    line."""
    if _is_lazy(lazy, i) and not para_open:
        return True
    return (is_blank(lines[i]) or raw_opener(lines[i], para_open) is not None
            or starts_block(lines[i])
            or (para_open and not _is_lazy(lazy, i) and is_setext_underline(lines[i]))
            or table_header_at(lines, i, lazy))


def run_end(lines, i, lazy=None):
    """End (exclusive) of the RUN starting at `lines[i]` -- Phase 1's text
    unit, the lines a definition is parsed over and a paragraph is grouped
    from: the consecutive lines up to the next `block_end` WITH A PARAGRAPH
    OPEN (a run is paragraph text -- a reference definition included, §4.7
    keeps the paragraph open -- so a type-7 HTML opener or an indented line
    does not end it, and a lazy candidate joins it), or the one line of a
    one-line block (§4.1 / §4.2).  A closing setext underline is not a run
    (the driver drops it before asking)."""
    if one_line_block(lines[i]):
        return i + 1
    j = i + 1
    while j < len(lines) and not block_end(lines, j, True, lazy):
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

    Grammar: <=3 columns of indentation (§2.2, `indentation`), a link label,
    `:`, optional whitespace incl. up to one line ending, a destination,
    optionally whitespace incl. up to one line ending and a title, then
    nothing but spaces/tabs before the line ending.
    """
    out, i = [], start
    while limit is None or len(out) < limit:
        col, j = indentation(s, i)
        if col >= 4:
            break
        raw, k = link_label(s, j)
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

