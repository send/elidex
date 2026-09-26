#!/usr/bin/env python3
"""PR #510 Codex R42's fixture controls -- a cases module carved at the R42 seam.

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

every cases module holds its OWN `CASES` and binds its own spellings
(`spellings()`); `plan_memo_selftest_cases.cases()` -- the one collection
step -- gathers them, and every case is a line of the golden manifest.
"""

from plan_memo_selftest_cases import LINK, SIB_TABLE, build, spellings
from plan_memo_selftest_cases_r26 import idcell, kindcell

# This module's OWN rows and spellings; `plan_memo_selftest_cases.cases()` gathers
# every cases module's list in one step; every case is a line of the golden manifest.
CASES = []
case, acase, rcase = spellings()

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

# ⚠ AND THE HALF OF §6.1 NOTHING WATCHED (PR #510 R45, found by re-running the
# fix's own mutation rather than by reading it).  §6.1 is TWO steps and the
# ORDER is the rule: line endings become spaces FIRST, and only then does the
# trim ask whether the result begins and ends with one.  Both shipped mutants
# patch the `if` -- the TRIM -- so step one itself could be replaced with a
# no-op and the whole suite stayed GREEN, while a real naming site was lost.
# The mutation is not equivalent: with step one the alt reads
# `Slot #11-zz-alpha owns it` and the slot is ONE site; without it the raw test
# sees `\n`, declines the trim, and the slug splits into `#11-zz-alph` + `a`,
# declared nowhere -- 0 sites, rc 0, no residue.
# ⚠ The measure is WHICH id, not how many: the space-padded twin above is the
# arm step one must NOT change, and it reports 1 either way.
case("POSITIVE", "(R42 §6.1/§6.4) §6.1's TWO steps run in ORDER: a code span padded with a LINE "
                 "ENDING inside a resolved image description is normalised to a space FIRST and only "
                 "then trimmed, so `` ![Slot #11-zz-alph`<NL>a ` owns it](img.png) `` has the alt "
                 "text `Slot #11-zz-alpha owns it` and the slot is ONE reported site.  Written "
                 "against the RAW content the trim declines -- `\\n` is not a space -- the slug "
                 "splits into `#11-zz-alph` and `a`, declared nowhere, and the run exits 0 saying "
                 "nothing.  ⚠ This is the half of the R42-7 fix that shipped UNPINNED: both R42-6 "
                 "mutants patch the trim, so step one could be made a no-op with every control green",
     build(), "See ![Slot #11-zz-alph`\na ` owns it](img.png).", 1)
R42_LINE_ENDING_FIRST = CASES[-1].name
"""Red under the mutant that drops §6.1 step one (the line-ending
substitution); the space-padded `R42_TRIM` twin stays green under it, which is
what makes this control's subject step one rather than the trim."""

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
# ⚠ The ESCAPING is cmark's SERIALIZER and not the deciding fact -- this
# comment said it was, and commonmark.js 0.31.2 emits the same characters
# UNESCAPED (`alt="UMBRELLA, not a <span>terminal unit"`).  What BOTH agree on,
# and all the fix rests on, is that the span's characters are IN the alt.  The
# retraction was applied to `plan_memo_stream.py` and not here, which is the
# `memory/feedback_sweep-obligations-not-only-statements.md` shape: a
# correction owes every site that carries the statement, not the one that was
# reported.  §6.4 reduces the description to the plain string
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


# -- R45: TWO POPULATION-SCOPE LOOPS THAT WERE CORRECT AND UNWATCHED.  Found by
# running the mutation rather than by reading the code: scoping either to
# `self.memos[:1]` -- the same edit shape `_declare`, `keep`, `data_rows`, the
# transitive walk and `_unbound_claims` all HAVE a mutant for -- left every one
# of the 695 controls green.  The population-scope ratchet covered five of
# seven loops, and the two it missed both report a LINKED memo's schema miss,
# which is the I-C silent-skip class this checker exists for
# (`memory/feedback_derived-populations-shrink-in-silence.md`: a population the
# next edit can shrink in silence).
# ⚠ The two fixtures discriminate SEPARATELY, measured: scoping `_unkeyed`
# leaves the width miss reported and vice versa, so neither control stands in
# for the other.
_SIB_UNKEYED = SIB_TABLE.replace("| **Wz** |", "| xxxxWz |")
_SIB_WIDTH = SIB_TABLE.replace("| `w.rs` | — | T1 | %s |", "| `w.rs` | — | T1 | %s | EXTRA |")

