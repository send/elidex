#!/usr/bin/env python3
"""PR #510 Codex R42's fixture controls -- the fifth module of the one `CASES` list.

Carved from `plan_memo_selftest_cases_r26.py` at the REVIEW-ROUND seam this
suite already splits on four times (`_cases_pr510.py` / `_cases_inline.py` /
`_cases_r26.py`, and `_mutants_pr510.py` / `_mutants_inline.py` /
`_mutants_r26.py` / `_mutants_r30.py`).  Taken at TOUCH TIME rather than at the
1,000-line bound: the R42-8 and R42-10 controls of 2026-09-21 took that module
from 773 to 924 lines, and CLAUDE.md asks for the split when the file is
touched, not when a review round reports it
(`memory/feedback_touch-time-split-means-while-writing.md`).

The seam was MEASURED, not chosen: an AST pass over the two halves reports the
R42 group using exactly TWO names from the rest of the module -- the fixture
builders `idcell` and `kindcell`, shared with the R30-3 controls that keep
them -- and the rest using none of the R42 group's.  Those two lost their
leading underscore in the same commit, because a name that crosses a module
boundary is not module-private.

R42's subject is CommonMark §6.4: what a resolved image's description reduces
to, row by row of §3.0b's closed inline list, plus the two contradictions a
blank id cell can carry.  Its mutants are `plan_memo_selftest_mutants_r30.py`'s,
which imports the control names from here.

Appends to the SAME `CASES` list; the runner is the one import site.
"""

from plan_memo_selftest_cases import CASES, LINK, SIB_TABLE, acase, build, case, rcase
from plan_memo_selftest_cases_r26 import idcell, kindcell

# -- R42: a blank id cell and an umbrella marker contradict each other.
# The reviewer's shape: the row is keyed by nothing, so it is absent from
# `ids`; assertion (a) sees the marker and emits no missing-marker seed; and
# assertion (b) skips a row whose `self_id` is None.  Every gate declines it
# for a different reason and the run exits 0 with a `Deps` edge unasserted.
_R42_MISS = ("schema", "id cell is blank")

case("POSITIVE", "(R42) a DELIBERATE blank id cell whose declaring field claims the row is an "
                 "umbrella is a contradiction, not a non-row: keyed by nothing it is absent from "
                 "`ids`, so assertion (b) never reads its `Deps` edge -- reported as a schema miss "
                 "at rc 2 rather than passed over at rc 0",
     idcell("**—**", marked=True), "", 1, measure=_R42_MISS)
R42_BLANK_MARKER = CASES[-1].name
"""Red under the mutant that accepts the blank-and-marked row again."""

case("NEGATIVE", "(R42) the partner that bounds it: the SAME blank id cell with no kind phrase in "
                 "its declaring field is the deliberate non-row the exemption is for, and stays "
                 "silent -- the blank is not what is reported, the contradiction is",
     idcell("**—**", marked=False), "", 0, measure=_R42_MISS)

case("NEGATIVE", "(R42) and the POINTER phrase on a blank id cell is NOT the contradiction: it says "
                 "another row owns this one, which is exactly what an unkeyed row is for -- the "
                 "#506 memo's `Function`/`eval` row is this shape, and a rule over every kind phrase "
                 "rather than the marker alone reported it",
     build(i7z="**—**", s7z="Owned by **9z**, which carries the marker."), "", 0, measure=_R42_MISS)


# -- R42-1 / R42-5a: §6.4 over §3.0b's CLOSED inline list.  The description
# renders as the plain string content of its inline children, and the branch
# had covered §6.2 / §6.3 / §6.4 (R30-3) while leaving §6.1 and §6.5 masked --
# the last two rows of a list the plan itself declares closed, which is why
# this is a CROSS-PRODUCT and not three more one-off cases: every row of §3.0b
# that can carry an id, inside one resolved image description, measured against
# the same id in bare prose.
_IMG = "See ![%s owns it](img.png)."

case("NEGATIVE", "(R42 §6.4) the baseline the cross-product is measured against: the id in BARE prose "
                 "outside any image is one reported site",
     build(), "9z owns it.", 1)
