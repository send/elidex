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
HTML blocks, §6.5 autolinks, §2.5 entity references.  Nothing here detects them.

Nothing in this module knows what a row id is; the disposition exception (an
id-only code span is the document spelling an id, not code) is applied by the
caller through `mask_spans(keep=...)`.
"""

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
    and tabs after it.  An unclosed fence runs to the end of the document.
    """
    out, i, n = set(), 0, len(lines)
    while i < n:
        m = _FENCE_OPEN.match(lines[i])
        if not m or (m.group(1)[0] == "`" and "`" in m.group(2)):
            i += 1
            continue
        ch, k = m.group(1)[0], len(m.group(1))
        closer = re.compile(r"^ {0,3}%s{%d,}[ \t]*$" % (re.escape(ch), k))
        out.add(i)
        i += 1
        while i < n:
            out.add(i)
            i += 1
            if closer.match(lines[i - 1]):
                break
    return out


# --------------------------------------------------------------------------
# Block starts that end a paragraph or a table (CommonMark §4 / GFM §4.10:
# "the table is broken at the first empty line, or beginning of another
# block-level structure").  ATX headings and thematic breaks are one-line
# blocks; a list item or a `>` line starts a new paragraph.
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
# consumed).  Leading / trailing pipe optional; cell content is trimmed.
# --------------------------------------------------------------------------


class Cell:
    """One body cell: `text` is the trimmed, unescaped content; `start` is the
    raw column of its first character; `raw(i)` maps an offset into `text`
    back to a raw column (an unescaped `\\|` shifts everything after it by one)."""

    __slots__ = ("text", "start", "_raw")

    def __init__(self, chars):
        # chars = [(char, raw_index)], already trimmed
        self.text = "".join(c for c, _ in chars)
        self._raw = [r for _, r in chars]
        self.start = self._raw[0] if self._raw else 0

    def raw(self, i):
        if i < len(self._raw):
            return self._raw[i]
        return (self._raw[-1] + 1) if self._raw else self.start

    def __repr__(self):
        return "Cell(%r@%d)" % (self.text, self.start)


def split_row(line):
    """-> [Cell, ...]: body cells of a GFM row (optional leading / trailing pipe
    stripped), split on unescaped `|` BEFORE any inline lexing."""
    parts, cur, i, n = [], [], 0, len(line)
    while i < n:
        c = line[i]
        if c == "\\" and i + 1 < n and line[i + 1] == "|":
            cur.append(("|", i + 1))
            i += 2
        elif c == "|":
            parts.append(cur)
            cur = []
            i += 1
        else:
            cur.append((c, i))
            i += 1
    parts.append(cur)
    stripped = line.strip()
    if stripped.startswith("|") and parts:
        parts = parts[1:]
    if stripped.endswith("|") and not stripped.endswith("\\|") and parts:
        parts = parts[:-1]
    out = []
    for p in parts:
        a, b = 0, len(p)
        while a < b and p[a][0] in " \t":
            a += 1
        while b > a and p[b - 1][0] in " \t":
            b -= 1
        cell = Cell(p[a:b])
        if not p[a:b]:
            # an empty cell still has a position: the column after its pipe
            cell.start = p[0][1] if p else 0
        out.append(cell)
    return out


_DELIM_CELL = re.compile(r":?-+:?")


def is_separator(cells):
    """GFM delimiter row: every cell is >=1 hyphen with an optional leading and
    trailing colon.  The row must carry at least one pipe (a bare `---` line is
    a setext underline or thematic break under CommonMark, not a table)."""
    return len(cells) > 0 and all(_DELIM_CELL.fullmatch(c.text) for c in cells)


def delimiter_width(line):
    """Cell count of `line` if it is a GFM delimiter row, else None."""
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


def code_spans(s):
    """[(start, end)] of every code span in `s`, backticks included."""
    runs = [(m.start(), m.end()) for m in _BACKTICKS.finditer(s)]
    out, i = [], 0
    while i < len(runs):
        a0, a1 = runs[i]
        n = a1 - a0
        j = i + 1
        while j < len(runs) and runs[j][1] - runs[j][0] != n:
            j += 1
        if j < len(runs):
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
    """Spaces, tabs and up to `newlines` line endings."""
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
        if s[i] == "\\" and i + 1 < len(s) and s[i + 1] in ASCII_PUNCT:
            out.append(s[i + 1])
            i += 2
        else:
            out.append(s[i])
            i += 1
    return "".join(out)


def _is_escape(s, j):
    return s[j] == "\\" and j + 1 < len(s) and s[j + 1] in ASCII_PUNCT


def link_destination(s, i):
    """Link destination at `i` -> (destination, end) or (None, i).

    `<...>`: no line ending, no unescaped `<` or `>`.  Bare: nonempty, no ASCII
    control character (0x00-0x1F, 0x7F) or space, does not start with `<`,
    parentheses only backslash-escaped or in balanced unescaped pairs.
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


def normalize_label(label):
    """§6.3 label matching: Unicode case fold, strip, collapse internal
    whitespace to one space."""
    return " ".join(label.split()).casefold()


def link_label(s, i):
    """A link label opening at `s[i] == '['` -> (raw_label, end) or (None, i).
    Up to 999 characters between the brackets, no unescaped `[` or `]`, at
    least one non-whitespace character."""
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
    if len(raw) > 999 or not raw.strip():
        return None, i
    return raw, j + 1


def _bracket_text(s, i):
    """`s[i] == '['`: the balanced bracket text starting here -> (text, end
    after `]`, contains_bracket) or None when unbalanced."""
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
    """After `](` at `k` -> (destination, end after `)`) or None."""
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

    Returns [(tail_start, end, destination)]: `tail_start` is the `]` closing
    the link text, so a caller masking the tail leaves the visible text -- the
    label, which is prose -- in the scanned stream.  Forms: inline
    `[text](dest "title")`; full `[text][label]`; collapsed `[text][]`;
    shortcut `[text]` (a link label not followed by `[]` or a link label).
    """
    out, i = [], 0
    while i < len(s):
        if _is_escape(s, i):
            i += 2
            continue
        if s[i] != "[":
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
                i += 1
                continue
        if not inner and text.strip() and len(text) <= 999:
            dest = defs.get(normalize_label(text))
            if dest is not None:
                out.append((tail, close, dest))
                i = close
                continue
        i += 1
    return out


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
