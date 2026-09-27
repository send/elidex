#!/usr/bin/env python3
"""The mutation-proof rows against the READER-GAP controls
(`plan_memo_selftest_cases_gap.py`): every row whose edit re-spells a gap a
reader sees -- the §2.1 predicate, the class derived from it, a pattern that
composes it through `plan_memo_stream.phrase` or directly -- or swaps one of
the two whitespace notions that are deliberately something else.

CARVED ON A SUBJECT, like `_mutants_ratchets.py` and `_mutants_population.py`,
from `plan_memo_selftest_mutants_r45.py`, where PR #510's Codex R22 of
2026-09-27 and its first review passes had appended the "R22 ws" / "R22 ws
roles" / "R22 file" / "R22 §6.2" rows; the "R22 gap" and "R22 ratchet" rows
were written here.  WHERE THE R22 ROWS LIVE: every 2026-09-27 row is here; the OLDER Codex R22 (2026-09-08) rows labelled "R22 #1" -
"R22 #3" (the phrase boundaries, the device names, the seed's shared spans)
are in `plan_memo_selftest_mutants_inline.py`; the R47-4 gap rows that
predate both stay in `_mutants_r45.py`.

`MUTANTS` here is this module's OWN list; `plan_memo_selftest_mutants.mutants()`
gathers every mutants module's list (`plan_memo_selftest_harness.registry_modules`
decides which modules those are, by CONTENT: a module holding its own list) in one explicit step.
"""

from plan_memo_selftest_cases_gap import (
    R22_GAP, R22_GAP_MIXED, R22_FILE_BOUNDARY, R22_FILE_BOUNDARY_END, R22_FLANK_VT, R22_WS_APPOSITIVE, R22_WS_ID_RUN,
    R22_WS_ID_RUN_VT, R22_WS_IDEOGRAPHIC, R22_WS_OUTSIDE, R22_WS_ROLES, R22_WS_ROW_NOUN,
)
from plan_memo_selftest_mutants import EMPHASIS, RATCHETS, ROLES, STREAM, TABLES, TOKENS

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
     '_APPOSITIVE = compile_gap("_APPOSITIVE", BEFORE + ROW_NOUN_ID + phrase("(?: )?" + DASH_CLASS + "(?: )?" + DECOR + "(?: )?$"),',
     '_APPOSITIVE = compile_gap("_APPOSITIVE", BEFORE + ROW_NOUN_ID + r"\\s*" + DASH_CLASS + r"\\s*" + DECOR + r"\\s*$",',
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
     'LICENSE_AFTER = compile_gap("LICENSE_AFTER", phrase(',
     'LICENSE_AFTER = compile_gap("LICENSE_AFTER", (lambda p: p.replace(" ", r"\\s+"))(',
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
     'ORDER_WORDS = compile_gap("ORDER_WORDS", phrase(', 'ORDER_WORDS = compile_gap("ORDER_WORDS", (',
     R22_WS_ROLES["order"][:1]),
    ("R22 ws roles: the kind-in-words vocabulary composes the gap (drop `phrase`)", ROLES,
     'bounded(phrase(r"is an umbrella|', 'bounded((r"is an umbrella|',
     R22_WS_ROLES["declares"][:1]),
    ("R22 ws roles: the role ranking composes the gap (drop `phrase` from the ordering entry)", ROLES,
     'ROLE_ORDERING = compile_gap("ROLE_ORDERING", phrase(', 'ROLE_ORDERING = compile_gap("ROLE_ORDERING", (',
     R22_GAP["ROLE ordering"][:1]),
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

