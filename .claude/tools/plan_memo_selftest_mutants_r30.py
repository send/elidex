#!/usr/bin/env python3
"""PR #510 R30-on mutation-proof rows -- the FIFTH module of the ONE `MUTANTS`
list, carved at the same review-round seam the registry has now used four times
(`plan_memo_selftest_mutants` holds the row shape, `run` and the pre-converge
rows; `_pr510` R1-R16 and the design re-gates; `_inline` R17-R25; `_r26` R26-R29).

⚠ TOUCH-TIME, AND ONE COMMIT LATE (PR #510 Axis 3 + Axis 5).  `_r26.py` reached
961 lines, and the §8 entry that recorded the debt asserted "the touch this PR
gave `_mutants_r26.py` was a re-point of two rows, not growth" -- in the COMMIT
THAT APPENDED TWO ROWS TO IT (`b8324d06`, +22 lines, 939 -> 961).  The trigger
that entry set ("the next commit that adds a mutant to it splits it FIRST") had
therefore already fired when it was written.  Two independent review axes
measured that, one commit apart.  The split is taken here, the §8 entry is gone,
and the invariant is a CONTROL now rather than a sentence
(`line_bound_control`), because a prose rule that its own author broke inside
one commit is the case for a mechanism, not for more care.

THE SEAM, unchanged from the four before it: a review ROUND.  `_r26.py` keeps
R26-R29 -- the operating envelope (encoding, host streams), the file-name
token's balanced-pair rule, `inline_pass`'s linear contract, the row-noun and
straddle properties, the population walk, the module map, the id scan and the
§4.6 tag list.  This module holds R30 on: the id cell's blank test, the raw-seed
and licensing linearity families, the code-span reading, the import seams and
the attribution sweep, the dash class, the run-agreement property, the R38
re-gate's second site and the Axis 5 report channel.

`R31_EMPHASIS_LINEAR` moved here with the split: it was declared in `_r26.py`
and named by exactly one row, which is in this half (measured, not assumed --
the seam check reported it as the ONE name crossing the boundary, and moving it
leaves ZERO).

`MUTANTS` is imported and appended to, exactly as the other four do; the runner
reads the one list at one import site.
"""

from plan_memo_selftest_cases_r26 import ( R30_3_DEMOTED_TAIL, R30_3_IMAGE_OPENER,
    R30_3_LINK_OPENER, R30_3_LOUD_MISS, R30_CODE_SPAN_READING, R30_KEYED, R30_PAIRED,
    R30_UNPAIRED, R31_1_DOLLAR_UNBOUND, R31_1_RENDERED_HEADER, R33_1_LONGER_WORD,
    R33_1_NOVEL_PREFIX, R33_1_REAL_NOUN, R33_2_EN_DASH, R33_2_NON_DASH, R34_1_CONTINUES,
    R34_1_FRAGMENT, R34_1_TRAILING, R34_2_BLANKS, R34_2_MASKED, R35_FRAGMENT_ID, R35_QUERY_ID,
    R38_CD_BLANK, R38_CD_MASKED,
)
from plan_memo_selftest_cases_r42 import ( R42_10_UNBOUND_CLAIM, R42_10_UNBOUND_ID_SHAPED,
    R42_10_UNBOUND_NO_CLAIM, R42_10_UNBOUND_RENDERED, R42_8_HTML_ALT, R42_8_HTML_ATTR_SITE,
    R42_8_HTML_NO_SEED_CELL, R42_8_HTML_NO_SEED_PROSE, R42_9_UNDET, R42_ALLSPACE,
    R42_BLANK_MARKER, R42_IMG_AUTO, R42_IMG_CODE, R42_LINE_ENDING_FIRST, R42_TRIM,
    R45_DELIM_ALL, R45_SHORT_DELIM, R45_TABLE_MISS_SCOPE, R45_UNKEYED_SCOPE,
    R47_1_DASH_SET, R47_1_EN_DASH, R47_2_UNBOUND_QUOTED, R47_2_UNBOUND_STRADDLE,
    R47_4_BASELINE, R47_4_BOUNDARY, R47_4_NBSP, R47_4_NON_WHITESPACE, R47_4_TAB,
    R47_4_UNDET_NBSP, R47_5_ALL_KINDS, R47_5_SECOND_ROW, R47_5_SECOND_TABLE,
    R47_5_TWO_MISSES, R47_5_TWO_PHRASES, R47_5_TWO_REFS,
)
from plan_memo_selftest_mutants import (
    BLOCKS, CHECK, CONFORMANCE, CONTROLS, EMPHASIS, GROWTH, HTML, IDS, INLINE_EXAMPLES, LEXER, LINKS,
    MEMO, MUTANTS, POPULATION, PROPERTIES, R27_GROWTH, RECORDS, ROLES, RUNNER, SIBLING, STREAM,
    TABLES, TOKENS,
)

R31_EMPHASIS_LINEAR = ("emphasis matching is linear: N unmatched delimiter runs cost O(N) work (the "
                       "Appendix's openers_bottom)")

# -- R30: the id cell's blank test.  FOUR rows, because the fix has four
# separable clauses and each one alone is a different silent skip:
#   (a) the decoration is not stripped off the raw text any more,
#   (b) the text asked is a RENDERING and not the raw cell,
#   (c) the rendering asked is the READER's and not the disposed stream,
#   (d) the question is asked only of the rows that declared nothing.
# (a) and (b) look like one clause and are not: (a) alone still calls `**` a
# blank if the reading goes back to the raw cell, and (b) alone still calls
# `**—**` unkeyed if the strip is re-injected on top of the reading.  Each
# row below names only the controls it actually reddens.
MUTANTS += [
    ("R30 id cell: the decoration strip is GONE (re-inject it on the reading -- the reported defect, "
     "which the reading alone does not fix)", TABLES,
     '    return cell_reading.strip(" \\t") in ID_CELL_BLANKS',
     '    return cell_reading.strip(" \\t").strip("*`").strip(" \\t") in ID_CELL_BLANKS',
     list(R30_UNPAIRED)),
    ("R30 id cell: the blank question is asked of a RENDERING, not of the raw cell (the direction the "
     "strip hid: a construct that RENDERS a blank)", POPULATION,
     '                if not is_blank_id_cell(stream(row.cells[s.idc].lexed, reader=True)):',
     '                if not is_blank_id_cell(row.cells[s.idc].text):',
     list(R30_PAIRED)),
    ("R30 id cell: the rendering is the READER's, not the disposed stream (which blanks a code span, "
     "so `` `?` `` would read as empty)", POPULATION,
     'is_blank_id_cell(stream(row.cells[s.idc].lexed, reader=True))',
     'is_blank_id_cell(stream(row.cells[s.idc].lexed))',
     [R30_CODE_SPAN_READING]),
    ("R30 id cell: `_unkeyed` judges only the rows that declared nothing (drop the guard: every keyed "
     "row is asked the blank question too)", POPULATION,
     '                if row.self_id is not None:\n                    continue',
     '                if False:\n                    continue',
     [R30_KEYED]),
]

