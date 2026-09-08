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

from plan_memo_selftest_mutants import CHECK, CONTROLS, MEMO, MUTANTS

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
