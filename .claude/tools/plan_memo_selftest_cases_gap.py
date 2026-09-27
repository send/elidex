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
the ASCII fold) stay in `_cases_r42.py`, where their round put them; this
module holds every R22-of-2026-09-27 reader-gap control.  Its mutants are
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
# one U+000B arm per vocabulary family; the ranking family has no case measure
# and is a function control (`plan_memo_selftest_controls.role_rank_gap_control`).
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
