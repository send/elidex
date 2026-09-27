#!/usr/bin/env python3
"""PR #510 R45-on mutation-proof rows -- the EIGHTH mutants module, carved
from `plan_memo_selftest_mutants_r30.py` at the review-round seam the registry
has used five times before (`plan_memo_selftest_mutants` holds the row shape,
`run` and the pre-converge rows; `_pr510` R1-R16 and the design re-gates;
`_inline` R17-R25; `_r26` R26-R29; `_r30` R30-R42).

TOUCH-TIME, NOT DEFERRED (CLAUDE.md "1000-line debt = touch-time split").
`_r30.py` stood at 998 lines when PR #510 Codex R22's whitespace fix owed rows
beside the R47-4 ones, and `line_bound_control` reads anything over 1000 as
over the bound.  The split is its own commit, ahead of the fix, and moves rows
only: no row, control or case changes, so the golden manifest is unchanged.

THE SEAM: `_r30.py` keeps R30-R42 -- the id cell's blank test, the raw-seed and
licensing linearity families, the code-span reading, the import seams, the
dash class, the file-token tails, the Axis 5 report channel and the R42
families (with the later amendments that were written into those families in
place, e.g. the R49-2 `Deps` rows beside R34-2).  This module holds R45 on: the
§6.4 falsifier's demotion filter, §6.1 step one, the population-scope loops,
the GFM delimiter cell, the R47 separator / gate / word-gap / scope rows, the
R47-6 refusal policy, the R48-2 spellings, and the R51 / R52 claim rows.
Measured, not assumed: an AST pass over the boundary finds NO name defined on
one side and read on the other, so the halves share only what both import.

`MUTANTS` here is this module's OWN list; `plan_memo_selftest_mutants.mutants()`
gathers every mutants module's list (`plan_memo_selftest_harness.registry_modules`
decides which modules those are, by CONTENT: a module holding its own list) in one explicit step.
"""

from plan_memo_selftest_cases_r42 import ( R42_10_UNBOUND_CLAIM, R42_LINE_ENDING_FIRST,
    R45_DELIM_ALL, R45_SHORT_DELIM, R45_TABLE_MISS_SCOPE, R45_UNKEYED_SCOPE,
    R47_1_DASH_SET, R47_1_EN_DASH, R47_2_UNBOUND_QUOTED, R47_2_UNBOUND_STRADDLE,
    R47_4_BASELINE, R47_4_BOUNDARY, R47_4_NBSP, R47_4_NON_WHITESPACE, R47_4_TAB,
    R47_4_UNDET_FOLD, R47_4_UNDET_NBSP, R47_5_ALL_KINDS, R47_5_SECOND_ROW, R47_5_SECOND_TABLE,
    R48_2_TWO_ROWS, R48_2_TWO_SPELLINGS, R51_TWO_STRADDLES, R52_OUTSIDE_QUOTED,
    R52_OUTSIDE_STRADDLE, R47_5_TWO_MISSES, R47_5_TWO_PHRASES, R47_5_TWO_REFS,
    R22_WS_APPOSITIVE, R22_WS_ID_RUN, R22_WS_ID_RUN_VT, R22_WS_IDEOGRAPHIC, R22_WS_OUTSIDE,
    R22_WS_ROLES, R22_WS_ROW_NOUN, R22_FILE_BOUNDARY, R22_FILE_BOUNDARY_END, R22_FLANK_VT,
)
from plan_memo_selftest_mutants import (
    BLOCKS, CONFORMANCE, EMPHASIS, MEMO, POPULATION, ROLES, STREAM, TABLES, TOKENS,
)

# This module's rows, in its OWN list: `plan_memo_selftest_mutants.mutants()` gathers every
# registry module's list in one explicit step, and none appends to another's.
MUTANTS = []

DEMOTED_AGREEMENT = ("CommonMark 0.31.2 §6.4: Phase 2's inline claim agrees with the spec's own html "
                     "for every §3.0b family DEMOTED into a resolved image description (the "
                     "cross-product the corpus cannot reach)")

