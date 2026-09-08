#!/usr/bin/env python3
"""The BARE TOKENS of this document family -- a `[C19]`-style citation id and a
bare `.md` file name -- for `plan-memo-umbrella-check.py`.

NOT COMMONMARK, which is the seam.  Every clause in `plan_memo_lexer.py`,
`plan_memo_html.py` and `plan_memo_blocks.py` belongs to a published grammar;
neither of these two does.  They are a tokenisation fact of the memos this
checker reads: a run of characters a reader sees as ONE name or ONE citation,
and out of which no scanner may read a row id.  They are found over whatever
text the caller holds -- rendered or raw, `file_and_cite_spans` says which
caller holds which and why -- never over a parse.

So this module sits BESIDE the lexer rather than inside it, and below it: it
imports the id grammar and `re`, and nothing of CommonMark, while the lexer
imports nothing of this.  Its consumers are the disposition
(`plan_memo_tables.dispose`), the raw-line seed
(`plan-memo-umbrella-check.py`), and `plan_memo_sibling`, which reads
`FILE_SUFFIX` from here for the ONE test on a link destination -- the
correspondence PR #510 R26-2 had to repair, and the reason the constant and the
token grammar consuming it belong in the same file as each other rather than in
the same file as §6.3.
"""

import re

from plan_memo_ids import ALNUM, CITE_ID

# --------------------------------------------------------------------------
# Bare tokens (no CommonMark construct, but a tokenisation fact of these
# documents): a `[C19]`-style citation id and a bare `.md` file name are read
# as one token, never as a run of row ids.  Found over the raw text, so a
# file name inside a code span is a token too.
# --------------------------------------------------------------------------

# WHAT A FILE NAME IS, decided here, once: a name is anything that ENDS IN
# `FILE_SUFFIX` -- the stem is unconstrained, so the suffix alone (`.md`) is
# a file name.  `plan_memo_sibling.sibling_path` stage (d) CONSUMES this
# constant for the same test on a link destination (a link to `.md` names
# the sibling file `.md`); the lexer defines it because the lexer sits below
# the resolver and reads it first.  Until PR #510 R20 the token arm required a
# stem of one character or more while `sibling_path` accepted the bare
# suffix, so beside a declared id `md` the prose `Read .md for details`
# reported `md` as a naming site.
FILE_SUFFIX = ".md"

# A bare `.md` file name is read by PATH SYNTAX, not a character class: the
# maximal run (possibly EMPTY -- the rule above) of non-whitespace characters
# ending in `FILE_SUFFIX`, bounded by spaces / tabs / line ends or the cell
# edge (`9z+notes.md`, `9z@notes.md`, `計画.md`, `.md` are file names --
# what `sibling_path` accepts), with the inline delimiters `[` `]` `<` `>`
# `` ` `` `|` excluded so a link's visible text (`[Slice 9z](slice-9z-sib.md)`)
# and a code span are not swallowed, and parentheses admitted only in BALANCED
# UNESCAPED PAIRS -- §6.3's own rule for a link destination, at any nesting
# depth (`(9z).md`, `foo((9z)).md`, `(m.md(9z)md).md`).  Trailing closing
# punctuation (`)` `,` `.` `;`) needs no autolink-style stripping rule: the
# token ENDS at the suffix, so anything after it is outside by construction
# (the GFM §6.9 extended-autolink trailing-punctuation rule is moot here, and
# is why none is picked).  The end boundary is the grammar's ASCII class
# (`ALNUM`), so `x.mdの` still ends the token; the citation shape is the
# grammar's `CITE_ID` (either case -- `[c1]` is `[C1]` under §6.3 label
# matching, and the unresolved-reference walk exempts it by the same
# predicate).  No §2.4 escape is honoured and none should be: the disposition
# hands this the block AS RENDERED, where `\(` has already become `(`, and the
# raw-line seed hands it a line that is never inline-parsed, where a backslash
# IS the character the reader sees.
#
# ⚠ WHY THIS IS A SCAN AND NOT A PATTERN (PR #510 R26-2).  "Balanced at any
# depth" is not a regular language, and the arm that stood here until R26
# approximated it with a FLAT chunk (`\([^\s()]*\)`).  That made the two
# readings of "is this a file name" disagree: over the prose `foo((9z)).md` the
# pattern could match only the suffix `.md`, leaving the declared id `9z`
# exposed to the naming scan, while `sibling_path` -- documented right here as
# consuming `FILE_SUFFIX` for the SAME test -- accepts nested-parenthesis `.md`
# paths without a murmur.  The reader that moved is THIS one, and it moved to
# the rule §6.3 already spells once for a link destination, because the
# resolver has no boundaries to find (its input is a destination the link
# grammar already delimited) while this reader has nothing BUT boundaries to
# find, so a weaker paren rule here was the only one of the two that was ever
# an approximation.
#
# AND IT IS LINEAR, which the pattern was not: `re` re-entered the arm at every
# start position, so `(a)`xN and `a`xN cost quadratic time (measured 3.95x /
# 4.00x per doubling; this scan is 2.00x / 1.98x).  One pass, one paren stack.
_CITE_TOKEN = re.compile(CITE_ID)
_ALNUM_AT = re.compile(ALNUM)