case("POSITIVE", "(R45 scope) a LINKED memo's UNKEYED row is reported: `_unkeyed` asks of the whole "
                 "population, not of `main`.  Scoped to `main` the sibling's `xxxxWz` id cell "
                 "declares nothing, the row is unscanned and the run exits 0 -- the I-C class, in "
                 "the memo the census exists to reach",
     build(), LINK, 1, sibling=_SIB_UNKEYED % "—",
     measure=("schema", "the row declares nothing and is unkeyed"))
R45_UNKEYED_SCOPE = CASES[-1].name
"""Red under the mutant scoping `_unkeyed` to `main`; green under the
table-miss one, which is what makes its subject that loop alone."""

case("POSITIVE", "(R45 scope) a LINKED memo's table WIDTH miss is reported: the table-miss loop asks "
                 "of the whole population too.  Scoped to `main` the sibling's 7-cell row under a "
                 "6-cell header leaves the run at 0 with nothing saying the row was skipped",
     build(), LINK, 1, sibling=_SIB_WIDTH % "—",
     measure=("schema", "a shifted read fabricates findings"))
R45_TABLE_MISS_SCOPE = CASES[-1].name
"""Red under the mutant scoping the table-miss loop to `main`; green under the
`_unkeyed` one."""


# -- R45: THE ONE REVIEW FINDING THIS LOOP LEFT WITH NO DISPOSITION AT ALL, and
# it is an FP -- but "the author probably knows it is an FP" is not a
# disposition, and nothing here watched the behaviour either way.  The finding
# asked that a delimiter cell require THREE hyphens, so that `|-|-|` would not
# admit a table.  Settled by EXECUTION against the GFM reference implementation
# rather than by reading the prose, which is the rule this PR adopted for its
# other two open spec questions:
#
#   $ printf '| a | b |\n|-|-|\n| 1 | 2 |\n' | cmark-gfm -e table
#   <table> ...
#
# cmark-gfm 0.29.0.gfm.13 makes a table from a ONE-hyphen delimiter, and from
# `:-`, `-:` and `:-:` too.  The checker is right and the finding is wrong; the
# three-hyphen rule is a different dialect's.  ⚠ `is_separator` had ZERO
# self-test references before this control and no fixture used a short
# delimiter, so the suite could not have told the two readings apart.
_R45_DELIM_ARMS = []
for _label, _delim in (("one hyphen", "-"), ("left-aligned", ":-"),
                       ("right-aligned", "-:"), ("centred", ":-:")):
    case("POSITIVE", "(R45 GFM §4.10) a delimiter cell of %s admits the table exactly as `---` does "
                     "-- \"cells whose only content are hyphens (-), and optionally, a leading or "
                     "trailing colon (:)\" sets no minimum length.  Verified against cmark-gfm "
                     "0.29.0.gfm.13, which makes a `<table>` from all four; a review finding asking "
                     "for a three-hyphen minimum is the FP this pins" % _label,
         build().replace("|---|---|---|---|---|---|", "|%s|%s|%s|%s|%s|%s|" % ((_delim,) * 6))
                .replace("|---|---|---|---|", "|%s|%s|%s|%s|" % ((_delim,) * 4)), "9z owns it.", 1)
    _R45_DELIM_ARMS.append(CASES[-1].name)

# ⚠ ALL FOUR ARMS, not `CASES[-1]`: bound after the loop it named only the
# `:-:` variant, so the mutant proved one spelling and the other three were
# decoration -- the same trap R47-1's mutant fell into, in a loop written one
# commit earlier.
R45_SHORT_DELIM = _R45_DELIM_ARMS[0]
R45_DELIM_ALL = list(_R45_DELIM_ARMS)
"""Red under the mutant re-injecting a three-hyphen minimum: the schema tables
stop being tables, nothing is declared, and the run says nothing."""


