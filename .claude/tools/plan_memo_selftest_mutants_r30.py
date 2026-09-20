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

from plan_memo_selftest_cases_r26 import (
    R30_3_DEMOTED_TAIL, R30_3_IMAGE_OPENER, R30_3_LINK_OPENER, R30_3_LOUD_MISS,
    R30_CODE_SPAN_READING, R30_KEYED, R30_PAIRED, R30_UNPAIRED, R31_1_RENDERED_HEADER,
    R33_1_LONGER_WORD, R33_1_NOVEL_PREFIX, R33_1_REAL_NOUN, R33_2_EN_DASH, R33_2_NON_DASH,
    R34_1_CONTINUES, R34_1_FRAGMENT, R34_1_TRAILING, R34_2_BLANKS, R34_2_MASKED,
    R35_FRAGMENT_ID, R35_QUERY_ID, R38_CD_BLANK, R38_CD_MASKED, R42_BLANK_MARKER,
)
from plan_memo_selftest_mutants import (
    BLOCKS, CHECK, CONTROLS, EMPHASIS, GROWTH, HTML, IDS, INLINE_EXAMPLES, LEXER, MEMO, MUTANTS,
    POPULATION, PROPERTIES, R27_GROWTH, RECORDS, ROLES, RUNNER, SIBLING, STREAM, TABLES, TOKENS,
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
     'UNDETERMINED = re.compile(bounded(r"KIND\\s*" + DASH_CLASS + r"?\\s*UNDETERMINED"),',
     'UNDETERMINED = re.compile(bounded(r"KIND\\s*[\\u2014-]?\\s*UNDETERMINED"),',
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
     "name carries reaches the wire's command substitution and is stripped there)", CONTROLS,
     'if c < " " or c == "\\x7f" else c',
     'if c == "\\x7f" else c',
     [AXIS5_PRINTABLE]),
    ("Axis 5 report channel: escape EVERYTHING (a printable line is mangled too: a green run over an "
     "unreadable report is the other direction of the same defect)", CONTROLS,
     'if c < " " or c == "\\x7f" else c',
     'if True else c',
     [AXIS5_PRINTABLE]),
]


# PR #510 Axis 3 + Axis 5: the 1000-line touch-time bound, as a mechanism.
# ⚠ THE SUBJECT MUST BE WHAT THE CONTROL READS, which is a swept source's LINE
# COUNT -- not the control's own threshold.  Mutating the constant would prove
# only that the control reads its own constant
# (`memory/feedback_surviving-mutation-means-the-probe-has-another-subject.md`),
# so this row grows a real file past the bound instead: 50 line endings inside
# the lexer's module docstring take it 961 -> 1,011.  The text still parses, so
# `load()` execs it and the control sees the patched source through `SOURCES`.
AXIS5_LINE_BOUND = ("PROPERTY: every source of this checker is under the 1000-line touch-time bound "
                    "(the invariant this PR broke, as a mechanism instead of a sentence)")

MUTANTS += [
    ("Axis 5 line bound: a source crosses 1000 lines (grow the lexer's docstring by 50 line endings: "
     "961 -> 1,011, which is the debt the touch-time rule exists to stop)", LEXER,
     '"""Phase 2 of CommonMark 0.31.2 "Appendix: A parsing strategy" -- INLINE',
     '"""' + "\n" * 50 + 'Phase 2 of CommonMark 0.31.2 "Appendix: A parsing strategy" -- INLINE',
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
    ("Axis 5 channel: unwrap a fails.append (the OTHER emit shape -- a line that reaches the reader "
     "through the FAIL block rather than the per-control listing)", RUNNER,
     'fails.append(printable("%s %s :: %s" % (kind, name, detail)))',
     'fails.append("%s %s :: %s" % (kind, name, detail))',
     [AXIS5_CHANNEL]),
]


# -- R42: the blank-and-marked contradiction.  TWO rows, because the clause has
# two separable halves and each alone is a different silent skip.
MUTANTS += [
    ("R42: accept the blank-and-marked row again (drop the contradiction clause: the row is keyed "
     "by nothing, assertion (b) never reads its Deps edge, rc 0)", POPULATION,
     '                spelled = [n for n in ("marker",) if hit.get(n)]',
     '                spelled = []',
     [R42_BLANK_MARKER]),
    ("R42: read the declaring field from the wrong moment (`row.field`, which this pass runs BEFORE "
     "-- it is None here, so every blank row reads as terminal and the clause is vacuous)", POPULATION,
     '                field = stream(row.cells[s.decl].lexed) if s.decl is not None else ""',
     '                field = row.field',
     [R42_BLANK_MARKER]),
]


# -- R42-3: the entry point's option set.  TWO rows, one per half of the claim.
R42_OPTIONS = ("PROPERTY: the entry point accepts a CLOSED option set and REFUSES its complement "
               "(an unknown option was discarded, so a misspelt --worklist returned the other "
               "format at rc 0)")

MUTANTS += [
    ("R42 options: discard an unknown option again (re-inject the filter that took every non-`--` "
     "argv entry as the path and said nothing about the rest: `--worklis` runs the other format "
     "at rc 0)", CHECK,
     '    unknown = sorted({a for a in argv[1:] if a.startswith("--")} - OPTIONS)',
     '    unknown = []',
     [R42_OPTIONS]),
    ("R42 options: widen the set until the complement is empty (every `--` spelling accepted: a "
     "closed set nobody can fall outside asserts nothing -- the other direction, which the rc "
     "probes alone cannot see)", CHECK,
     'OPTIONS = frozenset(("--self-test", "--mutants", "--worklist"))',
     'OPTIONS = frozenset(("--self-test", "--mutants", "--worklist", "--worklis"))',
     [R42_OPTIONS]),
]