# -- R45: the FALSIFIER's own demotion filter, one mutant per family, and one
# more that inverts the filter.  Each per-family row un-filters one count and
# the control goes red on that family alone; the vendored corpus stays green
# under each of them, which is the whole reason the control exists.
MUTANTS += [
    ("R45 §6.4 falsifier: a DEMOTED code span emits no `<code>` (un-filter the count -- the shape "
     "that shipped: `![a `b` c](img.png)` reported \"the html emits 0 `<code>`, Phase 2 claims 1\" "
     "against cmark's own `alt=\"a b c\"`)", CONFORMANCE,
     '    code = [e for e in lx.code if e[2] != "demoted"]',
     '    code = list(lx.code)',
     [DEMOTED_AGREEMENT]),
    ("R45 §6.4 falsifier: a DEMOTED autolink emits no `<a href=` (un-filter the count)", CONFORMANCE,
     '    auto = [e for e in lx.autolinks if e[2] != "demoted"]',
     '    auto = list(lx.autolinks)',
     [DEMOTED_AGREEMENT]),
    ("R45 §6.4 falsifier: a DEMOTED raw HTML span is not masked verbatim (un-filter the span list "
     "-- the R42-8 fix, now pinned by a control the corpus can reach)", CONFORMANCE,
     '[lx.text[a:b] for a, b, tag in lx.html if tag != "demoted"]',
     '[lx.text[a:b] for a, b, tag in lx.html]',
     [DEMOTED_AGREEMENT]),
    ("R45 §6.4 falsifier: the filter must not be applied EVERYWHERE -- invert it, so the count "
     "keeps only the DEMOTED spans and a bare code span claims none.  The direction the demoted "
     "arm alone cannot see: a filter that simply stopped counting passes it", CONFORMANCE,
     '    code = [e for e in lx.code if e[2] != "demoted"]',
     '    code = [e for e in lx.code if e[2] == "demoted"]',
     [DEMOTED_AGREEMENT]),
]


# -- R45: §6.1 STEP ONE, the half both R42-6 mutants missed.  They patch the
# TRIM; this patches the SUBSTITUTION, and the space-padded twin stays green
# under it -- which is the evidence that the two arms have different subjects.
MUTANTS += [
    ("R45 §6.1: line endings become spaces BEFORE the trim asks (neuter step one -- the raw test "
     "then declines the trim on a span padded with a line ending, and the slug splits in silence)",
     STREAM,
     '            norm = inner.replace("\\r\\n", " ").replace("\\r", " ").replace("\\n", " ")',
     '            norm = inner',
     [R42_LINE_ENDING_FIRST]),
]

# -- R45: the two population-scope loops the ratchet missed.  Same intent --
# scope a population-wide step to `main` -- as the rows that already exist for
# `_declare` / `keep` / `data_rows` / the walk / `_unbound_claims` (the edit is
# spelled differently among those); these two had no row, and both survived
# silently.
MUTANTS += [
    ("R45 scope: `_unkeyed` asks of the whole POPULATION (scope it to `main` -- a linked memo's "
     "unkeyed row goes unreported at rc 0)", POPULATION,
     '        for memo in self.memos:\n            self._unkeyed(memo)',
     '        for memo in self.memos[:1]:\n            self._unkeyed(memo)',
     [R45_UNKEYED_SCOPE]),
    ("R45 scope: the table-miss loop asks of the whole POPULATION (scope it to `main` -- a linked "
     "memo's width miss goes unreported at rc 0)", POPULATION,
     '        for memo in self.memos:\n            for t in memo.tables:\n'
     '                for lineno, msg in t.misses:',
     '        for memo in self.memos[:1]:\n            for t in memo.tables:\n'
     '                for lineno, msg in t.misses:',
     [R45_TABLE_MISS_SCOPE]),
]