# -- R47-1: the row-noun separator re-spelled the dash set.  `DASH` has one
# home and `ROW_NOUN_SEP` carried a second, narrower copy (`[ \t\n-]`), so a
# claim introduced with an EN or EM dash matched no appositive and the numeric
# id -- a declared KNOWN-MISS to the bare pass -- was never reported.  All four
# spellings, because the claim is that the SET is composed and not that one
# character was added; the space arm is the one no dash reading can explain.
_R47_1_ARMS = {}
for _label, _sep in (("an ASCII hyphen", "-"), ("an EN dash", "\u2013"),
                     ("an EM dash", "\u2014"), ("a plain space", "")):
    case("POSITIVE", "(R47-1) a numeric id introduced by a row noun and %s is one reported site: the "
                     "separator composes `plan_memo_ids.DASH`, which is the ONE home of the three "
                     "spellings, rather than re-spelling a narrower set.  With the ASCII hyphen "
                     "alone, `Slice \u2014 9 owns the close rule.` matched no appositive, the bare "
                     "pass deliberately ignores a numeric id, and the ownership claim produced no "
                     "site at rc 0" % _label,
         build(i9z="**9**", s9z="**UMBRELLA, not a terminal unit.** charter."),
         "Slice %s9 owns the close rule." % (_sep + " " if _sep else ""), 1)
    _R47_1_ARMS[_label] = CASES[-1].name

# ⚠ THE MUTANT MUST NAME A DASH ARM, AND THE FIRST VERSION NAMED THE LAST ONE
# REGISTERED -- the PLAIN SPACE, which no dash change can move, so the mutant
# SURVIVED (`memory/feedback_surviving-mutation-means-the-probe-has-another-
# subject.md`, caught by running it).  The space and hyphen arms are the
# BASELINE: they say the site is reported for reasons a dash cannot explain,
# and they must stay green under the mutant.  The en and em dash arms are the
# subject.
R47_1_DASH_SET = _R47_1_ARMS["an EM dash"]
"""Red under the mutant re-spelling the narrower ASCII-only separator."""
R47_1_EN_DASH = _R47_1_ARMS["an EN dash"]
"""The second dash the ASCII-only set drops; named separately so the mutant
proves both spellings and not just the one that was reported."""

# -- R47-2: the unbound-claim gate read the STREAM only.  A claim spelled
# across a masked span is what a READER sees and what the disposed stream does
# not, so the gate found nothing, the run emitted a non-gating `LEX-SPLIT?`
# seed and exited 0 -- the same I-C silent skip the gate exists to close, one
# construct further in.  ⚠ Its own follow-on: the gate landed one commit
# earlier in this same session.
_UNBOUND_STRADDLE = _UNBOUND.replace("**UMBRELLA, not a terminal unit.** carved.",
                                     "**UMBRELLA, not a `terminal` unit.** carved.")

case("POSITIVE", "(R47-2) a LINKED memo's unbound table whose kind claim STRADDLES a masked span is "
                 "the same miss as a plain one: `` **UMBRELLA, not a `terminal` unit.** `` is what a "
                 "reader sees and what the disposed stream does not, so a stream-only search found "
                 "nothing and the run exited 0 with the table's declarations and `Deps` gone.  Asked "
                 "through `kind_disagreements`, the CANONICAL question `_kind_residue` already gates "
                 "a bound row with -- not a second rule",
     build(), LINK, 1, sibling=_UNBOUND_STRADDLE % "**7z**",
     measure=("schema", "binding to NO schema"))
R47_2_UNBOUND_STRADDLE = CASES[-1].name
"""Red under the mutant dropping the second reading from the gate."""

