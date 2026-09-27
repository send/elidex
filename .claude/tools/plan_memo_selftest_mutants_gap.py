#!/usr/bin/env python3
"""The mutation-proof rows against the READER-GAP controls
(`plan_memo_selftest_cases_gap.py`): every row whose edit re-spells a gap a
reader sees -- the §2.1 predicate, the class derived from it, a pattern that
composes it through `plan_memo_stream.phrase` or directly -- or swaps one of
the two whitespace notions that are deliberately something else.

CARVED ON A SUBJECT, like `_mutants_ratchets.py` and `_mutants_population.py`,
from `plan_memo_selftest_mutants_r45.py`, where PR #510's Codex R22 of
2026-09-27 and its review passes had appended them.  WHERE THE R22 ROWS LIVE:
the rows labelled "R22 ws" / "R22 ws roles" / "R22 gap" / "R22 file" /
"R22 §6.2" are here; the OLDER Codex R22 (2026-09-08) rows labelled "R22 #1" -
"R22 #3" (the phrase boundaries, the device names, the seed's shared spans)
are in `plan_memo_selftest_mutants_inline.py`; the R47-4 gap rows that
predate both stay in `_mutants_r45.py`.

`MUTANTS` here is this module's OWN list; `plan_memo_selftest_mutants.mutants()`
gathers every mutants module's list (`plan_memo_selftest_harness.registry_modules`
decides which modules those are, by CONTENT: a module holding its own list) in one explicit step.
"""

from plan_memo_selftest_cases_gap import (
    R22_GAP, R22_FILE_BOUNDARY, R22_FILE_BOUNDARY_END, R22_FLANK_VT, R22_WS_APPOSITIVE, R22_WS_ID_RUN,
    R22_WS_ID_RUN_VT, R22_WS_IDEOGRAPHIC, R22_WS_OUTSIDE, R22_WS_ROLES, R22_WS_ROW_NOUN,
)
from plan_memo_selftest_mutants import EMPHASIS, ROLES, STREAM, TABLES, TOKENS

# This module's rows, in its OWN list: `plan_memo_selftest_mutants.mutants()` gathers every
# registry module's list in one explicit step, and none appends to another's.
MUTANTS = []

R22_WS_CLASS = ("PROPERTY: the reader-gap class every phrase composes (plan_memo_stream.GAP) is exactly "
                "CommonMark §2.1's Unicode whitespace as plan_memo_emphasis.is_unicode_whitespace "
                "defines it, over EVERY code point -- the derivation's BMP bound included")