MUTANTS += [
    ("R45 GFM §4.10: a delimiter cell is >=1 hyphen (re-inject a three-hyphen minimum -- the "
     "dialect the review finding asked for, which cmark-gfm refutes)", BLOCKS,
     '_DELIM_CELL = re.compile(r":?-+:?")',
     '_DELIM_CELL = re.compile(r":?---+:?")',
     list(R45_DELIM_ALL)),
]

MUTANTS += [
    ("R47-1 separator: the row-noun separator COMPOSES `DASH` (re-spell the narrower ASCII-only "
     "set -- an en/em-dash claim matches no appositive and the numeric id goes unreported at rc 0)",
     TABLES,
     'ROW_NOUN_SEP = ROW_NOUN + "(?:" + GAP + "|" + DASH_CLASS + ")+"',
     'ROW_NOUN_SEP = ROW_NOUN + "(?:" + GAP + "|-)+"',
     [R47_1_DASH_SET, R47_1_EN_DASH]),
    ("R47-2 gate: the unbound-claim gate asks BOTH readings (drop the disagreement arm -- a claim "
     "straddling a masked span is invisible to the stream and the table leaves the census at rc 0)",
     POPULATION,
     '        for name in kind_disagreements(cell.lexed):\n            hit[name] = True',
     '        pass',
     [R47_2_UNBOUND_STRADDLE]),
    ("R47-2 gate: the disagreement arm requires a STRADDLE (widen it to every disagreement -- a "
     "phrase quoted WHOLE becomes a claim, against I-A)", STREAM,
     '        if any(_straddles(blanks, m.start(), m.end()) for blanks, m in hit):\n'
     '            out.append(name)',
     '        if True:\n'
     '            out.append(name)',
     [R47_2_UNBOUND_QUOTED]),
]

MUTANTS += [
    ("R47-4 gap: a word gap is what a READER sees (re-spell the marker as a `re.escape`d literal -- "
     "U+0020 and nothing else, so `&nbsp;` drops the claim at rc 0)", STREAM,
     'MARKER_RE = re.compile(bounded(_phrase(MARKER)))',
     'MARKER_RE = re.compile(bounded(re.escape(MARKER)))',
     [R47_4_NBSP, R47_4_TAB]),
    ("R47-4 gap: the UNDETERMINED phrase composes the SAME gap (re-spell its `\\s` under `re.ASCII`, "
     "which is ASCII whitespace and not U+00A0)", STREAM,
     'bounded(phrase("KIND(?: " + DASH_CLASS + "?|" + DASH_CLASS + ")(?: )?UNDETERMINED"))',
     'bounded(r"KIND(?:\\s+" + DASH_CLASS + r"?|" + DASH_CLASS + r")\\s*UNDETERMINED")',
     [R47_4_UNDET_NBSP]),
    ("R47-4 fold: the UNDETERMINED phrase folds case in ASCII only (drop `re.ASCII` -- a letter folds "
     "from U+212A / U+0130 / U+0131, and the boundary admits U+017F too)", STREAM, "re.IGNORECASE | re.ASCII)", "re.IGNORECASE)",
     list(R47_4_UNDET_FOLD)),
    ("R47-4 gap: the gap is a WHITESPACE class, not a wildcard (widen it to `.` -- every arm above "
     "still passes, and only the word-boundary negative catches it)", STREAM,
     'GAP = UNICODE_WHITESPACE',
     'GAP = r"."',
     [R47_4_NON_WHITESPACE]),
]

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

ROLE_RANK_GAP = ("the role RANKING reads a reader's gap in its phrase words: `waits&nbsp;on` ranks "
                 "ordering and `placed&nbsp;on` owner, and neither across a U+000B")

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

