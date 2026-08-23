#!/usr/bin/env python3
"""Fixture builder and control registry for `plan-memo-umbrella-check.py --self-test`.

Every control is ONE record shape, `Case`: a fixture, a prose tail, an
optional sibling and extra files, a MEASURE and the EXACT value it must take.
Measures: `"sites"` (reported naming sites), `"rc"` (exit status),
`("finding", CODE)` (count of one finding code), `("note", TEXT)` (count of
report notes carrying TEXT), `("id", RID)` (1 if RID is declared, else 0),
`("schema", TEXT)` (count of SCHEMA findings carrying TEXT -- the one measure
that reads an rc-2 run, and requires rc 2 exactly when it counts one).
`case` / `acase` / `rcase` are spellings of the same record for the three
common measures.  The control kinds (POSITIVE / POSITIVE-NOVEL / NEGATIVE /
KNOWN-MISS) are defined in `plan_memo_umbrella_selftest.py`, which runs these
through the one factory `control`.  The fixture builder lives beside the cases
because the cases are its only parameterisation.
"""

from collections import namedtuple

Case = namedtuple("Case", "kind name text prose sibling files measure expect")


# A minimal memo carrying every table schema, so the fixtures exercise the real
# parse rather than a hand-built cell list.  `{stub}` is the stub table (a
# control drops it to prove a missing schema is rc 2); `{extra}` is a slot for
# a control's own table, placed before §5 so it sits inside the tables.
HEADER = """# fixture

## §0.5 Spec citation table

| ID | Citation | Anchor | Used by |
|---|---|---|---|
| [C1] | ECMA-262 §1 X | `#a` | {c1} |

{stub}

{extra}

## §5. Slice plan

| # | Slice | Primary module(s) | Slot | Tier | Deps |
|---|---|---|---|---|---|
| **9z** | **UMBRELLA, not a terminal unit.** {s9z} | `a.rs` | — | T1 | {d9z} |
| {i7z} | Terminal. {s7z} | `b.rs` | — | T1 | {d7z} |
| **9** | **UMBRELLA, not a terminal unit.** numeric id. | `c.rs` | — | T1 | — |
| **C** | **UMBRELLA, not a terminal unit.** single-letter id. | `d.rs` | — | T1 | — |
| **Qx** | {sqx} | `e.rs` | — | T1 | — |
| **Uz** | {suz} | `f.rs` | — | T1 | {duz} |

## §8. Slot ledger changes at landing

| Slot | Why deferred | Trigger | Re-eval |
|---|---|---|---|
| `#11-zz-alpha` | **UMBRELLA, not a terminal unit.** {wa} | {ta} | 2026-12-31 |
| `#11-zz-beta` | Terminal. {wb} | {tb} | 2026-12-31 |
"""

# The two terminal rows carry acceptance vocabulary in the BASE fixture, so a
# control that varies ONE row's cell measures that row.  Without it the
# accept-vocab negative control counted the other terminal row and could never
# reach 0 -- a control that cannot go green tests nothing.
STUB = """## §4 Stubs

| Site | Syntax | Emits | Observable | Tier | Slice |
|---|---|---|---|---|---|
| `x.rs` | `a` | nothing | none | T1 | 7z |"""

BLANK = dict(stub=STUB, extra="", c1="—", s9z="charter.", d9z="—", i7z="**7z**",
             s7z="Terminal.  Acceptance: the probe must return 3.", d7z="—",
             sqx="Terminal.  Acceptance: the probe must return 4.",
             suz="Terminal.  Acceptance: the probe must return 5.", duz="—",
             wa="why.", ta="now", wb="why.", tb="now")


def build(**kw):
    f = dict(BLANK)
    f.update(kw)
    return HEADER.format(**f)

CASES = []


def case(kind, name, text, prose, expect, sibling=None, files=None, measure="sites"):
    CASES.append(Case(kind, name, text, prose, sibling, files or {}, measure, expect))


def acase(kind, name, text, code, expect, prose="", sibling=None):
    case(kind, name, text, prose, expect, sibling, measure=("finding", code))


def rcase(kind, name, text, prose, rc, sibling=None, files=None):
    case(kind, name, text, prose, rc, sibling, files, measure="rc")


# ---------------------------------------------------------------- POSITIVE --
case("POSITIVE", "used-by cell names an umbrella outright",
     build(c1="Slice 9z (the close rule)"), "", 1)
case("POSITIVE", "Deps cell of a terminal row names an umbrella",
     build(d7z="**9z**"), "", 1)
case("POSITIVE", "trigger cell names another row's umbrella slot as a co-occasion",
     build(tb="now, or with `#11-zz-alpha`"), "", 1)
case("POSITIVE-NOVEL", "prose names a slot umbrella bare",
     build(), "Slot #11-zz-alpha lands before Slice 7z.", 1)
case("POSITIVE-NOVEL", "prose names a slot umbrella in bold",
     build(), "**#11-zz-alpha** owns the close rule outright.", 1)
