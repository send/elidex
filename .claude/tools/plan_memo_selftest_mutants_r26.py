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
    CHECK, CONTROLS, LEXER, MEMO, MUTANTS, SIBLING, TOKENS,
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
