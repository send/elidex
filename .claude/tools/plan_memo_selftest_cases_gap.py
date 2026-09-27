#!/usr/bin/env python3
"""The READER-GAP controls: every fixture whose subject is a gap a reader sees
between two words -- CommonMark 0.31.2 §2.1's "Unicode whitespace character" as
`plan_memo_emphasis.is_unicode_whitespace` defines it and `plan_memo_stream.GAP`
/ `plan_memo_stream.phrase` compose it -- and the two whitespace notions that
are deliberately NOT it (the file-name boundary, Python's `isspace()` on
purpose) or that read it through §6.2 (emphasis flanking).

CARVED ON A SUBJECT, not a review round (the shape `_cases_sibling.py` set),
at touch time: PR #510's Codex R22 of 2026-09-27 and its two review passes
wrote these into `plan_memo_selftest_cases_r42.py`, which reached 904 lines,
and the STOP-CLEAN attestation of 900c16eb asked for a read and a refuse arm at
every gap-bearing pattern -- more than that module could take under the
1000-line bound.  The older R47-4 kind-phrase gap controls (U+00A0, tab, LF,
the ASCII fold) stay in `_cases_r42.py`, where their round put them.  This
module holds every reader-gap CASE of the 2026-09-27 round; that round's
FUNCTION controls are `unicode_whitespace_class_control` (the class oracle, in
`plan_memo_selftest_invariants.py`) and `role_measure_control` (in
`plan_memo_selftest_controls.py`).  Its mutants are
`plan_memo_selftest_mutants_gap.py`'s.

This module holds its OWN `CASES` and binds its own spellings (`spellings()`);
`plan_memo_selftest_cases.cases()` gathers it with the rest.
"""

from plan_memo_selftest_cases import build, spellings
from plan_memo_selftest_cases_r26 import kindcell

CASES = []
case, acase, rcase = spellings()


# -- PR #510 Codex R22 of 2026-09-27: the gap is CommonMark §2.1's Unicode
# whitespace, not Python's.  `GAP` was `(?u:\s)`, whose set (`str.isspace()`)
# holds eight code points §2.1 and cmark 0.31.2 do not read as whitespace
# (measured through `cmark`: `x *<c>a*` opens `<em>` for each of the eight), so
# `KIND<U+001C>UNDETERMINED` -- `KINDUNDETERMINED` to a reader -- declared the
# kind and, with the `Deps` edge, raised UMBRELLA-CELL at rc 1.  ONE NEGATIVE
# PER WAY OUT OF PYTHON'S SET: bidirectional class B (U+001C), class S that
# `re.ASCII`'s `\s` also holds (U+000B), a C1 control (U+0085), and the two
# separators of categories Zl / Zp (U+2028 / U+2029) -- so a re-spelling that
# drops one route and keeps another is red on the one it keeps.
R22_WS_OUTSIDE = []
for _label, _gap in (("U+001C (INFORMATION SEPARATOR FOUR, bidirectional class B)", "\x1c"),
                     ("U+000B (LINE TABULATION, which `re.ASCII`'s `\\s` holds as well)", "\x0b"),
                     ("U+0085 (NEXT LINE, a C1 control)", "\x85"),
                     ("U+2028 (LINE SEPARATOR, category Zl)", "\u2028"),
                     ("U+2029 (PARAGRAPH SEPARATOR, category Zp)", "\u2029")):
    acase("NEGATIVE", "(R22 ws) `KIND`, then %s, then `UNDETERMINED` is no kind phrase: the character is "
                      "whitespace to `str.isspace()` and NOT to CommonMark §2.1 or cmark, so a reader "
                      "sees the two words run together and the `Deps` edge is a terminal's" % _label,
          kindcell("KIND%sUNDETERMINED" % _gap), "UMBRELLA-CELL", 0)
    R22_WS_OUTSIDE.append(CASES[-1].name)
acase("NEGATIVE", "(R22 ws) the MARKER composes the same gap: `UMBRELLA, not a` + U+001C + `terminal unit` "
                  "is not the marker, so the row is terminal and its `Deps` edge unasserted",
      kindcell("**UMBRELLA, not a\x1cterminal unit.**"), "UMBRELLA-CELL", 0)