case("POSITIVE", "prose orders an umbrella",
     build(), "Slice 9z lands before Slice 7z.", 1)
case("POSITIVE", "prose names an umbrella bare, with no row noun",
     build(), "9z owns the close rule outright.", 1)

# ---------------------------------------------------------- POSITIVE-NOVEL --
# None of these spellings occurs in the memo this checker was written against.
# They must be reported because the rule is stated as a complement -- an id is
# licensed only when attached to a derivation, to children, to a charter, to a
# memo or to a split -- and not as a list of bad phrasings.
case("POSITIVE-NOVEL", "novel verb: an umbrella is 'chartered to deliver'",
     build(), "9z is chartered to deliver the close rule by Q3.", 1)
case("POSITIVE-NOVEL", "novel role noun: an umbrella as 'the integrator'",
     build(), "The integrator for this artifact is 9z.", 1)
case("POSITIVE-NOVEL", "novel ordering idiom: 'downstream of'",
     build(), "Everything here sits downstream of 9z.", 1)
case("POSITIVE-NOVEL", "novel acceptance idiom: 'signs off'",
     build(), "9z signs off the observable once the probe is green.", 1)
case("POSITIVE-NOVEL", "novel spelling in a cell: an em-dash hand-off",
     build(c1="9z — hand-off"), "", 1)

# ---------------------------------------------------------------- NEGATIVE --
case("NEGATIVE", "the child-of construction",
     build(c1="the child of umbrella 9z that owns the close rule, via that "
             "umbrella's derivation, which mints it"), "", 0)
case("NEGATIVE", "the derivation-mints construction",
     build(c1="the child umbrella **9z**'s derivation mints for the close rule"), "", 0)
case("NEGATIVE", "possessor of children",
     build(), "umbrella 9z's children carry the obligation.", 0)
case("NEGATIVE", "possessor of a charter",
     build(), "The surface sits inside umbrella 9z's charter.", 0)
case("NEGATIVE", "statement about the row's kind",
     build(), "Slice 9z is an umbrella, so it ships no PR.", 0)
case("NEGATIVE", "a self-declaring row that MENTIONS a sibling stays in the population",
     build(suz="Unlike Slice 7z, **UMBRELLA, not a terminal unit.** charter."),
     "Uz owns the close rule.", 1)
case("NEGATIVE", "a row naming itself in its own cell",
     build(s9z="charter; 9z mints its children here."), "", 0)
case("POSITIVE", "a multi-character id inside a bold PHRASE, not bold itself",
     build(c1="**block-scope entry 9z**"), "", 1)
case("POSITIVE", "a backticked run of ids and separators is a Deps-shaped edge, not code",
     build(), "The same thing happened to `9z / 7z`, one layer down.", 1)
case("NEGATIVE", "a bold sentence opener is not row A",
     build(), "⚠ **A call at the finalizer sites is not the fix.**", 0)
case("POSITIVE", "a backticked BARE id is the document spelling an id, not code",
     build(), "The obligation is `9z`'s, and naming `9z` there names nobody.", 1)
case("NEGATIVE", "an id-looking token inside inline code",
     build(), "The probe reads `Reflect.construct(9z, [], D)` and stops.", 0)
case("POSITIVE", "a visible link LABEL is prose and is scanned",
     build(), "See [Slice 9z lands first](slice-9z-sib.md) for the walk.", 1)
case("NEGATIVE", "an id-looking token inside a file name",
     build(), "See [detail](slice-9z-sib.md) for the walk.", 0)
case("POSITIVE-NOVEL", "umbrella id is the LAST token of a link label",
     build(), "See [Slice 9z](slice-9z-sib.md) for the walk.", 1)
# The sibling population is discovered from the memo's own links, not passed
# by the caller: the violation below lives ONLY in the linked file.
case("POSITIVE-NOVEL", "a violation in a carved sibling the memo links",
     build(), "See [the walk](slice-9z-sib.md).", 1,
     sibling="Slice 9z lands before Slice 7z.")
case("POSITIVE-NOVEL", "the sibling link carries a section fragment",
     build(), "See [the walk](slice-9z-sib.md#acceptance).", 1,
     sibling="Slice 9z lands before Slice 7z.")
case("POSITIVE-NOVEL", "the sibling link carries a title",
     build(), 'See [the walk](slice-9z-sib.md "Acceptance cases").', 1,
     sibling="Slice 9z lands before Slice 7z.")
case("POSITIVE-NOVEL", "the sibling link uses an angle-bracket destination",
     build(), "See [the walk](<slice-9z-sib.md>).", 1,
     sibling="Slice 9z lands before Slice 7z.")
case("POSITIVE-NOVEL", "the sibling is linked by reference",
     build(), "See [the walk][sib].\n\n[sib]: slice-9z-sib.md", 1,
     sibling="Slice 9z lands before Slice 7z.")
case("POSITIVE-NOVEL", "the sibling is linked by a shortcut reference",
     build(), "See [the walk].\n\n[the walk]: slice-9z-sib.md", 1,
     sibling="Slice 9z lands before Slice 7z.")
