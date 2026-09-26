#!/usr/bin/env python3
"""The DISPOSITION and the two readings for `plan-memo-umbrella-check.py`:
how a lexed block becomes the text the document renders, and what a scanner
is allowed to read out of it.

Everything here answers "what does this block SAY?".  What a row IS -- the
schemas, `Row` / `Table`, `admit_table`, the id cell, the row nouns -- is
`plan_memo_tables.py`'s, and it imports this module and never the reverse.
The lexical substrate below (Phase 1 blocks: `plan_memo_blocks.py`; Phase 2
inline: `plan_memo_lexer.py`) is the lexer's; the document driver above is
`plan_memo_memo.py`'s `Memo`.

THE SEAM IS THE ONE THE OLD MODULE'S OWN DOCSTRING NAMED (PR #510 R31, the
touch-time split): it said the file held "the table schemas, `Row` / `Table`
and the ONE admission site `admit_table`, the kind markers, and the mask
disposition every scanner reads through" -- two subjects joined by an "and",
at 1,006 lines.  They are separated here at the four references that crossed
between them, and the direction is one way: the kind PHRASES came down with
the disposition (a phrase is read out of a disposed stream and nowhere else,
which is what `kind_disagreements` is), the id-RUN tokeniser came down with
its one reader (`id_only`), and `Table.bind` reaches up for nothing -- it
asks `rendered` of this module.

Every block is lexed ONCE, where it is minted (a cell in `split_row`, a
paragraph in `plan_memo_memo.Paragraph`); `dispose` -- run by `Population`
once the row ids are known -- tags each block's mask with the kind of every
span (`code` / `html` / `autolink` / `link` / `image` / `cite` / `file` /
`mark`), minus the id-only code spans and the `**` pairs that decorate one;
and `stream(lexed)` -- the block AS THE DOCUMENT RENDERS IT: the spans that
render nothing dropped, the spans that render text the checker refuses to
read blanked in place, the §2.5 references substituted -- is the ONE text
each predicate over a block reads.
"""

import bisect
import re

from plan_memo_ids import (
    CITE_ID, DASH_CLASS, DECOR_CHARS, ROW_KINDS, SHORT_ID, SLUG_ID, bounded, tokens,
)
from plan_memo_tokens import file_and_cite_spans


# THE KIND PHRASES, and the ONE rule the three share: each is bounded on both
# sides by the grammar's ASCII alphanumeric class (`plan_memo_ids.bounded`),
# because a phrase matcher with no edges matches INSIDE a longer word.
# Measured at PR #510 R22, when none of the three had them: `SUBUMBRELLA, not
# a terminal unit` and `UMBRELLA, not a terminal unitary claim` both read as
# the marker (a bare `in` test); `KIND UNDETERMINEDNESS` and `MANKIND
# UNDETERMINED` both made a row kind-undetermined, which with a nonempty
# `Deps` cell is an `UMBRELLA-CELL` finding and exit 1; `is a pointer rather
# than a slicer` made a row a pointer.  Fixed together and from one spelling,
# because an enumerated fix leaves the next member of the class authoritative.
MARKER = "UMBRELLA, not a terminal unit"
"""The marker's PHRASE, for reporting it (`split_units` names it in a
finding) and for composing the matcher.  Every match goes through
`MARKER_RE`; a bare `MARKER in text` is the unbounded reading R22 removed."""

GAP = r"(?u:\s)"
r"""ONE character class for "a gap a READER sees between two words", composed
into every kind phrase below.

⚠ THE THREE PHRASES SPELLED IT THREE DIFFERENT WAYS, AND TWO OF THEM DROPPED A
CLAIM (PR #510 R47-4).  The marker and the pointer were `re.escape`d LITERALS,
so their gaps were U+0020 and nothing else; `UNDETERMINED` used `\s` under
`re.ASCII`, which is ASCII whitespace and not U+00A0.  Measured against cmark
0.31.2, which is what a reader's renderer does:
`**UMBRELLA, not a&nbsp;terminal unit.**` renders the marker VERBATIM to a
reader and exited **0** -- no census claim, the row left the census as an
active terminal.  So did `&#10;` (a newline), `&#9;` (a tab) and a literal
U+00A0 typed straight into the cell, while `&#32;` -- the one spelling that
decodes to U+0020 -- worked.  A phrase nobody can see the difference in is a
phrase the census must read the same way.
⚠ Stated as a PROPERTY, not a list: `(?u:\s)` is Unicode whitespace, which is
every one of the 28 codepoints below U+3000 whose `str.isspace()` is true --
enumerating the ones a reviewer happened to name would leave the next one
authoritative, which is the failure `KIND_PHRASES`' own comment records for
the word boundaries. The scope `(?u:...)` is deliberate: `UNDETERMINED` keeps
`re.ASCII` for its case folding (under `a` a long s never folds to `s`), and
only the GAP is Unicode.
"""


def _phrase(text):
    """A literal phrase whose word gaps are `GAP` -- the ONE composer, so a
    fourth phrase cannot arrive with a fourth spelling of a space."""
    return GAP.join(re.escape(w) for w in text.split(" ")).replace(GAP, GAP + "+")


MARKER_RE = re.compile(bounded(_phrase(MARKER)))
"""The ONE matcher for the marker, read over a block's disposed STREAM (a
declaring field is one).  It must still read a marker the document SPLITS
with a construct that renders nothing -- `**UMBRELLA, not a *terminal*
unit.**` is the marker then a `.` once the emphasis delimiters are dropped
(design re-gate 4) -- so the boundary is a lookaround AROUND the phrase and
never a change to the phrase."""

