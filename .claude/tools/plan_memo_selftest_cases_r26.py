#!/usr/bin/env python3
"""PR #510 R26's fixture controls -- the fourth module of the one `CASES` list.

The mutant registry is split at THIS module's seam (`_cases_inline.py` says so
of its own), and `plan_memo_selftest_mutants_r26.py` was carved one commit
earlier when `_mutants_inline.py` reached 987 lines.  This module is the other
half of that pair, created with its first case rather than empty: a mutants
module with no cases module beside it would leave the stated correspondence
false, which is the kind of claim this whole checker exists to stop.

R26's subject is the checker's OPERATING ENVELOPE (what it assumes about its
host, what it costs) and the places where one reading of a text disagreed with
another; the two findings with fixture-shaped controls are here, while the
work-shaped ones are `plan_memo_selftest_work.py`'s and the swept ones are
`plan_memo_selftest_properties.py`'s (the checker as written) and
`plan_memo_selftest_invariants.py`'s (the checker run), by those modules' own
seams.

This is the TAIL module of the four, so R26 ON lands here -- exactly as its
mutant counterpart `plan_memo_selftest_mutants_r26.py` already says of itself
("R26 on").  R27's fixture-shaped control is below.
"""

from plan_memo_selftest_cases import CASES, acase, build, case, rcase

# ------------------------------------------------ PR #510 Codex R26 controls --
# R26-2: the bare file-name token and `plan_memo_sibling.sibling_path`
# disagreed about nested parentheses.  The token arm admitted a FLAT
# parenthesised chunk only, so over `foo((9z)).md` it could match no more than
# the suffix `.md` and left the declared id `9z` to the naming scan, while the
# resolver -- documented in the same comment as consuming `FILE_SUFFIX` for the
# SAME test -- accepts nested-parenthesis `.md` paths.  The reader that moved is
# the LEXER's, on to §6.3's own "balanced pair of unescaped parentheses", which
# holds at any depth.

case("NEGATIVE", "(R26 file) `foo((9z)).md` is ONE file name: nested balanced parentheses are §6.3's "
                 "rule at any depth, so the id inside the name is no naming site.  Until R26 the token "
                 "arm matched a FLAT chunk only, could reach no further left than the bare `.md`, and "
                 "reported `9z`",
     build(), "Read foo((9z)).md for the walk.", 0)
case("NEGATIVE", "(R26 file) `(m.md(9z)md).md`: the run is balanced ACROSS its groups, so it is one "
                 "name and not two tokens with the id exposed between them -- the second shape the flat "
                 "arm could not read, and the one that leaks an id rather than a parenthesis",
     build(), "Read (m.md(9z)md).md for the walk.", 0)
case("NEGATIVE", "(R26 file) `foo(9z).md`, the FLAT partner, green before the fix: a single balanced "
                 "pair always read as one name, which is what says the two controls above are about "
                 "NESTING and not about parentheses as such",
     build(), "Read foo(9z).md for the walk.", 0)
case("POSITIVE", "(R26 file) `foo((9z)) .md` -- the same characters with a space before the suffix -- "
                 "still REPORTS `9z`: a run ends at whitespace, so this is not a file name at all.  The "
                 "discriminating half: a fix that widened the token until every parenthesis was "
                 "swallowed would pass the three above and fail this one",
     build(), "Read foo((9z)) .md for the walk.", 1)
case("POSITIVE", "(R26 file) `9z(x.md)` reports `9z`: the run ending at the suffix may not hold the "
                 "parenthesis still OPEN there, so it starts one past it and the name is `x.md` -- the "
                 "half that says the start is the innermost open parenthesis and not the segment",
     build(), "Read 9z(x.md) for the walk.", 1)
case("NEGATIVE", "(R26 file) `a.md+9z+b.md` is ONE name ending at the LAST suffix, not the first: "
                 "leftmost-LONGEST, as a pattern would have read it.  Stopping at the first `.md` would "
                 "leave `9z` standing outside the token.  ⚠ The first fixture written for this clause "
                 "was `a.md.9z.md`, and it was NOT discriminating -- an id directly before `.md` is a "
                 "DOTTED NUMBER to the id grammar and no site either way, so both readings reported 0 "
                 "and the mutant survived.  The id has to be separated from the second suffix",
     build(), "Read a.md+9z+b.md for the walk.", 0)
