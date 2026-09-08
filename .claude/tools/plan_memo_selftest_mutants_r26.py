#!/usr/bin/env python3
"""PR #510 R26's mutation-proof rows -- the fourth module of the ONE `MUTANTS`
list, carved at the review-round seam the registry has used three times
(`plan_memo_selftest_mutants` holds the row shape, `run` and the pre-converge
rows; `_pr510` R1-R16 and the design re-gates; `_inline` R17-R25).

Carved when `_inline.py` reached 987 lines on this round's first two rows -- at
touch time, as a standalone commit, so that the rows R26 still owes arrive in a
bounded file rather than pushing that one past 1000.

WHAT MAKES R26 A SEAM AND NOT A NUMBER: its subject is not a CommonMark
construct, which is what `_inline.py` is organised around.  R26's findings are
about the checker's OPERATING ENVELOPE -- what it assumes about its host
(encoding) and what it costs (the linearity of `inline_pass`) -- plus the two
places where one reading of a text disagreed with another.  A row here answers
"is this checker still portable and still linear?", never "does this construct
lex?".

`MUTANTS` is imported and appended to, exactly as the other three do; the
runner reads the one list at one import site.
"""

from plan_memo_selftest_mutants import (
    CHECK, CONTROLS, GROWTH, HTML, INLINE_EXAMPLES, LEXER, MEMO, MUTANTS, SIBLING, TABLES, TOKENS,
)

R26_ENCODING = ("PROPERTY: no source of this checker performs text I/O without naming its encoding "
                "(the checker set and the self-test both, globbed)")

MUTANTS += [
    # TWO rows, one per HALF of the swept population, because the two halves
    # reach the sweep by different routes: a checker module's text comes from
    # `load()`'s `SOURCES`, a self-test module's from `patched_module`'s entry
    # in the same dict (added at R26-4 for exactly this proof).  One row would
    # leave the other route unwitnessed -- and it was the SELF-TEST half that
    # held all fifteen defects.
    ("R26-4 encoding: the production memo read names its encoding (drop it -- the checker set half of "
     "the sweep's population)", MEMO,
     '        with open(self.path, encoding="utf-8", newline="") as fh:',
     '        with open(self.path, newline="") as fh:',
     [R26_ENCODING]),
    ("R26-4 encoding: a self-test fixture write names its encoding (drop it -- the SELF-TEST half of "
     "the sweep's population, which `load()` alone would not show)", CONTROLS,
     '(pathlib.Path(d) / "child.md").write_text(twin, encoding="utf-8")',
     '(pathlib.Path(d) / "child.md").write_text(twin)',
     [R26_ENCODING]),
]

R26_STREAMS = ("PROPERTY: the entry point sets BOTH output streams to UTF-8 -- the absence a "
               "call-site sweep cannot report")

MUTANTS += [
    # The STREAM half.  Two rows, because the two streams are two claims: a
    # loop that reconfigures only `sys.stdout` leaves every diagnostic the
    # checker writes to stderr on the locale's encoding, and one control that
    # read either stream alone would call that fixed.
    ("R26-4 encoding: the entry point reconfigures its output streams (drop the encoding -- a "
     "`reconfigure` naming none leaves the locale's in place)", CHECK,
     # the call STAYS and loses only its encoding, so this row witnesses BOTH
     # halves at once: the stream control sees the wrong request, and the sweep
     # sees a `reconfigure` naming no encoding (which is why `reconfigure` is in
     # `_ENCODED_IO`).  Replacing the call with `pass` instead let the sweep
     # SURVIVE -- correctly: an ABSENCE is exactly what a call-site sweep cannot
     # see -- so the subject moved to the argument rather than the mutant being
     # softened; the absence itself is what the next row and the control's own
     # docstring answer for
     '            reconfigure(encoding="utf-8")', "            reconfigure(newline=None)",
     [R26_STREAMS, R26_ENCODING]),
    ("R26-4 encoding: BOTH streams, not just stdout (reconfigure stdout alone -- stderr keeps the "
     "locale's encoding and every diagnostic written there dies on a non-ASCII host)", CHECK,
     "    for stream in (sys.stdout, sys.stderr):", "    for stream in (sys.stdout,):",
     [R26_STREAMS]),
]

# -- R26-2: the file-name token reads §6.3's balanced-pair rule at any depth.
R26_NESTED = ("(R26 file) `foo((9z)).md` is ONE file name: nested balanced parentheses are §6.3's rule "
              "at any depth, so the id inside the name is no naming site.  Until R26 the token arm "
              "matched a FLAT chunk only, could reach no further left than the bare `.md`, and reported "
              "`9z`")