# §5's fifth row kind.  A kind-undetermined row carries the split and NOTHING
# else -- no ordering, no owner, no acceptance -- which is the same obligation
# the naming rule enforces against umbrellas.  Two spellings are in use; both
# are tolerated and the divergence is reported (a kind with two spellings is a
# kind no program can enumerate).
UNDETERMINED = re.compile(bounded("KIND" + GAP + "*" + DASH_CLASS + "?" + GAP + "*UNDETERMINED"),
                          re.IGNORECASE | re.ASCII)

# A row that is a POINTER into a slot rather than a slice of its own (§1.0's
# "SCHEDULED FROM ITS OWN SLOT" rows).  ⚠ Keyed on one spelling, and the safe
# polarity: a differently-spelled pointer row is terminal, and so REPORTED by
# the acceptance seed, never missed.
POINTER = re.compile(bounded(_phrase("is a pointer rather than a slice")))

KIND_PHRASES = (("marker", MARKER_RE), ("undetermined", UNDETERMINED), ("pointer", POINTER))
"""EVERY phrase whose presence or absence in a declaring field changes the
row's kind, as (name, matcher) -- the ONE enumeration, read by BOTH sides of
the kind question:

  * `Population._kind` takes its matches from here and never from a matcher
    of its own, so a phrase that decides a kind is necessarily a member (the
    self-test's `kind_phrase_gate_control` reads `_kind`'s own code object
    for a second matcher and fails on one);
  * `split_units` scans for each member, so the residue gate
    (`Population._kind_residue`) covers every member BY DEFAULT.

The construction, not the list, is the fix.  At PR #510 R22 each phrase grew
its own word boundary; at design re-gate 4 the residue gate was written for
the MARKER alone, and `KIND UNDETER`MINED`` -- which a reader reads as the
undetermined kind, since a code span contributes its content as plain text
(§6.1) -- silently reclassified the row as terminal at exit 0 (R23).  Both
are the same mistake: gating the phrase in front of you leaves every other
member of the class authoritative.  A phrase added below is gated by
arriving in this tuple, and cannot decide a kind without arriving here."""

# --------------------------------------------------------------------------
# Row identity, part one: the id RUN.  The id grammar itself -- the three
# kinds, the decoration, and the ONE boundary every reader consumes
# (`tokens`) -- is `plan_memo_ids.py`'s.  How a row is NAMED (`ROW_NOUN` and
# its composers) is part two, below `SCHEMAS`, because the nouns are derived
# from the schemas and cannot be spelled before them.
# --------------------------------------------------------------------------

# An id-only code span is tokenised by the declared-id GRAMMAR, longest
# alternative first (a `#11-` slug is atomic -- its internal hyphens are not
# separators), with the separators whitespace, list punctuation, `|` and `-`
# (a `Deps`-shaped edge, `9z | 7z` / `0a-0b`) between tokens.  ALL THREE
# kinds, not `ROW_ID`: a citation id is declared (the citation table keys
# its rows by it) and `` `[C1]` `` is the document spelling one.  A bare id
# in a cell or in prose is NOT tokenised on a list -- it is bounded by the
# grammar's continuation rule (`plan_memo_ids.tokens`); a hyphen bounds a
# short id, and `slice-9z-sib.md` is safe because a file name is a lexer
# `file` token, masked before the scan.
_ID_RUN_TOKEN = re.compile(r"(?P<id>%s|%s|%s)|(?P<sep>[\s,;/→>+&|-]+)" % (SLUG_ID, CITE_ID, SHORT_ID),
                           re.ASCII)
# --------------------------------------------------------------------------
# Disposition: the one place a lexical span meets the row ids
# --------------------------------------------------------------------------


def id_only(inner, keep):
    """The disposition exception: a code span whose content is only row ids
    (and separators) is the document SPELLING an id, and is a mention.  The
    run must be covered end to end by id tokens and separators."""
    if inner in keep:
        return True
    pos, ids = 0, []
    for m in _ID_RUN_TOKEN.finditer(inner):
        if m.start() != pos:
            return False
        pos = m.end()
        if m.group("id") is not None:
            ids.append(m.group("id"))
    return pos == len(inner) and bool(ids) and all(x in keep for x in ids)