for _label, _inner in (("§6.2 emphasis", "*9z*"),
                       ("§6.3 link text", "[9z](x)"),
                       ("§6.4 nested image alt", "![9z](i.png)"),
                       ("§6.1 code span", "`9z`"),
                       ("§6.5 autolink", "<xx:9z>")):
    case("POSITIVE", "(R42 §6.4) a declared id inside a %s inside a RESOLVED image description is one "
                     "reported site, the same as in bare prose: §6.4 reduces the description to the "
                     "plain string content of its inline children, so every row of §3.0b's closed "
                     "list contributes its text and none of its markup.  §6.1 and §6.5 were the two "
                     "rows the image-close branch never demoted -- a code span there BLANKED a kind "
                     "marker and an autolink contributed nothing at all" % _label,
         build(), _IMG % _inner, 1)
R42_IMG_AUTO = CASES[-1].name

# ⚠ THE CODE-SPAN ROW ABOVE DOES NOT DISCRIMINATE THE DEMOTION, and both of its
# mutants SURVIVED until this was measured: `` `9z` `` is an ID-ONLY span, which
# `code_mask` skips whether or not it was demoted, so the control was proving
# the id-only exception and not §6.4 (`memory/feedback_surviving-mutation-means-
# the-probe-has-another-subject.md`).  The discriminating shape is a code span
# holding PROSE -- which is what R42-1 actually reported -- and its measure is
# the VERDICT, because a blanked kind marker leaves the row terminal at rc 0
# where every other family gives rc 1.
acase("POSITIVE", "(R42 §6.4/§6.1) a kind marker wholly inside a CODE SPAN inside a resolved image "
                  "description is read: §6.4 reduces the description to the plain string content of "
                  "its inline children, so the alt text holds the marker and the row's `Deps` edge is "
                  "asserted.  The reported shape: the span stayed in `lx.code`, the disposition "
                  "blanked the whole marker, the row read TERMINAL and the run exited 0 -- while the "
                  "identical marker in a LINK inside the same image exited 1",
      kindcell("![`UMBRELLA, not a terminal unit.`](img.png)"), "UMBRELLA-CELL", 1)
R42_IMG_CODE = CASES[-1].name

# The DECORATION exception, measured rather than asserted: a decorated id reads
# the SAME inside a resolved image description as it does in bare prose, and the
# code-span form reads the same as the `**` form.  That equality is the whole
# claim -- the id-only test runs BEFORE the demotion, so the span's delimiters
# stand and `` `9z`7z `` does not glue into one token.  ⚠ Written first as "is
# still TWO ids", which is wrong in both positions: it is one, because `7z` is
# licensed where `9z` is not. The invariant is the EQUALITY, not the count.
for _where, _prose in (("inside a resolved image description", _IMG % "`9z`7z"),
                       ("in bare prose (the same reading)", "`9z`7z owns it."),
                       ("the `**` twin inside an image (the precedent)", _IMG % "**9z**7z"),
                       ("the `**` twin in bare prose", "**9z**7z owns it.")):
    case("POSITIVE", "(R42 §6.4/§6.1) a DECORATED id %s reports the one unlicensed id: the id-only "
                     "exception runs before the demotion, so the delimiters stand and the two ids do "
                     "not glue -- the precedent `dispose` already sets for the `**` pair, and the "
                     "reason a demoted code span is routed through `id_only` rather than blanked" % _where,
         build(), _prose, 1)

rcase("NEGATIVE", "(R42 §6.1) the partner that bounds the demotion: the SAME code span OUTSIDE an "
                  "image is a QUOTATION and its prose is not read -- a kind marker quoted whole stays "
                  "quoted (I-A), so the §6.4 demotion must not leak out of the description",
      build(suz="`UMBRELLA, not a terminal unit.`", duz="**7z**"), "", 0)


# -- R42-6: §6.1's trim is part of the CONTENT §6.4 reads.  A direct consequence
# of the demotion above -- a padded span only started reaching that branch once
# it stopped being masked -- and the three arms are the spec's own clause: the
# trim fires, it must NOT fire (all spaces), and the span is outside a
# description at all.
#
# ⚠ THE FIXTURE SPLITS AN ID WHOSE HALVES ARE UNDECLARED, and that is what makes
# the measure discriminate.  Written first over `` `Slice 9` z ` `` the count
# stayed 1 under BOTH mutants -- the wrong reading reports `9`, which the
# template also declares, so "one site" was true either way and the claim is
# about WHICH.  `#11-zz-alph` + `a` are declared nowhere, so the wrong reading
# reports NOTHING and the count is the verdict
# (`memory/feedback_surviving-mutation-means-the-probe-has-another-subject.md`).
case("POSITIVE", "(R42 §6.1/§6.4) a PADDED code span inside a resolved image description renders its "
                 "TRIMMED content: §6.1 removes one leading and one trailing space when the content "
                 "begins and ends with one, and §6.4 uses that string -- so "
                 "`` ![Slot #11-zz-alph` a ` owns it](img.png) `` has the alt text "
                 "`Slot #11-zz-alpha owns it` and the slot is ONE reported site.  Read verbatim it "
                 "splits into `#11-zz-alph` and `a`, which are declared nowhere: no site, rc 0, and "
                 "no residue to say the checker could not read it",
     build(), "See ![Slot #11-zz-alph` a ` owns it](img.png).", 1)