case("NEGATIVE", "(R47-2) and the I-A arm survives BY CONSTRUCTION, not by an exemption: a phrase "
                 "quoted WHOLE in an unbound table still declares nothing.  `kind_disagreements` "
                 "requires a STRADDLE, so the quoted form is not a disagreement and needed no case "
                 "carved out of the widened gate",
     build(), LINK, 0,
     sibling=_UNBOUND.replace("**UMBRELLA, not a terminal unit.** carved.",
                              "`UMBRELLA, not a terminal unit.` carved.") % "—",
     measure=("schema", "binding to NO schema"))
R47_2_UNBOUND_QUOTED = CASES[-1].name
"""Red under a mutant that widens the gate to every disagreement rather than to
a straddle -- the direction the positive above cannot see."""


# -- R47-4: the three kind phrases spelled the WORD GAP three different ways,
# and two of them dropped a census claim.  cmark 0.31.2 is the ground truth --
# `**UMBRELLA, not a&nbsp;terminal unit.**` renders the marker VERBATIM to a
# reader -- and the run exited 0, so the row left the census as an active
# terminal with nothing saying why.  `&#32;` is the ONE spelling that worked
# (it decodes to U+0020), which is why it is the baseline arm here: it says
# the fixture is not green for a generic reason.
_GAPS = (("a plain space (the baseline arm: this one ALWAYS worked)", " "),
         ("`&#32;` (decodes to U+0020 -- the arm the defect never touched)", "&#32;"),
         ("`&nbsp;` (U+00A0 -- the spelling a human actually types)", "&nbsp;"),
         ("a LITERAL U+00A0 typed into the cell", "\u00a0"),
         ("`&#10;` (a line ending, which a reader reads as a gap)", "&#10;"),
         ("`&#9;` (a tab)", "&#9;"))
for _label, _gap in _GAPS:
    acase("POSITIVE", "(R47-4) the marker phrase written with %s is the marker: a word GAP is what a "
                      "READER sees between two words, not U+0020 in particular.  cmark renders every "
                      "one of these as whitespace, so a census that reads only U+0020 drops the "
                      "claim and the row leaves as an active terminal at rc 0" % _label,
          kindcell("**UMBRELLA, not a%sterminal unit.**" % _gap), "UMBRELLA-CELL", 1)
    _n = CASES[-1].name
    if _gap == "&nbsp;":
        R47_4_NBSP = _n
    elif _gap == "&#9;":
        R47_4_TAB = _n
    elif _gap == "&#32;":
        R47_4_BASELINE = _n

# ⚠ THE OTHER DIRECTION, because widening a gap can loosen a BOUNDARY: the
# word edges `bounded()` added at R22 must still hold.  Without these the
# mutant "make the gap `.*`" would pass every arm above.
for _label, _cell in (("`SUBUMBRELLA, not a terminal unit`", "**SUBUMBRELLA, not a terminal unit.**"),
                      ("`UMBRELLA, not a terminal unitary claim`",
                       "**UMBRELLA, not a terminal unitary claim.**")):
    acase("NEGATIVE", "(R47-4) and the WORD BOUNDARY still holds after the gap widened: %s is not "
                      "the marker.  The gap is a character class between words, never a licence to "
                      "match inside a longer one" % _label,
          kindcell(_cell), "UMBRELLA-CELL", 0)
R47_4_BOUNDARY = CASES[-1].name

acase("POSITIVE", "(R47-4) the UNDETERMINED phrase has its own spelling of the gap and the same "
                  "defect: under `re.ASCII` its `\\s` is ASCII whitespace, so a U+00A0 between "
                  "`KIND` and `UNDETERMINED` read as no kind at all.  The scope `(?u:...)` widens "
                  "the GAP alone and leaves the ASCII case folding, which is why the phrase is "
                  "still not matched by a long-s spelling",
      kindcell("KIND\u00a0UNDETERMINED"), "UMBRELLA-CELL", 1)
R47_4_UNDET_NBSP = CASES[-1].name