R22_WS_OUTSIDE.append(CASES[-1].name)
acase("POSITIVE", "(R22 ws) U+3000 IDEOGRAPHIC SPACE between `KIND` and `UNDETERMINED` IS a gap: it is "
                  "category Zs, the space these memos' Japanese prose types, and the highest §2.1 "
                  "member -- the one a class enumerated over less than the BMP would lose",
      kindcell("KIND\u3000UNDETERMINED"), "UMBRELLA-CELL", 1)
R22_WS_IDEOGRAPHIC = CASES[-1].name

# The sweep's two other patterns whose subject is a reader's gap, each routed
# through `GAP`: the appositive that attributes a marker (mechanical,
# UMBRELLA-MARK), and the id run inside a code span (the disposition every
# scanner reads).  Each was `\s` under `re.ASCII` or `[ \t\n]`, so U+00A0
# between the words was no gap at all.
acase("POSITIVE", "(R22 ws) the appositive's gap after the id is a reader's: `Slice 9z&nbsp;— **UMBRELLA, "
                  "…**` attributes the marker to `9z` -- `\\s` under `re.ASCII` read no appositive, the "
                  "field became the row's own declaration and the UMBRELLA-MARK was lost at rc 0",
      build(wb="Slice 9z\u00a0— **UMBRELLA, not a terminal unit.** points into §8."), "UMBRELLA-MARK", 1)
R22_WS_APPOSITIVE = CASES[-1].name
acase("POSITIVE", "(R22 ws) and so is the gap between the row noun and the id: `Slice&nbsp;9z — "
                  "**UMBRELLA, …**` names the row `9z` -- `ROW_NOUN_SEP` read `[ \\t\\n]` and a dash, "
                  "so the noun named nothing and the attribution was lost the same way",
      build(wb="Slice\u00a09z — **UMBRELLA, not a terminal unit.** points into §8."), "UMBRELLA-MARK", 1)
R22_WS_ROW_NOUN = CASES[-1].name
case("POSITIVE", "(R22 ws) an id run inside a code span is split by a reader's gap: `` `Qx&nbsp;9z` `` in a "
                 "`Deps` cell is two ids, so the umbrella `9z` is a naming site -- with `\\s` under "
                 "`re.ASCII` the span was no id run, it was masked, and the site went unreported",
     build(d7z="`Qx\u00a09z`"), "", 1)
R22_WS_ID_RUN = CASES[-1].name
case("NEGATIVE", "(R22 ws) and only by one: a code span holding `Qx`, U+000B, `9z` is no id run -- "
                 "cmark does not read U+000B as whitespace, so the span is code, not a list of ids, "
                 "and is masked like any other",
     build(d7z="`Qx\x0b9z`"), "", 0)
R22_WS_ID_RUN_VT = CASES[-1].name

# The ROLES vocabularies compose the same gap (`plan_memo_stream.phrase`).  They
# spelled it `\s` under `re.ASCII` (U+000B yes, U+00A0 no) and as a literal
# U+0020 inside phrase words, so a reader's `&nbsp;` broke the phrase and a
# U+000B cmark does not read as whitespace completed it.  One U+00A0 arm and
# one U+000B arm per vocabulary family; the ranking's arms are rows of
# `_R22_GAP_TABLE` below, measured with the harness's `("role", ...)` measure.
R22_WS_ROLES = {}
for _fam, _label, _prose in (
        ("before", "the licensing phrase BEFORE the mention (`the child of`)",
         "The drain is the child of%s**9z** in this plan."),
        ("after", "the licensing phrase AFTER the mention (`'s derivation`)",
         "**9z**'s%sderivation mints the drain.")):
    case("NEGATIVE", "(R22 ws) %s is licensed across a U+00A0 gap: a reader reads the phrase, so the "
                     "mention is no naming site -- the `\\s` it was spelled with under `re.ASCII` "
                     "reported it" % _label,
         build(), _prose % " ", 0)
    R22_WS_ROLES[_fam] = [CASES[-1].name]
    case("POSITIVE", "(R22 ws) %s is NOT licensed across a U+000B: cmark does not read it as "
                     "whitespace, so the words run together and the mention is a site -- `\\s` "
                     "under `re.ASCII` licensed it" % _label,
         build(), _prose % "\x0b", 1)
    R22_WS_ROLES[_fam].append(CASES[-1].name)