# -- PR #510 Codex R31-2: a matched pair's cleared delimiters stayed ON the
# stack, so every later opener search walked them again and every later match
# re-marked the ones inside its own range.  The mutation re-injects exactly
# that -- mark dead IN PLACE, never unlink -- which leaves every correctness
# control green (the pairs are identical) and only the cost moves.  ⚠ The
# generated growth property does NOT kill this one: its atoms are single
# characters and self-contained constructs, so a delimiter atom repeated
# merges into one long run rather than the N separate runs this shape needs
# (measured GREEN over all 3,577 probes with the defect re-injected).  That is
# why the control here is the hand-written NESTED probe, and why both facts
# are written down rather than left for the next round to rediscover.
MUTANTS += [
    ("R31-2 §6.2: a cleared delimiter is REMOVED from the stack, not flagged in place (re-inject the "
     "in-place mark: every later search walks the dead ones again)", EMPHASIS,
     '    p, q = prv[k - bottom], nxt[k - bottom]\n'
     '    if p >= bottom:\n'
     '        nxt[p - bottom] = q\n'
     '    if q - bottom < len(prv):\n'
     '        prv[q - bottom] = p\n'
     '    delims[k].dead = True',
     '    delims[k].dead = True',
     [R31_EMPHASIS_LINEAR]),
]


# -- PR #510 Codex R30-3: three pieces of markup inside a resolved image's
# description, each disposed of wrongly and each with its own mutant, because
# each probe moves under exactly one of them (measured: every other pair
# survives).  §6.4 reduces a description to the plain string content of its
# inline children -- the children's TEXT, none of their markup.
MUTANTS += [
    ("R30-3 §6.4: a demoted link's `[` stays a mark (re-inject the pop: the bracket stands in the "
     "stream and a phrase crossing it is not there)", LEXER,
     '                images.append(out.pop()[:2] + ("demoted",))\n',
     '                images.append(out.pop()[:2] + ("demoted",))\n                opens.pop()\n',
     [R30_3_LINK_OPENER]),
    ("R30-3 §6.4: a DEMOTED construct's extent renders NOTHING, so it disposes as a mark and not as "
     "a blank (put the blank back: the two sides of a demoted tail become two runs)", STREAM,
     'base += [(a, b, "mark" if k == "demoted" else "image") for a, b, k in lx.images]',
     'base += [(a, b, "image") for a, b, _k in lx.images]',
     [R30_3_DEMOTED_TAIL]),
    # ONE edit, TWO controls, and they are the two DIRECTIONS of the same
    # clause: with the opener unrecorded a phrase that crosses a nested image's
    # `![` reads as text that is not there (the first), and a phrase that
    # straddles a top-level image's `![` stops straddling anything, so the
    # near-miss the checker owes its reader is not reported at all (the
    # second).  A row naming only the first would leave the loud half -- which
    # is the half that keeps a real miss from exiting 0 -- unwitnessed.
    ("R30-3 §6.4: a RESOLVED image's own `![` is recorded (drop the record: it stands in the stream "
     "as literal text, where a reader sees either a picture or nothing at all)", LEXER,
     '            images.append((pos - 1, pos + 1, "open"))\n', '',
     [R30_3_IMAGE_OPENER, R30_3_LOUD_MISS]),
]


# -- PR #510 Codex R31-1: a table's schema is what its header RENDERS.  Two
# clauses, two rows: the comparison itself, and the phase order that makes a
# rendering available to it.  Both land on the same control, because both
# leave the linked memo's table unbound and its whole population silently
# outside the run -- which is the finding.
MUTANTS += [
    ("R31-1 §2.5: a table's schema is matched on what its header RENDERS (compare the RAW cell text: "
     "a `&#35;` header is no schema and the memo's rows leave the run at rc 0)", TABLES,
     "        rendered_header = [rendered(c.lexed) for c in self.header.cells]",
     "        rendered_header = [c.text for c in self.header.cells]",
     [R31_1_RENDERED_HEADER]),
    ("R31-1 phase order: the header cells are lexed BEFORE the tables bind (drop the pre-resolve: "
     "`rendered` then reads a cell that has not been through the inline pass, which is its raw text "
     "again -- the same silent skip by the other clause)", MEMO,
     "        for t in self.tables:\n"
     "            for cell in t.header.cells:\n"
     "                cell.lexed.resolve(self.defs)\n"
     "            t.bind()\n",
     "        for t in self.tables:\n            t.bind()\n",
     [R31_1_RENDERED_HEADER]),
]


# -- PR #510 Codex R31-3 and R31-4: two quadratic scans, each re-injected
# here.  ⚠ BOTH SURVIVED THE WHOLE SUITE BEFORE THEIR CONTROLS WERE WRITTEN --
# measured, with each defect put back into the checker on disk and the full
# self-test run: 0 failures, the generated growth property included.  So this
# is not a pair of rows confirming what a sweep already saw; it is the proof
# that the two new work controls are the only thing watching these clauses.
R31_RAW_SEED_LINEAR = ("the always-run raw-content seed is linear: N id tokens and N file names on one "
                       "raw line are one pass, not a span sum per token")
R31_LICENCE_LINEAR = ("the licensing rule's backward look is linear: N mentions hand LICENSE_BEFORE "
                      "O(N) characters in all, not one growing prefix each")

MUTANTS += [
    ("R31-3 raw seed: a token asks the ONE span that can reach it (re-inject the sum over every span: "
     "N tokens x N file names on one line)", CHECK,
     "and not covers(spans, t.idstart, t.idend)",
     "and not any(a < t.idend and t.idstart < b for a, b, _ in spans)",
     [R31_RAW_SEED_LINEAR]),
    ("R31-4 licensing: the backward look starts at the last offset a phrase may begin at (re-inject "
     "R24's search from the block's start: one growing prefix per mention)", ROLES,
     "    starts = m.licence\n"
     "    j = bisect.bisect_left(starts, m.start) - 1\n"
     "    m.licensed = bool((j >= 0 and LICENSE_BEFORE.match(m.text, starts[j], m.start))\n"
     "                      or LICENSE_AFTER.match(m.text, m.end))",
     "    m.licensed = bool(LICENSE_BEFORE.search(m.text, 0, m.start)\n"
     "                      or LICENSE_AFTER.match(m.text, m.end))",
     [R31_LICENCE_LINEAR]),
    # The cheapest wrong answer, and the row that says the upper bound alone
    # does not report it: an index that offers NO candidate makes every
    # backward span zero, licenses nothing, and passes any bound on the
    # characters handed to the pattern.  It is the licensing rule's
    # `return False` -- the same shape `linear_html_attempts_control` had to
    # add a lower bound for at R26-3.
    ("R31-4 licensing: the index offers the candidates rather than none (return an empty index: the "
     "cheapest possible bound, which licenses nothing and passes every upper bound)", ROLES,
     "    return [m.start() for m in _LICENCE_KEYWORD.finditer(text)]",
     "    return []",
     [R31_LICENCE_LINEAR]),
]


