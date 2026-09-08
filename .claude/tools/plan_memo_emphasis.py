#!/usr/bin/env python3
"""CommonMark 0.31.2 §6.2 emphasis (and GFM 0.29 strikethrough): which
delimiter runs PAIR, and therefore which characters of a block render as
nothing at all.

WHY THIS EXISTS.  The scanners read the text a reader sees
(`plan_memo_tables.stream`), so every construct must say what characters it
contributes.  A `*` is the one character whose answer is not local: in
`**9z**7z` the four asterisks contribute nothing, in `9*z` the one asterisk
contributes itself, and only the §6.2 matching rules tell them apart.  Guess
either way and the stream lies -- drop an unmatched run and `9*z` fabricates
the row `9z`; keep a matched one and `Slice 9**z**`, which renders `Slice 9z`,
reports the row `9` (PR #510 design re-gate 4: a fabricated site on one row
and a lost site on another, and the same split hid a kind marker).

WHAT IS IMPLEMENTED.  The spec's own two halves, nothing more:

  * `run_at` + the flanking rules (§6.2 "delimiter run", "left-flanking
    delimiter run", "right-flanking delimiter run" and the six numbered
    rules' opening / closing conditions).  A delimiter run is a maximal run
    of `*`, `_` (CommonMark) or `~` (GFM strikethrough, one or two: "any
    text wrapped in a matching pair of one or two tildes", so a run of three
    or more is literal -- `a~~~b~~~c` renders verbatim, measured against
    GitHub's pipeline).  The classes are Unicode by the spec's own words
    ("Unicode whitespace character", "Unicode punctuation character" = a
    character of general category P* or S*, 0.31's definition), never ASCII:
    the surrounding text of these memos is Japanese as often as not.
  * `process` -- "Appendix: A parsing strategy", `process_emphasis`: one
    left-to-right walk over the delimiter stack, each closer matched to the
    nearest still-open compatible opener at or above `bottom`, with the
    spec's "rule of three" and commonmark.js's `openers_bottom` memo so a
    closer that found no opener is never re-searched below the same point.

WHAT IS NOT.  Nothing here builds a tree: the caller wants the CHARACTER
SPANS the delimiters occupy, since those are what the stream drops.  A
delimiter that pairs contributes nothing; a delimiter left over at the end
is literal text and contributes itself.  The one caller is
`plan_memo_lexer.inline_pass`, which pushes the runs as it meets them (so a
run inside a code span, an autolink, a raw HTML span or a link destination is
never a delimiter -- the scan jumps past those) and calls `process` where
the Appendix does: when a link or an image closes, over the delimiters
inside it, and once more at the end of the block.

The falsifier is the spec's own example list: `Emphasis and strong emphasis`
Examples 350-480, vendored with the other inline sections and consumed by
`plan_memo_selftest_conformance.py`, which requires one `<em>` per pair of
length 1 and one `<strong>` per pair of length 2 in the html the spec prints.
"""

import unicodedata

DELIMS = "*_~"

# GFM 0.29 "Strikethrough (extension)": "any text wrapped in a matching pair
# of one or two tildes" -- a run of three or more opens nothing (measured:
# `a~~~b~~~c` renders verbatim on GitHub's pipeline, while `a~b~c` and
# `a~~b~~c` both render `<del>b</del>`).
_MAX_RUN = {"~": 2}

# §2.1: "A Unicode whitespace character is a character in the Unicode Zs
# general category, or a tab (U+0009), line feed (U+000A), form feed
# (U+000C), or carriage return (U+000D)."  The line start counts as one
# ("the beginning and the end of the line count as Unicode whitespace").
_WS = "\t\n\f\r"


def _is_ws(ch):
    return ch is None or ch in _WS or unicodedata.category(ch) == "Zs"


def _is_punct(ch):
    """§2.1 (0.31.2): "A Unicode punctuation character is a character in the
    Unicode P (puncuation) or S (symbol) general categories." """
    return ch is not None and unicodedata.category(ch)[0] in "PS"


class Delimiter:
    """One delimiter run: the span it still occupies (`start` / `end`, which
    shrink from the inside as pairs consume it), its character, the ORIGINAL
    run length the rule of three reads, and whether it may open / close."""

    __slots__ = ("start", "end", "char", "orig", "can_open", "can_close", "dead")

    def __init__(self, start, end, char, can_open, can_close):
        self.start, self.end, self.char = start, end, char
        self.orig = end - start
        self.can_open, self.can_close, self.dead = can_open, can_close, False

    @property
    def length(self):
        return self.end - self.start