def code_mask(lx, keep):
    """Code spans of `lx` minus the id-only ones, and minus the `#11-` slugs
    of `keep` INSIDE the remaining spans (a backticked slug, alone or in a
    command line, is the document spelling an id: the same exception; the
    slug is read by the grammar's tokeniser inside the span, so
    `#11-zz-alpha_extra` is not `#11-zz-alpha`) -- the spans a reader of
    prose must skip."""
    out = []
    for a, b, tag in lx.code:
        if id_only(lx.text[a:b].strip("`"), keep):    # whitespace is a separator token
            continue
        if tag == "demoted":
            # ⚠ §6.4, INSIDE A RESOLVED IMAGE DESCRIPTION (PR #510 R42-1): the
            # description renders as the plain string CONTENT of its inline
            # children, so a code span there contributes its content and
            # NEITHER delimiter -- the backtick runs are marks and the content
            # is ordinary text.  The id-only test above runs FIRST and
            # unchanged, which is what keeps the decoration exception alive in
            # alt text exactly as `dispose` already keeps the `**` one: the
            # emphasis twin `![**9z**7z …](i.png)` reports two ids, and so does
            # the code twin.  Emitted as `mark` rather than skipped, because a
            # standing backtick would JOIN the two sides of it.
            run = len(lx.text[a:b]) - len(lx.text[a:b].lstrip("`"))
            tail = len(lx.text[a:b]) - len(lx.text[a:b].rstrip("`"))
            # ⚠ §6.1's TRIM IS PART OF THE CONTENT, not of the delimiters (PR
            # #510 R42-6, a direct consequence of the demotion above: the span
            # only started reaching this branch when it stopped being masked).
            # "If the resulting string both begins and ends with a space
            # character, but does not consist entirely of space characters, a
            # single space character is removed from the front and back" -- and
            # §6.4 uses THAT string, so `` ![Slice 9` z ` owns it](i.png) ``
            # renders the alt text `Slice 9z owns it` and the declared `9z` is
            # ONE id. Reading the padded content verbatim split it into `9` and
            # `z`, and the run exited 0 with no residue to say so. The trimmed
            # spaces join the marks, which is the same mechanism saying the same
            # thing: what the reader does not see does not separate.
            # ⚠ THE TRIM IS TESTED AGAINST THE NORMALISED BODY, not the raw
            # one (PR #510 R42-7): §6.1 is TWO steps and the order is the rule
            # -- line endings become spaces FIRST, and only then does the trim
            # ask whether the result begins and ends with one.  Written against
            # the raw content it missed every span whose padding IS a line
            # ending (`` `\nz ` `` renders `z`, and the raw test saw `\n` and
            # declined), which is the same half-implementation `_inner` records
            # having had at R32, three lines below this one.  Step one itself is
            # a SUBSTITUTION the lexer emits; what is masked here is the one
            # rendered character at each end, which is two source characters
            # where that character is a CRLF.
            inner = lx.text[a + run:b - tail]
            norm = inner.replace("\r\n", " ").replace("\r", " ").replace("\n", " ")
            if len(norm) > 1 and norm[0] == " " and norm[-1] == " " and norm.strip(" "):
                run += 2 if inner.startswith("\r\n") else 1
                tail += 2 if inner.endswith("\r\n") else 1
            out.append((a, a + run, "mark"))
            out.append((b - tail, b, "mark"))
            continue
        cut = a
        for t in tokens(lx.text, a, b):
            if t.kind == "slug" and t.id in keep:
                out.append((cut, t.idstart, "code"))
                cut = t.idend
        out.append((cut, b, "code"))
    return out


# WHAT A DISPOSED SPAN CONTRIBUTES TO THE STREAM, per kind -- the column
# §3.0b's table carries, read off the spec's rendering of each construct and
# nothing else:
#   True  = the construct renders TEXT the checker refuses to read, so its
#           span stands as blanks: it can hide an id but never JOIN what sits
#           on either side of it, because a reader does not read across it;
#   False = the construct renders NOTHING at all -- a raw HTML tag or comment
#           (§6.6: markup, not text), a link's tail (§6.3: `](dest)` prints
#           nothing) or a `mark` (the backslash of a §6.7 hard line break, a
#           link's `[`, a matched §6.2 / GFM delimiter run; a §2.4 escape's
#           backslash is NOT one -- an escape SUBSTITUTES, below) -- so its
#           span contributes no character
#           and the text on either side of it is ONE run, exactly as the
#           rendered document reads it.
# An image renders no text either, but its own tail is BLANK: `![alt](i.png)`
# puts a picture in the flow, not the letters of `alt`, so the two sides of
# it are not one word (the description is scanned as prose all the same --
# the stated deviation, §2 B×D).
RENDERS_TEXT = {"code": True, "autolink": True, "image": True, "cite": True, "file": True,
                "html": False, "link": False, "mark": False}