# -- PR #510 R31-4, the correctness half: the index the backward look is
# bounded by.  Three rows, one per way the argument it rests on can fail --
# a keyword that leaves the set, a literal the set holds only a truncation of,
# and the reachability claim itself (the LAST candidate before the mention is
# the only one that can reach it).
R31_LICENCE_INDEX = ("PROPERTY: the licensing rule's backward index decides what the whole preceding "
                     "text decides, at every position of a generated corpus -- and every phrase opens "
                     "with a literal the index holds")

MUTANTS += [
    # ⚠ BOTH ROWS MOVED AT R32, when the derivation became branch-aware and their
    # substrings stopped existing. The runner said so ("the mutant no longer
    # applies and proves nothing") rather than quietly passing -- the R22
    # `STAGE_C` rule holding for a third time.
    ("R31-4 index: every branch's literal is IN the index (drop one keyword: that phrase's sites "
     "leave the index silently and stop being licensable)", ROLES,
     '            if m:\n                out.add(m.group())',
     '            if m and m.group() != "mint":\n                out.add(m.group())',
     [R31_LICENCE_INDEX]),
    ("R31-4 index: the index keys on a branch's WHOLE literal prefix (key on the first four letters: "
     "`deri` is a superset that still matches, so only the structural half reports it)", ROLES,
     '            m = re.match(r"[a-z]+", branch)',
     '            m = re.match(r"[a-z]{1,4}", branch)',
     [R31_LICENCE_INDEX]),
    # The QUANTIFIER hazard, and the reason (a) is not merely a restatement of
    # the derivation: `mints?` matches `mint ` while the greedy literal run the
    # index keys on is `mints`, so every site spelling the shorter form leaves
    # the index -- a phrase re-spelled this way silently narrows what the
    # checker can license.  Both halves of the control report it, and that is
    # on purpose: the corpus holds `mint ` so the ORACLE sees this one, while
    # (a) is what covers the same re-spelling of a phrase no fragment builds.
    ("R31-4 index: no phrase quantifies the last character of its own literal (re-spell `mint(?:s|ed|"
     "ing)?` as `mints?(?:ed|ing)?`: the index keys on `mints` and `mint ` stops being licensable)",
     ROLES,
     r'    r"mint(?:s|ed|ing)?\s+(?:onto\s+)?",     # ... mints / minted / minting X',
     r'    r"mints?(?:ed|ing)?\s+(?:onto\s+)?",     # ... mints / minted / minting X',
     [R31_LICENCE_INDEX]),
    # The reachability claim, and the one row that tests it rather than the
    # index's contents: with the FIRST candidate taken instead of the last,
    # `children of children of 9z` is read from the outer `child`, where the
    # phrase does not reach the mention -- so a licensed site is reported.
    ("R31-4 index: the LAST candidate before the mention is the one applied (take the first: a match "
     "beginning earlier cannot reach the mention, which is the whole argument)", ROLES,
     "    j = bisect.bisect_left(starts, m.start) - 1\n"
     "    m.licensed = bool((j >= 0 and LICENSE_BEFORE.match(m.text, starts[j], m.start))",
     "    j = 0 if starts and starts[0] < m.start else -1\n"
     "    m.licensed = bool((j >= 0 and LICENSE_BEFORE.match(m.text, starts[j], m.start))",
     [R31_LICENCE_INDEX]),
]



# -- PR #510 Codex R32: §6.1 is TWO steps and the order is the rule.  The
# control is the spec's own §6.1 list rather than the probe the round arrived
# with, because what the probe exposed was the absence of the FIRST step, which
# reaches every multi-line code span -- so the mutant removes that step and the
# report names Examples 335 / 336 / 337 rather than the reported shape.
R32_CODE_READING = ("CommonMark 0.31.2 §6.1: a code span READS as the text the spec's own html puts "
                    "inside `<code>` (line endings converted, then the one-space trim)")

MUTANTS += [
    ("R32 §6.1: a code span's line endings are converted to spaces BEFORE the one-space trim (drop the "
     "conversion: the raw line ending stands in the reader's text and the trim never fires)", STREAM,
     '        body = body.replace("\\r\\n", " ").replace("\\r", " ").replace("\\n", " ")\n',
     '',
     [R32_CODE_READING]),
]



# -- PR #510 R32 design re-gate: the two claims that were offered as mechanical
# and were not.  Both rows patch a SELF-TEST module, so the control comes from
# the patched text.
R32_SEAMS = ("PROPERTY: every import-graph seam this suite states in prose is true of the imports "
             "(an \"only importer of X\" is a claim about the COMPLEMENT)")
R32_ATTRIB = ("PROPERTY: every written `module.symbol` attribution names the module that DEFINES that "
              "symbol (the class five touch-time splits left with no detector)")

MUTANTS += [
    ("R32 seams: a seam's allow-list is the set that may import it (widen one to every module: an "
     "\"only importer\" nobody can violate is a sentence about nothing)", RECORDS,
     '    "ast": ("ast", {"plan_memo_selftest_properties.py", "plan_memo_selftest_growth.py",\n'
     '                    "plan_memo_selftest_records.py"}, None),',
     '    "ast": ("ast", set(), None),',
     [R32_SEAMS]),
    # The OTHER direction, and the one the first row cannot report: a seam whose
    # names nothing imports passes any allow-list, so the control needs the
    # emptiness check that makes a vacuous seam red.
    ("R32 seams: a seam whose names NOTHING imports is red (rename one to a name no module imports: "
     "the allow-list is satisfied vacuously)", RECORDS,
     '    "the fixture runner": (("run_on",),',
     '    "the fixture runner": (("run_on_nothing_imports_this",),',
     [R32_SEAMS]),
    # ⚠ THE SUBJECT IS THE ATTRIBUTION, NOT THE ORACLE.  Written first as a patch
    # to the control's own map ("read the attribution's own module back as the
    # answer"), this mutant SURVIVED -- correctly, and not because the control is
    # weak: an oracle rewritten to agree with its subject is a degeneracy no
    # differential control can catch, since the comparison it would make is the
    # one that was removed.  A mutant against a control must move the thing the
    # control READS.  So it puts back one of the stale attributions the class is
    # about, in a checker source, and the control must find it.
    ("R32 attribution: a stale `module.symbol` is found (put back the pre-split spelling of the "
     "disposition, which moved to `plan_memo_stream.py` at `e7b49ed4`)", TOKENS,
     "(`plan_memo_stream.dispose`, stage 2) hands it the",
     "(`plan_memo_tables.dispose`, stage 2) hands it the",
     [R32_ATTRIB]),
]


