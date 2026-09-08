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