# -- R47-5: the DERIVED scope ratchet's closure.  `population_scope_control`
# enumerates every `ast.For` in the census module and demands a mutant that
# TRUNCATES that loop; these are the rows it demanded.  Nine of the twelve it
# found were already covered by a control that goes red -- the ratchet's job
# was to say WHICH, measured one loop at a time, not to invent them -- and
# three needed a fixture that takes a second iteration.
MUTANTS += [
    ("R47-5 scope: the population walk queues EVERY link of a memo (truncate the loop)", POPULATION,
     '            for f in memo.linked_files():',
     '            for f in list(memo.linked_files())[:1]:',
     ["diagnostics name a memo relative to the root memo's directory: `a/child.md` and `b/child.md` are two files, and a memo outside that directory is named by its absolute path"]),
    ("R47-5 scope: every UNANSWERED reference of a memo is reported (truncate the loop)", POPULATION,
     '            for lineno, label in memo.unresolved_references():',
     '            for lineno, label in list(memo.unresolved_references())[:1]:',
     [R47_5_TWO_REFS]),
    ("R47-5 scope: every TABLE of a memo is bound (truncate the loop)", POPULATION,
     '            for t in memo.tables:\n                for lineno, msg in t.misses:',
     '            for t in list(memo.tables)[:1]:\n                for lineno, msg in t.misses:',
     ["(rc) a schema body row whose width differs from its header is rc 2"]),
    ("R47-5 scope: every MISS of a table is reported (truncate the loop)", POPULATION,
     '                for lineno, msg in t.misses:',
     '                for lineno, msg in list(t.misses)[:1]:',
     [R47_5_TWO_MISSES]),
    ("R47-5 scope: every declared id is given a KIND (truncate the loop)", POPULATION,
     '        for row in self.ids.values():',
     '        for row in list(self.ids.values())[:1]:',
     [R47_5_ALL_KINDS]),
    ("R47-5 scope: every SCHEMA ROW of a memo is declared (truncate the loop)", POPULATION,
     '            for row in memo.schema_rows(s.name):\n                rid = row.self_id',
     '            for row in list(memo.schema_rows(s.name))[:1]:\n                rid = row.self_id',
     ["a self-declaring row that MENTIONS a sibling stays in the population"]),
    ("R47-5 scope: every TABLE of a memo is asked for an unbound claim (truncate the loop)", POPULATION,
     '            for t in memo.tables:\n                if t.schema is not None:',
     '            for t in list(memo.tables)[:1]:\n                if t.schema is not None:',
     [R47_5_SECOND_TABLE]),
    ("R47-5 scope: every ROW of an unbound table is asked (truncate the loop)", POPULATION,
     '                for row in [t.header] + t.rows:',
     '                for row in list([t.header] + t.rows)[:1]:',
     [R42_10_UNBOUND_CLAIM]),
    ("R47-5 scope: every SCHEMA ROW is read for the unkeyed miss (truncate the loop)", POPULATION,
     '            for row in memo.schema_rows(s.name):\n                if row.self_id is not None:',
     '            for row in list(memo.schema_rows(s.name))[:1]:\n                if row.self_id is not None:',
     [R47_5_SECOND_ROW]),
    ("R47-5 scope: every kind phrase the residue names is reported (truncate the loop)", POPULATION,
     '        for name in kind_disagreements(row.cells[row.schema.decl].lexed):',
     '        for name in list(kind_disagreements(row.cells[row.schema.decl].lexed))[:1]:',
     [R47_5_TWO_PHRASES]),
]

REFUSED_SILENCE = ("a destination the sibling resolver REFUSES leaves no finding, seed or note "
                   "naming it -- the stated policy, pinned rather than merely current")

# -- R47-6: the refusal POLICY, in both directions.  The first mutant reverses
# the polarity (a refused destination reports); the second breaks the
# discriminating half (nothing is walked at all), which is what says the
# control's silence half is not vacuous.
MUTANTS += [
    ("R47-6 refusal: a REFUSED destination stays silent (reverse the polarity -- report it, the "
     "reading §1's could-not-scan rule would ask for and the one §8 carries as open)", MEMO,
     '                f = sibling_path(self.path.parent, dest)\n'
     '                if f is not None and f not in seen:',
     '                f = sibling_path(self.path.parent, dest)\n'
     '                if f is None:\n'
     '                    f = self.path.parent / dest\n'
     '                if f is not None and f not in seen:',
     [REFUSED_SILENCE]),
    ("R47-6 refusal: ... and the ORDINARY sibling IS walked (drop every link -- the half that says "
     "\"nothing names these\" is not also true of a checker that reads no destinations)", MEMO,
     '                if f is not None and f not in seen:',
     '                if False and f not in seen:',
     [REFUSED_SILENCE]),
]