R42_TRIM = CASES[-1].name
"""Red under both R42-6 mutants: dropping the trim splits the slug, and trimming
unconditionally eats the separator the all-space arm needs."""

case("POSITIVE", "(R42 §6.1/§6.4) the arm the spec spells out, and the one an unconditional trim "
                 "breaks: a span that consists ENTIRELY of spaces is NOT trimmed (\"but does not "
                 "consist entirely of space characters\"), so "
                 "`` ![Slot #11-zz-alpha`  `owns it](img.png) `` keeps the separator and the slug "
                 "stands alone -- trimmed, it would fuse with what follows and the site would vanish.  "
                 "⚠ The span is the ONLY separator here, deliberately: written with a space after the "
                 "closing backtick the control stayed green under the mutant, because that space did "
                 "the separating and the claim was never tested",
     build(), "See ![Slot #11-zz-alpha`  `owns it](img.png).", 1)
R42_ALLSPACE = CASES[-1].name
"""Red under the mutant that trims unconditionally -- the arm `R42_TRIM` cannot
reach, since trimming IS correct on a padded span."""

case("NEGATIVE", "(R42 §6.1/§6.4) the partner that bounds the whole demotion: the SAME padded span "
                 "OUTSIDE an image is MASKED whole, so its content is not read, the slug never "
                 "completes and NOTHING is reported -- the trim belongs to §6.4's reading of a "
                 "description, never to a code span in ordinary prose",
     build(), "Slot #11-zz-alph` a ` owns it.", 0)


# -- R42-9: the blank-id contradiction, widened back by exactly ONE phrase.
# ⚠ The R42 correction narrowed the predicate to the MARKER because a real memo
# row refuted the POINTER arm -- and took the UNDETERMINED arm with it, though
# nothing had refuted that one.  §5 puts an undetermined row IN the naming
# population with the same no-owner obligation as an umbrella, so a blank id
# contradicts it for the same reason.  Narrowing to the case that was SHOWN
# rather than to the complement of what was REFUTED is the shape this checker
# keeps finding in the documents it reads.
_R42_9 = ("\n| — | %s | `g.rs` | — | T1 | **7z** |")

case("POSITIVE", "(R42-9) a blank id cell whose declaring field spells KIND UNDETERMINED is the same "
                 "contradiction as the marker: §5 gives an undetermined row the no-owner obligation "
                 "an umbrella has, and keyed by nothing it is absent from `ids`, so assertion (b) "
                 "never reads its `Deps` edge -- rc 2, not the silent 0 the marker-only predicate gave",
     build().replace("| **Uz** | Terminal.  Acceptance: the probe must return 5. | `f.rs` | — | T1 | — |", "| **Uz** | Terminal.  Acceptance: the probe must return 5. | `f.rs` | — | T1 | — |" + _R42_9 % "KIND UNDETERMINED here."), "", 1, measure=_R42_MISS)
R42_9_UNDET = CASES[-1].name

case("NEGATIVE", "(R42-9) and the arm that WAS refuted stays refuted: the POINTER phrase on a blank "
                 "id is the legitimate shape (`declaring_rows` names the #506 memo's "
                 "`Function`/`eval` row, `:1985`) -- another row owns this one, which is what an "
                 "unkeyed row is for",
     build().replace("| **Uz** | Terminal.  Acceptance: the probe must return 5. | `f.rs` | — | T1 | — |", "| **Uz** | Terminal.  Acceptance: the probe must return 5. | `f.rs` | — | T1 | — |" + _R42_9 % "This is a pointer rather than a slice."), "", 0,
     measure=_R42_MISS)
"""⚠ The phrase is the one `KIND_PHRASES` spells, verbatim.  Written as "Owned
by **9z**, which carries the marker" this fixture matched NO phrase at all, so
the control asserted "a blank row with no kind phrase is silent" -- true, and a
different claim.  Its mutant (widening the predicate back to the pointer arm)
survived, which is how the wrong subject was found rather than read."""