case("POSITIVE", "(R26 file) `9z)foo.md` still reports `9z`: a `)` that closes nothing may be held by "
                 "no run and crossed by none, so the name is `foo.md` and the id before it is outside "
                 "-- the second discriminating half, against a scan that balanced parentheses by "
                 "IGNORING the ones it could not match",
     build(), "Read 9z)foo.md for the walk.", 1)


# R26-3 / R26-1: `link_destination` bounds its parenthesis nesting at
# `DESTINATION_NESTING_LIMIT`.  §6.3 REQUIRES no such limit and permits one
# ("Implementations may impose limits on parentheses nesting to avoid
# performance issues, but at least three levels of nesting should be
# supported"); commonmark.js 0.31.2 imposes none, cmark 0.31.1 stops at 32, and
# the deepest destination in all 630 vendored examples is Example 496's depth 2.
# So the limit is taken for the reason the spec's parenthetical gives -- the
# SCAN -- and these two controls fix where the boundary now falls, since no
# conformance example can.

def _deep(depth):
    return "See [x](a" + "(" * depth + "z" + ")" * depth + ".md) for the walk."

rcase("POSITIVE", "(R26 §6.3) a destination nested 32 deep IS a link, so the memo it names is a "
                  "sibling this run could not read: rc 2.  Green before the limit as after it -- the "
                  "half that says the limit is a LIMIT and not a refusal of nested destinations",
      build(), _deep(32), 2)
rcase("NEGATIVE", "(R26 §6.3) a destination nested 33 deep is NOT a link, so `[x](...)` is literal "
                  "text and no memo is looked up: rc 0.  ⚠ This is a DIVERGENCE from commonmark.js, "
                  "which reads it as a link, and an agreement with cmark; the spec permits both and "
                  "the vendored corpus reaches depth 2, so nothing but this control says where the "
                  "boundary is",
      build(), _deep(33), 0)


# ------------------------------------------------ PR #510 Codex R27 controls --
# R27-2: `ROW_NOUN` was a hand-written `slices?|rows?|umbrellas?` and omitted
# `Slot` -- the name of a schema in `SCHEMAS` since before the checker was
# reviewed.  So a slot row that attributed a marker with its own schema's noun
# declared, to the checker, ITSELF: kind umbrella, no `UMBRELLA-MARK`, a
# pointer row inside the census, exit 0.  These two are the END-TO-END half
# (the reviewer's shape, and its mention-only partner); the derivation itself
# is swept over `SCHEMAS` by `plan_memo_selftest_invariants`'
# `row_noun_schema_control`, which is what makes the NEXT schema's noun a
# covered case rather than the next round's finding.

SLOT_PTR = "Slot %s — **UMBRELLA, not a terminal unit** — points into §8."
acase("POSITIVE", "(R27 noun) ``Slot `#11-zz-alpha` — **UMBRELLA, …**`` attributes the marker to the "
                  "named row: the containing row is a POINTER and the attribution finding is emitted. "
                  "The schema noun the §8 table's own id column is headed with, and the one spelling "
                  "the hand-written alternation left out",
      build(wb=SLOT_PTR % "`#11-zz-alpha`"), "UMBRELLA-MARK", 1)
acase("NEGATIVE", "(R27 noun) `Unlike Slot `#11-zz-alpha`, **UMBRELLA, …**` does NOT attribute -- the "
                  "new noun inherits the DASH discrimination rather than widening what a mention is. "
                  "The discriminating partner: a fix that anchored on the noun alone would pass the "
                  "row above and fail this one",
      build(wb="Unlike Slot `#11-zz-alpha`, **UMBRELLA, not a terminal unit.**"), "UMBRELLA-MARK", 0)


# ------------------------------------------------ PR #510 Codex R30 controls --
# R30: `is_blank_id_cell` stripped `*` and backticks off the RAW id cell
# unconditionally, so every decoration run the inline grammar pairs NOTHING
# with -- `**`, `*`, `` ` ``, ``` `` ```, `***` -- became the empty-string
# blank and its row left the census as a deliberate non-row, at rc 0, with its
# declaring field and its `Deps` edge unasserted.  The predicate now reads WHAT
# A READER SEES in the cell (`stream(reader=True)`), which is the one text that
# knows which decoration pairs; the miss moved after the disposition, where
# that reading exists, because it gates a finding and no declaration.
#
# The fixture is the reported shape end to end: the umbrella marker in the
# declaring field and a nonempty `Deps` edge, so a row that leaves the census
# takes real ownership data with it.  Each control measures the SCHEMA miss
# (the factory requires rc 2 exactly when it counts one), never rc alone.
#
# The names the R30 mutants must turn red are collected as the controls are
# registered (`CASES[-1].name`, the R25 idiom), so each is spelled ONCE.
_MISS = ("schema", "id cell does not start with an id")

