#!/usr/bin/env python3
"""Lexical substrate for `plan-memo-umbrella-check.py`: a subset of
CommonMark 0.31.2 and GFM 0.29, lexed by construction from the clauses
`docs/plans/2026-08-plan-memo-umbrella-checker.md` §3 lists.

Order (plan §2 "Lexing order"; CommonMark "Appendix: A parsing strategy"):
PHASE 1, block structure over raw lines (driven by `plan_memo_tables.py::
Memo`, with this module's `fenced_lines` / `definition_block` /
`starts_block` / `split_row`): fenced blocks (CommonMark §4.5) are masked
first; reference definitions (§4.7) are blocks of their own, recognised at a
block start from RAW lines; a GFM table row is split on RAW unescaped `|`
(GFM §4.10, incl. inside backticks -- Example 200) and a table ends at a
blank line or any block start, a definition included.  PHASE 2, per block
inline content (a paragraph or a cell): code spans (CommonMark §6.1,
backtick strings of equal length) and links / images (§6.3 / §6.4) are
lexed by ONE left-to-right pass
(`inline_pass`, "Appendix: A parsing strategy"): a code span is skipped as
met, an inline-link tail is parsed by lookahead on the raw text -- there is
no code pre-mask.  An image's bracket structure is parsed so that it is not
a link and a link may wrap it; its destination never joins the population,
its alt text is prose, its tail is masked.

What is NOT lexed, and is read as written: CommonMark §4.4 indented code, §4.6
HTML blocks, §5 container blocks (block quotes §5.1, list items §5.2 -- a
list-item or `>` line only ENDS a paragraph here, its content is not
re-parsed as a nested document), §6.5 autolinks, §2.5 entity references.
Nothing here detects them.

`Lexed` is the one Phase-2 value per block: code spans, links, images, and
the two bare tokens the scanners must not read an id out of (`[C19]`-style
citation ids, `.md` file names).  Nothing in
this module knows what a row id is; the disposition exception (an id-only code
span is the document spelling an id, not code) is applied over a `Lexed` by
`plan_memo_tables.py`.
"""

import bisect
import re
import string

ASCII_PUNCT = frozenset(string.punctuation)

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
_LIST_ITEM = re.compile(r"^ {0,3}(?:[-+*]|\d{1,9}[.)])(?:[ \t]|$)")
_QUOTE = re.compile(r"^ {0,3}>")


def is_blank(line):
    """CommonMark §4.9: a blank line "contains no characters, or only spaces
    or tabs" -- the ASCII class, not `str.strip()`'s Unicode whitespace (an
    NBSP-only line is paragraph text)."""
    return not line.strip(" \t")


def one_line_block(line):
    return bool(_ATX.match(line) or _THEMATIC.match(line))