# -- R42-8 / §6.6 inside a resolved image description.  THE ONE FINDING ON THIS
# SURFACE THAT RAN THE OTHER WAY: a FABRICATED report, not a missed one.  §8
# carried it undecided for two rounds because the vendored corpus cannot settle
# it -- 22 Images examples, none with a `<` in a description -- and a "fix" on
# the prose alone would have SILENCED a true report if the alt really did drop
# the markup.  Settled against both reference implementations at the corpus's
# own version (cmark 0.31.2 and commonmark.js 0.31.2):
#
#   ![UMBRELLA, not a <span>terminal unit](img.png)
#     -> <img src="img.png" alt="UMBRELLA, not a &lt;span&gt;terminal unit" />
#
# cmark ESCAPING the angle brackets is what decides it: they are the alt's
# CONTENT, not markup.  §6.4 reduces the description to the plain string
# content of its inline children, and an `html_inline` node's plain string
# content is its own SOURCE TEXT -- so the span renders its characters, where
# in ordinary prose it renders nothing at all.
#
# ⚠ THE PAIR IS THE SUBJECT, not either control alone.  A finding count of 0 is
# what ANY silence gives, so the baseline below (the same cell with the span
# taken out, at 1) is what makes the first control about the span.
acase("NEGATIVE", "(R42-8 §6.6) a raw HTML span inside a resolved image description is the alt's own TEXT, "
                  "so the kind marker is NOT read across it: `![UMBRELLA, not a <span>terminal unit]"
                  "(img.png)` has the alt text `UMBRELLA, not a <span>terminal unit` (cmark 0.31.2 and "
                  "commonmark.js 0.31.2 both, the latter escaping the brackets -- they are content) and "
                  "the marker phrase is not in it.  Masked as markup the span JOINED its two sides and "
                  "the row exited 1 on an ownership claim nobody made: a FABRICATED finding, the "
                  "opposite direction from every other §6.4 defect on this surface",
      kindcell("![UMBRELLA, not a <span>terminal unit](img.png)"), "UMBRELLA-CELL", 0)
R42_8_HTML_ALT = CASES[-1].name
"""Red under the mutants that stop demoting a raw HTML span, or re-mask it."""

acase("POSITIVE", "(R42-8 §6.6) the BASELINE that control is measured against -- the identical cell with "
                  "the span taken out reads the marker and exits 1, so the silence above is the span's "
                  "and not the fixture's",
      kindcell("![UMBRELLA, not a terminal unit](img.png)"), "UMBRELLA-CELL", 1)

# ⚠ AND THE CONTROL THAT SEPARATES "CONTRIBUTES ITS TEXT" FROM "MERELY DOES NOT
# JOIN": a blanked span would not join its two sides either, and would still
# HIDE an id inside it.  The reference puts the attribute value in the alt
# verbatim (`alt="x &lt;span title=&quot;9z owns it&quot;&gt;y"`), so there the
# id is document text and IS a naming site -- the exact reverse of the R17
# reading in bare prose, which is untouched beside it.
case("POSITIVE", "(R42-8 §6.6) an id inside a raw HTML ATTRIBUTE inside a resolved image description is a "
                 "reported naming site: the alt text spells the tag out, so `9z owns it` is document "
                 "text there.  This is what a blank mask cannot give -- it would not join the sides "
                 "either, and would still hide the id",
     build(), 'See ![x <span title="9z owns it">y](img.png).', 1)
R42_8_HTML_ATTR_SITE = CASES[-1].name
"""Red under the mutants that stop demoting a raw HTML span, or re-mask it."""

case("NEGATIVE", "(R42-8 §6.6) the R17 reading in BARE PROSE is untouched: the same span outside any "
                 "image is masked whole and its id is no naming site.  The arm that was refuted is "
                 "\"a raw HTML span renders nothing INSIDE A RESOLVED IMAGE DESCRIPTION\", not \"a raw "
                 "HTML span renders nothing\"",
     build(), 'See <span title="9z owns it">y</span>.', 0)

# The seed follows the reading, in both block shapes.  A demoted span hides
# nothing -- its text is in the scanned stream -- so it is not seeded, which is
# the rule an autolink already carries.  Bare prose still seeds (above, and the
# R17 pair in `plan_memo_selftest_cases_inline.py`).
acase("NEGATIVE", "(R42-8 §6.6 seed) a raw HTML span DEMOTED into a resolved image description is not "
                  "seeded -- the naming scan has already read it -- and a span crossing a LINE ENDING "
                  "is not seeded per line either",
      build(), "LEX-UNSUPPORTED?", 0, prose='See ![x <span\ntitle="9z owns it">y](img.png).')