R26_ACROSS = ("(R26 file) `(m.md(9z)md).md`: the run is balanced ACROSS its groups, so it is one name "
              "and not two tokens with the id exposed between them -- the second shape the flat arm "
              "could not read, and the one that leaks an id rather than a parenthesis")
R26_OPEN = ("(R26 file) `9z(x.md)` reports `9z`: the run ending at the suffix may not hold the "
            "parenthesis still OPEN there, so it starts one past it and the name is `x.md` -- the half "
            "that says the start is the innermost open parenthesis and not the segment")
R26_UNMATCHED = ("(R26 file) `9z)foo.md` still reports `9z`: a `)` that closes nothing may be held by "
                 "no run and crossed by none, so the name is `foo.md` and the id before it is outside "
                 "-- the second discriminating half, against a scan that balanced parentheses by "
                 "IGNORING the ones it could not match")
R26_LONGEST = ("(R26 file) `a.md+9z+b.md` is ONE name ending at the LAST suffix, not the first: "
               "leftmost-LONGEST, as a pattern would have read it.  Stopping at the first `.md` would "
               "leave `9z` standing outside the token.  ⚠ The first fixture written for this clause "
               "was `a.md.9z.md`, and it was NOT discriminating -- an id directly before `.md` is a "
               "DOTTED NUMBER to the id grammar and no site either way, so both readings reported 0 "
               "and the mutant survived.  The id has to be separated from the second suffix")
R26_AGREE = ("PROPERTY: every name the sibling resolver accepts, standing alone in prose, is ONE file "
             "token to the lexer (the correspondence FILE_SUFFIX's comment asserts)")
R26_TOKEN_LINEAR = ("file_and_cite_spans is linear: N parenthesis groups are one pass, not a re-scan "
                    "from every start position")

MUTANTS += [
    # ONE clause per row, because the scan is four clauses and any one of them
    # alone would let a defect through: what an open parenthesis does, what an
    # unmatchable close does, where a run may start, and which end wins.
    ("R26-2 file token: a parenthesis NESTS (drop the push: the stack never deepens, so a nested pair "
     "reads as an unmatchable close and cuts the run -- the flat arm's own defect, re-injected)", TOKENS,
     '        if c == "(":\n            stack.append(i)', '        if c == "(":\n            pass',
     [R26_NESTED, R26_ACROSS, R26_AGREE]),
    ("R26-2 file token: an unmatchable `)` ends the segment (drop it: a run holds a close that opens "
     "nothing, and the id before it is swallowed)", TOKENS,
     '                seg = i + 1     # an unmatchable `)`: no run holds it, none crosses it',
     '                pass',
     [R26_UNMATCHED]),
    ("R26-2 file token: a run starts one past the INNERMOST parenthesis still open (re-inject the "
     "segment start: the token holds an unclosed `(` and everything before it)", TOKENS,
     '            s = stack[-1] + 1 if stack else seg', '            s = seg',
     [R26_OPEN]),
    ("R26-2 file token: leftmost-LONGEST (keep the first end per start instead of the last: the token "
     "stops at the first suffix and leaves the rest of the name standing)", TOKENS,
     '            if s <= e - k:      # the suffix itself must lie inside the run\n                longest[s] = e',
     '            if s <= e - k:      # the suffix itself must lie inside the run\n                longest.setdefault(s, e)',
     [R26_LONGEST]),
    # The COST row, deliberately VALUE-PRESERVING: the replacement reaches the
    # same segment start by walking back to it, so nothing about the reading
    # changes and the only thing the control can be reacting to is the WORK.
    # (What `re` did was re-enter the arm at every start position; that is not
    # one substring here, and a mutant that also changed the answer would let a
    # behaviour control take the credit for killing it.)
    ("R26-2 work: the token scan is ONE pass (re-inject a walk back to the segment start at every "
     "candidate -- the same answer, quadratically)", TOKENS,
     '            s = stack[-1] + 1 if stack else seg',
     '            s = stack[-1] + 1 if stack else [seg for _ in range(i + 1)][-1]',
     [R26_TOKEN_LINEAR]),
    # The LOWER BOUNDS of the agreement sweep, which are not decoration: a
    # corpus that came back empty, or one holding only flat shapes, would
    # report the same "no disagreement" a clean sweep does.  This row shrinks
    # the corpus from the RESOLVER's side, which is the half the sweep does not
    # control, and the `deep >= 2` bound is the only thing that can see it.
    ("R26-2 agreement: the sweep's corpus must reach nesting depth 2 (make the resolver refuse every "
     "parenthesised name: the sweep then agrees about flat names only, and says so)", SIBLING,
     '    if not name.endswith(FILE_SUFFIX):                           # (d)',
     '    if not name.endswith(FILE_SUFFIX) or "(" in name:            # (d)',
     [R26_AGREE]),
]