acase("POSITIVE", "(R22 ws) the two-owner clause reads a reader's gap in its words AND between the "
                  "owners: `owned&nbsp;by **7z**&nbsp;and **Qx**` is two owners in one clause",
      build(s9z="charter.  The drain is owned by **7z** and **Qx**."), "TWO-OWNERS?", 1)
R22_WS_ROLES["owners"] = [CASES[-1].name]
acase("NEGATIVE", "(R22 ws) and not a U+000B: `owned by **7z**` + U+000B + `and **Qx**` is no two-owner "
                  "clause to cmark, which `\\s` under `re.ASCII` read as one",
      build(s9z="charter.  The drain is owned by **7z**\x0band **Qx**."), "TWO-OWNERS?", 0)
R22_WS_ROLES["owners"].append(CASES[-1].name)
for _fam, _label, _cell, _field, _code in (
        ("order", "ORDER-PROSE?'s `blocked by`", "Terminal.  This row is blocked%sby Slice **Qx**; the probe must return 3.",
         "s7z", "ORDER-PROSE?"),
        ("declares", "the kind-in-words seed's `is an umbrella`",
         "Terminal.  This row is%san umbrella by derivation.  Acceptance: must.", "sqx", "UMBRELLA-MARK?")):
    _kw = {_field: _cell % " "}
    if _field == "s7z":
        _kw["d7z"] = "—"
    acase("POSITIVE", "(R22 ws) %s is read across a U+00A0 gap: the vocabulary wrote its word gap as a "
                      "literal U+0020, so the phrase a reader reads seeded nothing" % _label,
          build(**_kw), _code, 1)
    R22_WS_ROLES[_fam] = [CASES[-1].name]
    _kw[_field] = _cell % "\x0b"
    acase("NEGATIVE", "(R22 ws) %s is NOT read across a U+000B, which cmark does not read as "
                      "whitespace" % _label,
          build(**_kw), _code, 0)
    R22_WS_ROLES[_fam].append(CASES[-1].name)

# The file-name BOUNDARY is Python's whitespace on purpose (`plan_memo_tokens.
# _NAME_BOUNDARY`): a wider boundary SPLITS a run, so the id beside a name is
# reported rather than masked.  Nothing pinned that until the R22 pre-push
# review, and swapping both `isspace()` sites to the §2.1 predicate left the
# self-test green while `9z<U+001C>notes.md` went from one site to none.  One
# POSITIVE per character that leaves §2.1's set by a different route, at the
# run's START (the segment reset), and one NEGATIVE at its END (the run
# boundary `_run_end_from` finds).
R22_FILE_BOUNDARY = []
for _label, _c in (("U+001C", "\x1c"), ("U+000B", "\x0b"), ("U+2028", " ")):
    case("POSITIVE", "(R22 file) `9z` + %s + `notes.md`: the file-name run STARTS after the %s, which "
                     "`str.isspace()` calls whitespace, so `9z` stands outside the name and is a naming "
                     "site -- read with §2.1's narrower set the run would swallow it" % (_label, _label),
         build(), "The close rule is in 9z%snotes.md here." % _c, 1)
    R22_FILE_BOUNDARY.append(CASES[-1].name)
case("NEGATIVE", "(R22 file) `9z-notes.md` + U+001C: the run ENDS at the U+001C, so the suffix ends the "
                 "name and `9z` stays masked inside it -- read with §2.1's set the run would continue "
                 "past the suffix, no name would stand, and `9z` would be reported",
     build(), "The close rule is in 9z-notes.md\x1c here.", 0)
R22_FILE_BOUNDARY_END = CASES[-1].name

# §6.2 READS §2.1, NOT `str.isspace()` (the same review): cmark treats U+000B as
# NON-whitespace for flanking -- `printf 'The close rule is 9**z\x0b** here.' |
# cmark` gives `9<strong>z\v</strong>` -- so the closer, preceded by U+000B, is
# right-flanking and the pair renders nothing: the reader reads `9z`.  Under
# `isspace()` the closer cannot close, the `**` stay literal, and `9**z` names
# no row.  The U+00A0 twin is whitespace to both, so it never closes.
case("POSITIVE", "(R22 §6.2) `9**z` + U+000B + `**`: the closer is preceded by U+000B, which §2.1 and "
                 "cmark do not read as whitespace, so it closes, the pair renders nothing and the "
                 "reader reads `9z` -- a naming site",
     build(), "The close rule is 9**z\x0b** here.", 1)