case("POSITIVE-NOVEL", "the reference definition has an angle-bracket destination",
     build(), "See [the walk][sib].\n\n[sib]: <slice-9z-sib.md>", 1,
     sibling="Slice 9z lands before Slice 7z.")
case("NEGATIVE", "an id in a titled link's destination is not a naming site",
     build(), 'See [detail](slice-9z-sib.md "the 9z walk") for the walk.', 0)

# -------------------------------------------------------------- KNOWN-MISS --
# These are wrong sites.  The checker does not report them, and that is the
# declared boundary of the seed, not a pass.
case("KNOWN-MISS", "purely numeric id with no row noun",
     build(), "Everything here lands after 9 and before 10.", 0)
case("KNOWN-MISS", "undecorated single letter with no row noun",
     build(), "The obligation is C's, and C integrates it.", 0)


# ---------------------------------------------------- assertion controls ----

acase("POSITIVE", "(a-seed) a row declaring the kind in words carries no marker",
      build(sqx="This row is an umbrella: three intersecting axes."),
      "UMBRELLA-MARK?", 1)
acase("POSITIVE", "(a-seed) fires on a cell QUOTING the criterion too -- a "
                  "measured false positive, kept visible rather than filtered",
      build(sqx="The criterion asks for a subsystem with no canonical algorithm; "
                "this row touches none, so it is terminal."),
      "UMBRELLA-MARK?", 1)
acase("POSITIVE", "(a) the marker outside the declaring field certifies nothing",
      build(sqx="body.", d7z="**UMBRELLA, not a terminal unit** stray"),
      "UMBRELLA-MARK", 1)
acase("POSITIVE", "(a) a marker that names ANOTHER row is not a self-declaration",
      build(wb="**(carved at PR-B)** Slice **9z** — **UMBRELLA, not a terminal unit** — "
               "with sub-slices; this slot points into §5."),
      "UMBRELLA-MARK", 1)
case("POSITIVE", "a KIND-UNDETERMINED row named as an owner is reported",
     build(suz="**KIND UNDETERMINED**: neither an umbrella nor a terminal unit."),
     "The close rule is owned by Slice **Uz**.", 1)
acase("POSITIVE", "(d) one clause assigning a deliverable to two owners",
      build(s9z="charter.  The drain is owned by **7z** and **Qx**."),
      "TWO-OWNERS?", 1)
acase("NEGATIVE", "(d) one owner is not two",
      build(s9z="charter.  The drain is owned by **7z**."), "TWO-OWNERS?", 0)
acase("POSITIVE", "(b) a KIND-UNDETERMINED row carrying a Deps edge is checked too",
      build(suz="**KIND UNDETERMINED**: neither an umbrella nor a terminal unit.", duz="**7z**"),
      "UMBRELLA-CELL", 1)
acase("POSITIVE", "(b) an umbrella row carrying a Deps edge",
      build(d9z="**7z**"), "UMBRELLA-CELL", 1)
acase("NEGATIVE", "(accept-vocab seed) a POINTER row is excluded from the population",
      build(sqx="This row is a pointer rather than a slice; the work is scheduled from its slot."),
      "ACCEPT-VOCAB?", 0)
acase("POSITIVE", "(c-seed) ordering vocabulary in prose against an empty Deps cell",
      build(s7z="Terminal.  This row lands before 9z and is a prerequisite of it.",
            d7z="—"),
      "ORDER-PROSE?", 1)
acase("NEGATIVE", "(c-seed) a Deps cell that CARRIES the prose's party is not reported",
      build(s7z="Terminal.  This row lands before Slice **Qx** and is a prerequisite of it.",
            d7z="**Qx**"),
      "ORDER-PROSE?", 0)
acase("POSITIVE", "(c-seed) prose names a SECOND party the non-empty Deps cell omits",
      build(s7z="Terminal.  Lands second behind Slice **Qx** and behind Slice **9z**.",
            d7z="**Qx**"),
      "ORDER-PROSE?", 1)
acase("POSITIVE", "(accept-vocab seed) a terminal row with neither `must` nor `acceptance`",
      build(sqx="Terminal.  Lowers the thing."), "ACCEPT-VOCAB?", 1)
acase("NEGATIVE", "(accept-vocab seed) a terminal row stating an acceptance condition",
      build(sqx="Terminal.  Acceptance: the probe must return 3."), "ACCEPT-VOCAB?", 0)
acase("NEGATIVE", "(b) an umbrella row with an empty Deps cell",
      build(), "UMBRELLA-CELL", 0)


# ------------------------------------------------ Slice 1 lexing controls --
# One control per clause of the plan's §3 rows and §2 invariants I-A/B/C/F.
# Every control here has a named mutant in `plan_memo_selftest_mutants.py`.

LINK = "See [the walk](slice-9z-sib.md)."
VIOLATION = "Slice 9z lands before Slice 7z."

# -- I-A fenced blocks (CommonMark §4.5)
case("NEGATIVE", "(fence) text inside a fenced block is not prose",
     build(), "```\n" + VIOLATION + "\n```", 0)
