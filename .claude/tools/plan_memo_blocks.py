#!/usr/bin/env python3
"""Phase 1 of CommonMark 0.31.2 "Appendix: A parsing strategy" -- BLOCK
structure -- for `plan-memo-umbrella-check.py`: the line grammar of the
block types -- the ONE indentation measure (§2.2 tab stops: `indentation` /
`unindented` / `is_indented`, read by every block start), raw extents
(indented code blocks §4.4, fenced code blocks §4.5, HTML blocks §4.6: ONE
opener rule `raw_opener`, ONE extent rule `raw_extent`), the markers of the
two §5 containers -- the block-quote marker (§5.1, `quote_content`) and the
list-item marker with the item's content indentation (§5.2, `item_marker`,
undone on the continuation lines by `strip_columns`) -- the block starts
that interrupt a paragraph (§4.1 / §4.2 / §5.1 / §5.2, the last under the
§5.2 interruption rule), the setext underline that closes one (§4.3), the
ONE block-boundary predicate `block_end` -- with the lazy-continuation arm
(§5.1 / §5.2, one mechanism) every container reads through it -- and the
run it bounds (`run_end`), GFM §4.10 table rows, and link reference
definitions (§4.7).  Driven line by line, in ONE forward pass, by
`plan_memo_memo.py::Memo._parse`, which owns the block state (what is
open: a raw extent, a table, a run) and recurses into a container's content
-- a block quote's, a list item's -- with the same pass; this module holds
no state and looks at no previous line.  The inline grammar a definition's
label, destination and title reuse (`link_label`, `link_destination`,
`link_title`, `_skip_ws`, `_escaped`) is Phase 2's, in
`plan_memo_lexer.py`, which this module imports and never the reverse.

Every block type of the spec's closed list (§4 leaf blocks, §5 container
blocks, GFM tables) has a disposition in the plan's §3.0 table, and every
one is LEXED (a leaf, a raw extent, or a container: §5.1 block quotes, §5.2
list items grouped into §5.3 lists).  The
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


def indentation(s, i=0, col=0):
    """(columns, j): the indentation of the line starting at `s[i]` in
    columns -- a space is one, a tab advances to the next multiple of 4
    (§2.2) -- and the index `j` of its first other character.  `col` is the
    column `s[i]` stands at (0 at a line start; after a list marker, the
    marker's end -- the tab stops are the LINE's, so `-\\tfoo`'s tab is three
    columns, not four)."""
    j, n = i, len(s)
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
# CommonMark §5.2 list items -- the second container.  "A list marker is a
# bullet list marker or an ordered list marker": `-`, `+` or `*`, or 1-9
# ASCII digits then `.` or `)`, after up to three spaces of indentation, and
# "followed by ... spaces of indentation": W = the marker's width, N = the
# spaces after it -- as written when 1-4; 1 when there are five or more (the
# item starts with indented code, Example 270) or none at all (the item
# starts with a blank line, Example 278).  The item's CONTENT is what follows
# the marker and N spaces on the first line and, on every later line, what
# follows W + N columns of indentation (`strip_columns`); a later line short
# of that is the item's LAZY CONTINUATION CANDIDATE (§5.2 rule 5, the same
# §5.1 mechanism: content only as paragraph continuation text), and the
# driver (`Memo._item`) runs the SAME Phase 1 over the content, so a nested
# list, a quote, a raw extent, a table, a definition or a paragraph inside
# an item is that block at its real line.  Sibling items of the same TYPE
# (§5.3: the same bullet character, or the same ordered delimiter) form one
# list (`Memo._list`), which is loose or tight by §5.3's rule.
#
# Block starts that end a paragraph or a table (GFM §4.10: "the table is
# broken at the first empty line, or beginning of another block-level
# structure"): ATX headings (§4.2) and thematic breaks (§4.1) are one-line
# leaf blocks; a block-quote line (§5.1) and a list-item line (§5.2) open
# their containers -- the latter under the §5.2 INTERRUPTION RULE where a
# paragraph is open: "when the first list item in a list interrupts a
# paragraph ... the list item must not begin with a blank line" and "must
# start with 1" when ordered, so `foo\n2. b` and `foo\n-` are one paragraph
# while `foo\n1. b` and `foo\n- b` are a paragraph and a list (commonmark.js
# 0.31.2 agrees on each).  On a LAZY candidate the rule does not apply: the
# container the line failed to match is closed, so any marker line there
# opens an item at the level outside it (`- a\n2. b`, `> a\n2. b`, `> a\n*`
# are each a container and then a list -- measured).
# --------------------------------------------------------------------------

# Every pattern here is matched against `unindented(line)` -- the line after
# its <=3 columns of indentation (§2.2) -- never against the raw line.
_ATX = re.compile(r"^#{1,6}(?:[ \t]|$)")
_THEMATIC = re.compile(r"^(?:(?:-[ \t]*){3,}|(?:\*[ \t]*){3,}|(?:_[ \t]*){3,})$")
# §5.2: the marker, matched at the index after the line's indentation; ASCII
# digits; "followed by" a space, a tab or the end of the line (`-foo` is text)
_LIST_MARKER = re.compile(r"(?:(?P<bullet>[-+*])|(?P<num>[0-9]{1,9})(?P<delim>[.)]))(?=[ \t]|$)")
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


def setext_underline(line):
    """§4.3: the heading a setext underline SHAPE makes -- `"h1"` for `=`,
    `"h2"` for `-` -- or None; the ONE reading of the underline (the driver
    takes the kind from here, never from a second look at the line).
    Whether it closes anything is `block_end`'s (a paragraph must be open,
    and not on a lazy line: `- Foo\\n---` is an item and a thematic break,
    Example 94 -- the `---` is the item's lazy candidate, a boundary by the
    thematic-break arm and never an underline there) and the driver's (the
    shape is read before a list marker is, as commonmark.js orders its block
    starts: `foo\\n-` is the heading, not a paragraph and an empty item)."""
    rest = unindented(line)
    if rest is None or not _SETEXT.match(rest):
        return None
    return "h1" if rest[0] == "=" else "h2"


def item_marker(line):
    """§5.2: the list item `line` opens -> (marker, offset, content), or None.
    `marker` is the marker's text as written -- `-`, `+`, `*`, or the number
    and its delimiter, `3.` / `1)` -- the fact §5.3 groups items into a
    list by ("of the same type": the same bullet character, or the same
    delimiter; `_same_list`) and an ordered list's start number; `offset`
    the item's content indentation in LINE columns (the marker's own
    indentation + W + N); `content` the first line of the content, its
    leading whitespace re-spelt in spaces from its true column as
    `quote_content` does (Example 7 `-\\t\\tfoo`: the marker ends at column 1,
    the tabs reach column 8 -- seven columns, so N = 1 and the content is
    six columns of indentation then `foo`, indented code holding `  foo`).
    A marker after four or more columns is §4.4 territory; a marker not
    followed by a space, a tab or the line's end is text (`-foo`, `1.foo`);
    ASCII digits only (`١.` is text); and a thematic break is no marker --
    §4.1: "when both a thematic break and a list item are possible
    interpretations of a line, the thematic break takes precedence" (`* * *`
    after `* Foo` ends the list, Example 61; `- - -` is `<hr />`, not an
    item holding `- -`), stated here so every reader of the marker (the
    driver, the sibling lookahead, `starts_block`) reads it once."""
    col, j = indentation(line)
    if col >= 4:
        return None
    m = _LIST_MARKER.match(line, j)
    if m is None or _THEMATIC.match(line[j:]):
        return None
    w = m.end() - j
    wcol, k = indentation(line, m.end(), col + w)   # the spaces after the marker, in line columns
    spaces = wcol - (col + w)
    blank = k >= len(line)
    n = 1 if blank or spaces >= 5 else spaces
    content = "" if blank else " " * (spaces - n) + line[k:]
    return m.group(0), col + w + n, content


def _same_list(a, b):
    """§5.3: markers `a` and `b` (from `item_marker`) open items of one
    list -- the same bullet character, or ordered markers with the same
    delimiter (`1.` and `2.`; not `1.` and `1)`, Example 302)."""
    return a == b if a in "-+*" else b not in "-+*" and a[-1] == b[-1]


def strip_columns(line, cols):
    """`line` after `cols` columns of its indentation (§2.2: a tab reaching
    past `cols` leaves the columns beyond as spaces) -- a list item's
    continuation line with the item's content indentation undone, as §5.2
    reads it.  The caller passes at most the line's indentation."""
    col, j = 0, 0
    while col < cols:
        col += 4 - col % 4 if line[j] == "\t" else 1
        j += 1
    return " " * (col - cols) + line[j:]


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