# -- PR #510 Codex R22 of 2026-09-27: the gap is §2.1's whitespace, derived from
# ONE predicate.  One row per place the definition can be re-spelled: the
# class the phrases compose, the predicate itself, the derivation's bound, and
# each other pattern the sweep routed through `GAP`.
MUTANTS += [
    ("R22 ws: the phrases' gap is §2.1's whitespace, not Python's (restore `(?u:\\s)` -- the reported "
     "defect: U+001C / U+000B / U+0085 / U+2028 / U+2029 read as a gap and a run-together phrase "
     "declares the kind)", STREAM,
     'GAP = UNICODE_WHITESPACE',
     'GAP = r"(?u:\\s)"',
     list(R22_WS_OUTSIDE) + [R22_WS_CLASS]),
    ("R22 ws: the ONE predicate is §2.1's, not `str.isspace()` (re-spell it -- the class derived from "
     "it follows, so the oracle stays green and only the fixtures can see it)", EMPHASIS,
     '    return ch in _WS or unicodedata.category(ch) == "Zs"',
     '    return ch.isspace()',
     list(R22_WS_OUTSIDE)),
    ("R22 ws: the class is enumerated over the WHOLE BMP (lower the bound to U+3000 -- the ideographic "
     "space, the highest Zs character, leaves the class)", EMPHASIS,
     '_BMP_END = 0x10000',
     '_BMP_END = 0x3000',
     [R22_WS_IDEOGRAPHIC, R22_WS_CLASS]),
    ("R22 ws: the appositive's gaps are composed by `phrase` (re-spell them `\\s` under `re.ASCII` -- "
     "U+00A0 after the id reads as no appositive and the UMBRELLA-MARK is lost)", TABLES,
     '_APPOSITIVE = re.compile(BEFORE + ROW_NOUN_ID + phrase("(?: )?" + DASH_CLASS + "(?: )?" + DECOR + "(?: )?$"),',
     '_APPOSITIVE = re.compile(BEFORE + ROW_NOUN_ID + r"\\s*" + DASH_CLASS + r"\\s*" + DECOR + r"\\s*$",',
     [R22_WS_APPOSITIVE]),
    ("R22 ws: the row-noun separator's whitespace is `GAP` (re-spell the ASCII `[ \\t\\n]` it was -- "
     "`Slice&nbsp;9z` names no row)", TABLES,
     'ROW_NOUN_SEP = ROW_NOUN + "(?:" + GAP + "|" + DASH_CLASS + ")+"',
     'ROW_NOUN_SEP = ROW_NOUN + "[ \\t\\n" + DASH + "]+"',
     [R22_WS_ROW_NOUN]),
    ("R22 ws: an id run's whitespace separator is `GAP` (re-spell it `\\s` under `re.ASCII` -- "
     "`` `Qx&nbsp;9z` `` is masked, and U+000B splits a run cmark does not)", STREAM,
     '(?P<sep>(?:%s|[,;/→>+&|-])+)"\n                           % (SLUG_ID, CITE_ID, SHORT_ID, GAP), re.ASCII)',
     '(?P<sep>[\\s,;/→>+&|-]+)"\n                           % (SLUG_ID, CITE_ID, SHORT_ID), re.ASCII)',
     [R22_WS_ID_RUN, R22_WS_ID_RUN_VT]),
]

ROLE_RANK_GAP = ("the role RANKING reads a reader's gap in every entry that holds one: `waits&nbsp;on` ranks "
                 "ordering, `placed&nbsp;on` / `chartered&nbsp;to` / `charters&nbsp;to` owner, and none across "
                 "a U+000B")

# -- the same round's roles half: every vocabulary in `plan_memo_roles` composes
# its word gaps through `plan_memo_stream.phrase`.  One row per family, each
# re-spelling that family's gaps the way it was spelled before: `\s` under
# `re.ASCII` for the licensing phrases and the two-owner clause, a literal
# U+0020 for the ranking and the two seeds.
MUTANTS += [
    ("R22 ws roles: the licensing phrases BEFORE the mention compose the gap (re-spell it `\\s` -- "
     "`child of&nbsp;**9z**` is reported, `child of<U+000B>**9z**` licensed)", ROLES,
     '_LICENCE_PHRASES = tuple(phrase(p) for p in (',
     '_LICENCE_PHRASES = tuple(p.replace(" ", r"\\s+") for p in (',
     list(R22_WS_ROLES["before"])),
    ("R22 ws roles: the licensing phrases AFTER the mention compose the gap (re-spell it `\\s`)", ROLES,
     'LICENSE_AFTER = re.compile(phrase(',
     'LICENSE_AFTER = re.compile((lambda p: p.replace(" ", r"\\s+"))(',
     list(R22_WS_ROLES["after"])),
    ("R22 ws roles: the two-owner clause composes the gap (restore its `\\s` spelling)", ROLES,
     '    phrase(r"\\b(?:owns?|owned by|owner is|carries|carried by) ")\n'
     '    + decorated_id(ROW_ID, "a")\n'
     '    + phrase(r"(?: )?(?:,(?: )?| and | or |(?: )?/(?: )?)")',
     '    r"\\b(?:owns?|owned by|owner is|carries|carried by)\\s+"\n'
     '    + decorated_id(ROW_ID, "a")\n'
     '    + r"\\s*(?:,\\s*|\\s+and\\s+|\\s+or\\s+|\\s*/\\s*)"',
     list(R22_WS_ROLES["owners"])),
    ("R22 ws roles: ORDER-PROSE?'s vocabulary composes the gap (drop `phrase` -- its word gaps are "
     "U+0020 alone again)", ROLES,
     'ORDER_WORDS = re.compile(phrase(', 'ORDER_WORDS = re.compile((',
     R22_WS_ROLES["order"][:1]),
    ("R22 ws roles: the kind-in-words vocabulary composes the gap (drop `phrase`)", ROLES,
     'bounded(phrase(r"is an umbrella|', 'bounded((r"is an umbrella|',
     R22_WS_ROLES["declares"][:1]),
    ("R22 ws roles: the role ranking composes the gap (drop `phrase` from the ordering entry)", ROLES,
     '    ("ordering", re.compile(phrase(', '    ("ordering", re.compile((',
     [ROLE_RANK_GAP]),
]