case("NEGATIVE", "(fence) a tilde fence masks too",
     build(), "~~~\n" + VIOLATION + "\n~~~", 0)
case("NEGATIVE", "(fence) an unclosed fence runs to the end of the document",
     build(), "```\n" + VIOLATION, 0)
case("NEGATIVE", "(fence) a closer of the OTHER character does not close",
     build(), "```\nx\n~~~\n" + VIOLATION, 0)
case("NEGATIVE", "(fence) a shorter closer does not close",
     build(), "````\nx\n```\n" + VIOLATION + "\n````", 0)
case("NEGATIVE", "(fence) a closer followed by text does not close",
     build(), "```\nx\n``` y\n" + VIOLATION, 0)
case("POSITIVE", "(fence) a backtick fence whose info string holds a backtick is not a fence",
     build(), "``` a`b\n" + VIOLATION, 1)
case("POSITIVE", "(fence) four spaces of indent is not a fence",
     build(), "    ```\n" + VIOLATION, 1)
case("NEGATIVE", "(fence) three spaces of indent is",
     build(), "   ```\n" + VIOLATION + "\n```", 0)
case("NEGATIVE", "(fence) a link inside a fence is not a link (A x B)",
     build(), "~~~\n[the walk](absent-file.md)\n~~~", 0)

# -- I-A code spans (CommonMark §6.1) over the block, not the line
case("NEGATIVE", "(span) a code span may cross a line ending",
     build(), "The chain `0a → then\n9z` was withdrawn.", 0)
case("NEGATIVE", "(span) backtick strings pair by EQUAL length",
     build(), "The run ``a ` 9z`` owns nothing.", 0)
case("POSITIVE", "(span) an unmatched backtick string is literal, not a mask to end of line",
     build(), "A lone ` backtick, then 9z owns it.", 1)
case("POSITIVE", "(span) a paragraph ends at a list item: a backtick open in one item "
                 "and closed in the next is literal",
     build(), "- a `9z owns it\n- b` end", 1)
case("POSITIVE", "(span) a paragraph ends at a blank line",
     build(), "open `here\n\n9z owns it` there", 1)
case("POSITIVE", "(span) a paragraph ends at an ATX heading",
     build(), "open `here\n## 9z owns it` there", 1)
case("POSITIVE", "(span) a paragraph ends at a `>` line",
     build(), "open `here\n> 9z owns it` there", 1)
case("POSITIVE", "(span) a paragraph ends at a table row",
     build(), "open `here\n| a | b |\n|---|---|\n| 9z owns it` | x |", 1)
case("NEGATIVE", "(span) a quoted kind marker is not a declaration (A x E)",
     build(sqx="Terminal.  A row quoting `UMBRELLA, not a terminal unit` as the criterion; "
               "the probe must return 4."),
     "Qx owns the close rule.", 0)

# -- I-C row split (GFM §4.10)
case("POSITIVE", "(row) an unescaped `|` inside backticks SPLITS the row: each half is "
                 "prose with a literal backtick, so the id on each side is a mention",
     build(), "| a | b |\n|---|---|\n| `9z owns | 9z owns` |", 2)
case("POSITIVE", "(row) `\\|` is `|` in the cell, so `9z \\| 7z` is an id-only run",
     build(c1=r"`9z \| 7z`"), "", 1)
case("POSITIVE", "(row) a table without leading and trailing pipes declares its rows",
     build(extra="Slot | Why deferred | Trigger | Re-eval\n"
                 "---|---|---|---\n"
                 "`#11-zz-gamma` | **UMBRELLA, not a terminal unit.** carved. | now | 2026-12-31"),
     "`#11-zz-gamma` owns the close rule.", 1)
case("NEGATIVE", "(table) header and delimiter of unequal width are not a table",
     build(), "| # | Slice | Primary module(s) | Slot | Tier | Deps |\n|---|\n"
              "| **Wz** | **UMBRELLA, not a terminal unit.** x | `w.rs` | — | T1 | — |\n\n"
              "Wz owns the close rule.", 0)
case("NEGATIVE", "(table) a delimiter cell is >=1 hyphen with optional colons; `:` alone is not",
     build(), "| # | Slice | Primary module(s) | Slot | Tier | Deps |\n|---|---|---|---|---|:|\n"
              "| **Wz** | **UMBRELLA, not a terminal unit.** x | `w.rs` | — | T1 | — |\n\n"
              "Wz owns the close rule.", 0)

# -- I-B links (CommonMark §6.3 / §4.7): every form reaches the sibling
SIB = dict(sibling=VIOLATION)
case("POSITIVE-NOVEL", "(link) bare destination with a balanced parenthesis pair",
     build(), "See [the walk](slice-9z-sib.md#(a)).", 1, **SIB)
case("POSITIVE-NOVEL", "(link) bare destination with an escaped parenthesis",
     build(), "See [the walk](slice-9z-sib.md#\\(a).", 1, **SIB)