MUTANTS += [
    # The hole R32's design re-gate measured: one TUPLE ENTRY holding two
    # branches. Before the derivation read branches, `sprout` never entered the
    # index, `sprout 9z` stopped being licensable, and BOTH halves of the
    # control passed -- the structural half because the entry does open with a
    # literal, the oracle half because no generated fragment spells `sprout`.
    ("R32 index: the keyword set is derived per BRANCH, not per tuple entry (collapse the split: an "
     "entry holding two alternatives indexes only the first)", ROLES,
     "    out.append(pattern[start:])\n    return out",
     "    out.append(pattern[start:])\n    return [pattern]",
     [R31_LICENCE_INDEX]),
]



# -- PR #510 Codex R33.  ⚠ R33-1's polarity is FABRICATION, not silence: without
# the boundary the checker emitted a mechanical failure the document does not
# support, which is the direction a reviewer of the REPORT cannot catch.
R33_DASH_SWEEP = ("PROPERTY: the separator dash set is spelled once, in plan_memo_ids.py (the class "
                  "three readers spelled three ways, disagreeing)")

MUTANTS += [
    ("R33-1 appositive: the row noun carries a LEFT boundary (drop `BEFORE`: the search starts inside "
     "a longer word and the containing row is read as a pointer)", TABLES,
     '_APPOSITIVE = re.compile(BEFORE + ROW_NOUN_ID',
     '_APPOSITIVE = re.compile(ROW_NOUN_ID',
     [R33_1_LONGER_WORD, R33_1_NOVEL_PREFIX]),
    # The OTHER direction, and the row the first one cannot report: a boundary
    # that refuses everything passes both negatives above and reports nothing at
    # all, which is the cheapest wrong answer here.
    ("R33-1 appositive: the boundary admits a REAL row noun (refuse every appositive: the two "
     "negatives stay green and the attribution stops happening)", TABLES,
     '_APPOSITIVE = re.compile(BEFORE + ROW_NOUN_ID',
     '_APPOSITIVE = re.compile(BEFORE + "(?!)" + ROW_NOUN_ID',
     [R33_1_REAL_NOUN]),
    ("R33-2 dashes: the set holds the EN dash (drop it: `KIND – UNDETERMINED` reads as terminal and "
     "the row's `Deps` is never asserted)", IDS,
     'DASH = "\\u2014\\u2013-"', 'DASH = "\\u2014-"',
     [R33_2_EN_DASH]),
    ("R33-2 dashes: the set is the three these documents use, not any punctuation (widen it: a `/` "
     "separator would declare a kind)", IDS,
     'DASH = "\\u2014\\u2013-"', 'DASH = "/\\u2014\\u2013-"',
     [R33_2_NON_DASH]),
    ("R33-2 sweep: a second dash class in a checker module is found (re-inject one at the reader that "
     "had it)", STREAM,
     'UNDETERMINED = re.compile(bounded("KIND" + GAP + "*" + DASH_CLASS + "?" + GAP + "*UNDETERMINED"),',
     'UNDETERMINED = re.compile(bounded("KIND" + GAP + "*[\\u2014-]?" + GAP + "*UNDETERMINED"),',
     [R33_DASH_SWEEP]),
]



# -- PR #510 Codex R34-2: which reading assertion (b) asks the `Deps` cell.
MUTANTS += [
    ("R34-2 Deps: emptiness is asked of the READER's rendering (re-inject the prose-scanning stream: "
     "a cell of only masked syntax reads empty and the row's edge is never reported)", ROLES,
     '        if not is_empty(stream(row.col("Deps").lexed, reader=True)):',
     '        if not is_empty(_stream(row, "Deps")):',
     R34_2_MASKED),
    # The OTHER direction.  ⚠ Written first as "read the RAW cell", this
    # SURVIVED -- correctly: `is_empty` decides by SHAPE, so a raw `\u2014` is as
    # empty as a rendered one and the blanks never move.  The clause these three
    # controls actually guard is the SHAPE rule, so that is what the mutant
    # removes: anything the reading leaves standing then counts as an edge, and
    # every deliberate blank is reported as carrying one.
    ("R34-2 Deps: emptiness is judged by SHAPE, not by 'the reading left something' (drop the shape "
     "rule: a cell a reader sees a dash in is reported as carrying an edge)", ROLES,
     '        if not is_empty(stream(row.col("Deps").lexed, reader=True)):',
     '        if stream(row.col("Deps").lexed, reader=True).strip():',
     R34_2_BLANKS),
]



# -- PR #510 Codex R34-1: the three tails, three rows.  Each names the control
# that is the ONLY one its edit moves.
MUTANTS += [
    ("R34-1 file token: the suffix TERMINATES the run (re-inject the alphanumeric test: a prefix of a "
     "longer run is masked and the ids in it are hidden)", TOKENS,
     "    if e < n and text[e] in \"#?\":",
     "    if e < n and not _ALNUM_AT.match(text, e):\n        return e\n    if False:",
     [R34_1_CONTINUES]),
    ("R34-1 file token: a FRAGMENT tail is still a name (drop the `#?` arm: the resolver follows that "
     "run and the lexer breaks it into pieces -- the one direction the correspondence forbids)", TOKENS,
     "    if e < n and text[e] in \"#?\":",
     "    if e < n and False:",
     [R34_1_FRAGMENT]),
    ("R34-1 file token: a TRAILING-PUNCTUATION tail is still a name (empty the set: a sentence-final "
     "period stops ending a file name)", TOKENS,
     # ⚠ the set was re-derived at R38 (GFM attribution dropped, `;` added, `"`
     # restored), so this row's substring moved with it.
     "_TRAILING = frozenset(\".,;:!?)'\\\"\")",
     "_TRAILING = frozenset()",
     [R34_1_TRAILING]),
]



# -- PR #510 Codex R35: the tail is part of the NAME, not permission to stop.
R35_RUN_AGREEMENT = ("PROPERTY: a run the sibling resolver FOLLOWS is one whole file span to the "
                     "lexer -- never a prefix with the remainder left for the naming scan (the "
                     "direction the correspondence forbids)")