def dispose(lx, keep):
    """Tag `lx.mask`: every span the scanners must not read an id out of, as
    (start, end, kind) -- `code` (minus id-only spans and kept slugs), `html`
    (a §6.6 raw HTML span, whole: an id inside an attribute or a comment is
    no naming site, exactly as on a raw HTML-block line -- the memo seeds
    it instead; PR #510 R17), `autolink` (a §6.5 autolink, whole: its text
    IS its destination, so an id inside it is no more a naming site than an
    id in a link's destination is -- and unlike a raw HTML span it is NOT
    seeded, because the construct is fully lexed and hides nothing; PR #510
    R21), `link` (the tail; the visible text stays, it
    is prose), `image` (the same, for an image -- its own tail and every
    construct demoted into its description), `cite`, `file`, and `mark`
    (every span that renders no character: the backslash of a §6.7 hard line
    break, a link's `[`, a matched §6.2 emphasis or GFM strikethrough
    delimiter run).  A
    reference definition is a Phase-1 block of its own, never inline
    content, so no block holds one to mask.

    THE DECORATION EXCEPTION (PR #510 design re-gate 4).  A `**`-pair whose
    content is only declared ids is not markup around prose: it is the
    document DECORATING an id, the way `` `9a` `` spells one, so its
    delimiters STAND in the stream and the id grammar reads them as the
    boundary they are (`plan_memo_ids`: "a decorated side is bounded by the
    decoration itself") -- `**9z**7z` is the id `9z` and then the id `7z`,
    not the token `9z7z`.  It is `id_only`, the SAME predicate the code-span
    disposition has always used for `` `9z` ``, asked of the emphasis span:
    one exception, spelled once, over both constructs.  Everywhere else the
    delimiters render nothing and are dropped, so `Slice 9**z**` names the
    row `9z` a reader sees, not the row `9` the asterisks used to bound
    (`*9z*` is not decoration -- `DECOR` is `**` and a backtick -- so a
    single-`*` pair around an id is dropped like any other emphasis).

    TWO STAGES, and the boundary is which TEXT the question is asked of (PR
    #510 R24).  Stage 1 is LEXICAL: the spans the inline parse alone decides
    (a code span, a raw HTML span, an autolink, a link's or an image's tail,
    a §6.7 hard break's backslash, a link's `[`, every §6.2 / GFM delimiter
    run) -- no
    rendered text is needed to place any of them.  Stage 2 asks the two
    questions that are about what the document RENDERS, and asks both of ONE
    text, `rd` (the stage-1 disposition with each blank filled in by what a
    reader sees there -- `stream(reader=True)`, the same rendering the residue
    detector compares against):

      * is a bare `.md` file name or a `[C19]` citation id standing here?
        A file name's boundaries are WHITESPACE boundaries and §2.5 can put
        whitespace where the source has none, so the source is the wrong text
        to ask: `9z&#32;notes.md owns it` renders `9z notes.md owns it`, where
        `9z` is a naming site, and a raw-text scan masked the whole run and
        lost the ownership claim (PR #510 R24 -- the reading introduced at R22
        was the last raw one left).
      * is a `**` pair's content only declared ids -- the decoration exception
        above?  `**&#57;z**7z` renders exactly what `**9z**7z` renders, and a
        raw-text `id_only` said no to the first and yes to the second, so the
        two documents disagreed about a site a reader cannot tell apart (found
        by enumerating the class, not reported).  The question is asked of the
        READER's text and not of the disposed stream, because ``**`x` 9z**7z``
        is the document bolding prose: a reader sees `x 9z`, while the disposed
        stream would show `9z` beside blanks, which whitespace-separates into
        an id-only run.

    The two answers are independent -- a `**` pair's content decides nothing
    about where a file name stands, and the reverse -- so ONE reading serves
    both and there is no third stage.  The file/cite spans are recorded in
    SOURCE coordinates (`Stream.at`), which is what a mask is in."""
    base = list(code_mask(lx, keep))    # already (a, b, kind): "code", or "mark" for a demoted span
    # ⚠ §6.6, INSIDE A RESOLVED IMAGE DESCRIPTION (PR #510, settled against
    # cmark 0.31.2 and commonmark.js 0.31.2 -- the two readings §8 carried as
    # undecidable on the vendored corpus, which holds 22 Images examples and
    # none with a `<` in a description).  A raw HTML span is NOT markup there:
    # §6.4 reduces the description to the plain string content of its inline
    # children, and an `html_inline` node's plain string content is its OWN
    # SOURCE TEXT.  Both references put it in the alt verbatim --
    # `![UMBRELLA, not a <span>terminal unit](img.png)` puts the span's own
    # characters in the alt: cmark 0.31.2 gives
    # `alt="UMBRELLA, not a &lt;span&gt;terminal unit"` and commonmark.js 0.31.2
    # gives `alt="UMBRELLA, not a <span>terminal unit"`.
    # ⚠ The ESCAPING is cmark's serializer, NOT the deciding fact -- the first
    # version of this comment said it was, and the second implementation does
    # the opposite.  What both agree on is that the characters are THERE.
    # So the span is dropped from the mask entirely and its characters stand as
    # ordinary text.  Masking it `html` (renders nothing, joins the two sides)
    # FABRICATED a finding: the marker phrase appeared across a `<span>` the
    # alt text spells out, and the row exited 1 on an ownership claim nobody
    # made.  ⚠ The opposite direction from every other §6.4 finding on this
    # surface -- a report invented, not a report missed -- which is why it was
    # never fixed on the prose alone.
    # ⚠ No line-ending substitution, and that is measured, not assumed: §6.1
    # normalises a CODE span's endings to spaces (`` ![a `b\nc` d](i.png) ``
    # -> `a b c d`) and §6.6 does not (`![a <span\nx>b](i.png)` keeps the
    # ending), so the `_line_endings_to_spaces` pass beside `code` has no twin
    # here.
    base += [(a, b, "html") for a, b, tag in lx.html if tag != "demoted"]
    # A DEMOTED autolink is the same rule as the demoted code span above:
    # §6.5 makes the URI the link's text, so inside a §6.4 description the URI
    # is what the alt text holds and the angle brackets render nothing.
    base += [(a, b, "autolink") if tag != "demoted" else (a, a + 1, "mark")
             for a, b, tag in lx.autolinks]
    base += [(b - 1, b, "mark") for a, b, tag in lx.autolinks if tag == "demoted"]
    base += [(a, b, "link") for a, b, _ in lx.links]
    # THE TAG DECIDES, and it is the tag the lexer already computes for the
    # conformance count: a resolved image's own delimiters (its `![` and its
    # tail) are BLANKS, because the construct renders a picture where they
    # stand and the text on either side of it is not one word; anything
    # DEMOTED into a resolved image's description -- a link, a nested image,
    # each one's opener and tail -- renders nothing at all under §6.4's plain
    # string content, so it is a `mark` and the description reads as the one
    # run a reader sees (PR #510 R30-3).
    base += [(a, b, "mark" if k == "demoted" else "image") for a, b, k in lx.images]
    base += [(a, b, "mark") for a, b in lx.marks]
    delims = [((oa, ob), (ca, cb), ch, use, ob, ca) for oa, ob, ca, cb, ch, use, _k in lx.emphasis]
    lx.mask = base + [(a, b, "mark") for pair in delims for a, b in pair[:2]]
    rd = stream(lx, reader=True)
    lx.mask = base + [(a, b, "mark") for op, cl, ch, use, ob, ca in delims
                      if not (ch == "*" and use == 2 and id_only(_reading(rd, ob, ca), keep))
                      for a, b in (op, cl)]
    lx.tokens = [(rd.at(a), rd.at(b), kind) for a, b, kind in file_and_cite_spans(rd)]
    lx.mask += lx.tokens