case("POSITIVE-NOVEL", "(link) a non-ASCII space is destination content (only ASCII space "
                       "and controls end it)",
     build(), "See [the walk](slice-9z-sib.md#\u00a0x).", 1, **SIB)
case("NEGATIVE", "(link) an ASCII control character (0x7F) ends a bare destination",
     build(), "See [the walk](slice-9z-sib.md#\x7fx).", 0, **SIB)
case("NEGATIVE", "(link) `<dest>` may not contain an unescaped `<`",
     build(), "See [the walk](<slice-9z-sib.md#<x>).", 0, **SIB)
case("POSITIVE-NOVEL", "(link) `<dest>` may contain an escaped `>`",
     build(), "See [the walk](<slice-9z-sib.md#\\>x>).", 1, **SIB)
case("NEGATIVE", "(link) a `(…)` title may not contain an unescaped `(`",
     build(), "See [the walk](slice-9z-sib.md (a(b)).", 0, **SIB)
case("POSITIVE-NOVEL", "(link) a `\"…\"` title may contain an escaped `\"`",
     build(), 'See [the walk](slice-9z-sib.md "a\\"b").', 1, **SIB)
case("POSITIVE-NOVEL", "(link) collapsed reference `[label][]`",
     build(), "See [the walk][].\n\n[the walk]: slice-9z-sib.md", 1, **SIB)
case("POSITIVE-NOVEL", "(link) label matching collapses internal whitespace",
     build(), "See [the   walk][].\n\n[the walk]: slice-9z-sib.md", 1, **SIB)
case("POSITIVE-NOVEL", "(link) label matching is a Unicode case FOLD, not lower()",
     build(), "See [the straße][].\n\n[THE STRASSE]: slice-9z-sib.md", 1, **SIB)
case("NEGATIVE", "(link) `[label][undefined]` is neither a full reference nor a shortcut "
                 "(§6.3 Example 571: a shortcut is not followed by a link label) -- the "
                 "unanswered label is a schema miss, not a link to the sibling",
     build(), "See [the walk][nope].\n\n[the walk]: slice-9z-sib.md", 1, **SIB,
     measure=("schema", "unresolved reference 'nope'"))
case("POSITIVE-NOVEL", "(link) full reference whose text holds nested brackets; the label "
                       "is the link's tail, not prose, and so is the definition",
     build(), "See [the [x] walk][9z].\n\n[9z]: slice-9z-sib.md", 1, **SIB)
case("POSITIVE-NOVEL", "(def) the FIRST definition of a label wins",
     build(), "See [the walk][sib].\n\n[sib]: slice-9z-sib.md\n[sib]: clean.md", 1,
     sibling=VIOLATION, files={"clean.md": "nothing here.\n"})
case("POSITIVE-NOVEL", "(def) one line ending is allowed before the destination",
     build(), "See [the walk][sib].\n\n[sib]:\nslice-9z-sib.md", 1, **SIB)
case("NEGATIVE", "(def) text after the destination is not a definition, so the reference "
                 "is unanswered: a schema miss",
     build(), "See [the walk][sib].\n\n[sib]: slice-9z-sib.md junk", 1, **SIB,
     measure=("schema", "unresolved reference 'sib'"))
case("NEGATIVE", "(def) a definition cannot interrupt a paragraph: the reference is "
                 "unanswered, and reported ONCE (`[text][label]` re-scans `[label]`)",
     build(), "See [the walk][sib].\n\ntext\n[sib]: slice-9z-sib.md", 1, **SIB,
     measure=("schema", "unresolved reference 'sib'"))
case("NEGATIVE", "(link) a link inside a code span is not a link (A x B)",
     build(), "The memo says `[the walk](absent-file.md)` and stops.", 0)

# -- I-F one population: sibling-declared rows join census, keep-set, assertions
SIB_TABLE = ("## §5 carved\n\n| # | Slice | Primary module(s) | Slot | Tier | Deps |\n"
             "|---|---|---|---|---|---|\n"
             "| **Wz** | **UMBRELLA, not a terminal unit.** carved. | `w.rs` | — | T1 | %s |\n"
             "| **Tq** | Terminal.  Acceptance: the probe must return 6. | `t.rs` | — | T1 | — |\n")
case("POSITIVE-NOVEL", "(population) an umbrella declared in a linked memo is in the census",
     build(), LINK + "\n\nWz owns the close rule.", 1, sibling=SIB_TABLE % "—")
case("POSITIVE-NOVEL", "(population) a terminal id declared in a linked memo is in the "
                       "keep-set, so `Tq / 9z` is an id-only run, not code",
     build(), LINK + "\n\nThe same thing happened to `Tq / 9z`.", 1, sibling=SIB_TABLE % "—")
case("POSITIVE-NOVEL", "(population) the population is transitive: a memo linked from a "
                       "linked memo is scanned",
     build(), LINK, 1, sibling="See [further](far.md).", files={"far.md": VIOLATION + "\n"})