def starts_block(line):
    return bool(one_line_block(line) or _LIST_ITEM.match(line) or _QUOTE.match(line))


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
    indexes in `breaks` (each `\\|` drops the backslash and keeps the `|`)."""
    pieces, segments, off = [], [], 0
    start = a
    for k in sorted(x for x in breaks if a <= x < b):
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
    bounds, breaks, start, n = [], [], 0, len(line)
    for i, c in enumerate(line):
        if c != "|":
            continue
        # §2.4 parity (`_escaped`): only an ODD backslash run escapes the `|`
        # (`a\\|b` has an unescaped pipe and is two cells); the odd backslash
        # is consumed
        if _escaped(line, i):
            breaks.append(i - 1)
        else:
            bounds.append((start, i))
            start = i + 1
    bounds.append((start, n))
    stripped = line.strip(" \t")    # the same space/tab class as cell trimming
    if stripped.startswith("|") and bounds:
        bounds = bounds[1:]
    if stripped.endswith("|") and bounds and not _escaped(stripped, len(stripped) - 1):
        bounds = bounds[:-1]       # the same parity: `\\|` at the end is a trailing pipe
    out = []
    for a, b in bounds:
        while a < b and line[a] in " \t":
            a += 1
        while b > a and line[b - 1] in " \t":
            b -= 1
        out.append(_cell(line, a, b, breaks))
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


# --------------------------------------------------------------------------
# CommonMark §6.1 code spans: a backtick string (a run of one or more
# backticks) opens a span closed by the NEXT backtick string of equal length;
# a string with no equal-length partner is literal, and scanning resumes after
# it.  A span may contain line endings, so the unit is the block's inline
# content, never a line.  Code spans and brackets are recognised by ONE
# left-to-right pass (`inline_pass`, below the link grammar).
# --------------------------------------------------------------------------

_BACKTICKS = re.compile(r"`+")


def _escaped(s, i):
    """Whether `s[i]` sits behind an ODD run of backslashes (§2.4: the pairs
    before it escape each other, the odd one escapes `s[i]`)."""
    k = i
    while k > 0 and s[k - 1] == "\\":
        k -= 1
    return (i - k) % 2 == 1


def code_spans(s):
    """[(start, end)] of every code span in `s`, backticks included -- a view
    over `inline_pass`'s code tokens (there is no separate pre-mask: a
    backtick string inside a link destination the pass consumed by lookahead
    is not a code span)."""
    return inline_pass(s, {})[0]


def blank_spans(s, spans):
    """`s` with every span replaced by spaces (line endings kept), so offsets
    survive and a later grammar cannot see inside a masked construct."""
    if not spans:
        return s
    buf = list(s)
    for a, b in spans:
        for k in range(a, b):
            if buf[k] != "\n":
                buf[k] = " "
    return "".join(buf)


# --------------------------------------------------------------------------
# CommonMark §6.3 links and §4.7 link reference definitions
# --------------------------------------------------------------------------


def _skip_ws(s, i, newlines=1):
    """Spaces, tabs and up to `newlines` line endings -- §6.3: the inline
    link's components "may be separated by spaces, tabs, and up to one line
    ending"; §4.7 allows the same separator between a definition's colon, destination and title."""
    seen = 0
    while i < len(s):
        if s[i] in " \t":
            i += 1
        elif s[i] == "\n" and seen < newlines:
            seen += 1
            i += 1
        else:
            break
    return i


def _unescape(s):
    """Backslash escapes (CommonMark §2.4): a backslash before an ASCII
    punctuation character is removed; any other backslash is literal."""
    out, i = [], 0
    while i < len(s):
        if _is_escape(s, i):
            out.append(s[i + 1])
            i += 2
        else:
            out.append(s[i])
            i += 1
    return "".join(out)


def _is_escape(s, j):
    return s[j] == "\\" and j + 1 < len(s) and s[j + 1] in ASCII_PUNCT


def link_destination(s, i):
    """Link destination at `i` -> (destination, end) or (None, i).  §6.3:
    `<...>`: no line ending, no unescaped `<` or `>`.  Bare: nonempty, no ASCII
    control character (§2.1: U+0000-1F or U+007F) or space, does not start
    with `<`, parentheses only backslash-escaped or in balanced unescaped
    pairs.
    """
    if i < len(s) and s[i] == "<":
        j = i + 1
        while j < len(s):
            if _is_escape(s, j):
                j += 2
            elif s[j] in "<>\n":
                break
            else:
                j += 1
        if j < len(s) and s[j] == ">":
            return _unescape(s[i + 1:j]), j + 1
        return None, i
    j, depth = i, 0
    while j < len(s):
        c = s[j]
        if c == " " or ord(c) <= 31 or ord(c) == 127:
            break
        if _is_escape(s, j):
            j += 2
            continue
        if c == "(":
            depth += 1
        elif c == ")":
            if depth == 0:
                break
            depth -= 1
        j += 1
    if j > i and depth == 0:
        return _unescape(s[i:j]), j
    return None, i


def link_title(s, i):
    """Link title at `i` -> end offset, or None if `s[i]` opens no valid title.
    `"…"` (no unescaped `"`), `'…'` (no unescaped `'`), `(…)` (no unescaped
    `(` or `)`)."""
    if i >= len(s) or s[i] not in "\"'(":
        return None
    opener = s[i]
    close = {"\"": "\"", "'": "'", "(": ")"}[opener]
    j = i + 1
    while j < len(s):
        if _is_escape(s, j):
            j += 2
        elif s[j] == close:
            return j + 1
        elif opener == "(" and s[j] == "(":
            return None
        else:
            j += 1
    return None


