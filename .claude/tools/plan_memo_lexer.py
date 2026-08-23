#!/usr/bin/env python3
"""Lexical substrate for `plan-memo-umbrella-check.py`: a subset of
CommonMark 0.31.2 and GFM 0.29, lexed by construction from the clauses
`docs/plans/2026-08-plan-memo-umbrella-checker.md` §3 lists.

Order (plan §2 "Lexing order"): fenced blocks (CommonMark §4.5) are masked
first; a GFM table row is split on RAW unescaped `|` (GFM §4.10, incl. inside
backticks -- Example 200); per block inline content (a paragraph or a cell)
code spans (CommonMark §6.1, backtick strings of equal length) are lexed, then
links (CommonMark §6.3 / §4.7) over the stream with code spans masked.

What is NOT lexed, and is read as written: CommonMark §4.4 indented code, §4.6
HTML blocks, §5 container blocks (block quotes §5.1, list items §5.2 -- a
list-item or `>` line only ENDS a paragraph here, its content is not
re-parsed as a nested document), §6.5 autolinks, §2.5 entity references.
Nothing here detects them.

`Lexed` is the one lexical value per block: code spans, the leading run of
reference definitions, links, and the two bare tokens the scanners must not
read an id out of (`[C19]`-style citation ids, `.md` file names).  Nothing in
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
    return not line.strip()


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
        self.lexed = Lexed(text, cell=True)

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
    bounds, breaks, start, i, n = [], [], 0, 0, len(line)
    while i < n:
        c = line[i]
        if c == "\\" and i + 1 < n and line[i + 1] == "|":
            breaks.append(i)
            i += 2
        elif c == "|":
            bounds.append((start, i))
            start = i + 1
            i += 1
        else:
            i += 1
    bounds.append((start, n))
    stripped = line.strip()
    if stripped.startswith("|") and bounds:
        bounds = bounds[1:]
    if stripped.endswith("|") and not stripped.endswith("\\|") and bounds:
        bounds = bounds[:-1]
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
# content, never a line.
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
    """[(start, end)] of every code span in `s`, backticks included.

    An OPENING backtick string behind an odd run of backslashes loses its
    first backtick to the escape (§2.4 / §6.1: a backslash-backtick pair is a literal
    backtick, not a backtick string of length one).  Inside a span backslashes
    are literal (§6.1: "backslash escapes do not work in code spans"), so a
    closer is read raw."""
    runs = [(m.start(), m.end()) for m in _BACKTICKS.finditer(s)]
    out, i = [], 0
    while i < len(runs):
        a0, a1 = runs[i]
        if _escaped(s, a0):
            a0 += 1
        n = a1 - a0
        j = i + 1
        while n and j < len(runs) and runs[j][1] - runs[j][0] != n:
            j += 1
        if n and j < len(runs):
            out.append((a0, runs[j][1]))
            i = j + 1
        else:
            i += 1
    return out


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
    ending"; §4.7 says the same of a definition's components."""
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


def _bracket_text(s, i):
    """`s[i] == '['`: the balanced bracket text starting here -> (text, end
    after `]`, contains_bracket) or None when unbalanced.  §6.3 link text:
    brackets inside it only backslash-escaped or as a matched pair; "links
    may not contain other links" -- `contains_bracket` is what the caller
    uses to refuse a collapsed / shortcut reading of nested bracket text."""
    depth, j, inner = 0, i, False
    while j < len(s):
        if _is_escape(s, j):
            j += 2
            continue
        if s[j] == "[":
            depth += 1
            if depth > 1:
                inner = True
        elif s[j] == "]":
            depth -= 1
            if depth == 0:
                return s[i + 1:j], j + 1, inner
        j += 1
    return None


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