MUTANTS += [
    ("R48-2 spellings: `_phrases` returns EVERY occurrence (re-inject `search` -- one Match per "
     "phrase, so a field using both undetermined spellings contributes one and the consistency "
     "gate sees a set of size one)", POPULATION,
     '        return {name: list(rx.finditer(field or "")) for name, rx in KIND_PHRASES}',
     '        return {name: [m] if m else [] for name, m in\n'
     '                ((n, rx.search(field or "")) for n, rx in KIND_PHRASES)}',
     [R48_2_TWO_SPELLINGS]),
    ("R48-2 spellings: every collected occurrence reaches `spellings` (truncate the loop -- the "
     "shape the derived scope ratchet demanded a row for the moment the loop was written)",
     POPULATION,
     '        for _m in hit["undetermined"]:\n            self.spellings.add(_m.group(0))',
     '        for _m in hit["undetermined"][:1]:\n            self.spellings.add(_m.group(0))',
     [R48_2_TWO_SPELLINGS]),
]

MUTANTS += [
    ("R51 claims: EVERY disagreement the canonical question returns is marked (truncate the loop "
     "inside `_claims` -- the one the ratchet credited to a mutant that never touched it, because "
     "it matched a target PREFIX)", POPULATION,
     '        for name in kind_disagreements(cell.lexed):\n            hit[name] = True',
     '        for name in list(kind_disagreements(cell.lexed))[:1]:\n            hit[name] = True',
     [R51_TWO_STRADDLES]),
]

MUTANTS += [
    ("R52 outside-field: the marker-outside-the-declaring-field check asks the CANONICAL question "
     "(re-inject the stream-only search -- a marker split across a masked construct certifies "
     "nothing and nothing says so)", ROLES,
     '        if any(pop._claims(c).get("marker")',
     '        if any(__import__("plan_memo_stream").MARKER_RE.search(\n'
     '                   __import__("plan_memo_stream").stream(c.lexed))',
     [R52_OUTSIDE_STRADDLE]),
]

GAP_SWEEP = ("PROPERTY: the checker spells a reader's gap ONE way -- no string literal holds `\\s` / `\\S`, "
             "no `.isspace()` is called, and no pattern literal handed to `re` holds a U+0020 outside a "
             "character class unless it went through `plan_memo_stream.phrase` (unknown sites are red; the "
             "exemptions are keyed on function and expression)")

# -- the R22 pre-push review: the two readings that were right and unpinned
# (the file-name boundary is Python's set on purpose; §6.2 reads §2.1), and the
# sweep that makes "one spelling of a gap" a check rather than a sentence.
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
    ("R22 sweep: a `\\s` planted in a vocabulary is red (re-spell `naming`'s gap as `\\s+`)", ROLES,
     '    r"naming ",', '    r"naming\\s+",',
     [GAP_SWEEP]),
    ("R22 sweep: a NEW `.isspace()` in an exempt function is red -- the exemption is keyed on the "
     "expression, not the function or the line (add a second, equivalent call)", TOKENS,
     '    while j < n and not (text[j].isspace() or text[j] in _NAME_BOUNDARY):',
     '    while j < n and not (text[j].isspace() or text[j:j + 1].isspace() or text[j] in _NAME_BOUNDARY):',
     [GAP_SWEEP]),
    ("R22 sweep: a literal U+0020 word gap in a pattern that bypasses `phrase` is red (add `not landed` "
     "to `RETIRED`)", ROLES,
     r'RETIRED = re.compile(r"\bMERGED\b|\bRETIRED\b|\bLANDED\b", re.ASCII)',
     r'RETIRED = re.compile(r"\bMERGED\b|\bRETIRED\b|\bLANDED\b|\bnot landed\b", re.ASCII)',
     [GAP_SWEEP]),
]