# ⚠ THE ARM A WILDCARD GAP BREAKS, and the reason the word-boundary negatives
# above cannot stand in for it: widening the gap to `.` leaves every boundary
# intact (`unitary` still fails on its right lookaround), so the mutant that
# turns GAP into a wildcard SURVIVED against them -- measured, not reasoned.
# A gap is WHITESPACE, so a visible character between the words is not one.
acase("NEGATIVE", "(R47-4) a NON-whitespace character where a gap would be is not a gap: "
                  "`UMBRELLA, not-a terminal unit` is not the marker.  The word boundaries cannot "
                  "see this -- they guard the two ENDS of the phrase -- so it is the only arm that "
                  "fails when the gap class is widened from whitespace to a wildcard",
      kindcell("**UMBRELLA, not-a terminal unit.**"), "UMBRELLA-CELL", 0)
R47_4_NON_WHITESPACE = CASES[-1].name


# -- R47-5: the loops the DERIVED scope ratchet found unpinned.  Three of the
# twelve survived truncation with every control green -- measured, one loop at
# a time -- so each gets the fixture that needs a SECOND iteration.  The other
# nine are pinned by controls that already existed; the ratchet's job was to
# say which, not to invent them.
case("POSITIVE", "(R47-5 scope) TWO unresolved references are both reported: the loop over a memo's "
                 "unanswered references walks all of them.  Truncated it reports the first and the "
                 "second reference's memo is missing from the population with nothing saying so",
     build(), "See [x][aa] and [y][bb].", 2,
     measure=("schema", "unresolved reference"))
R47_5_TWO_REFS = CASES[-1].name

case("POSITIVE", "(R47-5 scope) TWO table misses in one memo are both reported: the loop over a "
                 "table's misses walks all of them.  Truncated, the second row is unscanned at rc 2 "
                 "with only the first named -- a reader fixes one and believes the table is clean",
     build().replace("| **7z** |", "| **7z** | EXTRA |", 1).replace("| **Qx** |", "| **Qx** | EXTRA |", 1),
     "", 2, measure=("schema", "a shifted read fabricates findings"))
R47_5_TWO_MISSES = CASES[-1].name

case("POSITIVE", "(R47-5 scope) TWO kind phrases straddling masked spans in ONE declaring field are "
                 "both reported: `_kind_residue` walks every phrase `kind_disagreements` returns.  "
                 "Truncated, the row's second undecidable kind is silent",
     kindcell("**UMBRELLA, not a `terminal` unit.**  KIND `x` UNDETERMINED"), "", 2,
     measure=("schema", "ACROSS a span this checker does not read as prose"))
R47_5_TWO_PHRASES = CASES[-1].name

case("POSITIVE", "(R47-5 scope) EVERY declared id is given a kind, not just the first: the loop "
                 "over `self.ids` walks the whole map.  Truncated, only one row is classified and "
                 "the census under-counts the no-owner rows it exists to take",
     build(), "", 1, measure=("note", "[CENSUS] 4 no-owner"))
R47_5_ALL_KINDS = CASES[-1].name


# ⚠ TWO MORE, AND THEY EXIST BECAUSE MY OWN MEASUREMENT WAS WRONG.  The probe
# that decided which unpinned loops were "already covered" used
# `src.replace(line, ..., 1)` while `for t in memo.tables:` and
# `for row in memo.schema_rows(s.name):` each occur TWICE -- so it truncated
# the FIRST occurrence both times and reported the second as covered on the
# strength of a measurement of the first.  The widened mutant anchors then
# SURVIVED, which is how it surfaced: the mutation proof measured what the
# probe had only claimed.
case("POSITIVE", "(R47-5 scope) a memo's SECOND table is asked for an unbound claim too: the "
                 "sibling carries a bound table and then an unbound one whose row carries the "
                 "marker.  Truncated to the first table the claim is silent and the run exits 0",
     build(), LINK, 1, sibling=(SIB_TABLE % "—") + "\n\n" + _UNBOUND % "**7z**",
     measure=("schema", "binding to NO schema"))
R47_5_SECOND_TABLE = CASES[-1].name