R22_FLANK_VT = CASES[-1].name
case("NEGATIVE", "(R22 §6.2) the U+00A0 twin: a closer preceded by U+00A0 is not right-flanking (both "
                 "readings call it whitespace), the `**` stay literal and `9**z` names nothing",
     build(), "The close rule is 9**z ** here.", 0)


# -- THE TABLE (the STOP-CLEAN attestation of 900c16eb, and the reviews of
# 01bd2c5d and f85f4e1b).  Each `_R22_GAP_TABLE` row is one GAP POSITION of one
# gap-bearing pattern, written as a fixture template -- so positions are counted
# PER TEMPLATE, and one regex gap can have several (`derivation|that` and
# `derivation|mention` are the same gap after `derivation`).  Every row declares
# BOTH arms: the words joined by U+00A0 (read) and by U+000B (refused).  A
# `_R22_GAP_MIXED` row declares ONE arm, a refuse arm on its own run (VT+NBSP),
# for a gap an adjacent required gap would otherwise absorb.  Every declared arm
# is GENERATED from its row's template by the loop after `_R22_GAP_MIXED` --
# there is no reuse of an older case, so every template is exercised (the
# review of f85f4e1b: a reused arm was checked by its number only, and a row
# whose arms were both reused never ran its own template).
#
# WHICH PATTERNS HAVE ROWS is a hand enumeration, not a guarantee.  The table
# was built by hand at 22f6a2a2 (70 rows over 10 compiled patterns); 87f361c6
# added `NOUN_ANCHOR`'s row after a since-removed namespace walk counted 13;
# f85f4e1b replaced the ranking's function control with table rows; the names
# `ROLE_ORDERING` / `ROLE_OWNER` date from e39996ad.  `git log --reverse -S`
# names each first: `_R22_GAP_TABLE = (`, `("NOUN_ANCHOR", "noun|id"` and
# `("ROLE ordering", "waits|on"` over this file, `ROLE_ORDERING = ` over
# `plan_memo_roles.py`; over `plan_memo_selftest_controls.py` it names
# ea497230 then f85f4e1b for the ranking control's `def` line (added, then
# removed).  The 13: MARKER_RE, UNDETERMINED, POINTER,
# _ID_RUN_TOKEN, _APPOSITIVE, LICENSE_BEFORE, LICENSE_AFTER, NOUN_ANCHOR,
# ROLE_ORDERING, ROLE_OWNER, OWNS_TWO, ORDER_WORDS, DECLARES.  No control
# enforces rows for a new gap-bearing pattern: §8 (17) of
# `docs/plans/2026-08-plan-memo-umbrella-checker.md`.
# WHICH GAPS the rows reach is measured, not enforced: at the tree of the
# first commit whose message contains GAP-REACH SCRIPT, each of the 71 gaps of
# the 13 patterns, re-spelled alone as `(?a:\s)`, flips at least one arm,
# except ORDER_WORDS' `ordered before` / `sequenced after` (#4 / #5), which
# have no rows.  That commit's message holds the script:
#   git log --reverse --grep='GAP-REACH SCRIPT' --format=%H | head -1 | xargs git log -1 --format=%B
# The two are behaviour-neutral; this prints [True, True]:
#   python3 -c 'import sys; sys.path.insert(0,".claude/tools"); import plan_memo_roles as r; print([bool(r.ORDER_WORDS.search(s)) for s in ("ordered\x0bbefore", "sequenced\x0bafter")])'
# ⚠ THE ROW CHECKS after `_R22_GAP_MIXED`.  No mutant pins them: a mutant that
# disables one is unobservable while the table is valid.
_R22_GAP_TABLE = (
    # (pattern, position, fixture, template with {g}, read expectation, refuse expectation)
    # fixture: "prose" = a prose line, measured in naming sites; ("role", r) = a
    # prose line, measured by the RANKING's verdict on the ONE reported site --
    # 1 if it ranks r, 0 if it does not, -1 if the run reports no site or
    # several; (cell, code) = a cell of the fixture memo, measured in `code`
    # findings (None = sites).
    ("LICENCE child", "child|of", "prose", "The drain is the child{g}of **9z** in this plan.", 0, 1),
    ("LICENCE child", "of|mention", "prose", "The drain is the child of{g}**9z** in this plan.", 0, 1),
    ("LICENCE derivation", "derivation|that", "prose", "It is the derivation{g}that **9z** runs.", 0, 1),
    ("LICENCE derivation", "that|mention", "prose", "It is the derivation that{g}**9z** runs.", 0, 1),
    ("LICENCE derivation", "derivation|mention", "prose", "It is the derivation{g}**9z** runs.", 0, 1),
    ("LICENCE naming", "naming|mention", "prose", "Avoid naming{g}**9z** itself.", 0, 1),
    ("LICENCE mint", "mints|onto", "prose", "The plan mints{g}onto **9z** later.", 0, 1),
    ("LICENCE mint", "onto|mention", "prose", "The plan mints onto{g}**9z** later.", 0, 1),
    ("LICENCE mint", "mints|mention", "prose", "The plan mints{g}**9z** later.", 0, 1),
    ("LICENCE the", "the|mention", "prose", "The drain is the child of the{g}**9z** in this plan.", 0, 1),
    ("LICENSE_AFTER", "'s|derivation", "prose", "Note that **9z**'s{g}derivation mints it.", 0, 1),
    ("LICENSE_AFTER", "'s|own", "prose", "Note that **9z**'s{g}own charter says so.", 0, 1),
    ("LICENSE_AFTER", "own|charter", "prose", "Note that **9z**'s own{g}charter says so.", 0, 1),
    ("LICENSE_AFTER", "mention|whose", "prose", "Note that **9z**{g}whose charter says so.", 0, 1),
    ("LICENSE_AFTER", ",|whose", "prose", "Note that **9z**,{g}whose charter says so.", 0, 1),
    ("LICENSE_AFTER", "whose|charter", "prose", "Note that **9z**, whose{g}charter says so.", 0, 1),
    ("LICENSE_AFTER", "mention|is", "prose", "Note that **9z**{g}is an umbrella here.", 0, 1),
    ("LICENSE_AFTER", "is|an", "prose", "Note that **9z** is{g}an umbrella here.", 0, 1),
    ("LICENSE_AFTER", "an|umbrella", "prose", "Note that **9z** is an{g}umbrella here.", 0, 1),
    ("LICENSE_AFTER", "mention|runs", "prose", "Note that **9z**{g}runs at its own start.", 0, 1),
    ("LICENSE_AFTER", "runs|at", "prose", "Note that **9z** runs{g}at its own start.", 0, 1),
    ("LICENSE_AFTER", "at|its", "prose", "Note that **9z** runs at{g}its own start.", 0, 1),
    ("LICENSE_AFTER", "its|own", "prose", "Note that **9z** runs at its{g}own start.", 0, 1),
    ("LICENSE_AFTER", "own|start", "prose", "Note that **9z** runs at its own{g}start.", 0, 1),
    ("LICENSE_AFTER", "mention|became", "prose", "Note that **9z**{g}became an umbrella here.", 0, 1),
    ("LICENSE_AFTER", "became|an", "prose", "Note that **9z** became{g}an umbrella here.", 0, 1),
    ("LICENSE_AFTER", "became an|umbrella", "prose", "Note that **9z** became an{g}umbrella here.", 0, 1),
    ("OWNS_TWO keyword", "owned|by", ("s9z", "TWO-OWNERS?"), "charter.  The drain is owned{g}by **7z** and **Qx**.", 1, 0),
    ("OWNS_TWO keyword", "owner|is", ("s9z", "TWO-OWNERS?"), "charter.  The drain's owner{g}is **7z** and **Qx**.", 1, 0),
    ("OWNS_TWO keyword", "carried|by", ("s9z", "TWO-OWNERS?"), "charter.  The drain is carried{g}by **7z** and **Qx**.", 1, 0),
    ("OWNS_TWO keyword", "keyword|owner", ("s9z", "TWO-OWNERS?"), "charter.  The drain owns{g}**7z** and **Qx**.", 1, 0),
    ("OWNS_TWO join", "owner|and", ("s9z", "TWO-OWNERS?"), "charter.  The drain is owned by **7z**{g}and **Qx**.", 1, 0),
    ("OWNS_TWO join", "and|owner", ("s9z", "TWO-OWNERS?"), "charter.  The drain is owned by **7z** and{g}**Qx**.", 1, 0),
    ("OWNS_TWO join", "owner|or", ("s9z", "TWO-OWNERS?"), "charter.  The drain is owned by **7z**{g}or **Qx**.", 1, 0),
    ("OWNS_TWO join", "or|owner", ("s9z", "TWO-OWNERS?"), "charter.  The drain is owned by **7z** or{g}**Qx**.", 1, 0),
    ("OWNS_TWO join", "owner|,", ("s9z", "TWO-OWNERS?"), "charter.  The drain is owned by **7z**{g}, **Qx**.", 1, 0),
    ("OWNS_TWO join", ",|owner", ("s9z", "TWO-OWNERS?"), "charter.  The drain is owned by **7z**,{g}**Qx**.", 1, 0),
    ("OWNS_TWO join", "owner|/", ("s9z", "TWO-OWNERS?"), "charter.  The drain is owned by **7z**{g}/ **Qx**.", 1, 0),
    ("OWNS_TWO join", "/|owner", ("s9z", "TWO-OWNERS?"), "charter.  The drain is owned by **7z** /{g}**Qx**.", 1, 0),
    ("ORDER_WORDS", "lands|first", ("s7z", "ORDER-PROSE?"), "Terminal.  This row lands{g}first; the probe must return 3.", 1, 0),
    ("ORDER_WORDS", "lands|second", ("s7z", "ORDER-PROSE?"), "Terminal.  This row lands{g}second; the probe must return 3.", 1, 0),
    ("ORDER_WORDS", "prerequisite|of", ("s7z", "ORDER-PROSE?"), "Terminal.  This row is a prerequisite{g}of it; the probe must return 3.", 1, 0),
    ("ORDER_WORDS", "blocked|by", ("s7z", "ORDER-PROSE?"), "Terminal.  This row is blocked{g}by it; the probe must return 3.", 1, 0),
    ("ORDER_WORDS", "depends|on", ("s7z", "ORDER-PROSE?"), "Terminal.  This row depends{g}on it; the probe must return 3.", 1, 0),
    ("DECLARES", "is|an", ("sqx", "UMBRELLA-MARK?"), "Terminal.  This row is{g}an umbrella by derivation.  Acceptance: must.", 1, 0),
    ("DECLARES", "an|umbrella", ("sqx", "UMBRELLA-MARK?"), "Terminal.  This row is an{g}umbrella by derivation.  Acceptance: must.", 1, 0),
    ("DECLARES", "not|a", ("sqx", "UMBRELLA-MARK?"), "Terminal.  This row is not{g}a terminal unit by derivation.  Acceptance: must.", 1, 0),
    ("DECLARES", "a|terminal", ("sqx", "UMBRELLA-MARK?"), "Terminal.  This row is not a{g}terminal unit by derivation.  Acceptance: must.", 1, 0),
    ("DECLARES", "terminal|unit", ("sqx", "UMBRELLA-MARK?"), "Terminal.  This row is not a terminal{g}unit by derivation.  Acceptance: must.", 1, 0),
    ("DECLARES", "≥3|intersecting", ("sqx", "UMBRELLA-MARK?"), "Terminal.  This row has ≥3{g}intersecting axes.  Acceptance: must.", 1, 0),
    ("DECLARES", "three|intersecting", ("sqx", "UMBRELLA-MARK?"), "Terminal.  This row has three{g}intersecting axes.  Acceptance: must.", 1, 0),
    ("DECLARES", "no|canonical", ("sqx", "UMBRELLA-MARK?"), "Terminal.  This row has no{g}canonical algorithm.  Acceptance: must.", 1, 0),
    ("DECLARES", "canonical|algorithm", ("sqx", "UMBRELLA-MARK?"), "Terminal.  This row has no canonical{g}algorithm.  Acceptance: must.", 1, 0),
    ("MARKER_RE", "UMBRELLA,|not", ("kind", "UMBRELLA-CELL"), "**UMBRELLA,{g}not a terminal unit.**", 1, 0),
    ("MARKER_RE", "not|a", ("kind", "UMBRELLA-CELL"), "**UMBRELLA, not{g}a terminal unit.**", 1, 0),
    ("MARKER_RE", "a|terminal", ("kind", "UMBRELLA-CELL"), "**UMBRELLA, not a{g}terminal unit.**", 1, 0),
    ("MARKER_RE", "terminal|unit", ("kind", "UMBRELLA-CELL"), "**UMBRELLA, not a terminal{g}unit.**", 1, 0),
    ("UNDETERMINED", "KIND|UNDETERMINED", ("kind", "UMBRELLA-CELL"), "KIND{g}UNDETERMINED", 1, 0),
    ("UNDETERMINED", "KIND|dash", ("kind", "UMBRELLA-CELL"), "KIND{g}— UNDETERMINED", 1, 0),
    ("UNDETERMINED", "dash|UNDETERMINED", ("kind", "UMBRELLA-CELL"), "KIND —{g}UNDETERMINED", 1, 0),
    ("POINTER", "is|a", ("sqx", "ACCEPT-VOCAB?"), "This row is{g}a pointer rather than a slice.", 0, 1),
    ("POINTER", "a|pointer", ("sqx", "ACCEPT-VOCAB?"), "This row is a{g}pointer rather than a slice.", 0, 1),
    ("POINTER", "pointer|rather", ("sqx", "ACCEPT-VOCAB?"), "This row is a pointer{g}rather than a slice.", 0, 1),
    ("POINTER", "rather|than", ("sqx", "ACCEPT-VOCAB?"), "This row is a pointer rather{g}than a slice.", 0, 1),
    ("POINTER", "than|a", ("sqx", "ACCEPT-VOCAB?"), "This row is a pointer rather than{g}a slice.", 0, 1),
    ("POINTER", "a|slice", ("sqx", "ACCEPT-VOCAB?"), "This row is a pointer rather than a{g}slice.", 0, 1),
    ("_APPOSITIVE", "id|dash", ("wb", "UMBRELLA-MARK"), "Slice 9z{g}— **UMBRELLA, not a terminal unit.** points into §8.", 1, 0),
    ("_APPOSITIVE", "dash|marker", ("wb", "UMBRELLA-MARK"), "Slice 9z —{g}**UMBRELLA, not a terminal unit.** points into §8.", 1, 0),
    ("ROW_NOUN_SEP", "noun|id", ("wb", "UMBRELLA-MARK"), "Slice{g}9z — **UMBRELLA, not a terminal unit.** points into §8.", 1, 0),
    ("_ID_RUN_TOKEN", "id|id", ("d7z", None), "`Qx{g}9z`", 1, 0),
    # the review of 01bd2c5d: the anchored naming pass composes GAP through
    # `ROW_NOUN_SEP` and had no row -- `Slice&nbsp;C` reported nothing and
    # `Slice<U+000B>C` a site under a re-spelling nothing turned red
    ("NOUN_ANCHOR", "noun|id", "prose", "The close rule is Slice{g}C here.", 1, 0),
    # and the appositive's third gap, between the decoration and the marker
    ("_APPOSITIVE", "decor|marker", ("wb", "UMBRELLA-MARK"),
     "Slice 9z — **{g}UMBRELLA, not a terminal unit.** points into §8.", 1, 0),
    # the RANKING's two gap-bearing entries (`ROLE_PATTERNS`), measured by the
    # role the report prints for the one reported site -- ordinary rows since the
    # review of be5e1db5, not a function control on a separate coverage path
    ("ROLE ordering", "waits|on", ("role", "ordering"), "The close rule waits{g}on **9z** here.", 1, 0),
    ("ROLE owner", "placed|on", ("role", "owner"), "The close rule placed{g}on **9z** here.", 1, 0),
    ("ROLE owner", "chartered|to", ("role", "owner"), "The close rule chartered{g}to **9z** here.", 1, 0),
    ("ROLE owner", "charters|to", ("role", "owner"), "The close rule charters{g}to **9z** here.", 1, 0),
)
_R22_GAP_MIXED = (
    # (pattern, position, fixture, template, refuse expectation, the refuse run):
    # U+000B then U+00A0 in front of `and` / `or`.
    ("OWNS_TWO join", "optional|and, mixed run", ("s9z", "TWO-OWNERS?"),
     "charter.  The drain is owned by **7z**{g}and **Qx**.", 0, "\x0b\u00a0"),
    ("OWNS_TWO join", "optional|or, mixed run", ("s9z", "TWO-OWNERS?"),
     "charter.  The drain is owned by **7z**{g}or **Qx**.", 0, "\x0b\u00a0"),
)
R22_GAP = {}
"""pattern -> the names of its read and refuse cases, every position -- what each
pattern's re-spelling mutant in `plan_memo_selftest_mutants_gap.py` must turn red."""
R22_GAP_MIXED = []
"""the MIXED-run refuse arms.  The rows that turn them red are listed by the
MIXED-ARM PROBE, in the body of the first commit whose message holds that phrase:
`git log --reverse --grep='MIXED-ARM PROBE' --format=%H | head -1 | xargs git log -1 --format=%B`."""
# THE TABLE'S OWN INVARIANTS, checked where it is defined and LOUD at import
# (a bad row stops the self-test from loading): every `_R22_GAP_TABLE` row
# declares both expectations and they differ, every `_R22_GAP_MIXED` row
# declares its refuse expectation, and no (pattern, position) appears twice
# across the two tables.  Each needs nothing but the tables themselves.
_SEEN = set()
for _row in _R22_GAP_TABLE + _R22_GAP_MIXED:
    if (_row[0], _row[1]) in _SEEN:
        raise ValueError("gap table: %s %s appears twice" % (_row[0], _row[1]))
    _SEEN.add((_row[0], _row[1]))