MUTANTS += [
    ("R35 file token: a fragment/query tail EXTENDS the span (stop at the suffix instead: the id in "
     "the tail is left for the naming scan)", TOKENS,
     "    if e < n and text[e] in \"#?\":",
     "    if e < n and text[e] in \"#?\":\n        return e\n    if False:",
     [R35_FRAGMENT_ID, R35_QUERY_ID, R35_RUN_AGREEMENT]),
    # ⚠ A ROW FOR "the tail's own trailing punctuation is not part of the name"
    # WAS WRITTEN AND RETIRED (R35), and the reason recorded for it was WRONG
    # when written (R38 re-gate). It said "a period is not an id", but
    # `_TRAILING` then also held `*`, `_` and `~`, which ARE decoration
    # characters, and a decorated id token's extent BEGINS at its decoration --
    # so over `notes.md#**`9z`` the clause flips the id from reported to masked
    # at the unit level. The retirement still stands (1,152 generated probes
    # through the pipeline found no difference, so it is cosmetic to every
    # predicate here), and it is true NOW for a reason no one intended: R38
    # narrowed `_TRAILING` to `.,;:!?)'"`, which holds no decoration character
    # at all. ⚠ The honest statement is the narrow one -- "no `_TRAILING`
    # character can begin an id core, and `covers` is an overlap test" -- not
    # "a period is not an id". A conclusion that survives its reason being
    # false is this PR's own recurring shape, recorded rather than smoothed.
]



# -- PR #510 Codex R36-2: the run boundary is a fact of the TEXT, computed once
# per run, not once per suffix asking.
MUTANTS += [
    ("R36-2 file token: the run boundary is REUSED across the suffixes of one run (pass 0 as the "
     "cache: every suffix re-walks to the same boundary)", TOKENS,
     "            run_end = _run_end_from(text, e, n, run_end)",
     "            run_end = _run_end_from(text, e, n, 0)",
     ["file_and_cite_spans is linear: N parenthesis groups are one pass, not a re-scan from every "
      "start position"]),
]



# -- R38 design re-gate: the obligation's SECOND site.
MUTANTS += [
    ("R38 seed: the cd-seed asks Deps emptiness of the READER's rendering, like assertion (b) "
     "(re-inject the prose stream at the site R34-2 left behind)", ROLES,
     '        empty = is_empty(stream(row.col("Deps").lexed, reader=True))',
     '        empty = is_empty(_stream(row, "Deps"))',
     R38_CD_MASKED),
    ("R38 seed: emptiness is still SHAPE (drop the shape rule here too: a deliberate blank stops "
     "being one)", ROLES,
     '        empty = is_empty(stream(row.col("Deps").lexed, reader=True))',
     '        empty = not stream(row.col("Deps").lexed, reader=True).strip()',
     [R38_CD_BLANK]),
]


# PR #510 Axis 5: the report channel.  TWO rows, one per HALF of the escape --
# dropping the C0 arm is the defect that was found (a control name carrying a
# literal NUL, silently stripped by the wire's `$(...)`), and dropping the
# "leave a printable character alone" half is the over-escape nobody would
# notice from a green run.
AXIS5_PRINTABLE = ("the runner's report channel escapes every C0 control character and DEL, so a run "
                   "line a control names with one is still greppable")

MUTANTS += [
    ("Axis 5 report channel: drop the C0 arm of the escape (only DEL is escaped: the NUL a control "
     "name carries reaches the wire's command substitution and is stripped there)", CHECK,
     'if c < " " or c == "\\x7f" else c',
     'if c == "\\x7f" else c',
     [AXIS5_PRINTABLE]),
    ("Axis 5 report channel: escape EVERYTHING (a printable line is mangled too: a green run over an "
     "unreadable report is the other direction of the same defect)", CHECK,
     'if c < " " or c == "\\x7f" else c',
     'if True else c',
     [AXIS5_PRINTABLE]),
]


# PR #510 Axis 3 + Axis 5: the 1000-line touch-time bound, as a mechanism.
# ⚠ THE SUBJECT MUST BE WHAT THE CONTROL READS, which is a swept source's LINE
# COUNT -- not the control's own threshold.  Mutating the constant would prove
# only that the control reads its own constant
# (`memory/feedback_surviving-mutation-means-the-probe-has-another-subject.md`),
# so this row grows a real file past the bound instead: 400 line endings inside
# the lexer's module docstring carry it past 1000.  ⚠ The figure the row used to
# name (961 -> 1,011) went stale the moment §6.3's grammar was carved out at
# R42-7 and the lexer became 714 lines -- so the row states the INJECTION and
# the bound, never the file's current size. The text still parses, so
# `load()` execs it and the control sees the patched source through `SOURCES`.
AXIS5_LINE_BOUND = ("PROPERTY: every source of this checker is under the 1000-line touch-time bound "
                    "(the invariant this PR broke, as a mechanism instead of a sentence)")

MUTANTS += [
    ("Axis 5 line bound: a source crosses 1000 lines (grow the lexer's docstring by 400 line "
     "endings, whatever its current size: the debt the touch-time rule exists to stop)", LEXER,
     '"""Phase 2 of CommonMark 0.31.2 "Appendix: A parsing strategy" -- INLINE',
     '"""' + "\n" * 400 + 'Phase 2 of CommonMark 0.31.2 "Appendix: A parsing strategy" -- INLINE',
     [AXIS5_LINE_BOUND]),
]


# PR #510 Axis 5's CRIT, as a row: the attack that proved the escape's own
# control has the wrong subject.  ⚠ Axis 5 APPLIED this by hand (delete the
# `printable(` call sites, keep the function) and the whole suite stayed green
# while a raw NUL returned to the log.  Two rows, one per report module, because
# the two channels are two files and a fix to one says nothing about the other.
AXIS5_CHANNEL = ("PROPERTY: every line the run REPORTS goes through the escape -- measured over the "
                 "EMIT SITES, the subject the escape function's own control cannot reach")

MUTANTS += [
    ("Axis 5 channel: unwrap the control runner's per-control line (the exact attack that stayed "
     "green before this control existed: the escape function is untouched, only its caller drops it)",
     RUNNER,
     'print(printable("  %-4s [%s] %s (%s)" % ("ok" if ok else "FAIL", kind, name, detail[:90])))',
     'print("  %-4s [%s] %s (%s)" % ("ok" if ok else "FAIL", kind, name, detail[:90]))',
     [AXIS5_CHANNEL]),
    ("Axis 5 channel: unwrap the FAIL block's print (the other line a reader sees -- the per-control "
     "listing and the failure summary are two sites, and a fix to one says nothing about the other)",
     RUNNER,
     'print(printable("FAIL: %s" % f))',
     'print("FAIL: %s" % f)',
     [AXIS5_CHANNEL]),
]


