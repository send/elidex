#!/usr/bin/env python3
"""Phase 2 of CommonMark 0.31.2 "Appendix: A parsing strategy" -- INLINE
structure -- for `plan-memo-umbrella-check.py`: a subset of CommonMark
0.31.2 and GFM 0.29, lexed by construction from the clauses
`docs/plans/2026-08-plan-memo-umbrella-checker.md` §3 lists.  Block
structure (Phase 1: fences, block starts, the one `block_end` predicate,
table rows, reference definitions) is `plan_memo_blocks.py`, which imports
this module's inline grammar; the order is plan §2 "Lexing order".

Per block inline content (a paragraph or a cell): code spans (CommonMark
§6.1, backtick strings of equal length) and links / images (§6.3 / §6.4)
are lexed by ONE left-to-right pass (`inline_pass`): a code span is skipped
as met, an inline-link tail is parsed by lookahead on the raw text -- there
is no code pre-mask.  An image's bracket structure is parsed so that it is
not a link and a link may wrap it; its destination never joins the
population, its alt text is prose, its tail is masked.  Inline constructs
outside the lexed clauses (§6.5 autolinks, §2.5 entity references, §6.2
emphasis beyond the decoration the id grammar reads) are read as written;
the block types not modelled are the plan's §3.0 table.

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
    or after `a1` (§6.1: a code span "ends with a backtick string of equal
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

    Each failed reference is recorded ONCE.  After a failed FULL reference
    `[text][label]` (an image's too) the scan resumes after the literal
    `]`, so `[label]` is re-scanned -- it must be: §6.3 Example 571,
    `[foo][bar][baz]` with only `baz` defined, links `[bar][baz]`, and
    commonmark.js renders `![alt][missing][baz]` as `![alt]` plus that
    link.  When that re-scan closes as a SHORTCUT it fails for the very
    reason the full form did (same label, same `defs`) and is the same
    site, not a second one: it is not recorded, so `![alt][missing]` never
    leaves a bare `[missing]` behind for the orphan rule to read (§6.4: an
    undefined image reference is literal text, never a memo the author
    meant to link).
    """
    runs = [(m.start(), m.end()) for m in _BACKTICKS.finditer(s)]
    code, out, images, unresolved, stack, i, n = [], [], [], [], [], 0, len(s)
    relabel = -1        # the `[` of the label of the last failed full reference
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
                if form is not None and not (form == "shortcut" and pos == relabel):
                    unresolved.append((pos, label, form, is_img))
                if form == "full":
                    relabel = i + 1     # `[label]` is re-scanned next (Example 571), not re-recorded
                i += 1                  # literal `]`; the opener is gone; the tail is NOT consumed
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



# --------------------------------------------------------------------------
# Bare tokens (no CommonMark construct, but a tokenisation fact of these
# documents): a `[C19]`-style citation id and a bare `.md` file name are read
# as one token, never as a run of row ids.  Found over the raw text, so a
# file name inside a code span is a token too.
# --------------------------------------------------------------------------

# A bare `.md` file name is read by PATH SYNTAX, not a character class: the
# maximal run of non-whitespace characters ending in `.md`, bounded by
# spaces / tabs / line ends or the cell edge (`9z+notes.md`, `9z@notes.md`,
# `計画.md` are file names -- what `sibling_path` would accept), with the
# inline delimiters `[` `]` `<` `>` `` ` `` `|` excluded so a link's visible
# text (`[Slice 9z](slice-9z-sib.md)`) and a code span are not swallowed,
# and parentheses admitted only as a balanced pair (`(9z).md`).  Trailing
# closing punctuation (`)` `,` `.` `;`) needs no autolink-style stripping
# rule: the token ENDS at `.md`, so anything after it is outside by
# construction (the GFM §6.9 extended-autolink trailing-punctuation rule is
# moot here, and is why none is picked).  The end boundary is the ASCII id
# class, so `x.mdの` still ends the token.
_TOKEN = re.compile(r"(?P<cite>\[[A-Z][0-9]+\])"
                    r"|(?P<file>(?:[^\s\[\]()<>`|]|\([^\s()]*\))+\.md(?![0-9A-Za-z]))")


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