# -- THE TABLE's rows (the STOP-CLEAN attestation of 900c16eb): one per table
# pattern of `plan_memo_selftest_cases_gap._R22_GAP_TABLE`, each re-spelling
# THAT pattern's gaps as Python's ASCII whitespace `(?a:\s)` -- U+000B in,
# U+00A0 out -- whatever flags the pattern is compiled with, so the ONE edit
# turns every read arm AND every refuse arm of the pattern red.  Which patterns
# need table rows is the registry's to say (`plan_memo_stream.compile_gap`), and
# `plan_memo_selftest_ratchets.gap_registry_control` reads the table, not this
# list: a pattern with no "R22 gap" row here is not red on that account.  The
# rows for `NOUN_ANCHOR` and the id run are below.
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
     'LICENSE_AFTER = compile_gap("LICENSE_AFTER", phrase(', 'LICENSE_AFTER = compile_gap("LICENSE_AFTER", ' + _ASCII_GAP,
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
     'ORDER_WORDS = compile_gap("ORDER_WORDS", phrase(', 'ORDER_WORDS = compile_gap("ORDER_WORDS", ' + _ASCII_GAP,
     R22_GAP["ORDER_WORDS"]),
    ("R22 gap: DECLARES composes the gap (re-spell it `(?a:\\s)`)", ROLES,
     'bounded(phrase(r"is an umbrella|', 'bounded(' + _ASCII_GAP + 'r"is an umbrella|',
     R22_GAP["DECLARES"]),
    ("R22 gap: the role ranking's ordering entry composes the gap (re-spell it `(?a:\\s)`)", ROLES,
     'ROLE_ORDERING = compile_gap("ROLE_ORDERING", phrase(', 'ROLE_ORDERING = compile_gap("ROLE_ORDERING", ' + _ASCII_GAP,
     R22_GAP["ROLE ordering"]),
    ("R22 gap: the role ranking's owner entries compose the gap (re-spell them `(?a:\\s)`)", ROLES,
     'ROLE_OWNER = compile_gap("ROLE_OWNER", phrase(', 'ROLE_OWNER = compile_gap("ROLE_OWNER", ' + _ASCII_GAP,
     R22_GAP["ROLE owner"]),
    ("R22 gap: the MARKER composes the gap (re-spell it `(?a:\\s)`)", STREAM,
     'MARKER_RE = compile_gap("MARKER_RE", bounded(_phrase(MARKER)))',
     'MARKER_RE = compile_gap("MARKER_RE", bounded(re.escape(MARKER).replace("\\\\ ", r"(?a:\\s)+")))',
     R22_GAP["MARKER_RE"]),
    ("R22 gap: UNDETERMINED composes the gap (re-spell it `(?a:\\s)`)", STREAM,
     'bounded(phrase("KIND(?: "', 'bounded(' + _ASCII_GAP + '"KIND(?: "',
     R22_GAP["UNDETERMINED"]),
    ("R22 gap: POINTER composes the gap (re-spell it `(?a:\\s)` -- the vocabulary nothing pinned)", STREAM,
     'POINTER = compile_gap("POINTER", bounded(_phrase("is a pointer rather than a slice")))',
     'POINTER = compile_gap("POINTER", bounded(re.escape("is a pointer rather than a slice").replace("\\\\ ", r"(?a:\\s)+")))',
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

# -- the review of 01bd2c5d: the pattern the hand-traced table missed, the id
# run's own "R22 gap" row (so the rule has no exception), and the one optional
# gap only a mixed run reaches.
MUTANTS += [
    ("R22 gap: the anchored naming pass composes GAP (re-spell NOUN_ANCHOR's gap `(?a:\\s)` -- "
     "`Slice&nbsp;C` names nothing, `Slice<U+000B>C` names C)", ROLES,
     'NOUN_ANCHOR = compile_gap("NOUN_ANCHOR", BEFORE + ROW_NOUN_SEP)',
     'NOUN_ANCHOR = compile_gap("NOUN_ANCHOR", BEFORE + ROW_NOUN_SEP.replace(__import__("plan_memo_stream").GAP, r"(?a:\\s)"))',
     R22_GAP["NOUN_ANCHOR"]),
    ("R22 gap: the id run's separator composes GAP (re-spell it `(?a:\\s)`)", STREAM,
     '% (SLUG_ID, CITE_ID, SHORT_ID, GAP)', '% (SLUG_ID, CITE_ID, SHORT_ID, r"(?a:\\s)")',
     R22_GAP["_ID_RUN_TOKEN"]),
    ("R22 gap: the two-owner clause's OPTIONAL leading gap composes the gap (re-spell that one gap "
     "alone -- `**7z**<U+000B><U+00A0>and **Qx**` becomes a two-owner clause)", ROLES,
     '    + phrase(r"(?: )?(?:,(?: )?| and | or |(?: )?/(?: )?)")',
     '    + r"(?:(?a:\\s)+)?" + phrase(r"(?:,(?: )?| and | or |(?: )?/(?: )?)")',
     list(R22_GAP_MIXED) + [n for n in R22_GAP["OWNS_TWO join"] if "owner|," in n]),
]

# -- the gap REGISTRY (the review of ee2a1d7f replaced the namespace walk):
# rows that re-register, unregister or bypass a pattern, rows that change only
# the TABLE (the self-test module `plan_memo_selftest_cases_gap`, installed under
# its own name, so the mutants module keeps its import-time `R22_GAP`), and one
# row per red condition of `_gap_problems`, killed by the partner's fixture.
GAP_RATCHET = ("PROPERTY: every gap pattern registered through plan_memo_stream.compile_gap has rows in "
               "plan_memo_selftest_cases_gap with a READ and a REFUSE arm, each row's expectations present "
               "and distinct and every declared arm generated, and no module-level gap pattern escapes the "
               "registry")
GAP_RATCHET_PARTNER = ("PROPERTY: the gap ratchet's cores and compile_gap, one fixture per behaviour -- "
                       "compile_gap's duplicate-label refusal, the depth-0 backstop, and each red condition reported by "
                       "exactly its own fixture")
_RETIRED = 'RETIRED = re.compile(r"\\bMERGED\\b|\\bRETIRED\\b|\\bLANDED\\b", re.ASCII)'
MUTANTS += [
    ("R22 ratchet: a module-level gap pattern compiled with plain `re.compile` is red (the backstop)",
     ROLES, _RETIRED, _RETIRED + '\n_UNREGISTERED = re.compile(phrase("left right"))',
     [GAP_RATCHET]),
    ("R22 ratchet: ... also when it is bound under a module-level `if`", ROLES,
     _RETIRED, _RETIRED + '\nif True:\n    ZC = re.compile(phrase("left right"))',
     [GAP_RATCHET]),
    ("R22 ratchet: a REGISTERED pattern with no rows is red (register one)", ROLES,
     _RETIRED, _RETIRED + '\n_UNPINNED = compile_gap("_UNPINNED", phrase("left right"))',
     [GAP_RATCHET]),
    ("R22 ratchet: a row naming a label that is no longer registered is red (compile POINTER with a "
     "literal U+0020 and plain `re.compile` -- the reviewer's re-spelling)", STREAM,
     'POINTER = compile_gap("POINTER", bounded(_phrase("is a pointer rather than a slice")))',
     'POINTER = re.compile(bounded(re.escape("is a pointer rather than a slice")))',
     [GAP_RATCHET]),
    ("R22 ratchet: `compile_gap` refuses a label it has already registered (drop the check)", STREAM,
     "    if label in GAP_PATTERNS:\n", "    if False:\n",
     [GAP_RATCHET_PARTNER]),
    ("R22 ratchet: a pattern whose ROWS are gone is red while its label is registered (delete "
     "NOUN_ANCHOR's table row)", "plan_memo_selftest_cases_gap.py",
     '    ("NOUN_ANCHOR", "noun|id", "prose", "The close rule is Slice{g}C here.", 1, 0),\n',
     '',
     [GAP_RATCHET]),
    ("R22 ratchet: a table row without both expectations is red (drop the id run's refuse column)",
     "plan_memo_selftest_cases_gap.py",
     '    ("_ID_RUN_TOKEN", "id|id", ("d7z", None), "`Qx{g}9z`", 1, 0),',
     '    ("_ID_RUN_TOKEN", "id|id", ("d7z", None), "`Qx{g}9z`", 1, None),',
     [GAP_RATCHET]),
    ("R22 ratchet: a row's read and refuse expectations must differ (make `naming`'s both 1)",
     "plan_memo_selftest_cases_gap.py",
     '    ("LICENCE naming", "naming|mention", "prose", "Avoid naming{g}**9z** itself.", 0, 1),',
     '    ("LICENCE naming", "naming|mention", "prose", "Avoid naming{g}**9z** itself.", 1, 1),',
     [GAP_RATCHET]),
    ("R22 ratchet: an arm the generator skips is red (drop the second mixed-run arm)",
     "plan_memo_selftest_cases_gap.py",
     '            for p, pos, fix, tpl, want, run in _R22_GAP_MIXED])',
     '            for p, pos, fix, tpl, want, run in _R22_GAP_MIXED[:1]])',
     [GAP_RATCHET]),
] + [
    ("R22 ratchet condition %s: disable it (the partner's fixture for it goes quiet)" % cond, RATCHETS,
     find, repl, [GAP_RATCHET_PARTNER])
    for cond, find, repl in (
        ("empty-registry", "    if not labels:\n", "    if False:\n"),
        ("gapless-label", "if gap not in registry[lab].pattern]", "if False]"),
        ("unregistered-pattern", '    out += [("unregistered-pattern", "%s.%s" % k) for k in unregistered]',
         '    out += [("unregistered-pattern", "%s.%s" % k) for k in unregistered if False]'),
        ("the backstop's identity test", "            and id(v) not in ids]", "            and False]"),
        ("duplicate-row", "for k, n in sorted(counts.items()) if n > 1]", "for k, n in sorted(counts.items()) if False]"),
        ("missing-column", "        if read is None or refuse is None:\n", "        if False:\n"),
        ("equal-expectations", "        elif read == refuse:\n", "        elif False:\n"),
        ("mixed-missing-arm", "        if refuse is None:\n", "        if False:\n"),
        ("unmapped-row", "for pat in sorted(used) if pat not in mapping]", "for pat in sorted(used) if False]"),
        ("unused-mapping", "for pat in sorted(mapping) if pat not in used]", "for pat in sorted(mapping) if False]"),
        ("unknown-label", "for lab in sorted(covered) if lab not in labels]", "for lab in sorted(covered) if False]"),
        ("uncovered-label", "for lab in sorted(labels) if lab not in covered]", "for lab in sorted(labels) if False]"),
        ("no-read-arm", "for lab in sorted(covered) if lab not in has_read]", "for lab in sorted(covered) if False]"),
        ("arm-not-generated (no record)",
         '            out.append(("arm-not-generated", "%s: no generated arm recorded" % where))',
         "            pass"),
        ("arm-not-generated (the record's expectation)", "        elif got[0] != want:\n", "        elif False:\n"),
        ("arm-not-generated (the collected case's expectation)",
         "        elif expect_of.get(got[1], object()) != want:\n", "        elif False:\n"),
    )
]
