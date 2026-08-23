#!/usr/bin/env python3
"""Phase 1 of CommonMark 0.31.2 "Appendix: A parsing strategy" -- BLOCK
structure -- for `plan-memo-umbrella-check.py`: fenced code blocks (§4.5),
the block starts that interrupt a paragraph (§4.1 / §4.2 / §4.3 / §5.1 /
§5.2 / §4.6), the ONE block-boundary predicate `block_end`, GFM §4.10 table
rows, and link reference definitions (§4.7).  Driven line by line by
`plan_memo_tables.py::Memo`; the inline grammar a definition's label,
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


def fenced_lines(lines):
    """0-based indices of every line inside a fenced code block, fence lines
    included.  Opener: <=3 spaces of indent, >=3 backticks or tildes (not
    mixed); a backtick fence's info string may not contain a backtick.  Closer:
    same character, at least as long, <=3 spaces of indent, nothing but spaces
    and tabs after it.  An unclosed fence runs to "the end of the containing
    block (or document)" (§4.5); this lexer has no container blocks (see the
    header), so that is the end of the document.
    """
    out, i, n = set(), 0, len(lines)
    while i < n:
        m = _FENCE_OPEN.match(lines[i])
        if not m or (m.group(1)[0] == "`" and "`" in m.group(2)):
            i += 1
            continue
        ch, k = m.group(1)[0], len(m.group(1))
        closer = re.compile(r"^ {0,3}" + re.escape(ch) + "{%d,}" % k + r"[ \t]*$")
        out.add(i)
        i += 1
        while i < n:
            out.add(i)
            i += 1
            if closer.match(lines[i - 1]):
                break
    return out


def raw_lines(lines):
    """THE ONE raw-extent map of Phase 1: {0-based line index: "fence" |
    "html"} for every line whose content is RAW -- never inline-parsed,
    always a run / paragraph / table end.  Fenced code blocks (§4.5,
    `fenced_lines`) and HTML blocks (§4.6) share it: an HTML block is a leaf
    block "treated as raw HTML", from a line meeting a start condition to
    the first line meeting its end condition (types 1-5 by content, possibly
    the opener itself; types 6 and 7 at the next blank line or the end of
    the document).  A type-7 opener counts only at a block start ("all
    types of HTML blocks except type 7 may interrupt a paragraph"): here,
    after a blank, raw or one-line-block line, or at the document start --
    commonmark.js: `text\n<span>` stays a paragraph, `<span>\nx\ny\n\nz`
    is a raw block to the blank.  Computed once; `block_end`, `_runs`,
    `_blocks`, `find_tables` and the LEX-UNSUPPORTED? seed all read it."""
    out = {i: "fence" for i in fenced_lines(lines)}
    i, n = 0, len(lines)
    while i < n:
        if i in out:
            i += 1
            continue
        t = html_block_type(lines[i])
        at_start = i == 0 or (i - 1) in out or is_blank(lines[i - 1]) or one_line_block(lines[i - 1])
        if t is None or (t == "t7" and not at_start):
            i += 1
            continue
        start, end = i, i
        if not html_block_ends(t, lines[i]):    # the opener may meet the end condition itself
            end = i + 1
            while end < n:
                if t in ("t6", "t7") and is_blank(lines[end]):
                    end -= 1
                    break
                if html_block_ends(t, lines[end]):
                    break
                end += 1
            end = min(end, n - 1)
        for k in range(start, end + 1):
            out[k] = "html"                     # the ONE marking site of an HTML extent
        i = end + 1
    return out


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
    RAW extents, `raw_lines`, seeded from that map.)"""
    if _QUOTE.match(line):
        return "quote"
    if at_block_start and _INDENTED.match(line):
        return "indented-code"
    return None


def starts_block(line):
    """The block starts that INTERRUPT a paragraph (and so end a run, a
    paragraph and a GFM table) -- CommonMark §4 / §5, each verified against
    commonmark.js 0.31.2: an ATX heading (§4.2), a thematic break (§4.1), a
    list item (§5.2; ⚠ local policy, stricter: any marker line, where the
    spec lets only a non-empty item, an ordered one starting at 1,
    interrupt), a `>` line (§5.1), a setext underline (§4.3: the paragraph
    before it becomes a heading -- `[foo]:\n---` is a heading, not a
    definition), an HTML block opener of type 1-6 (§4.6; type 7 cannot
    interrupt).  NOT in the set, by the same oracle: an indented line (§4.4
    "cannot interrupt a paragraph": `[foo]:\n    code` is a definition with
    destination `code`) and a reference definition (§4.7 "cannot interrupt a
    paragraph").  A RAW line (a fence or HTML block, `raw_lines`) and a
    blank line end a run too; they are never lines a run may start on."""
    if one_line_block(line) or _LIST_ITEM.match(line) or _QUOTE.match(line) or _SETEXT.match(line):
        return True
    t = html_block_type(line)
    return t is not None and t != "t7"


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


def block_end(lines, i, raw):
    """THE ONE block-boundary predicate (plan §2 I-A): whether raw line `i`
    ends the run / paragraph / table before it -- a blank line (§2.1), a RAW
    line (a fenced code block §4.5 or an HTML block §4.6, `raw_lines`), a
    paragraph-interrupting block start (`starts_block`) or a GFM table
    header (`table_header_at`).  `_runs`, `_blocks`, `find_tables` and,
    through the runs, `definition_block`'s continuation lines all read this
    and nothing else."""
    return (i in raw or is_blank(lines[i]) or starts_block(lines[i])
            or table_header_at(lines, i))




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
    RAW text of the rest of the RUN (every remaining line up to the next
    `block_end`: a blank line, a fence, a paragraph-interrupting block start,
    a table header; what the block phase hands over), or None.  Returns
    (label, destination, end_offset).  The label (`link_label`, §6.3: up to
    999 characters, may span lines) and the title (`link_title`) are read
    over that whole text; §4.7 "may not contain a blank line" and "a
    definition's continuation line cannot be a block start" hold by
    construction, because the run ends there (`[foo]:\n---` is a setext
    heading, `[foo]:\n#` / `>` / `***` a paragraph and a block, `[foo]:\n
    code` a definition -- commonmark.js 0.31.2 agrees on each).  Read before
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