def starts_block(line, para_open=False):
    """The NON-RAW block starts that end a run, a paragraph and a GFM table
    -- CommonMark §4 / §5, each verified against commonmark.js 0.31.2: an
    ATX heading (§4.2), a thematic break (§4.1), a `>` line (§5.1: the
    container opens), a list-item line (§5.2: the container opens) -- the
    last under the §5.2 INTERRUPTION RULE where a paragraph is open
    (`para_open`, the ONE context bit `raw_opener` reads too): only an item
    that is not empty and, if ordered, starts at 1 interrupts (`foo\\n2. b`,
    `foo\\n-` stay one paragraph; `foo\\n1. b`, `foo\\n- b` are a paragraph
    and a list); with no paragraph open -- a block start, a table's next
    row, a container's LAZY candidate (`block_end` passes False there: the
    unmatched container is closed, so `- a\\n2. b` and `> a\\n2. b` are a
    container and then a list) -- any marker line opens an item.  The RAW
    block starts -- an indented line (§4.4), a fence (§4.5), an HTML block
    opener (§4.6: types 1-6 interrupt, type 7 only where no paragraph is
    open) -- are `raw_opener`'s, and a setext underline (§4.3: the paragraph
    before it becomes a heading -- `[foo]:\n---` is a heading, not a
    definition -- but with NO paragraph open there is nothing to underline,
    so after a table's rows `===` is a one-cell row and `---` a thematic
    break, GFM §4.10) is `block_end`'s third arm, the one gated on the same
    bit.  NOT in the set, by the same oracle: an indented line (§4.4 "cannot
    interrupt a paragraph": `[foo]:\n    code` is a definition with
    destination `code`) and a reference definition (§4.7 "cannot interrupt
    a paragraph")."""
    if one_line_block(line) is not None or quote_content(line) is not None:
        return True
    m = item_marker(line)
    if m is None:
        return False
    marker, _, content = m
    return not para_open or (not is_blank(content) and (marker in "-+*" or int(marker[:-1]) == 1))


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
    indented line -- right after a table's rows opens a block.  Both ends
    are cmark-gfm's reading, MEASURED (`gh api -X POST /markdown -f
    mode=gfm -f text=...`, design re-gate 3): `<span>` after the rows is an
    HTML block, and an indented line -- `    | a |` or `\t| a |` -- is the
    table and then `<pre><code>`, never a row (an earlier docstring claimed
    cmark-gfm's row continuation read it as a row; that was never
    measured and is false).  The one LOCAL policy at a table's end is the
    width miss on `===` (plan §2 I-C), not the boundary."""
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
    `\t| a | b |\n\t|---|---|` is indented code (§4.4), not a table.  The
    DELIMITER row may not be a lazy candidate (§5.1): a line without the
    quote marker is paragraph continuation text only, so `> | a |\n|---|`
    is one paragraph (cmark-gfm: the delimiter row arrives in an unmatched
    container and opens nothing).  The HEADER row may be lazy: cmark-gfm
    reads the header out of the open paragraph's last line when the
    delimiter row carries the marker (`> a\n| h |\n> |---|\n> | 1 |` is
    quote[p(a), table(h; 1)], measured, design re-gate 3), so the lazy
    line is content here -- as a table header, the one boundary a lazy line
    can be with a paragraph open (`Memo._parse` hands it to the table
    instead of ending the quote)."""
    if (i + 1 >= len(lines) or is_indented(lines[i]) or is_indented(lines[i + 1])
            or _is_lazy(lazy, i + 1)):
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
    (`starts_block`, the same bit deciding the §5.2 interruption rule -- and
    passed False on a lazy candidate, whose container is closed by any
    marker line), a setext underline where a paragraph is open (§4.3:
    it closes that paragraph; with none open there is nothing to underline
    -- after a table's rows `===` is a one-cell body row, GFM §4.10 Example
    202, and `---` the thematic break `starts_block` already names), or a
    GFM table header (`table_header_at`).  `lazy` (§5.1 / §5.2, `_is_lazy`) marks
    a container's lines that carried no marker (a block quote's) or fell
    short of the content indentation (a list item's): such a line is content
    only as paragraph continuation text, so with NO paragraph open it is a
    boundary outright (the container ends there: after a raw extent, a
    table, a one-line block or a heading), and with one open it is a
    boundary by every arm EXCEPT the setext underline -- "the setext heading
    underline cannot be a lazy continuation line", so `> foo\\nbar\\n===` is
    one paragraph (Example 93) while `> Foo\\n---` is a quote and a thematic
    break (Example 92) and `- Foo\\n---` an item and one (Example 94) -- and,
    by the table-header arm, the boundary is the lazy line becoming the
    header of a table inside the quote (the driver reads that case:
    `table_header_at`).  `run_end` (a run: paragraph text, `para_open`
    True) and `admit_table` (a table body: no paragraph is open, False) read
    this and nothing else, and the driver `Memo._parse` classifies a line
    outside a run by the same arms; no caller looks back at a previous
    line."""
    if _is_lazy(lazy, i) and not para_open:
        return True
    return (is_blank(lines[i]) or raw_opener(lines[i], para_open) is not None
            or starts_block(lines[i], para_open and not _is_lazy(lazy, i))
            or (para_open and not _is_lazy(lazy, i) and setext_underline(lines[i]) is not None)
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