# -- the R22 pre-push review: the two readings that were right and unpinned
# (the file-name boundary is Python's set on purpose; §6.2 reads §2.1).
MUTANTS += [
    ("R22 file: the file-name run STARTS at Python's whitespace (swap the segment reset to the §2.1 "
     "predicate -- `9z<U+001C>notes.md` swallows `9z` and the site is lost)", TOKENS,
     '        if c in _NAME_BOUNDARY or c.isspace():',
     '        if c in _NAME_BOUNDARY or __import__("plan_memo_emphasis").is_unicode_whitespace(c):',
     list(R22_FILE_BOUNDARY)),
    ("R22 file: the file-name run ENDS at Python's whitespace (swap the run-end scan to the §2.1 "
     "predicate -- the run continues past `9z-notes.md<U+001C>`, no name stands and `9z` is reported)",
     TOKENS,
     '    while j < n and not (text[j].isspace() or text[j] in _NAME_BOUNDARY):',
     '    while j < n and not (__import__("plan_memo_emphasis").is_unicode_whitespace(text[j])\n'
     '                         or text[j] in _NAME_BOUNDARY):',
     [R22_FILE_BOUNDARY_END]),
    ("R22 §6.2: flanking reads §2.1, not `str.isspace()` (re-spell `_is_ws` -- a closer after U+000B "
     "cannot close, the `**` stay literal and `9**z` names nothing)", EMPHASIS,
     '    return ch is None or is_unicode_whitespace(ch)',
     '    return ch is None or ch.isspace()',
     [R22_FLANK_VT]),
]