R42_8_HTML_NO_SEED_PROSE = CASES[-1].name
"""Red under the mutant that seeds a demoted span from a PARAGRAPH again."""

acase("NEGATIVE", "(R42-8 §6.6 seed) the same in a CELL, which is the other half of the seed's "
                  "population and its own site: the paragraph control cannot see it",
      build(suz='![x <span title="9z owns it">y](img.png)'), "LEX-UNSUPPORTED?", 0)
R42_8_HTML_NO_SEED_CELL = CASES[-1].name
"""Red under the mutant that seeds a demoted span from a CELL again."""


# -- R42-10: A LINKED MEMO'S UNBOUND TABLE THAT MAKES A CENSUS CLAIM.  The
# schema-miss gate asks only of `main` -- deliberately, a linked detail memo
# may hold no slot ledger -- so a linked memo whose slice table binds to NO
# schema declared nothing and the run exited 0.  The reported framing was an
# IMAGE header, which is an FP (above); the defect underneath is wider and has
# nothing to do with images.
#
# ⚠ THE PREDICATE WAS CHOSEN BY MEASUREMENT, over the three candidates §8
# carried.  "The first column tokenises as row ids" is exactly right on these
# four fixtures and reports 151 tables over the 141 plan memos on this disk (a
# landing record's `obj` / `R1`...`R7` review tables) -- the negative below
# pins it.  A header NEAR-MISS is silent on that corpus too, but it fires on a
# renamed header whose table DECLARES NOTHING, which the second negative pins.
# What is left is the claim itself: a kind phrase is an assertion about the
# census, so a table carrying one and binding to nothing is a contradiction.
# Measured with the shipped predicate: 141 memos, 0 findings.
_UNBOUND = SIB_TABLE.replace("| # | Slice |", "| No. | Slice |")

case("POSITIVE", "(R42-10) a LINKED memo whose slice table binds to no schema -- its header reads "
                 "`No.` where the schema reads `#` -- but carries an umbrella row with a `Deps` "
                 "edge: the claim is made and the whole table leaves the census.  It exited 0, with "
                 "no gate reporting it, because the schema-miss gate asks only of `main`",
     build(), LINK, 1, sibling=_UNBOUND % "**7z**",
     measure=("schema", "binding to NO schema"))
R42_10_UNBOUND_CLAIM = CASES[-1].name
"""Red under the mutants that drop the gate or narrow it back to `main`."""

case("NEGATIVE", "(R42-10) the SAME unbound header with no kind phrase in it is silent -- the gate "
                 "is keyed on the CLAIM, not on the header.  This is the control a header-near-miss "
                 "predicate fails: `No.` differs from `#` there too, and that table declares nothing",
     build(), LINK, 0,
     sibling=_UNBOUND.replace("**UMBRELLA, not a terminal unit.** carved.", "an ordinary carve.") % "—",
     measure=("schema", "binding to NO schema"))
R42_10_UNBOUND_NO_CLAIM = CASES[-1].name
"""Red under a mutant that keys the gate on the header instead of the claim."""

case("NEGATIVE", "(R42-10) a documentation table whose first column is ID-SHAPED is silent: a landing "
                 "record's review-round table (`obj`, `R1`...`R7`) keys its rows and declares "
                 "nothing.  This is the control the REJECTED predicate fails -- it reports 151 such "
                 "tables over the 141 plan memos on this disk",
     build(), LINK, 0,
     sibling=("## rounds\n\n| obj | Round | Outcome |\n|---|---|---|\n"
              "| R1 | 1 | carried |\n| R7 | 7 | closed |\n"),
     measure=("schema", "binding to NO schema"))
R42_10_UNBOUND_ID_SHAPED = CASES[-1].name
"""Red under a mutant that keys the gate on the id shape of the first column."""

case("NEGATIVE", "(R42-10) the phrase is read off the RENDERED cell, as the census reads a declaring "
                 "field: a marker inside a CODE SPAN in an unbound table is not a claim, exactly as "
                 "it is not one in a bound table's declaring field",
     build(), LINK, 0,
     sibling=_UNBOUND.replace("**UMBRELLA, not a terminal unit.** carved.",
                              "`UMBRELLA, not a terminal unit.` carved.") % "—",
     measure=("schema", "binding to NO schema"))
R42_10_UNBOUND_RENDERED = CASES[-1].name
"""Red under the mutant that reads the raw cell text instead of the rendering."""