R30_UNPAIRED = []
"""The cells whose decoration pairs NOTHING and which the strip blanked: the
reported class, red under the mutant that re-injects that strip."""


def _idcell(cell):
    return build(i7z=cell, s7z="**UMBRELLA, not a terminal unit.**", d7z="**9z**")


case("POSITIVE", "(R30 id) an id cell `**` is an unmatched strong-emphasis run: §6.2 pairs nothing "
                 "with it and a reader sees `**`, so the row is UNKEYED -- a schema miss, rc 2.  The "
                 "reported shape: before R30 the unconditional strip made it the empty-string blank "
                 "and this row -- marker in its declaring field, a `Deps` edge -- exited 0",
     _idcell("**"), "", 1, measure=_MISS)
R30_UNPAIRED.append(CASES[-1].name)
case("POSITIVE", "(R30 id) an id cell `*` is a single unmatched delimiter: the same run one character "
                 "short, and not a `DECOR_MARK` at all -- unkeyed, rc 2",
     _idcell("*"), "", 1, measure=_MISS)
R30_UNPAIRED.append(CASES[-1].name)
case("POSITIVE", "(R30 id) an id cell `` ` `` is one backtick that opens no code span (§6.1 wants a "
                 "closing backtick string): a reader sees the backtick, so the row is unkeyed, rc 2",
     _idcell("`"), "", 1, measure=_MISS)
R30_UNPAIRED.append(CASES[-1].name)
case("POSITIVE", "(R30 id) an id cell ``` `` ``` is ONE backtick string of length 2 that closes "
                 "nothing (§6.1: the closer must be of EQUAL length), so it renders literally.  The "
                 "case that says the pairing authority is the inline LEXER and not the id grammar's "
                 "decoration walk: that walk reads two `DECOR_MARKS`, so a hand-rolled peel of matched "
                 "pairs -- the other shape offered for this fix -- strips it to nothing and calls the "
                 "row a blank",
     _idcell("``"), "", 1, measure=_MISS)
R30_UNPAIRED.append(CASES[-1].name)
case("POSITIVE", "(R30 id) an id cell `***` renders literally too: the strip took the whole run "
                 "whatever its length, so a longer run was as blank as a shorter one -- unkeyed, rc 2",
     _idcell("***"), "", 1, measure=_MISS)
R30_UNPAIRED.append(CASES[-1].name)
case("POSITIVE", "(R30 id) an id cell `` `?` `` is unkeyed: a code span RENDERS its content (§6.1) and "
                 "a reader sees `?`.  The discriminating case for WHICH rendering: the disposed stream "
                 "-- what every other predicate over a block reads -- blanks a code span, so under it "
                 "this cell is empty and the row is a blank, which is the silent-skip class re-opened",
     _idcell("`?`"), "", 1, measure=_MISS)
R30_CODE_SPAN_READING = CASES[-1].name
"""The one control that separates the READER's rendering from the disposed
stream: red under the mutant that asks the blank question of the stream."""

R30_PAIRED = []
"""The cells a reader sees a blank in, through a construct that RENDERS: red
under the mutant that goes back to reading the raw cell."""

case("NEGATIVE", "(R30 id) an id cell `**—**` is STILL a deliberate blank: here the `**` pair closes "
                 "(§6.2), and paired decoration around a blank is what the strip was right about.  The "
                 "partner that bounds the fix's reach: refusing decoration outright would pass all six "
                 "above and fail this one",
     _idcell("**—**"), "", 0, measure=_MISS)
R30_PAIRED.append(CASES[-1].name)
case("NEGATIVE", "(R30 id) an id cell `` `—` `` is a deliberate blank: the code span renders its "
                 "content and a reader sees the em dash -- the same partner for the other `DECOR_MARK`",
     _idcell("`—`"), "", 0, measure=_MISS)