def run_at(s, i):
    """The delimiter run of `s` starting at `i` (the caller enters at the
    run's first character and jumps to its `end`, so a run is read once and
    whole), or None when `s[i]` is no delimiter character.  A run GFM does not
    admit -- three or more tildes -- is still the RUN, and simply opens and
    closes nothing, because reading it as a shorter one from its second
    character is how `a~~~b~~~c`, which renders verbatim, would grow a
    strikethrough.  The
    characters around the run are read from the RAW text, as commonmark.js
    reads them (`this.subject.charAt(this.pos - 1)`), so a run after a code
    span is preceded by that span's backtick -- punctuation -- and the block's
    first character is preceded by nothing, which the spec reads as
    whitespace.

    §6.2: "A left-flanking delimiter run is a delimiter run that is (1) not
    followed by Unicode whitespace, and either (2a) not followed by a Unicode
    punctuation character, or (2b) followed by a Unicode punctuation
    character and preceded by Unicode whitespace or a Unicode punctuation
    character."  Right-flanking is the mirror.  Then: a `*` run can open iff
    it is left-flanking and close iff it is right-flanking; a `_` run can
    open iff it is left-flanking and either not right-flanking or preceded by
    punctuation, and close iff it is right-flanking and either not
    left-flanking or followed by punctuation (§6.2's rules 5 and 6, which
    keep `snake_case` intact).  A `~` run reads the `*` conditions
    (cmark-gfm's strikethrough extension).
    """
    ch = s[i]
    if ch not in DELIMS:
        return None
    j = i
    while j < len(s) and s[j] == ch:
        j += 1
    if j - i > _MAX_RUN.get(ch, len(s)):
        return Delimiter(i, j, ch, False, False)
    prev = s[i - 1] if i else None
    nxt = s[j] if j < len(s) else None
    left = not _is_ws(nxt) and (not _is_punct(nxt) or _is_ws(prev) or _is_punct(prev))
    right = not _is_ws(prev) and (not _is_punct(prev) or _is_ws(nxt) or _is_punct(nxt))
    if ch == "_":
        return Delimiter(i, j, ch, left and (not right or _is_punct(prev)),
                         right and (not left or _is_punct(nxt)))
    return Delimiter(i, j, ch, left, right)


def _matches(opener, closer):
    """Whether `closer` may take `opener`.

    §6.2 rule 9/10, the "rule of three": "If one of the delimiters can both
    open and close emphasis, then the sum of the lengths of the delimiter
    runs containing the opening and closing delimiters must not be a multiple
    of 3 unless both lengths are multiples of 3."  The lengths are the
    ORIGINAL run lengths (commonmark.js's `origdelims`), not what is left of
    them.  GFM strikethrough instead pairs equal lengths only (`~~x~` is no
    strikethrough), which is that extension's whole matching rule.
    """
    if opener.char == "~":
        return opener.length == closer.length
    if (closer.can_open or opener.can_close) and closer.orig % 3 and (opener.orig + closer.orig) % 3 == 0:
        return False
    return True


def process(delims, bottom=0):
    """Pair the delimiters of `delims[bottom:]` -- "Appendix: A parsing
    strategy", `process_emphasis` -- and return the pairs as
    `(open_start, open_end, close_start, close_end, char, use)`, `use` being
    the number of delimiter characters each side spends (2 = strong, 1 =
    emphasis).  Delimiters that pair are consumed from the INSIDE, so
    `***a***` yields a strong pair inside an emphasis pair; delimiters left
    over are literal and are simply not returned.

    A pair is `("em", …)` unless the caller demotes it: §6.4 renders a
    resolved image's description as the `alt` attribute's plain string
    content, so emphasis inside one contributes its characters and no tag at
    all (`![foo *bar*]` is `alt="foo bar"`), exactly as a link there is
    demoted to a masked tail.

    `openers_bottom` is commonmark.js's: a closer that finds no opener
    records where its search stopped, keyed by (character, its run length mod
    3, whether it can also open), so the next closer of the same key never
    searches below that point again -- without it a paragraph of unmatched
    delimiters is quadratic.  The walk removes a closer that cannot also open
    when it fails, and every delimiter between a matched pair (they are
    inside the emphasis and can never pair with anything outside it).
    """
    pairs, openers_bottom, closer = [], {}, bottom
    while closer < len(delims):
        d = delims[closer]
        if d.dead or not d.can_close:
            closer += 1
            continue
        key = (d.char, d.orig % 3, d.can_open)
        floor = openers_bottom.get(key, bottom)
        j, found = closer - 1, None
        while j >= floor:
            o = delims[j]
            if not o.dead and o.char == d.char and o.can_open and _matches(o, d):
                found = j
                break
            j -= 1
        if found is None:
            openers_bottom[key] = closer
            if not d.can_open:
                d.dead = True
            closer += 1
            continue
        o = delims[found]
        use = 2 if o.length >= 2 and d.length >= 2 else 1
        if o.char == "~":
            use = o.length
        pairs.append((o.end - use, o.end, d.start, d.start + use, o.char, use, "em"))
        o.end -= use
        d.start += use
        for k in range(found + 1, closer):
            delims[k].dead = True
        if not o.length:
            o.dead = True
        if not d.length:
            d.dead = True
            closer += 1
    for d in delims[bottom:]:
        d.dead = True
    return pairs