def links(s, defs):
    """Every link in `s` (a block's inline content with code spans masked),
    resolved through `defs` (normalised label -> destination).

    Returns ([(tail_start, end, destination)], [(offset, label)]): `tail_start`
    is the `]` closing the link text, so a caller masking the tail leaves the
    visible text -- the label, which is prose -- in the scanned stream.  Forms:
    inline `[text](dest "title")`; full `[text][label]`; collapsed `[text][]`;
    shortcut `[text]` (a link label not followed by `[]` or a link label).

    The second list is every full or collapsed reference whose label `defs`
    does not define: such a site is prose under §6.3, and a population the
    author meant to link is silently lost unless the caller reports it.  A
    shortcut `[text]` is not listed (every `[C19]` citation is one); the memo
    reports a shortcut only when a definition of its label exists somewhere
    the grammar cannot read it.
    """
    out, unresolved, i = [], [], 0
    while True:
        i = s.find("[", i)
        if i < 0:
            break
        # a `[` behind an odd run of backslashes is escaped (§2.4: the pairs
        # before it escape each other)
        k = i
        while k > 0 and s[k - 1] == "\\":
            k -= 1
        if (i - k) % 2:
            i += 1
            continue
        bt = _bracket_text(s, i)
        if bt is None:
            i += 1
            continue
        text, close, inner = bt
        tail = close - 1
        if close < len(s) and s[close] == "(":
            r = _inline_tail(s, close + 1)
            if r is not None:
                out.append((tail, r[1], r[0]))
                i = r[1]
                continue
        if close < len(s) and s[close] == "[":
            if close + 1 < len(s) and s[close + 1] == "]":
                # collapsed: the label is the text
                dest = defs.get(normalize_label(text)) if not inner else None
                if dest is not None:
                    out.append((tail, close + 2, dest))
                    i = close + 2
                    continue
                if not inner:
                    unresolved.append((i, text))
                i += 1
                continue
            raw, end = link_label(s, close)
            if raw is not None:
                dest = defs.get(normalize_label(raw))
                if dest is not None:
                    out.append((tail, end, dest))
                    i = end
                    continue
                # a link label follows, so `[text]` is not a shortcut either
                unresolved.append((i, raw))
                i += 1
                continue
        if not inner and _has_label_content(text) and len(text) <= 999:
            dest = defs.get(normalize_label(text))
            if dest is not None:
                out.append((tail, close, dest))
                i = close
                continue
            unresolved.append((i, text))
        i += 1
    return out, unresolved


def reference_definitions(s):
    """CommonMark §4.7 link reference definitions at the START of `s` (a
    paragraph's inline content; a definition cannot interrupt a paragraph).
    Consumes consecutive definitions.  Returns ([(label, dest, end)], rest_offset).

    Grammar: <=3 spaces, a link label, `:`, optional whitespace incl. up to one
    line ending, a destination, optionally whitespace incl. up to one line
    ending and a title, then nothing but spaces/tabs before the line ending.
    """
    out, i = [], 0
    while True:
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
        eol = _line_end(s, k)
        if eol is None:
            # a title may follow on this line or the next
            k2 = _skip_ws(s, k)
            if k2 == k:
                break
            t = link_title(s, k2)
            eol = _line_end(s, t) if t is not None else None
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


# --------------------------------------------------------------------------
# Bare tokens (no CommonMark construct, but a tokenisation fact of these
# documents): a `[C19]`-style citation id and a bare `.md` file name are read
# as one token, never as a run of row ids.  Found over the raw text, so a
# file name inside a code span is a token too.
# --------------------------------------------------------------------------

_TOKEN = re.compile(r"(?P<cite>\[[A-Z][0-9]+\])|(?P<file>[\w./-]+\.md\b)")


class Lexed:
    """The lexical facts of one block's inline content (a paragraph or a
    cell), computed once: `code` = code spans; `definitions` / `defs_end` =
    the leading run of reference definitions over the code-masked stream (a
    paragraph only -- a table cell is inline content under GFM §4.10 and
    holds no §4.7 definition, so `cell=True` parses none); `tokens` =
    [(start, end, "cite" | "file")].  `resolve` then sets `links` =
    [(tail_start, end, destination)] and `unresolved` = [(offset, label)] of
    the references no definition answers; `mask` is set by the disposition
    step in `plan_memo_tables.py` once the row ids are known."""

    __slots__ = ("text", "code", "_masked", "definitions", "defs_end", "tokens",
                 "links", "unresolved", "mask")

    def __init__(self, text, cell=False):
        self.text = text
        self.code = code_spans(text)
        masked = blank_spans(text, self.code)
        self.definitions, self.defs_end = ([], 0) if cell else reference_definitions(masked)
        self._masked = masked
        self.tokens = [(m.start(), m.end(), m.lastgroup) for m in _TOKEN.finditer(text)]
        self.links, self.unresolved = [], []
        self.mask = None

    def resolve(self, defs):
        """Links (§6.3 / §4.7) over the masked stream, past the definitions;
        `defs` = normalised label -> destination."""
        start = self.defs_end
        found, unresolved = links(self._masked[start:], defs)
        self.links = [(a + start, b + start, dest) for a, b, dest in found]
        self.unresolved = [(a + start, label) for a, label in unresolved]

    def orphan_definitions(self):
        """[(offset, label)] of definition-shaped lines the grammar could not
        read as definitions (a §4.7 definition cannot interrupt a paragraph),
        for the memo's unresolved-reference report; `offset` is where the
        line starts, which is where its label bracket is read as a shortcut."""
        out, off = [], self.defs_end
        for line in self._masked[self.defs_end:].split("\n"):
            defs, _ = reference_definitions(line)
            if defs:
                out.append((off + (len(line) - len(line.lstrip(" "))), defs[0][0]))
            off += len(line) + 1
        return out