# -- R42: the blank-and-marked contradiction.  TWO rows, because the clause has
# two separable halves and each alone is a different silent skip.
MUTANTS += [
    ("R42: accept the blank-and-marked row again (drop the contradiction clause: the row is keyed "
     "by nothing, assertion (b) never reads its Deps edge, rc 0)", POPULATION,
     '                spelled = [n for n in ("marker", "undetermined") if hit.get(n)]',
     '                spelled = []',
     [R42_BLANK_MARKER]),
    ("R42: read the declaring field from the wrong moment (`row.field`, which this pass runs BEFORE "
     "-- it is None here, so every blank row reads as terminal and the clause is vacuous)", POPULATION,
     '                field = stream(row.cells[s.decl].lexed) if s.decl is not None else ""',
     '                field = row.field',
     [R42_BLANK_MARKER]),
]


# -- R42-3: the entry point's option set.  TWO rows, one per half of the claim.
R42_OPTIONS = ("PROPERTY: the entry point's CLI contract is per MODE -- a closed option set whose "
               "complement is refused, and for each mode the flags it accepts and the positional "
               "count it takes (a known flag in the wrong mode returned 0 for the wrong operation)")
R42_BYTES = ("PROPERTY: no C0 control character or DEL from a MEMO reaches stdout through the "
             "checker's own report (the property the emit-site sweep is a proxy for, asked by "
             "running the program over a hostile memo)")
R42_CHANNEL = ("PROPERTY: every line the run REPORTS goes through the escape -- measured over the "
               "EMIT SITES, the subject the escape function's own control cannot reach")

MUTANTS += [
    # ⚠ TWO ROWS WERE RETIRED HERE (PR #510 R42-4), and the reason is a
    # MEASUREMENT: once the per-mode check landed, "discard an unknown option
    # again" SURVIVED -- the mode check refuses `--worklis` on its own, so the
    # separate set guard was a weaker second spelling and the guard itself is
    # gone. "Widen the set until the complement is empty" went with it; the
    # complement direction is covered by the mode rows below, which widen a
    # MODE's allowed set and are red.
]


# -- R42-4: the two halves the previous round's fixes left open, each of which
# was a direct consequence of the fix beside it.
# ⚠ A FOURTH ROW WAS WRITTEN AND WITHDRAWN: "name the report modules by hand
# again". It SURVIVED, correctly -- with every emit site wrapped, narrowing the
# population changes nothing observable, so the row asserted nothing. What makes
# a narrowed population consequential is an UNWRAPPED site inside the part it
# drops, which is the `report bytes` row below (it unwraps a site in the entry
# point, the module the hand-written pair left out). The derived population's
# value was demonstrated once, by measurement rather than by a mutant: it took
# the control from "0 not escaped" over 2 modules to 6 unescaped over 4.
MUTANTS += [
    ("R42-4 modes: accept a known flag in any mode (drop the per-mode flag check: `--mutants "
     "memo.md` runs the memo report and returns 0 without executing one mutant)", CHECK,
     '    misplaced = sorted(given - allowed)',
     '    misplaced = []',
     [R42_OPTIONS]),
    ("R42-4 modes: drop the positional-count half (two memo paths, or none, run whichever mode "
     "was selected and say nothing -- the direction the flag check alone cannot see)", CHECK,
     '    sel, allowed, want = next((m for m in MODES if m[0] is None or m[0] in given), MODES[-1])',
     '    sel, allowed, want = next((m for m in MODES if m[0] is None or m[0] in given), MODES[-1]); want = max(want, len(paths))',
     [R42_OPTIONS]),
    ("R42-4 report bytes: stop escaping the checker's OWN report (a memo-controlled ESC reaches "
     "the default report and can clear a terminal or forge a line in a captured log)", CHECK,
     'print(printable("    %s:%d [%s] {%s}  %s"\n'
     '                      % (m.file, m.lineno, m.source, role[m.key], m.context()[:190])))',
     'print("    %s:%d [%s] {%s}  %s"\n'
     '                      % (m.file, m.lineno, m.source, role[m.key], m.context()[:190]))',
     [R42_BYTES, R42_CHANNEL]),
]


# -- R42-1 / R42-5a: §6.4's last two rows.  ONE row per family, because the two
# were missed independently and a fix to one says nothing about the other.
MUTANTS += [
    ("R42 §6.4: stop demoting CODE SPANS into the description (§6.1 back to masked: a kind marker "
     "there is blanked and the row leaves the census at rc 0)", LEXER,
     '            dem_code.append((code_bottom, len(code)))',
     '            pass',
     [R42_IMG_CODE]),
    ("R42 §6.4: stop demoting AUTOLINKS into the description (§6.5 back to masked: the URI §6.5 makes "
     "the link text contributes nothing and the id in it is reported nowhere)", LEXER,
     '            dem_auto.append((auto_bottom, len(auto)))',
     '            pass',
     [R42_IMG_AUTO]),
    ("R42 §6.4: blank a demoted code span outright instead of routing it through `id_only` (the "
     "decoration exception stops surviving into alt text: `` `9z`7z `` glues)", STREAM,
     '        if tag == "demoted":',
     '        if False:',
     [R42_IMG_CODE]),
]


MUTANTS += [
    ("R42-6 §6.1: drop the trim on a demoted span (the padding stands in the alt text and a declared "
     "id split by it is two tokens: `Slice 9` + `z`, no site, rc 0)", STREAM,
     '            if len(norm) > 1 and norm[0] == " " and norm[-1] == " " and norm.strip(" "):',
     '            if False:',
     [R42_TRIM]),
    ("R42-6 §6.1: trim unconditionally (drop the spec's own \"not entirely spaces\" arm: an all-space "
     "span is trimmed, which JOINS what the reader sees separated)", STREAM,
     '            if len(norm) > 1 and norm[0] == " " and norm[-1] == " " and norm.strip(" "):',
     '            if len(norm) > 1 and norm[0] == " " and norm[-1] == " ":',
     [R42_ALLSPACE]),
]