# -- R26-3 (and R26-1): `inline_pass`'s linear contract, falsified in three
# more places and bounded in all three.  R23 fixed the first (the image
# demotion) and the docstring went on claiming linearity through both; the
# question this round asked was "is `inline_pass` linear", not "is this loop".
R26_TAIL_LINEAR = ("inline_pass is linear over a malformed inline-link tail: `[`xN + `](`xN is O(N), "
                   "bounded by §6.3's permitted nesting limit")
R26_HTML_ATTEMPTS = ("inline_pass does not RUN the §6.6 grammar at a `<` whose closer stands nowhere "
                     "ahead (the one cost the Python-level witnesses cannot see)")
R26_INDEX_CURSOR = ("the closer index carries a cursor: 600 openers with no closer start at most one "
                    "search per literal, not one per opener")
R26_CODE_CLOSER = ("a §6.1 code span's closer is found by an index of the runs BY LENGTH, not by "
                   "walking them (0.27 x L^1.5 before)")
R26_DEEP_LINK = ("(R26 §6.3) a destination nested 32 deep IS a link, so the memo it names is a "
                 "sibling this run could not read: rc 2.  Green before the limit as after it -- the "
                 "half that says the limit is a LIMIT and not a refusal of nested destinations")
R26_DEEP_LITERAL = ("(R26 §6.3) a destination nested 33 deep is NOT a link, so `[x](...)` is literal "
                    "text and no memo is looked up: rc 0.  ⚠ This is a DIVERGENCE from commonmark.js, "
                    "which reads it as a link, and an agreement with cmark; the spec permits both and "
                    "the vendored corpus reaches depth 2, so nothing but this control says where the "
                    "boundary is")

MUTANTS += [
    ("R26-3 §6.3: the destination's parenthesis nesting is BOUNDED (lift the bound: 33 levels are a "
     "link again, which is both the reported false rc-2 miss and the quadratic tail scan)", LEXER,
     "            if depth > DESTINATION_NESTING_LIMIT:", "            if depth > 10 ** 9:",
     [R26_DEEP_LITERAL, R26_TAIL_LINEAR]),
    # The bound in the OTHER direction, and it is the one that keeps the limit
    # honest: §6.3 requires "at least three levels of nesting", and Example 496
    # (`[link](foo(and(bar)))`) is the corpus's deepest at 2.  A limit chosen
    # low enough to be cheap would pass every control above.
    ("R26-3 §6.3: the bound is above what the spec requires and the corpus uses (drop it to 1: "
     "Example 496's two levels stop being a link)", LEXER,
     "DESTINATION_NESTING_LIMIT = 32", "DESTINATION_NESTING_LIMIT = 1",
     [INLINE_EXAMPLES, R26_DEEP_LINK]),
    ("R26-3 §6.6: the tag grammar is not RUN at a `<` whose closer stands nowhere ahead (drop the "
     "gate: every opener re-scans to the end of the text, as it did before)", LEXER,
     "            m = _match_tag(s, i) if closers.reachable(lit, at) else None",
     "            m = _match_tag(s, i)",
     [R26_HTML_ATTEMPTS]),
    # The gate must be per-ALTERNATIVE.  `>` is the closer every alternative
    # shares, so a gate written in one line -- ask for `>` and be done -- passes
    # a text that ends in a bare `>` and re-scans it N times.  Two rows, one per
    # lazy alternative that would fall through to it.
    ("R26-3 §6.6: a processing instruction needs a `?>`, not merely a `>` (drop its row: a text of "
     "`<?x` openers ending in a bare `>` re-runs the lazy scan at every one)", HTML,
     '_CLOSERS = (("<!--", "-->", 2), ("<![CDATA[", "]]>", 9), ("<?", "?>", 2))',
     '_CLOSERS = (("<!--", "-->", 2), ("<![CDATA[", "]]>", 9))',
     [R26_HTML_ATTEMPTS]),
    ("R26-3 §6.6: a CDATA section needs a `]]>`, not merely a `>` (drop its row: the same defect one "
     "alternative over)", HTML,
     '    for opener, closer, off in _CLOSERS:\n        if s.startswith(opener, i):\n            return closer, i + off',
     '    for opener, closer, off in _CLOSERS[:1]:\n        if s.startswith(opener, i):\n            return closer, i + off',
     [R26_HTML_ATTEMPTS]),
    # The gate must also be TRUE sometimes: a gate that refuses everything is
    # linear and useless, and no cost control can tell the two apart.
    ("R26-3 §6.6: the closer index answers YES when the closer is there (refuse always: no raw HTML "
     "span is ever recognised)", LEXER,
     "        return j >= 0", "        return False",
     [INLINE_EXAMPLES, R26_HTML_ATTEMPTS]),
    ("R26-3 §6.6: the closer index carries a CURSOR (re-search at every query: the same answers, one "
     "scan of the text per opener)", LEXER,
     "        if j is None or 0 <= j < i:", "        if True:",
     [R26_INDEX_CURSOR]),
    ("R26-3 §6.1: a code span's closer is found by the index BY LENGTH (re-inject the walk over every "
     "run: the same answer, 0.27 x L^1.5)", LEXER,
     '    same = by_len.get(k)\n    if not same:\n        return None\n    j = bisect.bisect_left(same, (a1, 0))\n    return same[j][1] if j < len(same) else None',
     '    same = sorted(r for lst in by_len.values() for r in lst)\n'
     '    j = bisect.bisect_left(same, (a1, 0))\n'
     '    while j < len(same):\n'
     '        if same[j][1] - same[j][0] == k:\n'
     '            return same[j][1]\n'
     '        j += 1\n'
     '    return None',
     [R26_CODE_CLOSER]),
]