R30_PAIRED.append(CASES[-1].name)
case("POSITIVE-NOVEL", "(R30 id) an id cell `&#8212;` is a deliberate blank: §2.5 renders the em dash, "
                       "which the RAW reading could not see -- this cell was a schema miss before R30 "
                       "and is a non-row after it, the one verdict the fix reverses in the other "
                       "direction",
     _idcell("&#8212;"), "", 0, measure=_MISS)
R30_PAIRED.append(CASES[-1].name)
acase("POSITIVE", "(R30 id) the keyed partner, end to end: `**7z**` keys the row, so the marker and "
                  "the `Deps` edge ARE asserted -- UMBRELLA-CELL, rc 1.  This is the run the six "
                  "unkeyed cells above silently left, and it must not be reached by asking the blank "
                  "question of a row that declared an id",
      _idcell("**7z**"), "UMBRELLA-CELL", 1)
R30_KEYED = CASES[-1].name
"""The keyed partner: red under the mutant that asks the blank question of
EVERY row rather than of the rows that declared nothing."""


# ---------------------------------------------- PR #510 Codex R30-3 controls --
# R30's THIRD finding, and it arrived with R31's four because the round's
# thread fetch stopped at 100 of 105 -- the disposition that called R30 two
# findings is corrected in the same round these controls land.
#
# §6.4 reduces an image's description to its "plain string content" when
# rendering, so a bracket construct inside a RESOLVED image's description
# contributes its TEXT and none of its markup.
# ⚠ THE EARLIER WORDING HERE QUOTED A SENTENCE THE SPEC DOES NOT CONTAIN ("the
# plain string content of its INLINE CHILDREN") -- "inline children" is an AST
# term, and §6.4 states this as a rendering recommendation rather than as a
# definition.  No CommonMark PROSE is vendored anywhere in this tree, so
# neither wording is checkable here; the fragments the lexer quotes are the one
# spelling, and this reads as a paraphrase rather than as a quotation.  What IS
# checkable is the behaviour, and that is what the controls below do.  Three separate pieces of markup
# stand in such a description and each one was disposed of wrongly:
#
#   (1) a demoted LINK's `[`, which was POPPED off `opens` and so left standing
#       in the stream -- `![KIND [UNDETERMINED](x)](img.png)` renders the alt
#       text `KIND UNDETERMINED` (vendored Example 575: `![foo [bar](/url)]
#       (/url2)` -> `alt="foo bar"`) and read as `KIND [UNDETERMINED`;
#   (2) a demoted construct's TAIL, disposed as a BLANK rather than as a mark,
#       so the two sides of it were two runs where a reader sees one word;
#   (3) the image's OWN `![`, never recorded at all, so it stood in the stream
#       as literal text.
#
# Each clause has its own probe below, because each is the only one its mutant
# moves (measured: the other two survive the other two probes).  The declaring
# field is the subject in all three and the `Deps` cell is nonempty, so the
# reported shape is the reviewer's: a row that reads terminal takes real
# ownership data out of the census at rc 0.

def _kindcell(cell):
    """The Uz row declaring an unsettled kind in `cell`, with an edge to the
    terminal row -- so the run says UMBRELLA-CELL at rc 1 when the phrase is
    read and says nothing at all when it is not."""
    return build(suz=cell, duz="**7z**")


acase("POSITIVE", "(R30-3 §6.4) a kind phrase crossing a LINK inside a resolved image description is "
                  "read: `![KIND [UNDETERMINED](x)](img.png)` renders the alt text `KIND UNDETERMINED` "
                  "(Example 575), so the row is kind-undetermined and its `Deps` edge is asserted.  The "
                  "reviewer's shape: the demoted link's `[` was popped off the opener list, stood in "
                  "the stream, and the row left the census as terminal at rc 0",
      _kindcell("![KIND [UNDETERMINED](x)](img.png)"), "UMBRELLA-CELL", 1)
R30_3_LINK_OPENER = CASES[-1].name
"""Red under the mutant that pops the demoted link's `[` again."""

acase("POSITIVE", "(R30-3 §6.4) the same phrase crossing a demoted link's TAIL: `![KIND [](x)"
                  "UNDETERMINED](img.png)` renders `KIND UNDETERMINED` too, because an empty label "
                  "contributes an empty string and the tail contributes nothing.  This is the clause "
                  "the opener case cannot see -- there the whole phrase stands before the tail -- and "
                  "a tail disposed as a BLANK puts a run boundary through the middle of the phrase",
      _kindcell("![KIND [](x)UNDETERMINED](img.png)"), "UMBRELLA-CELL", 1)
