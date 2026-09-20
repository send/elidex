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
(`plan_memo_stream.dispose`), the raw-line seed
(`plan-memo-umbrella-check.py`), and `plan_memo_sibling`, which reads
`FILE_SUFFIX` from here for the ONE test on a link destination -- the
correspondence PR #510 R26-2 had to repair, and the reason the constant and the
token grammar consuming it belong in the same file as each other rather than in
the same file as §6.3.
"""

import bisect
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
# depth (`(9z).md`, `foo((9z)).md`, `(m.md(9z)md).md`).
#
# ⚠ THE SUFFIX MUST TERMINATE THE RUN, apart from TRAILING PUNCTUATION (PR #510
# R34-1).  Until R34 the end test was only "not followed by an ASCII
# alphanumeric", so the reader took a PREFIX of a run whose whole extent is no
# file name at all: over `9z+notes.md_tail owns it` it masked `9z+notes.md`
# (because `_` is not alphanumeric), `sibling_path` rejects that run, and the
# declared umbrella id `9z` was hidden from the naming scan -- an ownership
# claim that produced no site, at rc 0.  This also contradicted the rule stated
# two paragraphs up, "the maximal run ... ending in FILE_SUFFIX", which
# `9z+notes.md` is not.
#
# So the GFM §6.9 extended-autolink trailing-punctuation rule is NOT moot and is
# picked: the run may end in a trailing-punctuation tail, and what precedes that
# tail must be the suffix.  ⚠ The earlier claim that `x.mdの` "still ends the
# token" is GONE, and the measurement is why: over all 71 memos of this family,
# `.md` is followed by a non-space character 41 times and EVERY ONE is trailing
# punctuation (`:` `)` `'` `;` `,` `.`) -- a non-ASCII continuation occurs ZERO
# times, so the case that clause defended has no instances, while the ASCII
# continuation it permitted is the hiding class above.  The citation shape is the
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

# The punctuation a file-name run may END in after the suffix: LOCAL POLICY,
# derived from what these documents actually write, and labelled as policy
# because no spec settles it (the §3 coverage map's own convention for a rule
# with no spec clause).
#
# ⚠ IT WAS ATTRIBUTED TO "GFM §6.9's extended-autolink trailing punctuation"
# UNTIL R38's design re-gate, AND IT IS NOT THAT SET.  GFM 0.29 §6.9 lists
# `? ! . , : * _ ~`; this one added `'` `"` `)` -- `)` is governed by GFM's
# separate parenthesis rule, not that sentence -- and, worse, OMITTED `;`,
# which this PR's OWN corpus measurement names among the characters that
# actually follow `.md` here (`:` `)` `'` `;` `,` `.`, 41 occurrences over 71
# memos).  Comment, set and ledger gave three different answers.  ⚠ No GFM
# artefact is vendored anywhere in this tree and `webref` does not cover GFM,
# so a GFM citation here could not have been checked by anything either.
#
# So it is the MEASURED set plus the two sentence-enders of the same class as
# `.`: `* _ ~` were GFM's and occur zero times here, and widening a masking rule
# beyond what the corpus writes is the DANGEROUS direction -- a character in
# this set hides the ids in the run before it, while a character missing from it
# merely REPORTS them.
#
# ⚠ AND `"` IS IN THE MEASUREMENT, which the first pass at this set dropped on
# the ground that it was "GFM's and occurs zero times here".  It occurs FOUR
# times, and not in prose: a `.md` name inside a raw-HTML attribute
# (`<div data-note="slice-9z-sib.md">`) ends at the closing quote, which is the
# seed's own population and has a control ("(seed) a raw HTML-block line whose
# only declared id sits INSIDE a `.md` file name seeds nothing").  Dropping it
# turned that control red -- the same mistake as omitting `;`, made in the same
# edit that was fixing it: a set narrowed against the very measurement cited
# for it.
_TRAILING = frozenset(".,;:!?)'\"")


def _run_end_from(text, e, n, cached):
    """The end of the file-name RUN containing `e`, reusing `cached` -- the end
    last computed -- when `e` has not yet passed it.

    ⚠ THE RE-WALK WAS QUADRATIC AND IT WAS MINE (PR #510 R36-2).  R34-1 gave
    the end test a scan to the run boundary, and `file_and_cite_spans` applies
    that test at EVERY suffix, so one run holding N suffixes (`a.md` repeated)
    re-walked the same boundary N times: measured 0.054 / 0.203 / 0.817 s over
    1,000 / 2,000 / 4,000, x3.8 then x4.0 per doubling.

    ⚠ AND THE FIRST FIX FOR IT WAS ALSO WRONG, caught by this suite's own cost
    control: precomputing every offset's run end is a second full pass over the
    text, which is linear but doubles the constant, and
    `linear_file_token_control` went red at its stated ceiling.  The boundary a
    suffix needs is the one for the run it is IN, suffixes inside a run are met
    in increasing order, so ONE cached boundary serves every suffix of that run
    and the walk is amortised O(1) with no extra pass."""
    if e < cached:
        return cached
    j = e
    while j < n and not (text[j].isspace() or text[j] in _NAME_BOUNDARY):
        j += 1
    return j


def _terminates_run(text, e, n, run_end):
    """The END OF THE FILE SPAN when the run ends acceptably at `e` -- the
    suffix is the last of the NAME and what follows it is not more name -- or
    None when the run continues into more name.

    TWO TAILS ARE ALLOWED, AND THE RESOLVER IS WHY (PR #510 R34-1).  It is the
    authority on "is this a name I would follow", and measured against it:

      * a FRAGMENT or QUERY tail (`#frag`, `?q=1`, `#`, `#a)b`) -- the resolver
        follows the WHOLE run, stripping the tail, so the lexer must not break
        the run into pieces and read an id out of the remainder.  That is the
        one direction the correspondence forbids outright
        (`file_token_resolver_agreement_control`), and requiring the suffix to
        END the run bluntly created it: two NEGATIVE controls went red because
        `slice-9z-sib.md#…` stopped masking and the declared id was reported;
      * a TRAILING-PUNCTUATION tail (`.` `,` `)` `:` `'` …) -- here the readers
        differ LEGITIMATELY and the difference is delimiting, not disagreement:
        the resolver is handed a destination the link grammar already bounded,
        while this reader has nothing but boundaries to find, so a period that
        ends a sentence is prose.  The resolver rejects `notes.md.`; the name
        inside it, `notes.md`, is one it follows.

    What is refused is the third case: a run that CONTINUES into more name
    (`9z+notes.md_tail`), which the resolver rejects whole and out of which the
    old test -- "not followed by an ASCII alphanumeric" -- still masked a
    prefix, hiding the ids in it."""
    if e < n and text[e] in "#?":
        # THE TAIL IS PART OF THE NAME, not merely permission to stop (PR #510
        # R35).  R34-1 admitted a fragment or query here and still recorded the
        # span ENDING AT THE SUFFIX, so `notes.md#9z owns it` masked `notes.md`
        # and left `#9z` standing -- the naming scan read the id out of it and
        # reported a site.  The resolver FOLLOWS that whole run, so this is the
        # very split the correspondence forbids, re-created by the fix that
        # cited the correspondence.  The span therefore covers the tail, less
        # any trailing punctuation, which is prose on this side of the reader.
        while run_end > e and text[run_end - 1] in _TRAILING:
            run_end -= 1
        return run_end
    return e if all(c in _TRAILING for c in text[e:run_end]) else None


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

      * the DISPOSITION (`plan_memo_stream.dispose`, stage 2) hands it the
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
    run_end = 0
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
        if text[e - k:e] == FILE_SUFFIX:
            run_end = _run_end_from(text, e, n, run_end)
            span_end = _terminates_run(text, e, n, run_end)
        else:
            span_end = None
        if span_end is not None:
            s = stack[-1] + 1 if stack else seg
            if s <= e - k:      # the suffix itself must lie inside the run
                longest[s] = span_end
    out, pos = [], 0
    for s in sorted(longest):
        e = longest[s]
        if s >= pos and e > pos:
            out.append((s, e, "file"))
            pos = e
    out.extend((m.start(), m.end(), "cite") for m in _CITE_TOKEN.finditer(text))
    out.sort()
    return out


def covers(spans, a, b):
    """Whether `[a, b)` meets one of `spans` -- the ONE reader of what
    `file_and_cite_spans` returns, given a name so its callers do not each
    re-spell the test.

    IT READS ONLY THE SPANS THAT COULD OVERLAP.  The list is ORDERED BY START
    AND NON-OVERLAPPING by construction (the file runs are selected leftmost
    to rightmost, each beginning at or after the previous one's end, and a
    citation cannot overlap a file name because `[` and `]` bound a run), so
    the first span that could reach `a` is a bisect away and there is at most
    one to test: any later span begins after this one ends.

    A caller that asks this of every token instead summed the whole list per
    token, which is quadratic in the line -- the always-run raw-line seed
    (`plan-memo-umbrella-check.py::lex_unsupported_seed`) over a raw HTML
    block of `9z note.md` repeated measured 2.9x then 3.2x per doubling (PR
    #510 R31-3).  It is the same defect `plan_memo_stream._straddles` had at
    R27-3 against `Stream.blanks`, whose ordering is stated for the same
    reason, and the same window closes it."""
    j = bisect.bisect_left(spans, (a,))
    if j and spans[j - 1][1] > a:       # the span before `a` may reach into it
        j -= 1
    return j < len(spans) and spans[j][0] < b and spans[j][1] > a