# The characters that BOUND a file-name run and are not whitespace: the inline
# delimiters the rule above excludes.  Whitespace is asked of the character
# itself (`str.isspace()`, which agrees with `re`'s `\s` on every code point of
# planes 0-1, verified over 0x0000-0x11000).
_NAME_BOUNDARY = frozenset("[]<>`|")


def file_and_cite_spans(text):
    """[(start, end, "cite" | "file")] over `text`: the ONE reading of "a
    bare `.md` file name / a citation id stands here" (the rule above), given
    a name so that reading has exactly one caller-visible spelling.

    ONE LEFT-TO-RIGHT PASS with a paren stack, then one selection.  The pass
    records, for every position where `FILE_SUFFIX` ends and the next character
    is not `ALNUM`, the LEFTMOST start a run ending there may have: one past
    the innermost parenthesis still open there, or the start of the current
    SEGMENT (the text since the last whitespace, inline delimiter, or
    unmatchable `)` -- none of which any run may contain, and none of which any
    run may cross).  The selection is leftmost-longest, the reading a pattern
    would have given: per start keep the longest end (the ends arrive in
    increasing order), then walk the starts upward taking each one that begins
    at or after the previous token's end.  Citations are matched separately and
    merged by position -- they cannot overlap a file name, because `[` and `]`
    bound one.

    WHICH TEXT is the caller's to say, and the two callers say different
    things because they hold different texts (PR #510 R24; until then both
    read raw source and one of them was wrong about it):

      * the DISPOSITION (`plan_memo_tables.dispose`, stage 2) hands it the
        block AS A READER SEES IT and maps the spans back to source offsets.
        It must: a file name's boundaries are whitespace boundaries, and §2.5
        can put whitespace where the source has none, so over the source
        `9z&#32;notes.md owns it` the maximal run is the whole of
        `9z&#32;notes.md` while the document reads `9z notes.md owns it`,
        where `9z` is a naming site the raw reading masked away.
      * the RAW-LINE seed (`plan-memo-umbrella-check.py::lex_unsupported_seed`)
        hands it the line AS WRITTEN, because for that line raw text IS the
        rendered text: the line belongs to a raw extent or a §6.6 span the
        inline parser never enters, so no §2.4 escape and no §2.5 reference
        is ever applied to it -- and the seed's own id scan reads that same
        raw line, so its two halves agree by construction.  ⚠ DECLARED MISS:
        that makes the seed's "had the text been prose" counterfactual a
        BLOCK-level one, not an inline one -- a `&#32;` on a raw HTML line
        seeds as the six characters it is written as, exactly as the seed's
        id scan reads them.  A seed never gates, and the alternative (inline-
        parsing a line whose whole point is that it is not inline-parsed)
        would be a second rendering of a text that has none.
        Until PR #510 R22 the seed scanned the raw line with a hand-written
        half of this predicate (the citation arm only), so
        `<div data-note="slice-9z-sib.md">` reported a `9z` naming site that
        the identical file name in a paragraph does not."""
    n, k = len(text), len(FILE_SUFFIX)
    stack, seg, longest = [], 0, {}
    for i, c in enumerate(text):
        if c in _NAME_BOUNDARY or c.isspace():
            del stack[:]
            seg = i + 1
            continue
        if c == "(":
            stack.append(i)
        elif c == ")":
            if stack:
                stack.pop()
            else:
                seg = i + 1     # an unmatchable `)`: no run holds it, none crosses it
        e = i + 1
        if text[e - k:e] == FILE_SUFFIX and (e == n or not _ALNUM_AT.match(text, e)):
            s = stack[-1] + 1 if stack else seg
            if s <= e - k:      # the suffix itself must lie inside the run
                longest[s] = e
    out, pos = [], 0
    for s in sorted(longest):
        e = longest[s]
        if s >= pos and e > pos:
            out.append((s, e, "file"))
            pos = e
    out.extend((m.start(), m.end(), "cite") for m in _CITE_TOKEN.finditer(text))
    out.sort()
    return out