acase("POSITIVE-NOVEL", "(b) a sibling umbrella's Deps edge is asserted",
      build(), "UMBRELLA-CELL", 1, prose=LINK, sibling=SIB_TABLE % "**7z**")


# ------------------------------------------------------ exit-status controls --

rcase("POSITIVE", "(rc) a linked memo that is not on disk is rc 2, never clean",
      build(), "See [gone](absent-file.md).", 2)
rcase("POSITIVE", "(rc) a schema with no matching table is rc 2",
      build(stub=""), "", 2)
rcase("POSITIVE", "(rc) a schema body row whose width differs from its header is rc 2",
      build(extra="| # | Slice | Primary module(s) | Slot | Tier | Deps |\n"
                  "|---|---|---|---|---|---|\n"
                  "| **Wz** | **UMBRELLA, not a terminal unit.** | `w.rs` | — | T1 | — | extra |"),
      "", 2)
rcase("POSITIVE", "(rc) the same id declared in two memos is rc 2",
      build(), LINK, 2, sibling=SIB_TABLE.replace("**Wz**", "**9z**") % "—")
rcase("POSITIVE", "(rc) the undetermined kind written two ways is KIND-SPELLING, rc 1",
      build(sqx="**KIND UNDETERMINED**: open.", suz="**KIND-UNDETERMINED**: open."), "", 1)
rcase("NEGATIVE", "(rc) a link cycle (A -> B -> A) is walked once and is not an error",
      build(), LINK, 0, sibling="See [back](fixture.md).")
rcase("NEGATIVE", "(rc) a code-quoted link to an absent file is not a link: rc 0",
      build(), "The memo says `[the walk](absent-file.md)` and stops.", 0)
rcase("NEGATIVE", "(rc) a slice header over a one-cell delimiter row is not a table, "
                  "so its wide body row is not a width miss",
      build(), "| # | Slice | Primary module(s) | Slot | Tier | Deps |\n|---|\n"
               "| **Wz** | **UMBRELLA, not a terminal unit.** x | `w.rs` | — | T1 | — |", 0)


# ------------------------------------------------ /code-review high controls --
# One control (and, in `plan_memo_selftest_mutants.py`, one mutant) per fix.

# F1 / F8: the id cell reads the one decorated-id grammar at its START
case("POSITIVE", "(id) an id cell with trailing prose declares the id at its start",
     build(i7z="**7z** — MERGED"), "", 1, measure=("id", "7z"))
case("POSITIVE", "(id) a backticked slug with trailing prose declares the slug",
     build(tb="now", extra="| Slot | Why deferred | Trigger | Re-eval |\n|---|---|---|---|\n"
                           "| `#11-zz-gamma` (carved from #483) | Terminal. Acceptance: must. | now | x |"),
     "", 1, measure=("id", "#11-zz-gamma"))
INTL = "`Intl` → owned externally by [[intl-icu-deferral]] (**no slot minted here**)"
case("POSITIVE", "(id) the `Intl`-shaped cell declares the 4-character id at its start",
     build(i7z=INTL), "", 1, measure=("id", "Intl"))
case("NEGATIVE", "(id) the `Intl`-shaped cell never mints the whole cell as an id",
     build(i7z=INTL), "", 0, measure=("id", INTL))
case("POSITIVE", "(id) a cell that does not start with an id declares nothing: the row is "
                 "unkeyed (its Deps edge would go unasserted), so the run is a schema miss",
     build(i7z="(none)"), "", 1, measure=("schema", "id cell does not start with an id"))
case("NEGATIVE", "(id) an id cell `-` is empty: not declared, not a schema miss",
     build(i7z="-"), "", 0, measure=("schema", "id cell does not start with an id"))
case("NEGATIVE", "(id) an id cell `–` (en dash) is empty by shape",
     build(i7z="\u2013"), "", 0, measure=("schema", "id cell does not start with an id"))
case("NEGATIVE", "(id) an id cell `-` is not an id",
     build(i7z="-"), "", 0, measure=("id", "-"))
acase("POSITIVE", "(c-seed) a Deps cell `n/a` is empty, so ordering prose is reported",
      build(s7z="Terminal.  This row lands first; the probe must return 3.", d7z="n/a"),
      "ORDER-PROSE?", 1)
acase("POSITIVE", "(c-seed) a Deps cell `N/A` is empty (the lexical exception is case-insensitive)",
      build(s7z="Terminal.  This row lands first; the probe must return 3.", d7z="N/A"),
      "ORDER-PROSE?", 1)
acase("POSITIVE", "(c-seed) a Deps cell `–` (en dash) is empty by shape: no alphanumeric",
      build(s7z="Terminal.  This row lands first; the probe must return 3.", d7z="\u2013"),
      "ORDER-PROSE?", 1)
acase("POSITIVE", "(c-seed) a Deps cell `--` is empty by shape",
      build(s7z="Terminal.  This row lands first; the probe must return 3.", d7z="--"),
      "ORDER-PROSE?", 1)