def _reading(rd, a, b):
    """The reader's text (`stream(reader=True)`) of the SOURCE range `[a, b)`.

    `Stream.src` maps a stream offset to the source offset it came from and is
    non-decreasing (a drop skips source offsets, a substitution repeats one),
    so the first stream offset whose source is at or past `a` is where that
    source range begins to render -- a bisect, the inverse of `Stream.at`.  A
    source range that renders nothing gives an empty reading, which is what it
    reads as."""
    return rd[bisect.bisect_left(rd.src, a):bisect.bisect_left(rd.src, b)]


def _inner(kind, text):
    """The text a READER sees where this checker blanks: a code span's content
    without its backtick strings, an autolink's URL without its angle
    brackets, and -- for the spans that are already their own text -- the span
    as written.

    §6.1 IS TWO STEPS AND THE ORDER IS THE RULE (PR #510 R32).  "First, line
    endings are converted to spaces"; THEN, if what results begins and ends
    with a space and is not all spaces, one space is removed from each end.
    This function did only the second step, and asked it of the RAW content --
    so a span whose content opens with a line ending had neither step fire, and
    the line ending stood in the reader's text where the spec puts a space.
    Falsified on every multi-line code span in the vendored corpus, not only
    where a trim interacts: Example 335 `` ``\nfoo\nbar  \nbaz\n`` `` renders
    `foo bar   baz`, 336 `` ``\nfoo \n`` `` renders `foo `, 337
    `` `foo   bar \nbaz` `` renders `foo   bar  baz`.  The reported shape was
    `Slice 9` followed by a span whose content is `\nz `, which renders
    `Slice 9z`: the run was silent -- no token AND no `[LEX-SPLIT?]` residue,
    so nothing sent a reader to the text.

    ⚠ The three §2.1 line endings are converted, not just `\n`: a memo written
    with CRLF or with bare CR is one document of many lines by §2.1, and
    `line_ending_control` holds the whole census invariant under all three."""
    if kind == "code":
        k = len(text) - len(text.lstrip("`"))
        body = text[k:len(text) - k]
        body = body.replace("\r\n", " ").replace("\r", " ").replace("\n", " ")
        if len(body) > 1 and body[0] == " " and body[-1] == " " and body.strip(" "):
            body = body[1:-1]
        return body
    if kind == "autolink":
        return text[1:-1]
    # ⚠ NO `image` CASE, AND THAT IS DELIBERATE (PR #510 R42-10).  Two
    # readings were tried here and BOTH broke a shipped control: the
    # description (wrong text -- the span handed here is the TAIL, the
    # description is not in it) and blanks (which stopped the phrase in
    # `KIND ![UNDETERMINED](img.png)` from straddling, so a reported near-miss
    # went silent).  This function serves the RESIDUE display, where a reader is
    # sent to the raw text and the markup is what they must see.  The header
    # comparison wants the opposite and is fixed where it lives, in `rendered`.
    # `memory/feedback_control-rewritten-to-bless-the-defect.md`: a control that
    # goes red under a fix is the fix's subject being wrong, not the control's.
    # ⚠ AN EXPLICIT CLOSED SET, NOT A SILENT FALLBACK (PR #510 R42-10).  This
    # was `return text` for every other kind, and the `image` case above was
    # missing -- so a schema header spelled `![#](i.png)` compared as its RAW
    # syntax, the table bound as non-schema, and a linked memo's umbrella row
    # with a nonempty `Deps` left the census at rc 0.  A fallback that answers
    # for kinds nobody enumerated is where the next kind hides; the kinds whose
    # reader text IS the span are named, and anything else is a defect this
    # function must not paper over.  ⚠ The closed set is `RENDERS_TEXT`'s KEYS,
    # not its TRUE half: a mutant that flips a kind to text-rendering (the RG4
    # rows, which re-inject a raw HTML span or a link tail as text) must reach
    # this function and get an answer, or it crashes instead of turning its
    # control red -- and a crash proves nothing about a clause.  For those
    # kinds the reader text IS the span, which is exactly what the mutant makes
    # observable.  A kind NOBODY declared is the one this refuses.
    if kind in RENDERS_TEXT:
        return text
    raise KeyError("no reader text defined for the disposed kind %r: a kind that reaches "
                   "the stream is declared in RENDERS_TEXT and read here" % (kind,))


class Stream(str):
    """A block's rendered text, carrying the map back to the source offsets
    the report and the id grammar are written in (`at`) and, in this stream's
    OWN coordinates, the spans of it the checker refuses to read as prose
    (`blanks`).  A `str`, so every predicate reads it as before; the map
    exists because a stream character no longer sits at its own source offset
    once a construct that renders nothing has been dropped.

    `blanks` is ORDERED BY START AND NON-OVERLAPPING, and by construction
    rather than by anyone's care: `stream()` appends each blank at the end of
    the buffer it has built so far and merges a run of them into the entry
    beside it, so a later blank can neither begin before an earlier one nor
    reach back into it.  `_straddles` binary-searches it, and that is the
    property the search reads."""

    def __new__(cls, text, src, blanks):
        o = str.__new__(cls, text)
        o.src, o.blanks = src, blanks
        return o

    def at(self, i):
        """The offset in the block's raw text that stream offset `i` came
        from (the end sentinel for `i` at or past the stream's end, so a
        match's `end` maps as its `start` does)."""
        return self.src[min(i, len(self.src) - 1)]