# -- R42-8: the emit-site predicate, inverted to the COMPLEMENT.
MUTANTS += [
    ("R42-8 channel: unwrap the population summary (an `IfExp`, memo-controlled through "
     "`pop.display` -- the exact shape the %-only predicate could not see)", CHECK,
     '    print(printable("  population (transitive over the memo\'s links): %s"\n'
     '          % ", ".join(pop.display(m.path) for m in pop.memos[1:]) if len(pop.memos) > 1\n'
     '          else "  population: the memo alone (it links no other memo)"))',
     '    print("  population (transitive over the memo\'s links): %s"\n'
     '          % ", ".join(pop.display(m.path) for m in pop.memos[1:]) if len(pop.memos) > 1\n'
     '          else "  population: the memo alone (it links no other memo)")',
     [AXIS5_CHANNEL]),
    ("R42-8 channel: narrow the predicate back to a `%` over a literal (the list of shapes it was "
     "written against: every other way of building a line goes unread)", RECORDS,
     '        if isinstance(node, ast.Constant) and isinstance(node.value, str):\n'
     '            return True\n        if escaped(node):',
     '        if not (isinstance(node, ast.BinOp) and isinstance(node.op, ast.Mod)):\n'
     '            return True\n        if escaped(node):',
     [AXIS5_CHANNEL]),
]


# -- R42-9: both directions of the blank-id phrase set.
MUTANTS += [
    ("R42-9: narrow the blank-id contradiction back to the MARKER alone (the over-narrowing a review "
     "round reported: an UNDETERMINED row with a `Deps` cell exits 0 and no gate reports it)",
     POPULATION,
     '                spelled = [n for n in ("marker", "undetermined") if hit.get(n)]',
     '                spelled = [n for n in ("marker",) if hit.get(n)]',
     [R42_9_UNDET]),
    ("R42-9: widen it to every kind phrase (the POINTER arm back: the #506 memo's `Function`/`eval` "
     "row at `:1985` is a legitimate empty-id row and would become a FATAL)", POPULATION,
     '                spelled = [n for n in ("marker", "undetermined") if hit.get(n)]',
     '                spelled = [n for n in ("marker", "undetermined", "pointer") if hit.get(n)]',
     ["(R42-9) and the arm that WAS refuted stays refuted: the POINTER phrase on a blank id is the "
      "legitimate shape (`declaring_rows` names the #506 memo's `Function`/`eval` row, `:1985`) -- "
      "another row owns this one, which is what an unkeyed row is for"]),
]


# -- R42-8 §6.6 inside a resolved image description.  FOUR mutants over THREE
# decisions, because each is the only one its own mutant moves: does the lexer
# RECORD the demotion, does the disposition READ that record, and does the seed
# follow it -- in a paragraph and in a cell, two sites, and the paragraph mutant
# leaves the cell one standing.
MUTANTS += [
    ("R42-8 §6.6: the lexer records a raw HTML span demoted into a resolved image description "
     "(drop the range -- the span stays markup, masked and seeded as in bare prose)", LEXER,
     '            dem_html.append((html_bottom, len(html)))\n', '',
     [R42_8_HTML_ALT, R42_8_HTML_ATTR_SITE, R42_8_HTML_NO_SEED_PROSE, R42_8_HTML_NO_SEED_CELL]),
    ("R42-8 §6.6: the disposition READS that record (re-mask the demoted span -- the tag is still "
     "recorded, so the seed still skips it and only the READING moves)", STREAM,
     '    base += [(a, b, "html") for a, b, tag in lx.html if tag != "demoted"]',
     '    base += [(a, b, "html") for a, b, tag in lx.html]',
     [R42_8_HTML_ALT, R42_8_HTML_ATTR_SITE]),
    ("R42-8 §6.6 seed: a demoted span is not seeded from a PARAGRAPH (re-inject the seed)", MEMO,
     '            for a, b, tag in p.lexed.html:\n                if tag == "demoted":\n'
     '                    continue\n',
     '            for a, b, tag in p.lexed.html:\n',
     [R42_8_HTML_NO_SEED_PROSE]),
    ("R42-8 §6.6 seed: ... and not from a CELL either -- its own site, which the paragraph mutant "
     "leaves standing", MEMO,
     '                    for a, b, tag in cell.lexed.html:\n                        if tag == "demoted":\n'
     '                            continue\n',
     '                    for a, b, tag in cell.lexed.html:\n',
     [R42_8_HTML_NO_SEED_CELL]),
]

# -- R42-10: the unbound-claim gate.  FIVE mutants: the gate itself, the two
# NARROWINGS (to `main`, and to the raw cell text) and the two predicates the
# measurement REJECTED -- the id shape and the header near-miss.  The last two
# are re-injected here rather than argued about, because the reason they are
# wrong is a corpus count and a corpus count is not a control: what a control
# can hold is that the shipped predicate is silent where they speak.
MUTANTS += [
    ("R42-10: the unbound-claim gate exists (drop the call -- a linked memo's unbound table makes "
     "its census claim and leaves the population at rc 0)", POPULATION,
     '        self._unbound_claims()\n', '',
     [R42_10_UNBOUND_CLAIM, R31_1_DOLLAR_UNBOUND]),
    ("R42-10: the gate asks of the whole POPULATION (narrow it to `main`, where the schema-miss "
     "gate already asks -- the linked half is the whole defect)", POPULATION,
     '        for memo in self.memos:\n            for t in memo.tables:\n                if t.schema is not None:',
     '        for memo in self.memos[:1]:\n            for t in memo.tables:\n                if t.schema is not None:',
     [R42_10_UNBOUND_CLAIM, R31_1_DOLLAR_UNBOUND]),
    ("R42-10: the phrase is read off the RENDERED cell (read the raw text -- a marker inside a code "
     "span becomes a claim)", POPULATION,
     'for n, rx in KIND_PHRASES if rx.search(stream(c.lexed))), None)',
     'for n, rx in KIND_PHRASES if rx.search(c.text)), None)',
     [R42_10_UNBOUND_RENDERED]),
    # ⚠ THESE TWO ROWS BROKE TWICE IN ONE SESSION, and the second break is the
    # instructive one: re-pointed at the loop HEADER, the injected predicate was
    # overwritten by the two `hit =` assignments below it, so the mutant ran and
    # changed nothing -- it SURVIVED while looking re-pointed.  The anchor is the
    # whole assignment block now, so a re-injected predicate is the only one that
    # runs.  Both rows target the same substring; each is applied to a fresh copy.
    ("R42-10: the REJECTED id-shape predicate (re-inject it: every unbound table whose EVERY BODY "
     "ROW starts with a row id -- 151 of them over the corpus §8 names)", POPULATION,
     '                    hit = next((n for c in row.cells\n'
     '                                for n, rx in KIND_PHRASES if rx.search(stream(c.lexed))), None)\n'
     '                    if hit is None:\n'
     '                        hit = next((n for c in row.cells\n'
     '                                    for n in kind_disagreements(c.lexed)), None)',
     # ⚠ ANCHORED. `tokens()` is a SCANNER: `next(tokens("prose with 9z inside"))`
     # is not None, so the unanchored form re-injects "CONTAINS an id" (318
     # tables) while the figure beside it is "STARTS with an id" (151). The
     # number and the thing it justifies were different predicates one level
     # down from where that was last corrected.
     '                    _t = __import__("plan_memo_ids").tokens\n'
     '                    _st = lambda c: (lambda k: k is not None and k.start == 0)(\n'
     '                        next(_t((c or "").strip(" \\t")), None))\n'
     '                    hit = ("marker" if t.rows and all(\n'
     '                        r.cells and _st(r.cells[0].text) for r in t.rows) else None)',
     [R42_10_UNBOUND_ID_SHAPED]),
    ("R42-10: the REJECTED header-near-miss predicate (re-inject it: a renamed header is reported "
     "whether or not the table declares anything)", POPULATION,
     '                    hit = next((n for c in row.cells\n'
     '                                for n, rx in KIND_PHRASES if rx.search(stream(c.lexed))), None)\n'
     '                    if hit is None:\n'
     '                        hit = next((n for c in row.cells\n'
     '                                    for n in kind_disagreements(c.lexed)), None)',
     '                    hit = next((sc.name for sc in __import__("plan_memo_tables").SCHEMAS\n'
     '                                if len(sc.header) == len(t.header.cells)\n'
     '                                and sum(1 for a, b in zip([c.text for c in t.header.cells],\n'
     '                                                          sc.header) if a != b) <= 1), None)',
     [R42_10_UNBOUND_NO_CLAIM]),
]