# §6.3 / §2.1: the characters label matching strips and collapses are spaces,
# tabs and line endings -- NOT Unicode whitespace (`str.split()` would fold a
# no-break space into a space and match two labels the spec keeps apart).
_LABEL_WS = re.compile(r"[ \t\r\n]+")


def normalize_label(label):
    """§6.3 label matching: "perform the Unicode case fold, strip leading and
    trailing spaces, tabs, and line endings, and collapse consecutive internal
    spaces, tabs, and line endings to a single space"."""
    return _LABEL_WS.sub(" ", label.strip(" \t\r\n")).casefold()


def _has_label_content(raw):
    """§6.3: "at least one character that is not a space, tab, or line ending"."""
    return bool(raw.strip(" \t\r\n"))


def link_label(s, i):
    """A link label opening at `s[i] == '['` -> (raw_label, end) or (None, i).
    §6.3: it "ends with the first right bracket (]) that is not
    backslash-escaped"; no unescaped `[` or `]` inside; at most 999
    characters between the brackets; at least one character that is not a
    space, tab, or line ending."""
    if i >= len(s) or s[i] != "[":
        return None, i
    j = i + 1
    while j < len(s):
        if _is_escape(s, j):
            j += 2
        elif s[j] in "[]":
            break
        else:
            j += 1
    if j >= len(s) or s[j] != "]":
        return None, i
    raw = s[i + 1:j]
    if len(raw) > 999 or not _has_label_content(raw):
        return None, i
    return raw, j + 1


def _is_image(s, i):
    """Whether the `[` at `s[i]` opens an image (§6.4): an unescaped `!`
    stands right before it.  Images are not links -- their destination never
    joins the population, their text is read as written -- and a link may
    wrap one (`[![alt](img.png)](sib.md)` links `sib.md`)."""
    return i > 0 and s[i - 1] == "!" and not _escaped(s, i - 1)


def _inline_tail(s, k):
    """After `](` at `k` -> (destination, end after `)`) or None.  §6.3 inline
    link: optional spaces/tabs/one line ending, an optional destination, then
    (separated the same way) an optional title, then `)`."""
    i = _skip_ws(s, k)
    dest, j = link_destination(s, i)
    if dest is None:
        dest, j = "", i
    k2 = _skip_ws(s, j)
    if k2 > j:
        t = link_title(s, k2)
        if t is not None:
            k2 = _skip_ws(s, t)
    if k2 < len(s) and s[k2] == ")":
        return dest, k2 + 1
    return None


def _reference_tail(s, opener, close, defs):
    """The reference forms at the `]` of `s[close]`, whose `[` is `s[opener]`
    (§6.3 precedence after the inline form): full `[text][label]`, collapsed
    `[text][]`, shortcut `[text]` -> (end, dest, form, label).  `dest` is
    None when no definition answers (the memo reports it as unresolved);
    `form` is None when the text is not a label at all (literal brackets,
    nothing to report).  ONE label grammar: the text of a collapsed /
    shortcut reference is a label iff `link_label` reads `[text]` from the
    opener (it stops at the first unescaped `[` or `]`, and the stack pairs
    `close` with the opener, so when it reads a label it closes at `close`)."""
    nxt = close + 1
    if nxt < len(s) and s[nxt] == "[":
        if nxt + 1 < len(s) and s[nxt + 1] == "]":
            form, end = "collapsed", nxt + 2
        else:
            raw, end = link_label(s, nxt)
            if raw is not None:
                # a link label follows, so `[text]` is not a shortcut either
                return end, defs.get(normalize_label(raw)), "full", raw
            form, end = "shortcut", close + 1
    else:
        form, end = "shortcut", close + 1
    raw, _ = link_label(s, opener)
    if raw is None:
        return end, None, None, None
    return end, defs.get(normalize_label(raw)), form, raw