# -- THE ENUMERATION's rows (the STOP-CLEAN attestation of 900c16eb): one per
# gap-bearing pattern of `plan_memo_selftest_cases_gap._R22_GAP_TABLE`, each
# re-spelling THAT pattern's gaps as Python's ASCII whitespace `(?a:\s)` --
# U+000B in, U+00A0 out -- whatever flags the pattern is compiled with, so the
# ONE edit turns every read arm AND every refuse arm of the pattern red.
_ASCII_GAP = '(lambda p: p.replace(" ", r"(?a:\\s)+"))('
MUTANTS += [
    ("R22 gap: `child of` composes the gap (re-spell it `(?a:\\s)`)", ROLES,
     '    r"child(?:ren)? (?:of )?",', '    r"child(?:ren)?(?a:\\s)+(?:of(?a:\\s)+)?",',
     R22_GAP["LICENCE child"]),
    ("R22 gap: `derivation that` composes the gap (re-spell it `(?a:\\s)`)", ROLES,
     '    r"derivation (?:that )?",', '    r"derivation(?a:\\s)+(?:that(?a:\\s)+)?",',
     R22_GAP["LICENCE derivation"]),
    ("R22 gap: `naming` composes the gap (re-spell it `(?a:\\s)`)", ROLES,
     '    r"naming ",', '    r"naming(?a:\\s)+",',
     R22_GAP["LICENCE naming"]),
    ("R22 gap: `mints onto` composes the gap (re-spell it `(?a:\\s)`)", ROLES,
     '    r"mint(?:s|ed|ing)? (?:onto )?",', '    r"mint(?:s|ed|ing)?(?a:\\s)+(?:onto(?a:\\s)+)?",',
     R22_GAP["LICENCE mint"]),
    ("R22 gap: LICENSE_BEFORE's optional `the` composes the gap (re-spell it `(?a:\\s)`)", ROLES,
     'phrase(r"(?:the )?$")', 'r"(?:the(?a:\\s)+)?$"',
     R22_GAP["LICENCE the"]),
    ("R22 gap: LICENSE_AFTER composes the gap (re-spell every gap `(?a:\\s)`)", ROLES,
     'LICENSE_AFTER = re.compile(phrase(', 'LICENSE_AFTER = re.compile(' + _ASCII_GAP,
     R22_GAP["LICENSE_AFTER"]),
    ("R22 gap: the two-owner clause's keyword half composes the gap (re-spell it `(?a:\\s)`)", ROLES,
     '    phrase(r"\\b(?:owns?|owned by|owner is|carries|carried by) ")',
     '    ' + _ASCII_GAP + 'r"\\b(?:owns?|owned by|owner is|carries|carried by) ")',
     R22_GAP["OWNS_TWO keyword"]),
    ("R22 gap: the two-owner clause's joining half composes the gap (re-spell it `(?a:\\s)`)", ROLES,
     '    + phrase(r"(?: )?(?:,(?: )?| and | or |(?: )?/(?: )?)")',
     '    + ' + _ASCII_GAP + 'r"(?: )?(?:,(?: )?| and | or |(?: )?/(?: )?)")',
     R22_GAP["OWNS_TWO join"]),
    ("R22 gap: ORDER_WORDS composes the gap (re-spell it `(?a:\\s)`)", ROLES,
     'ORDER_WORDS = re.compile(phrase(', 'ORDER_WORDS = re.compile(' + _ASCII_GAP,
     R22_GAP["ORDER_WORDS"]),
    ("R22 gap: DECLARES composes the gap (re-spell it `(?a:\\s)`)", ROLES,
     'bounded(phrase(r"is an umbrella|', 'bounded(' + _ASCII_GAP + 'r"is an umbrella|',
     R22_GAP["DECLARES"]),
    ("R22 gap: the role ranking's ordering entry composes the gap (re-spell it `(?a:\\s)`)", ROLES,
     '    ("ordering", re.compile(phrase(', '    ("ordering", re.compile(' + _ASCII_GAP,
     [ROLE_RANK_GAP]),
    ("R22 gap: the role ranking's owner entries compose the gap (re-spell them `(?a:\\s)`)", ROLES,
     '    ("owner", re.compile(phrase(', '    ("owner", re.compile(' + _ASCII_GAP,
     [ROLE_RANK_GAP]),
    ("R22 gap: the MARKER composes the gap (re-spell it `(?a:\\s)`)", STREAM,
     'MARKER_RE = re.compile(bounded(_phrase(MARKER)))',
     'MARKER_RE = re.compile(bounded(re.escape(MARKER).replace("\\\\ ", r"(?a:\\s)+")))',
     R22_GAP["MARKER_RE"]),
    ("R22 gap: UNDETERMINED composes the gap (re-spell it `(?a:\\s)`)", STREAM,
     'bounded(phrase("KIND(?: "', 'bounded(' + _ASCII_GAP + '"KIND(?: "',
     R22_GAP["UNDETERMINED"]),
    ("R22 gap: POINTER composes the gap (re-spell it `(?a:\\s)` -- the vocabulary nothing pinned)", STREAM,
     'POINTER = re.compile(bounded(_phrase("is a pointer rather than a slice")))',
     'POINTER = re.compile(bounded(re.escape("is a pointer rather than a slice").replace("\\\\ ", r"(?a:\\s)+")))',
     R22_GAP["POINTER"]),
    ("R22 gap: the appositive composes the gap (re-spell it `(?a:\\s)` -- a U+000B made a false "
     "gating UMBRELLA-MARK)", TABLES,
     'ROW_NOUN_ID + phrase("(?: )?"', 'ROW_NOUN_ID + ' + _ASCII_GAP + '"(?: )?"',
     R22_GAP["_APPOSITIVE"]),
    ("R22 gap: the row-noun separator's union composes GAP (re-spell it `(?a:\\s)`)", TABLES,
     'ROW_NOUN_SEP = ROW_NOUN + "(?:" + GAP + "|" + DASH_CLASS + ")+"',
     'ROW_NOUN_SEP = ROW_NOUN + "(?:(?a:\\\\s)|" + DASH_CLASS + ")+"',
     R22_GAP["ROW_NOUN_SEP"]),
]