def stream(lx, reader=False):
    """`lx.text` AS THE DOCUMENT RENDERS IT, under the disposition above:
    every §2.5 character reference substituted by the character it stands for,
    every span that renders no character dropped, and every span that renders
    text the checker refuses to read blanked in place (code spans -- id-only
    spans and kept slugs were excepted in `dispose` -- autolinks, image tails,
    citation ids, file names).  This is the ONE text every predicate over a
    block reads: the id scanners (`plan_memo_ids.tokens` over this stream),
    the kind-marker reader (a quoted marker is not a declaration; a marker
    split by `*terminal*` or `&#44;` still IS one, since the reader reads
    one phrase), the seeds' vocabularies (a `gates` inside a code span is not
    ordering prose), the licensing rule's context.  `dispose` first -- the
    mask is None before it.

    Blanks keep their length and their line endings, so a masked construct
    stays a boundary and a report coordinate stays findable; a drop does not,
    which is why the result carries `Stream.at`.  Where a span of each kind
    overlaps, the DROP wins: that a construct renders nothing is a fact of
    the spec, while a blank is this checker's policy about text that IS
    rendered.  ⚠ The case that USED to settle it -- a link tail holding a
    `.md` file token, where blanking the token inside the dropped tail would
    leave the tail's two sides apart though the document reads them as one --
    CAN NO LONGER ARISE: since R24 the file and citation tokens are read off
    the RENDERING, where the tail is already gone, so no `file` span is
    emitted inside one.  The ordering therefore stands with no witness, and
    the mutant that proved it is retired (that row in
    `plan_memo_selftest_mutants_inline.py` carries the re-runnable
    measurement).  It is kept because `disp` is a max over 0 < 1 < 2 and the
    ordering is what makes that max total -- not because a case exercises it.
    If you find a shape where a blank span overlaps a drop span it belongs
    here as a control; the probe is `lx = Lexed(t); lx.resolve({});
    dispose(lx, keep)`, then intersect the positions of the spans whose
    `RENDERS_TEXT[kind]` is true with those of the rest.  Four shapes were
    tried when this was written -- a link with a `.md` destination, a numeric
    reference inside a destination, a code span in link text, an image with a
    `.md` destination -- and none overlapped.  Four is a sample, not a proof.

    `reader=True` is the SAME rendering with the blanks filled in by what a
    reader sees there (`_inner`): not a text any predicate reads -- I-A is
    the disposition, and a quoted marker declares nothing -- but the one the
    residue detector compares against, since the difference between the two
    IS everything this checker refuses to read (`split_units`)."""
    assert lx.mask is not None, "stream() before dispose(): the Population has not run yet"
    text = lx.text
    disp = bytearray(len(text))         # 0 = text, 1 = blank, 2 = drop
    for a, b, kind in lx.mask:
        v = 1 if RENDERS_TEXT[kind] else 2
        for k in range(a, b):
            if v > disp[k]:
                disp[k] = v
    # A §2.4 escape and a §2.5 reference are the two spellings of "this text
    # renders as that character", and both substitute -- EXCEPT where the
    # character would spell a DECORATION the document does not have: the
    # stream carries exactly one kind of markup, the id decoration a kept
    # span stands for (`dispose`), and `\*\*C\*\*` / `&#42;&#42;C&#42;&#42;`
    # are text, not the bold `**C**` no reader sees (measured: the umbrella
    # memo quotes a `grep` pattern in that shape).
    #
    # Those spans are BLANKED, which is the third disposition and the only
    # one that is true of them: the construct renders TEXT (one punctuation
    # character) that this checker refuses to read (reading it would spell
    # markup the document does not have), which is exactly what a blank
    # says.  Standing as WRITTEN was the fourth, and it is not a disposition
    # at all -- it leaves the entity's SOURCE letters where the id scanner
    # reads them, so a row `ast` was named by every `&ast;` in the document
    # and a row `42` by every `&#42;` (PR #510 R23; the mirror of the `Slice
    # 9**z**` fabrication design re-gate 4 closed, and the same rule closes
    # both: what stands in the stream is what the document RENDERS).
    # Dropping them is the other wrong answer: `9&ast;z` renders `9*z`, and
    # a drop would join the two sides into the id `9z` a reader does not
    # read.  A blank keeps the span's length and its coordinates, and no id
    # can straddle one of these: the character it stands for is a `*` or a
    # backtick, which is in no id token's character class, so the residue
    # (`_units`) cannot fire on it -- proved by the class, not by luck.
    subst, decor = {}, {}
    for a, b, ch in lx.subst:
        (decor if ch in DECOR_CHARS else subst)[a] = (b, ch)
    for a, (b, _ch) in decor.items():
        if disp[a] == 0:                # inside a dropped or blanked span, that span wins
            for k in range(a, b):
                disp[k] = 1
    # start -> (end, the text a READER sees there): the outermost blank span
    # at each start (an inner span of a blanked one is inside its text, not
    # beside it), read by the reader's rendering alone
    blank_at = {}
    if reader:
        for a, b, kind in lx.mask:
            if RENDERS_TEXT[kind] and disp[a] == 1 and (a not in blank_at or blank_at[a][0] < b):
                blank_at[a] = (b, _inner(kind, text[a:b]))
        for a, (b, ch) in decor.items():
            # what a reader sees where a decoration spelling is blanked is
            # the one character it renders; a span of `lx.mask` that starts
            # here instead (a `.md` token can) is the outer construct and
            # keeps it
            if a not in blank_at and disp[a] == 1:
                blank_at[a] = (b, ch)
    buf, src, blanks, i, n = [], [], [], 0, len(text)
    while i < n:
        if disp[i] == 0 and i in subst:
            end, ch = subst[i]
            buf.append(ch)
            src.extend([i] * len(ch))
            i = end
            continue
        if disp[i] == 0:
            buf.append(text[i])
            src.append(i)
        elif disp[i] == 1:
            if reader and i in blank_at:
                end, body = blank_at[i]
                blanks.append((len(buf), len(buf) + len(body)))
                buf.extend(body)
                src.extend([i] * len(body))
                i = end
                continue
            if not reader and (not blanks or blanks[-1][1] != len(buf)):
                blanks.append((len(buf), len(buf) + 1))
            elif not reader:
                blanks[-1] = (blanks[-1][0], len(buf) + 1)
            buf.append("\n" if text[i] == "\n" else " ")
            src.append(i)
        i += 1
    src.append(n)
    return Stream("".join(buf), src, blanks)