R30_3_DEMOTED_TAIL = CASES[-1].name
"""Red under the mutant that disposes a demoted construct as a blank."""

acase("POSITIVE", "(R30-3 §6.4) and the phrase crossing a nested IMAGE's markup: `![KIND ![](i.png)"
                  "UNDETERMINED](img.png)` renders `KIND UNDETERMINED` (Example 574: a nested image's "
                  "alt text is folded into the enclosing one's).  The inner `![` is the third piece of "
                  "markup, and it was recorded nowhere at all -- neither clause above reaches it, since "
                  "a link's `[` is recorded and a tail is recorded",
      _kindcell("![KIND ![](i.png)UNDETERMINED](img.png)"), "UMBRELLA-CELL", 1)
R30_3_IMAGE_OPENER = CASES[-1].name
"""Red under the mutant that stops recording a resolved image's own `![`."""

case("POSITIVE", "(R30-3 §6.4) a resolved image's own `![` is a BLANK where it is NOT demoted, and the "
                 "difference is loud: in `KIND ![UNDETERMINED](img.png)` the phrase STRADDLES the "
                 "image, and the text around an image and the text of its description are two runs (a "
                 "reader of the first sees a picture where the second stands), so the row is not "
                 "kind-undetermined -- but the near-miss is reported as a schema miss at rc 2 rather "
                 "than passed over.  The discriminating half of the clause above: recording the opener "
                 "as a blank is what makes the phrase visibly CROSS something, and without the record "
                 "the `![` is literal text, nothing is straddled, and the run exits 0 saying nothing",
     _kindcell("KIND ![UNDETERMINED](img.png)"), "", 1,
     measure=("schema", "ACROSS a span this checker does not read as prose"))
R30_3_LOUD_MISS = CASES[-1].name
"""The other direction of the image-opener clause: red under the same mutant,
and it is the one that says the blank is not merely cosmetic."""

acase("POSITIVE", "(R30-3 §6.4) the phrase with the inner markup simply REMOVED -- `![KIND "
                  "UNDETERMINED](img.png)` -- is the three positives' CONSTANT: a description whose "
                  "plain string content is the phrase outright, read as prose exactly as §6.4 reduces "
                  "it.  Green before R30-3 and after, which is what makes the three above claims about "
                  "the MARKUP a description encloses rather than about descriptions",
      _kindcell("![KIND UNDETERMINED](img.png)"), "UMBRELLA-CELL", 1)


# ------------------------------------------------ PR #510 Codex R31 controls --
# R31-1: a table's schema was matched on its header cells' RAW text, so a
# header spelling `#` as `&#35;` (§2.5, a character reference that renders the
# character) was not the `slice` schema's header.  The table was admitted as a
# NON-schema one, and a non-schema table declares nothing and asserts nothing:
# not one row but every row of that memo left the run silently, at rc 0, with
# its `Deps` edges unasserted.  The subject is a LINKED memo because that is
# where it bites -- a linked memo is not required to carry every schema, so
# nothing else says the table went missing.
#
# The fix is two clauses, and each has its own mutant: `Table.bind` compares
# what the header RENDERS (`rendered`, the reader's rendering with no keep-set
# exception), and `Memo.__init__` resolves the header cells BEFORE binding,
# since a cell that has not been through the inline pass renders its raw text.

_SIB_SCHEMA = """# sibling

## §5. Slice plan

| %s | Slice | Primary module(s) | Slot | Tier | Deps |
|---|---|---|---|---|---|
| **8z** | **UMBRELLA, not a terminal unit.** charter. | `g.rs` | — | T1 | — |
| **6z** | Terminal.  Acceptance: the probe must return 6. | `h.rs` | — | T1 | **8z** |
"""

case("POSITIVE", "(R31-1 §2.5) a linked memo whose `slice` header spells `#` as `&#35;` IS that schema: "
                 "the character reference renders `#`, so the table is bound, `8z` is declared an "
                 "umbrella and the terminal row's `Deps` edge is a naming site.  The reported shape -- "
                 "before R31 the raw comparison made it a non-schema table and every declaration and "
                 "assertion in the memo left the run at rc 0",
     build(), "See [the walk](slice-9z-sib.md).", 1, sibling=_SIB_SCHEMA % "&#35;")
