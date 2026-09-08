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
`plan_memo_selftest_properties.py`'s, by those modules' own seams.
"""

from plan_memo_selftest_cases import build, case

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