# ------------------------------------------------------ PR #510 Codex R27 --

R27_NOUN_PROPERTY = ("PROPERTY: every row-keyed schema's NAME is a row noun and no other schema's is "
                     "(the nouns are derived from SCHEMAS, so the next schema's is covered by default)")
R27_SLOT_ATTRIBUTES = ("(R27 noun) ``Slot `#11-zz-alpha` — **UMBRELLA, …**`` attributes the marker to "
                       "the named row: the containing row is a POINTER and the attribution finding is "
                       "emitted. The schema noun the §8 table's own id column is headed with, and the "
                       "one spelling the hand-written alternation left out")

MUTANTS += [
    # THE DEFECT AS REPORTED, put back: the alternation that stood before R27-2,
    # which is also what any "add the missing word" fix would leave behind one
    # schema later.  Both halves go red -- the swept derivation because `slot`
    # is gone from it, the fixture because the reviewer's field stops
    # attributing -- and that pairing is the point: the property is what makes
    # the fixture's spelling one CASE of a rule rather than the rule.
    ("R27-2: the row nouns are derived from SCHEMAS (re-inject the hand-written alternation: `Slot` "
     "names no row again, and a slot row that attributes a marker declares itself an umbrella)", TABLES,
     '    nouns = set(GENERIC_ROW_NOUNS)\n'
     '    row_kinds = set(ROW_KINDS)\n'
     '    nouns |= {s.name for s in SCHEMAS if s.kinds and set(s.kinds) <= row_kinds}\n'
     '    return "(?ai:%s)" % "|".join(re.escape(n) + "s?" for n in sorted(nouns, key=lambda n: (-len(n), n)))',
     '    return r"(?ai:slices?|rows?|umbrellas?)"',
     [R27_NOUN_PROPERTY, R27_SLOT_ATTRIBUTES]),
    # The derivation is a FILTER, not a union.  Dropping the kind test admits
    # `citation` and `stub`, and `Citation `#11-zz-alpha` — **UMBRELLA, …**`
    # starts attributing -- a category error the positive half cannot see.
    ("R27-2: only ROW-KEYED schemas name rows (drop the kind filter: a citation table's name becomes "
     "a row noun)", TABLES,
     '    nouns |= {s.name for s in SCHEMAS if s.kinds and set(s.kinds) <= row_kinds}',
     '    nouns |= {s.name for s in SCHEMAS}',
     [R27_NOUN_PROPERTY]),
    # The two GENERIC nouns are no schema's, so nothing but the sweep's own
    # generic half says the derivation still carries them.
    ("R27-2: the generic nouns survive the derivation (drop them: `row` and `umbrella` belong to no "
     "schema and would leave with the literal)", TABLES,
     '    nouns = set(GENERIC_ROW_NOUNS)', '    nouns = set()',
     [R27_NOUN_PROPERTY]),
]


R27_GROWTH = ("the scans are linear over a corpus GENERATED from the grammar: every branch character, "
              "delimiter, HTML opener, bracket construct and id kind, each repeated and each PAIR of "
              "them interleaved, and no source line grows worse than its input")