case("POSITIVE", "(R47-5 scope) a memo's SECOND schema row is read for the unkeyed miss too: the "
                 "sibling's first row is keyed and its second is not.  Truncated to the first row "
                 "the unkeyed one declares nothing and leaves the census at rc 0",
     build(), LINK, 1, sibling=SIB_TABLE.replace("| **Tq** |", "| xxxxTq |") % "—",
     measure=("schema", "the row declares nothing and is unkeyed"))
R47_5_SECOND_ROW = CASES[-1].name


# -- R48-2: `_phrases` kept only the FIRST match per phrase, so a field
# spelling BOTH supported undetermined forms contributed ONE spelling and the
# KIND-SPELLING consistency gate saw a set of size one.  The same two spellings
# in two different ROWS were reported, which is what made it look covered --
# and is the discriminating partner here.
case("POSITIVE", "(R48-2) TWO undetermined spellings in ONE declaring field are both collected: the "
                 "KIND-SPELLING gate is about the DOCUMENT's spellings, so a field using both "
                 "reports exactly as two rows using one each do.  Reading only the first match, the "
                 "set had size one and the run exited 0 on the condition that gate exists for",
     build(s9z="KIND UNDETERMINED and KIND \u2013 UNDETERMINED"), "", 1,
     measure=("finding", "KIND-SPELLING"))
R48_2_TWO_SPELLINGS = CASES[-1].name

case("POSITIVE", "(R48-2) the discriminating twin -- the SAME two spellings split across two rows "
                 "-- was always reported, which is why the one-field case looked covered",
     build(s9z="KIND UNDETERMINED", s7z="KIND \u2013 UNDETERMINED"), "", 1,
     measure=("finding", "KIND-SPELLING"))
R48_2_TWO_ROWS = CASES[-1].name


# -- R49-1 / R49-2.
case("POSITIVE", "(R49-1) a BLANK-id row whose declaring field spells a kind ACROSS a masked span is "
                 "the same contradiction as the clean spelling: the blank-id check asked the "
                 "disposed stream alone, so `` **UMBRELLA, not a `terminal` unit.** `` on a blank-id "
                 "row exited 0 while the clean form exited 2.  Asked through the ONE contradiction "
                 "site now, so a caller cannot forget the arm",
     build(i7z="**—**", s7z="**UMBRELLA, not a `terminal` unit.**", d7z="**9z**"), "", 1,
     measure=("schema", "id cell is blank -- a DELIBERATE non-row"))
R49_1_BLANK_STRADDLE = CASES[-1].name

case("POSITIVE", "(R49-1) the clean twin, which was ALWAYS reported -- so the case above is a claim "
                 "about the READING and not about the fixture",
     build(i7z="**—**", s7z="**UMBRELLA, not a terminal unit.**", d7z="**9z**"), "", 1,
     measure=("schema", "id cell is blank -- a DELIBERATE non-row"))

for _label, _deps in (("an EMPTY label", "[](slice-9z-sib.md)"),
                      ("a punctuation-only label", "[\u2192](slice-9z-sib.md)")):
    acase("POSITIVE", "(R49-2) an umbrella's `Deps` cell whose whole content is a resolved link with "
                      "%s carries an edge: the reader rendering drops a link's TAIL, so the cell had "
                      "no alphanumeric character and read as EMPTY -- while the population walker "
                      "was following that very sibling.  A resolved link is an edge whatever its "
                      "label renders as" % _label,
          build(d9z=_deps), "UMBRELLA-CELL", 1, sibling="# sibling\n")
    if _deps.startswith("[]"):
        R49_2_EMPTY_LABEL = CASES[-1].name
    else:
        R49_2_ARROW_LABEL = CASES[-1].name

acase("NEGATIVE", "(R49-2) and a genuinely blank `Deps` cell is still empty: the links arm is an "
                  "ADDITION to the shape rule, not a replacement -- `—` carries no edge and no link",
      build(d9z="**—**"), "UMBRELLA-CELL", 0)
R49_2_REAL_BLANK = CASES[-1].name