def _code_closer(s, runs, a1, k):
    """The end offset of the first backtick string of length `k` starting at
    or after `a1` (§6.1: the closer is "the next backtick string of equal
    length"), or None.  `runs` = every backtick string of `s`, in order."""
    j = bisect.bisect_left(runs, (a1, 0))
    while j < len(runs):
        ra, rb = runs[j]
        if rb - ra == k:
            return rb
        j += 1
    return None


def inline_pass(s, defs):
    """ONE left-to-right pass over a block's inline content -- CommonMark
    0.31.2 "Appendix: A parsing strategy", Phase 2 "inline structure" --
    recognising backtick strings (§6.1) and brackets (§6.3 / §6.4) together,
    and resolving references through `defs` (normalised label ->
    destination).  Returns (code, links, images, unresolved).

    Backtick strings: a run opens a code span closed by the next run of
    equal length; the scan jumps past the span (brackets inside it are never
    delimiters: `` `[a](x.md)` `` is code); an unmatched run is literal and
    the scan resumes after it.  Inside a span backslashes are literal (§6.1:
    "backslash escapes do not work in code spans"), so a closer is read raw.
    An escaped backtick (`\\` + `` ` ``, §2.4) is a literal character and
    opens nothing.

    Brackets, per the Appendix's "look for link or image": a stack of `[` /
    `![` openers, each "active"; on `]` the nearest opener is popped -- "if
    we do find one, but it's not active, we remove the inactive delimiter
    from the stack, and return a literal text node ]"; if active, "we parse
    ahead to see if we have an inline link/image, reference link/image,
    collapsed reference link/image, or shortcut reference link/image" -- the
    inline tail is parsed by LOOKAHEAD ON THE RAW TEXT and the scan jumps
    past it, so a backtick inside a destination (`[sib](slice`x`.md)`) is
    consumed by the link, while a backtick BEFORE the `]` (`[not a
    `link](/foo`)`) opens a span that swallows the `]` and no link forms.
    "If we don't, then we remove the opening delimiter from the delimiter
    stack and return a literal text node ]"; if we do, the link or image is
    emitted and "if we have a link (and not an image), we also set all [
    delimiters before the opening delimiter to inactive.  (This will prevent
    us from getting links within links.)"

    Linear in the bracket structure: no substring is re-parsed.  `code` =
    [(start, end)] backticks included; `links` = [(tail_start, end,
    destination)] with `tail_start` the `]` closing the link text, so a
    caller masking the tail leaves the visible text -- prose -- in the
    scanned stream; `images` = [(tail_start, end)] (§6.4: an image's
    destination never joins the population, its alt text is prose, its tail
    is masked); `unresolved` = [(offset, label, form, is_image)], every
    reference whose label `defs` does not define, with its FORM (`"full"` /
    `"collapsed"` / `"shortcut"`) decided by this one escape-honouring parse
    -- a caller never re-walks the raw text -- and whether the opener was an
    image (literal image syntax under §6.4, never a memo the author meant to
    link).  Such a LINK site is prose under §6.3, and a population the
    author meant to link is silently lost unless the caller reports it; the
    memo exempts a shortcut (every `[C19]` citation is one) unless a
    definition of its label exists somewhere the grammar cannot read it.
    """
    runs = [(m.start(), m.end()) for m in _BACKTICKS.finditer(s)]
    code, out, images, unresolved, stack, i, n = [], [], [], [], [], 0, len(s)
    while i < n:
        c = s[i]
        if _is_escape(s, i):
            i += 2                      # §2.4: `\[` / `\]` / `\`` are literal
            continue
        if c == "`":
            a1 = i
            while a1 < n and s[a1] == "`":
                a1 += 1
            close = _code_closer(s, runs, a1, a1 - i)
            if close is None:
                i = a1                  # an unmatched backtick string is literal
            else:
                code.append((i, close))
                i = close
            continue
        if c == "[":
            stack.append([i, _is_image(s, i), True])
            i += 1
            continue
        if c != "]" or not stack:
            i += 1
            continue
        pos, is_img, active = stack.pop()
        if not active:
            i += 1                      # literal `]`; the opener is gone
            continue
        dest, end, form = None, None, None
        if i + 1 < n and s[i + 1] == "(":
            r = _inline_tail(s, i + 2)
            if r is not None:
                dest, end = r
        if end is None:
            end, dest, form, label = _reference_tail(s, pos, i, defs)
            if dest is None:
                if form is not None:
                    unresolved.append((pos, label, form, is_img))
                i += 1                  # literal `]`; the opener is gone
                continue
        if is_img:
            images.append((i, end))
        else:
            out.append((i, end, dest))
            for opener in stack:        # links may not contain links
                if not opener[1]:
                    opener[2] = False
        i = end
    return code, out, images, unresolved