R31_1_RENDERED_HEADER = CASES[-1].name
"""Red under both R31-1 mutants: the raw comparison, and the bind that runs
before the header cells are lexed."""

case("POSITIVE", "(R31-1 §2.5) the same sibling with the header spelled `#` outright reports the same "
                 "one site -- the discriminating half: the memo, the rows and the edge are the "
                 "control's constants and only the SPELLING of one header cell differs, so the case "
                 "above is a claim about the reading and not about the fixture",
     build(), "See [the walk](slice-9z-sib.md).", 1, sibling=_SIB_SCHEMA % "#")

case("POSITIVE-NOVEL", "(R31-1 §2.4) a header cell spelling `#` as the ESCAPE `\\#` binds too: §2.4 "
                       "renders a backslash escape as the character itself, and this is the spelling "
                       "the reviewer did not name -- the fix reads the rendering rather than "
                       "enumerating the two syntaxes that reach it",
     build(), "See [the walk](slice-9z-sib.md).", 1, sibling=_SIB_SCHEMA % "\\#")

case("NEGATIVE", "(R31-1 §2.5) a header cell rendering `$` (`&#36;`) is NOT the `slice` schema: the "
                 "reading moved and the COMPARISON did not loosen, so the table stays non-schema and "
                 "declares nothing.  The partner that bounds the fix -- a match that normalised the "
                 "header instead of rendering it would pass the three above and fail this one",
     build(), "See [the walk](slice-9z-sib.md).", 0, sibling=_SIB_SCHEMA % "&#36;")


# ---------------------------------------- PR #510 Codex R32 controls (R31-1) --
# THE TWO CLAUSES R31-1 ADDED WITHOUT A PROBE, against the discipline stated 200
# lines above in the same commit ("each clause has its own probe").  `admit_table`
# claims a table's SHAPE is settled over raw text at block level, before any
# inline construct exists, so a §2.5 character reference cannot spell it: a
# delimiter row of `&#45;` is no delimiter row and the table never forms, and a
# `&#124;` does not split a cell.  Three sibling clauses of the same fix each got
# a control; these two got none, and the design re-gate found them.
#
# ⚠ NO GFM ARTEFACT IS VENDORED ANYWHERE IN THIS TREE -- no examples, no prose --
# and `webref` does not cover GFM, so unlike the `&#35;` header clauses (which
# CommonMark Examples 26 and 12 settle) these two are checked by BEHAVIOUR here
# and by nothing else.  That is worth knowing rather than implying otherwise;
# vendoring GFM §4.10 the way §4.6's tag list is vendored is the standing fix.

_R32_SIB = """# sibling

## §5. Slice plan

| # | Slice | Primary module(s) | Slot | Tier | Deps |
|%s|---|---|---|---|---|
| **8z** | **UMBRELLA, not a terminal unit.** charter. | `g.rs` | — | T1 | — |
| **6z** | Terminal.  Acceptance: the probe must return 6. | `h.rs` | — | T1 | %s |
"""

case("POSITIVE", "(R32 §2.5/GFM) a delimiter row spelled `&#45;&#45;&#45;` is NO delimiter row: a table's "
                 "shape is settled over RAW text before any inline construct exists, so the character "
                 "reference never becomes a hyphen, the table never forms, and the linked memo declares "
                 "nothing.  The clause `admit_table` states and nothing probed",
     build(), "See [the walk](slice-9z-sib.md).", 0,
     sibling=_R32_SIB % ("&#45;&#45;&#45;", "**8z**"))
case("POSITIVE", "(R32 §2.5/GFM) the same sibling with a plain `---` delimiter row DOES form the table "
                 "and names the umbrella -- the discriminating half: the memo, the rows and the edge "
                 "are the control's constants and only the delimiter row's spelling differs",
     build(), "See [the walk](slice-9z-sib.md).", 1,
     sibling=_R32_SIB % ("---", "**8z**"))
case("POSITIVE", "(R32 §2.5/GFM) a `&#124;` inside a cell does NOT split it: the row keeps its six "
                 "cells, so it is no schema miss and the `Deps` edge is still read.  Split at the "
                 "reference, the row would have seven cells and the run would be rc 2 -- which is why "
                 "the measure here is the SITE and not merely the exit status",
     build(), "See [the walk](slice-9z-sib.md).", 1,
     sibling=_R32_SIB % ("---", "**8z** &#124; x"))