def _straddles(blanks, a, b):
    """Whether `[a, b)` holds a character inside one of the `blanks` AND one
    outside every one of them -- the unit is read ACROSS a span, as opposed
    to sitting wholly inside one (a quoted marker: I-A's disposition, on
    purpose) or wholly outside every one (the ordinary reading).

    ONLY THE BLANKS THAT OVERLAP `[a, b)` ARE READ.  Summing over the whole
    list made the always-run seed (`lex_split_seed` -> `split_units`)
    quadratic in the block: every candidate token and every kind phrase asked
    this question, and a paragraph of N code spans holds N blanks, so a block
    of `9z `x` ` repeated cost N^2 even though no unit straddled anything (PR
    #510 R27-3; measured 3.6x then 3.9x per doubling).  `Stream.blanks` is
    ordered and non-overlapping by construction, so the overlapping window is
    a bisect away and the answer is the same one.

    The early exit is not an optimisation but the second half of the
    question: once the covered length reaches the unit's own, every remaining
    character is inside a blank and no later one can add to it."""
    j = bisect.bisect_left(blanks, (a,))
    if j and blanks[j - 1][1] > a:      # the blank before `a` may reach into it
        j -= 1
    inside = 0
    while j < len(blanks) and blanks[j][0] < b:
        x, y = blanks[j]
        inside += min(b, y) - max(a, x)
        if inside == b - a:
            return False                # wholly inside the blanks: not across them
        j += 1
    return inside > 0


def rendered(lx):
    """The plain text a READER sees in this block, with NO keep-set exception
    -- the reading a question about the DOCUMENT is asked of, as opposed to a
    question about what a scanner may read out of the stream.

    ITS ONE CALLER IS `Table.bind` (PR #510 R31-1).  A table's schema was
    matched on the header cells' RAW text, so a header spelled with any
    equivalent inline syntax -- `&#35;` or `\\#` or `` `#` `` for `#` -- was not
    the schema's header, the table was admitted as a non-schema one, and every
    declaration and every assertion in it left the run silently: not one row
    but a whole memo's rows, at rc 0.  The reading is the same one
    `Population._unkeyed` already asks of an id cell (`stream(reader=True)`),
    which is why there is no third spelling of "what does this cell say".

    THE EXCEPTIONS ARE DROPPED ON PURPOSE, and the reason they cannot matter
    here is the reason they exist: `dispose`'s two keep-set exceptions (an
    id-only code span, an id-only `**` pair) fire only where a span's content
    is entirely DECLARED IDS, and a schema's header cells are fixed literals
    of `SCHEMAS` -- `#`, `Slice`, `Deps` -- none of which is an id.  Asking
    with an empty keep is therefore the same answer, and it is the only one
    available: the keep-set is the set of declared ids, which is read from the
    rows of the tables this decides the schema of.

    ⚠ The row's own ID CELL is still read RAW (`bare_id`), and that is the
    same circularity from the other side: `**&#57;z**7z` renders `9z7z`, and
    which of `9z7z` / `9z` a reader sees THERE is decided by the decoration
    exception, which is keyed on the declared ids being computed.  The raw
    cell is the reading with every decoration standing, which is what that
    grammar is written against; `_unkeyed` then asks the rendered question of
    whatever declared nothing, so no cell escapes both."""
    dispose(lx, frozenset())
    return str(stream(lx, reader=True))


def _readings(lx):
    """THE TWO READINGS of one block, in the order every reader of them takes
    them: what a READER sees (`reader=True`, the blanks filled in with the
    text the constructs render), and the disposed STREAM this checker's
    predicates read.  The residue is exactly their disagreement, so both
    consumers -- the seed (`split_units`) and the census gate
    (`kind_disagreements`) -- take the pair from here."""
    return stream(lx, reader=True), stream(lx)