def links(s, defs):
    """The bracket half of `inline_pass` -> (links, images, unresolved)."""
    return inline_pass(s, defs)[1:]


def definition_block(block, off):
    """Phase 1 ("Appendix: A parsing strategy", block structure): the
    reference definition (§4.7) starting at offset `off` of `block` -- the
    RAW text of the rest of the paragraph block (every remaining line up to
    the next blank line, fence or table row; what the block phase hands
    over), or None.  Returns (label, destination, end_offset).  The label
    (`link_label`, §6.3: up to 999 characters, may span lines) and the
    title (`link_title`) are read over that whole text, and §4.7 "may not
    contain a blank line" holds by construction: the block ends at one.
    Read before any inline parsing: a backtick in the destination
    (`[sib]: slice`x`.md`) is destination text, not a code span.  The caller
    decides whether `off` is a block start (a definition cannot interrupt a
    paragraph); here the definition is only recognised."""
    defs, _ = reference_definitions(block, limit=1, start=off)
    return defs[0] if defs else None


def definition_shape(block, off):
    """Whether `block[off:]` OPENS like a definition -- up to three spaces, a
    link label, a colon -- whatever follows.  A line of this shape that is
    not a definition (inside a paragraph, or invalid: a title crossing a
    blank line, junk after the destination) is text the author meant as a
    definition: an orphan, so that a shortcut naming its label is reported
    rather than exempted.  Returns the raw label or None."""
    j = off
    while j < off + 3 and j < len(block) and block[j] == " ":
        j += 1
    raw, k = link_label(block, j)
    if raw is None or k >= len(block) or block[k] != ":":
        return None
    return raw


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


# --------------------------------------------------------------------------
# Bare tokens (no CommonMark construct, but a tokenisation fact of these
# documents): a `[C19]`-style citation id and a bare `.md` file name are read
# as one token, never as a run of row ids.  Found over the raw text, so a
# file name inside a code span is a token too.
# --------------------------------------------------------------------------

_TOKEN = re.compile(r"(?P<cite>\[[A-Z][0-9]+\])|(?P<file>[\w./-]+\.md\b)")


class Lexed:
    """The lexical facts of one block's INLINE content (a paragraph or a
    cell) -- Phase 2 of "Appendix: A parsing strategy"; block structure
    (fences, reference definitions, tables, paragraphs) is Phase 1, decided
    over raw lines by `plan_memo_tables.py::Memo`, and a reference
    definition is never inline content.  `tokens` = [(start, end, "cite" |
    "file")] over the raw text.  `resolve(defs)` runs `inline_pass` and sets
    `code` = code spans, `links` = [(tail_start, end, destination)],
    `images` = [(tail_start, end)] and `unresolved` = [(offset, label, form,
    is_image)] of the references no definition answers; `mask` is set by the
    disposition step in `plan_memo_tables.py` once the row ids are known."""

    __slots__ = ("text", "code", "tokens", "links", "images", "unresolved", "mask")

    def __init__(self, text):
        self.text = text
        self.tokens = [(m.start(), m.end(), m.lastgroup) for m in _TOKEN.finditer(text)]
        self.code, self.links, self.images, self.unresolved = [], [], [], []
        self.mask = None

    def resolve(self, defs):
        """The inline pass over the RAW text (code spans and brackets
        together; no pre-mask), with `defs` = normalised label -> destination."""
        self.code, self.links, self.images, self.unresolved = inline_pass(self.text, defs)