acase("POSITIVE", "(b) a Deps cell `nil` -- a word outside the lexical exceptions -- is NOT "
                  "empty: the stated polarity is a reported edge (false rc 1), never a silent skip",
      build(d9z="nil"), "UMBRELLA-CELL", 1)

# F2 / F3: the population is read from every block, and only from local paths
rcase("POSITIVE", "(rc) a link to an absent memo inside a table CELL is rc 2",
      build(c1="[gone](absent-file.md)"), "", 2)
case("POSITIVE-NOVEL", "(population) a violation in a sibling linked ONLY from a cell is reported",
     build(c1="[the walk](slice-9z-sib.md)"), "", 1, **SIB)
rcase("NEGATIVE", "(rc) an absolute URL ending in `.md` is not a sibling on disk: rc 0",
      build(), "See [the spec](https://example.org/notes/spec.md).", 0)
rcase("NEGATIVE", "(rc) a protocol-relative `//host/x.md` is not a sibling on disk: rc 0",
      build(), "See [the spec](//example.org/spec.md).", 0)

# F4: the FIRST marker occurrence decides attribution
acase("NEGATIVE", "(a) a self-declaring field that later says a sibling 'is not it' stays "
                  "self-declaring",
      build(wb="**UMBRELLA, not a terminal unit** charter. Unlike Slice **7z** — "
               "**UMBRELLA, not a terminal unit** is not it."),
      "UMBRELLA-MARK", 0)

# F5: the undetermined spelling is collected even beside the marker
rcase("POSITIVE", "(rc) a row carrying the marker AND one undetermined spelling, beside another "
                  "row's other spelling, is KIND-SPELLING rc 1",
      build(s9z="charter. **KIND-UNDETERMINED** for its second half.",
            sqx="**KIND UNDETERMINED**: open."), "", 1)

# F6: the Deps cell's ids are the population's own reading of it
acase("POSITIVE", "(c-seed) a Deps cell naming a FILE whose name holds the id does not carry the id",
      build(s7z="Terminal.  This row lands after Slice **9z**; the probe must return 3.",
            d7z="see slice-9z-sib.md"),
      "ORDER-PROSE?", 1)
acase("POSITIVE", "(c-seed) a Deps cell `xxxxC` does not carry the id `C`",
      build(s7z="Terminal.  This row lands after Slice **C**; the probe must return 3.",
            d7z="xxxxC"),
      "ORDER-PROSE?", 1)

# F7: an all-terminal memo is clean
ALL_TERMINAL = build().replace("**UMBRELLA, not a terminal unit.**", "Terminal. Acceptance: must.")
rcase("NEGATIVE", "(rc) a memo whose every row is terminal is rc 0, not a schema miss",
      ALL_TERMINAL, "", 0)
case("NEGATIVE", "(rc) an all-terminal memo reports a zero census",
     ALL_TERMINAL, "", 1, measure=("note", "[CENSUS] 0 no-owner"))

# F9: a backslash-escaped backtick is literal (CommonMark §2.4 / §6.1)
case("POSITIVE", "(span) a backtick behind a backslash is literal and opens no span",
     build(), "Slice 9z owns \\`x` and then `Slice 9z` lands first", 2)

# F10: a table cell holds no reference definition (GFM §4.10: inline content)
case("POSITIVE", "(def) a cell shaped like a definition is inline content and is scanned",
     build(c1='[note]: /x "Slice 9z owns the close rule"'), "", 1)

# F12: declared bounds of the slot pass
case("NEGATIVE", "(fence) a `#11-` slug inside a fenced block is not a naming site",
     build(), "```\n#11-zz-alpha owns the close rule.\n```", 0)
case("NEGATIVE", "(link) a `#11-` slug in a link DESTINATION is not a naming site",
     build(), "See [the slot](#11-zz-alpha) for the close rule.", 0)

# F13: a reference no definition answers is reported, not silently dropped
# F13 / Stage 6 #1: a reference no definition answers means the memo it meant
# to link is outside the population -- §1 "could not scan" is never clean
case("POSITIVE", "(link) a full reference no definition answers is a schema miss",
     build(), "See [the walk][sib].", 1,
     measure=("schema", "unresolved reference 'sib'"))
rcase("POSITIVE", "(rc) a full reference no definition answers is rc 2, never clean",
      build(), "See [the walk][sib].", 2)
case("POSITIVE", "(link) a shortcut whose only definition sits mid-paragraph is a schema miss",
     build(), "See [the walk].\n\ntext\n[the walk]: slice-9z-sib.md", 1,
     measure=("schema", "unresolved reference 'the walk'"))
case("NEGATIVE", "(link) a `[C19]` citation is a shortcut with no definition anywhere: no miss",
     build(), "Per [C1] the probe must return 3.", 0,
     measure=("schema", "unresolved reference"))

# C8: slug disposition is the disposition step's, not the scanner's
case("POSITIVE", "(span) a kept slug inside a command-line code span is a naming site",
     build(tb="now, after `plan-check --slot #11-zz-alpha` is green"), "", 1)