def _units(st, keep):
    """The units of ONE rendering `st` that are read ACROSS one of its blanks
    -- an id token of `keep`, or a member of `KIND_PHRASES` -- as (kind,
    text, offset in the block's RAW text)."""
    if not st.blanks:
        return []
    # `ROW_KINDS`, the grammar's closed set, and NOT its complement -- the same
    # spelling the bare and anchored naming passes read (PR #510 R24; this said
    # `t.kind != "cite"`, which agrees exactly today and diverges the moment a
    # kind is added to `KINDS` without being a row kind).
    out = [("id", t.id, st.at(t.idstart)) for t in tokens(st)
           if t.id in keep and t.kind in ROW_KINDS and _straddles(st.blanks, t.idstart, t.idend)]
    for name, rx in KIND_PHRASES:
        out += [(name, m.group(0), st.at(m.start())) for m in rx.finditer(st)
                if _straddles(st.blanks, m.start(), m.end())]
    return out


def split_units(lx, keep):
    """THE RESIDUE, reported rather than decided (the plan's §3.0b): every
    lexical unit read across a span this checker refuses to read as prose, as
    (kind, text, offset in the block's RAW text) -- an id token of `keep`, or
    a `KIND_PHRASES` member -- in EITHER of the two readings.

    Both directions, because the disagreement is symmetric and a rule stated
    over one of them leaves the other authoritative (PR #510 R23).  The
    reader reads a unit the disposed stream does not: `KIND UNDETER`MINED``
    is the undetermined kind to a reader (§6.1: the code span contributes
    `MINED` as plain text) and nothing to the stream.  And the stream reads
    one the READER does not: a blank stands as spaces, so `KIND `x`
    UNDETERMINED` is the undetermined kind to `UNDETERMINED`'s `\\s*` and
    `KIND x UNDETERMINED` -- no kind at all -- to a reader.  An id cannot
    make that second shape (a blank's filler is a space, which bounds every
    id token), so it is the phrases that need the second scan; scanning both
    renderings for both is one rule rather than that carve-out.

    The mechanism that fixes the rest of this class -- the stream IS the
    rendered text -- cannot reach here by construction: inside a code span,
    an autolink, a citation id or a file name the checker does not read the
    rendered text, deliberately (I-A: a quoted marker declares nothing, a
    `9z` in a shell command names no row), so where a unit STRADDLES such a
    span the two readings disagree and neither is the checker's to pick.
    §1 forbids a clean exit for a could-not-scan, so the disagreement is
    printed; and where it decides a gating census -- a kind phrase in a row's
    declaring field -- `Population._kind_residue` raises it as a schema miss
    instead (`plan_memo_population.py`).

    The comparison is exact and needs no threshold: both readings come from
    the ONE builder, and a unit is in the residue exactly when its extent
    covers characters on both sides of a blank's edge.  A unit that straddles
    in BOTH readings is one disagreement and is reported once: the two
    renderings map it back to the same raw offset, so the record is the
    same, and the reader's spelling of it is the one reported."""
    seen = {}
    for st in _readings(lx):
        for kind, text, off in _units(st, keep):
            seen.setdefault((kind, off), (kind, text, off))
    return list(seen.values())


def kind_disagreements(lx):
    """THE CENSUS QUESTION the residue answers: the `KIND_PHRASES` the two
    readings of `lx` disagree about, BECAUSE one of them reads the phrase
    across a blank -- the gating half of `split_units`, asked per phrase
    (`Population._kind_residue`).

    Both conjuncts are load-bearing, and each is a measured case:

      * the readings must DISAGREE about the phrase.  A field that spells a
        phrase cleanly somewhere reads the same kind under both renderings
        even if it straddles a blank elsewhere, and the census is not in
        doubt;
      * and the disagreement must come from a STRADDLE.  A phrase quoted
        WHOLE (`` `UMBRELLA, not a terminal unit` ``) also makes the two
        readings differ -- and is I-A's deliberate disposition, not a
        could-not-scan: a quoted phrase declares nothing, and that is a
        decision, not a doubt."""
    rd, st = _readings(lx)
    out = []
    for name, rx in KIND_PHRASES:
        hit = [(rd.blanks, m) for m in rx.finditer(rd)]
        other = list(rx.finditer(st))
        # ⚠ PRESENCE, and a COUNT comparison was tried here and REVERTED
        # (PR #510 R48-1).  A field carrying a CLEAN phrase beside a straddling
        # one reads as agreement, and a review round showed a case where that
        # hides a real doubt: `Slice 7z -- **UMBRELLA, not a `terminal` unit.**
        # Then **UMBRELLA, not a terminal unit.**` is rc 2 with the straddling
        # marker ALONE and rc 1 once the clean one is added, because the reader
        # attributes the FIRST marker to `7z` (a pointer) while the stream
        # misses it and reads the later one as self-declaring (an umbrella).
        # ⚠ Counting does not separate that from the case the R23 control
        # RATIFIES -- `KIND UNDETERMINED.  Also KIND UNDETER`MINED`.` also has
        # 2 vs 1, and there the kind is undetermined under both readings, so
        # the census genuinely is not in doubt.  The property that tells them
        # apart is whether the two readings declare the same KIND AND the same
        # ATTRIBUTION, which this function cannot ask: it is per-phrase and has
        # no row, while `_kind`'s ordering and `attributed_to_other` are the
        # Population's.  Carved in §8 rather than approximated here.
        if bool(hit) == bool(other):
            continue
        hit = hit or [(st.blanks, m) for m in other]
        if any(_straddles(blanks, m.start(), m.end()) for blanks, m in hit):
            out.append(name)
    return out