DEMOTED_AGREEMENT = ("CommonMark 0.31.2 §6.4: Phase 2's inline claim agrees with the spec's own html "
                     "for every §3.0b family DEMOTED into a resolved image description (the "
                     "cross-product the corpus cannot reach)")

# -- R45: the FALSIFIER's own demotion filter, one mutant per family.  Each
# un-filters one count and the control goes red on that family alone; the
# vendored corpus stays green under all three, which is the whole reason the
# control exists.
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

# -- R45: the two population-scope loops the ratchet missed.  Same edit shape as
# the five rows that already exist for `_declare` / `keep` / `data_rows` / the
# walk / `_unbound_claims`; these two had no row, and both survived silently.
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
     'ROW_NOUN_SEP = ROW_NOUN + "[ \\t\\n" + DASH + "]+"',
     'ROW_NOUN_SEP = ROW_NOUN + r"[ \\t\\n-]+"',
     [R47_1_DASH_SET, R47_1_EN_DASH]),
    ("R47-2 gate: the unbound-claim gate asks BOTH readings (drop the disagreement arm -- a claim "
     "straddling a masked span is invisible to the stream and the table leaves the census at rc 0)",
     POPULATION,
     '                    if hit is None:\n'
     '                        hit = next((n for c in row.cells\n'
     '                                    for n in kind_disagreements(c.lexed)), None)\n',
     '',
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
     'bounded("KIND" + GAP + "*" + DASH_CLASS + "?" + GAP + "*UNDETERMINED")',
     'bounded(r"KIND\\s*" + DASH_CLASS + r"?\\s*UNDETERMINED")',
     [R47_4_UNDET_NBSP]),
    ("R47-4 gap: the gap is a WHITESPACE class, not a wildcard (widen it to `.` -- every arm above "
     "still passes, and only the word-boundary negative catches it)", STREAM,
     'GAP = r"(?u:\\s)"',
     'GAP = r"."',
     [R47_4_NON_WHITESPACE]),
]

# -- R47-5: the DERIVED scope ratchet's closure.  `population_scope_control`
# enumerates every `ast.For` in the census module and demands a mutant that
# TRUNCATES that loop; these are the rows it demanded.  Nine of the twelve it
# found were already covered by a control that goes red -- the ratchet's job
# was to say WHICH, measured one loop at a time, not to invent them -- and
# three needed a fixture that takes a second iteration.
MUTANTS += [
    ("R47-5 scope: the population walk queues EVERY link of a memo (truncate the loop at line ~88)", POPULATION,
     '            for f in memo.linked_files():',
     '            for f in list(memo.linked_files())[:1]:',
     ["diagnostics name a memo relative to the root memo's directory: `a/child.md` and `b/child.md` are two files, and a memo outside that directory is named by its absolute path"]),
    ("R47-5 scope: every UNANSWERED reference of a memo is reported (truncate the loop at line ~94)", POPULATION,
     '            for lineno, label in memo.unresolved_references():',
     '            for lineno, label in list(memo.unresolved_references())[:1]:',
     [R47_5_TWO_REFS]),
    ("R47-5 scope: every TABLE of a memo is bound (truncate the loop at line ~106)", POPULATION,
     '            for t in memo.tables:\n                for lineno, msg in t.misses:',
     '            for t in list(memo.tables)[:1]:\n                for lineno, msg in t.misses:',
     ["(rc) a schema body row whose width differs from its header is rc 2"]),
    ("R47-5 scope: every MISS of a table is reported (truncate the loop at line ~107)", POPULATION,
     '                for lineno, msg in t.misses:',
     '                for lineno, msg in list(t.misses)[:1]:',
     [R47_5_TWO_MISSES]),
    ("R47-5 scope: every declared id is given a KIND (truncate the loop at line ~123)", POPULATION,
     '        for row in self.ids.values():',
     '        for row in list(self.ids.values())[:1]:',
     [R47_5_ALL_KINDS]),
    ("R47-5 scope: every SCHEMA ROW of a memo is declared (truncate the loop at line ~152)", POPULATION,
     '            for row in memo.schema_rows(s.name):\n                rid = row.self_id',
     '            for row in list(memo.schema_rows(s.name))[:1]:\n                rid = row.self_id',
     ["a self-declaring row that MENTIONS a sibling stays in the population"]),
    ("R47-5 scope: every TABLE of a memo is asked for an unbound claim (truncate the loop at line ~211)", POPULATION,
     '            for t in memo.tables:\n                if t.schema is not None:',
     '            for t in list(memo.tables)[:1]:\n                if t.schema is not None:',
     [R47_5_SECOND_TABLE]),
    ("R47-5 scope: every ROW of an unbound table is asked (truncate the loop at line ~214)", POPULATION,
     '                for row in [t.header] + t.rows:',
     '                for row in list([t.header] + t.rows)[:1]:',
     [R42_10_UNBOUND_CLAIM]),
    ("R47-5 scope: every SCHEMA ROW is read for the unkeyed miss (truncate the loop at line ~265)", POPULATION,
     '            for row in memo.schema_rows(s.name):\n                if row.self_id is not None:',
     '            for row in list(memo.schema_rows(s.name))[:1]:\n                if row.self_id is not None:',
     [R47_5_SECOND_ROW]),
    ("R47-5 scope: every kind phrase the residue names is reported (truncate the loop at line ~388)", POPULATION,
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