# C7: the population decides the pointer kind
acase("NEGATIVE", "(accept-vocab seed) a row whose marker is ATTRIBUTED to another row is a pointer "
                  "and owes no acceptance condition",
      build(sqx="Slice **9z** — **UMBRELLA, not a terminal unit** — points there."),
      "ACCEPT-VOCAB?", 0)


# ------------------------------------------- /elidex-review Stage 6 controls --

# #3: a bare id is bounded by the COMPLEMENT of the id-continuation class, not
# by a punctuation list (`?` / `!` / `"` / `“ ”` were unreported under the list)
case("POSITIVE-NOVEL", "(bare) an id before `?` is bounded",
     build(), "Who integrates it -- 9z?", 1)
case("POSITIVE-NOVEL", "(bare) an id before `!` is bounded",
     build(), "The integrator is 9z!", 1)
case("POSITIVE-NOVEL", "(bare) an id inside ASCII double quotes is bounded",
     build(), 'The integrator is "9z" here.', 1)
case("POSITIVE-NOVEL", "(bare) an id inside curly double quotes is bounded",
     build(), "The integrator is \u201c9z\u201d here.", 1)
case("POSITIVE-NOVEL", "(bare) a hyphen bounds a short id: `after 9z-7z` names 9z",
     build(), "Everything lands after 9z-7z here.", 1)
case("NEGATIVE", "(bare) a bare `.md` file name holding an id is a file token, not a site",
     build(), "Read slice-9z-sib.md for the walk.", 0)
case("NEGATIVE", "(bare) a dotted number is one token: `§6.9z` names no row",
     build(), "See §6.9z for the integrator.", 0)
case("POSITIVE", "(bare) a full stop after an id is a boundary",
     build(), "The integrator is 9z.", 1)
case("POSITIVE", "(bare) a decorated side is bounded by its decoration: `` `9z`-`7z` `` is two ids",
     build(), "Sub-slices are spelled `9z`-`7z` here.", 1)
case("POSITIVE", "(bare) the decoration closes the token even against an id character: `**9z**7z`",
     build(), "The pair **9z**7z is spelled oddly.", 1)

# #8: every predicate reads the block's ONE disposed stream
acase("NEGATIVE", "(c-seed) ordering vocabulary inside a code span is code, not prose",
      build(s7z="Terminal.  The probe must return 3.  See `x gates y`.", d7z="—"),
      "ORDER-PROSE?", 0)
acase("POSITIVE", "(accept-vocab seed) a retirement word inside a code span does not retire the row",
      build(s7z="Terminal.  No accept words.  Run `grep MERGED`."), "ACCEPT-VOCAB?", 1)
acase("NEGATIVE", "(d) an ownership clause inside a code span is code, not a two-owner claim",
      build(s9z="charter.  The probe prints `the drain is owned by **7z** and **Qx**, twice`."),
      "TWO-OWNERS?", 0)
case("POSITIVE", "(licence) a licensing phrase inside a code span licenses nothing",
     build(), "Everything here sits with 9z`'s charter` spelled as code.", 1)

# #11: the seeds' row maps are keyed on the memo's resolved path, not its basename
_LINE_7Z = next(i for i, l in enumerate(build().split("\n"), 1) if l.startswith("| **7z**"))
SAME_NAME_SIB = ("\n" * (_LINE_7Z - 3)       # the Rz row lands on the main memo's 7z line
                 + "| # | Slice | Primary module(s) | Slot | Tier | Deps |\n"
                 + "|---|---|---|---|---|---|\n"
                 + "| **Rz** | Terminal.  Acceptance: must. | `r.rs` | — | T1 | **9z** |\n")
acase("POSITIVE", "(c-seed) a sibling of the SAME basename in another directory, whose Deps cell "
                  "at the same line names the party, does not discharge the main memo's row",
      build(s7z="Terminal.  Lands second behind Slice **Qx** and behind Slice **9z**.",
            d7z="**Qx**"),
      "ORDER-PROSE?", 1, prose="See [the twin](sub/fixture.md).")
CASES[-1] = CASES[-1]._replace(files={"sub/fixture.md": SAME_NAME_SIB})

# #15: §6.3 label classes are space / tab / line ending, not Unicode whitespace
case("POSITIVE-NOVEL", "(link) a label holding only a non-breaking space is a label (§6.3: "
                       "at least one character that is not a space, tab, or line ending)",
     build(), "See [\u00a0][].\n\n[\u00a0]: slice-9z-sib.md", 1, **SIB)
case("NEGATIVE", "(link) label matching collapses spaces, tabs and line endings only: a "
                 "non-breaking space is not a space, so the reference is unanswered",
     build(), "See [the\u00a0walk][].\n\n[the walk]: slice-9z-sib.md", 1, **SIB,
     measure=("schema", "unresolved reference"))
acase("NEGATIVE", "(stream) ordering vocabulary in a link TITLE is the link's tail, not prose",
      build(s7z='Terminal.  The probe must return 3.  See [the walk](slice-9z-sib.md "lands before").',
            d7z="—"),
      "ORDER-PROSE?", 0)