for _row in _R22_GAP_TABLE:
    if _row[4] is None or _row[5] is None or _row[4] == _row[5]:
        raise ValueError("gap table: %s %s must declare two different expectations, not %r / %r"
                         % (_row[0], _row[1], _row[4], _row[5]))
for _row in _R22_GAP_MIXED:
    if _row[4] is None:
        raise ValueError("gap table: mixed row %s %s declares no refuse expectation" % (_row[0], _row[1]))
_ARMS = ([(p, pos, fix, tpl, "read", "\u00a0", "U+00A0", read, False)
          for p, pos, fix, tpl, read, _refuse in _R22_GAP_TABLE]
         + [(p, pos, fix, tpl, "refuse", "\x0b", "U+000B", refuse, False)
            for p, pos, fix, tpl, _read, refuse in _R22_GAP_TABLE]
         + [(p, pos, fix, tpl, "refuse", run, "U+000B U+00A0", want, True)
            for p, pos, fix, tpl, want, run in _R22_GAP_MIXED])
for _pat, _pos, _fix, _tpl, _arm, _gap, _label, _want, _mixed in _ARMS:
    _name = ("(R22 gap) %s, the gap %s: %s there is %s -- %s" % (
        _pat, _pos, _label,
        "a gap a reader sees, so the phrase is read" if _arm == "read"
        else "no whitespace to cmark, so the words run together and the phrase is refused",
        _tpl.replace("{g}", "<%s>" % _label)))
    _kind = "POSITIVE" if _want else "NEGATIVE"
    _text = _tpl.replace("{g}", _gap)
    if _fix == "prose":
        case(_kind, _name, build(), _text, _want)
    elif _fix[0] == "role":
        case(_kind, _name, build(), _text, _want, measure=_fix)
    elif _fix[0] == "kind":
        acase(_kind, _name, kindcell(_text), _fix[1], _want)
    elif _fix[1] is None:
        case(_kind, _name, build(**{_fix[0]: _text}), "", _want)
    else:
        _cells = {_fix[0]: _text}
        if _fix[0] == "s7z":
            _cells["d7z"] = "—"
        acase(_kind, _name, build(**_cells), _fix[1], _want)
    (R22_GAP_MIXED if _mixed else R22_GAP.setdefault(_pat, [])).append(_name)

# The role measure's -1 (`plan_memo_selftest_controls.role_measure_control`
# pins it).  The mutation runner installs the patched harness as a new module;
# the records keep the original's `measure`, so only a control that imports
# the harness when it runs sees the mutant.
case("NEGATIVE", "(R22 gap) the role measure is -1, never 0, when the run reports NO site "
     "(`The close rule waits on here.` -- nothing is ranked because nothing is reported)",
     build(), "The close rule waits on here.", -1, measure=("role", "ordering"))