R27_STRADDLE = ("PROPERTY: _straddles answers its own definition (a character inside a blank and a "
                "character outside every blank), over every blank layout of eight positions and every "
                "extent inside it")

MUTANTS += [
    # -- R27-1: the Appendix's deactivation is a COUNTER, not a walk.
    #
    # The COST row first, and it is the one the generated sweep exists for: a
    # no-op walk over the stack at every link close gives every one of the 630
    # conformance examples the same answer and every fixture control the same
    # count, and costs 1+2+...+N.  Nothing but a growth measure can see it,
    # and no per-shape witness had been written for it in four rounds.
    ("R27-1 Appendix: a link's deactivation of the openers below it is O(1) (re-inject the walk over "
     "the whole stack at every close: the same answers, quadratic over `![`xN then N resolved links)",
     LEXER,
     "            closed += 1                 # links may not contain links: every",
     "            for _opener in stack:       # the retired walk, doing nothing\n"
     "                pass\n"
     "            closed += 1                 # links may not contain links: every",
     [R27_GROWTH]),
    # The two CORRECTNESS halves of the same clause -- a closing link DOES
    # deactivate, and an IMAGE opener is exempt -- are R1-3 and R3-1 in
    # `plan_memo_selftest_mutants_pr510.py`, whose anchors moved onto this
    # counter with the fix and whose fixture controls are sharper than a
    # second row here would be.  What R27-1 adds is the COST row above, which
    # neither of them can state.
    # -- R27-3: `_straddles` reads only the blanks that overlap the extent.
    ("R27-3 residue: `_straddles` reads only the OVERLAPPING blanks (re-inject the sum over the whole "
     "list: the same answers, quadratic in a block of N blanked spans)", TABLES,
     "    j = bisect.bisect_left(blanks, (a,))",
     # the RETIRED implementation, re-injected verbatim in front of the window
     # search: the differential family (`straddle_definition_control`) says the
     # two answer alike, so nothing but a growth measure can move
     "    inside = sum(max(0, min(b, y) - max(a, x)) for x, y in blanks)\n"
     "    return 0 < inside < b - a\n"
     "    j = bisect.bisect_left(blanks, (a,))",
     [R27_GROWTH]),
    ("R27-3 residue: the blank BEFORE the extent may reach into it (drop the step back: a unit whose "
     "left end is inside a blank that started earlier reads as wholly outside)", TABLES,
     "    if j and blanks[j - 1][1] > a:      # the blank before `a` may reach into it\n        j -= 1",
     "    if False:\n        j -= 1",
     [R27_STRADDLE]),
    ("R27-3 residue: an extent WHOLLY inside the blanks does not straddle them (flip the early exit: a "
     "quoted marker becomes a disagreement)", TABLES,
     "            return False                # wholly inside the blanks: not across them",
     "            return True                 # wholly inside the blanks: not across them",
     [R27_STRADDLE]),
    # -- The generated sweep's own halves, which no checker mutant can reach.
    ("R27 sweep: every branch character is an ATOM of the corpus (drop them: the pairs that hold a "
     "character no construct spells stop being generated, and every probe left still passes)", GROWTH,
     "    atoms = set(_scan_alphabet()) | set(BRACKET_ATOMS)", "    atoms = set(BRACKET_ATOMS)",
     [R27_GROWTH]),
    ("R27 sweep: every id KIND the grammar declares has a document spelling in the corpus (drop one: "
     "the sweep would silently stop pairing that kind's token with anything)", GROWTH,
     'ID_SPELLINGS = {"short": ("9z", "9z "), "slug": ("#11-a", "`#11-a` "), "cite": ("[C1]", "[C1] ")}',
     'ID_SPELLINGS = {"short": ("9z", "9z "), "slug": ("#11-a", "`#11-a` ")}',
     [R27_GROWTH]),
    # The CONFIRMATION stage decides; the sweep only nominates.  A confirmation
    # run at the sweep's own sizes would report §6.3's constant-bounded
    # destination scan as a defect -- four probes of the corpus, green today --
    # so this row proves the two stages are two sizes and not one.
    ("R27 sweep: the confirmation runs at a size where a CONSTANT-bounded scan has plateaued (confirm "
     "at the sweep's sizes instead: §6.3's destination limit reads as growth)", GROWTH,
     "GROWTH_SWEEP, GROWTH_CONFIRM = 6, 96", "GROWTH_SWEEP, GROWTH_CONFIRM = 6, 6",
     [R27_GROWTH]),
]
