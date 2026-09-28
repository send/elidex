#!/usr/bin/env python3
"""PR #510 mutation-proof rows of Codex R38 of 2026-09-27 (a corpus-sweeping
proof control fails on any excluded or skipped item) and Codex R40 of
2026-09-28 (the report banner's display name).  Carved at the review-round
seam `_mutants_r26.py` / `_r30.py` / `_r45.py` use.

`MUTANTS` here is this module's OWN list; `plan_memo_selftest_mutants.mutants()`
gathers every mutants module's list (`plan_memo_selftest_harness.registry_modules`
decides which modules those are, by CONTENT: a module holding its own list) in one explicit step.
"""

MUTANTS = []

# -- PR #510 Codex R38 of 2026-09-27: the conformance run's success condition.
CONFORMANCE_EXCLUSION = ("the CommonMark conformance run FAILS when any example is excluded: a planted GFM "
                         "table beside an aligned paragraph is red, the paragraph alone green")
MUTANTS += [
    ("R38 conformance: an excluded example fails the run (restore `not fails and passed > 0`)",
     "plan_memo_selftest_conformance.py",
     "    return not fails and not unexpected and passed > 0, \"\\n\".join(lines)",
     "    return not fails and passed > 0, \"\\n\".join(lines)",
     [CONFORMANCE_EXCLUSION]),
    ("R38 conformance: the header counts an unexpected exclusion as a FAIL (count only the misaligned)",
     "plan_memo_selftest_conformance.py",
     "                                                               len(fails) + len(unexpected))]",
     "                                                               len(fails))]",
     [CONFORMANCE_EXCLUSION]),
]

# The same class in three generated-population controls: each skip is now red.
# A planted skip turns each one red; the floor it had before stayed green.
R38_RENDER = ("PROPERTY: the verdict is invariant under a §2.5 re-spelling of any prose character the "
              "document renders the same (the rendered-text rule, swept position by position)")
R38_AGREE = ("PROPERTY: every name the sibling resolver accepts, standing alone in prose, is ONE file "
             "token (`plan_memo_tokens.file_and_cite_spans`) (the correspondence FILE_SUFFIX's comment asserts)")
R38_RUN = ("PROPERTY: a run the sibling resolver FOLLOWS leaves no id outside its file span -- never a "
           "prefix with the remainder left for the naming scan (the direction the correspondence forbids)")
MUTANTS += [
    ("R38 render: the swept characters are the declared set (plant a branch on `q`, a character the "
     "prose never holds: the sweep excludes it in silence)", "plan_memo_lexer.py",
     '        if c == "&":', '        if c == "&" or c == "q":',
     [R38_RENDER]),
    ("R38 agree: every generated name resolves (plant a resolver that refuses `((`: 20 of 40 left)",
     "plan_memo_sibling.py",
     '    raw = re.split(r"[#?]", dest, 1)[0]',
     '    raw = re.split(r"[#?]", dest, 1)[0]\n    if "((" in dest:\n        return None',
     [R38_AGREE]),
    ("R38 run: every generated run is followed (plant a resolver that refuses a fragment: 12 of 40 left)",
     "plan_memo_sibling.py",
     '    raw = re.split(r"[#?]", dest, 1)[0]',
     '    raw = re.split(r"[#?]", dest, 1)[0]\n    if "#" in dest:\n        return None',
     [R38_RUN]),
]

# -- PR #510 Codex R40 of 2026-09-28: the root memo's banner.
BANNER_DISPLAY = ("the report banner names the root memo by its display name: the same line whether the "
                  "checker is invoked with an absolute or a relative path")
MUTANTS += [
    ("R40 banner: the root memo is printed by its display name (restore the raw `pop.main.path`)",
     "plan-memo-umbrella-check.py",
     'print(printable("plan-memo-umbrella-check  --  %s" % pop.display(pop.main.path)))',
     'print(printable("plan-memo-umbrella-check  --  %s" % pop.main.path))',
     [BANNER_DISPLAY]),
]