# -- R51 audit: the loop inside `_claims` was pinned ONLY by the ratchet's
# prefix aliasing -- truncating it left every control green.
# ⚠ THE FIRST FIXTURE FOR THIS DID NOT DISCRIMINATE AND THE MUTANT SURVIVED.
# It spelled the second phrase `` KIND `x` UNDETERMINED ``, whose blank renders
# as SPACES -- so `KIND   UNDETERMINED` still matches the STREAM, the phrase
# came from `_phrases` rather than from the disagreement loop, and truncating
# the loop changed nothing.  Both phrases must fail the stream for the loop to
# be the subject: `` KIND UNDETER`MINED` `` leaves `KIND UNDETER` behind, which
# matches nothing, while a reader sees `KIND UNDETERMINED`.
# (`memory/feedback_surviving-mutation-means-the-probe-has-another-subject.md`)
case("POSITIVE", "(R51) a blank-id row whose field straddles a masked span with TWO kind phrases "
                 "names BOTH: `_claims` marks every disagreement the canonical question returns, "
                 "not the first.  Truncated, the miss reads one kind and a reader deleting that one "
                 "believes the row is settled",
     build(i7z="**—**", s7z="**UMBRELLA, not a `terminal` unit.** and KIND UNDETER`MINED`",
           d7z="**9z**"), "", 1,
     measure=("schema", "spells a kind (marker/undetermined)"))
R51_TWO_STRADDLES = CASES[-1].name


# -- R51 audit: the R49-2 links arm changes `assertion_cd_seed` too, which the
# commit that added it did not say.  Pinned on THAT side so the reading is a
# decision and not a side effect.
case("NEGATIVE", "(R51) the links arm reaches the cd-seed as well: a row with ordering vocabulary in "
                 "its prose and a bare `[](sib.md)` in its `Deps` cell emits NO `ORDER-PROSE?` seed, "
                 "because the cell is not empty.  Measured 1 -> 0 when the arm landed, and unstated "
                 "until an audit asked.  ⚠ The alternative reading -- a label-less link names a FILE "
                 "and never says which ROW the ordering is against -- is recorded in "
                 "`deps_is_empty`'s docstring; this control is what makes flipping it a decision",
     build(s7z="Terminal.  This lands before the rewrite.", d7z="[](slice-9z-sib.md)"), "", 0,
     measure=("finding", "ORDER-PROSE?"), sibling="# sibling\n")
R51_CD_SEED_LINK = CASES[-1].name


# -- R52: the FOURTH site of the contradiction question, and the FIRST outside
# the census module -- which is exactly the hole the kind-question ratchet
# declared one commit earlier and did not cover.  A declared blind spot is a map
# of where the next finding lands, measured again.
_OUTSIDE = "| **7z** | Terminal. Terminal.  Acceptance: the probe must return 3. | `b.rs` |"
for _label, _mk, _want in (
        ("STRADDLING a masked construct", "**UMBRELLA, not a `terminal` unit.**", 1),
        ("spelled cleanly (the twin that was ALWAYS reported)", "**UMBRELLA, not a terminal unit.**", 1),
        ("quoted WHOLE (I-A: a quotation certifies nothing, and still does not)",
         "`UMBRELLA, not a terminal unit.`", 0)):
    acase("POSITIVE" if _want else "NEGATIVE",
          "(R52) a marker OUTSIDE the declaring field, %s: assertion (a)'s outside-field check "
          "searched the disposed STREAM alone, so the straddled spelling was invisible and the row "
          "left at rc 0 with only a non-gating seed while the clean spelling exited 1.  Asked "
          "through the canonical `_claims` now -- which keeps the quoted case by construction, "
          "since a phrase quoted whole is not a straddle" % _label,
          build().replace(_OUTSIDE, _OUTSIDE.replace("| `b.rs` |", "| " + _mk + " |")),
          "UMBRELLA-MARK", _want)
    if _want and "STRADDLING" in _label:
        R52_OUTSIDE_STRADDLE = CASES[-1].name
    elif not _want:
        R52_OUTSIDE_QUOTED = CASES[-1].name
