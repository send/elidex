#!/usr/bin/env python3
"""Firing proofs and controls for `plan-memo-umbrella-check.py`: the runner.

A checker that has never been shown to fire is a checker that reports `0`
for a class it cannot see.  Every check in the companion has at least one
control, and the controls come in four kinds:

  POSITIVE          a wrong site the checker MUST report.
  POSITIVE-NOVEL    a wrong site written in a spelling that appears NOWHERE in
                    the memo and nowhere in the licensing tables.  This is the
                    control that decides whether the predicate is a rule or a
                    transcription of the sites it was written against.  If a
                    novel wrong spelling passes, the checker is a grep for
                    yesterday's mistakes.
  NEGATIVE          a licensed site the checker must NOT report.
  KNOWN-MISS        a wrong site the checker is KNOWN not to report.  These are
                    RED and stay red.  They are printed with the run so a
                    reader never reads the finding count as a coverage
                    statement.  Turning one green by widening the claim -- for
                    instance by redefining the population so the missed site is
                    out of scope -- is the failure
                    `feedback_control-rewritten-to-bless-the-defect` names.

Every control runs `check()` -- the SAME pipeline `main()` runs, not a copy of
it.  The self-test is four modules with one import direction:

  plan_memo_umbrella_selftest.py    this runner: `run()` -- load, run every
                                    control of the registry, print, then the
                                    mutation proof on request;
  plan_memo_selftest_controls.py    the function-shaped controls and
                                    `registry()`, the one name -> (kind,
                                    control) table;
  plan_memo_selftest_properties.py  the PROPERTY controls -- every control that
                                    enumerates its own population and sweeps it
                                    (the source-text, AST, code-object,
                                    re-spelling and line-ending sweeps); its
                                    `registry()` fragment is merged into that
                                    one table, and its entries are exactly the
                                    `PROPERTY: ...` names;
  plan_memo_selftest_work.py        the controls whose measure is WORK rather
                                    than text (the linearity witnesses); its
                                    `registry()` fragment is merged into that
                                    one table;
  plan_memo_selftest_cases.py /     the record-shaped controls (`Case`), the
  _selftest_cases_pr510.py /        fixture builder; one `CASES` list filled
  _selftest_cases_inline.py         by three modules at the review-round seam
                                    (pre-converge / R1-R16 / R17 on, the
                                    Phase-2 inline construct family);
  plan_memo_selftest_harness.py     the module loader, the fixture runner,
                                    the control factory, the work witnesses.

The mutants (a re-executable proof that each control can go red) are
`plan_memo_selftest_mutants.py` (the pre-converge rows + the runner),
`plan_memo_selftest_mutants_pr510.py` (PR #510 rounds R1-R16) and
`plan_memo_selftest_mutants_inline.py` (R17 on), all appending to the same
`MUTANTS`, split at the cases modules' seams.

Run:  python3 .claude/tools/plan-memo-umbrella-check.py --self-test [--mutants]
"""

from plan_memo_selftest_controls import empty_registry_fails, registry
from plan_memo_selftest_harness import load, unload


def run(mutants=False):
    fails = []
    counts = {}
    print("=" * 74)
    print("plan-memo-umbrella-check  --  self-test")
    print("=" * 74)
    M = load()
    reg = registry()
    for name, (kind, control) in reg.items():
        counts[kind] = counts.get(kind, 0) + 1
        ok, detail = control(M)
        if kind == "KNOWN-MISS":
            # `ok` means "reported 0": the site IS wrong, so the control stays red
            print("  RED  [KNOWN-MISS] %s -- %s (this site IS wrong)" % (name, detail))
            if not ok:
                fails.append("KNOWN-MISS %s now reports; update the declared miss class" % name)
            continue
        if not ok:
            fails.append("%s %s :: %s" % (kind, name, detail))
        print("  %-4s [%s] %s (%s)" % ("ok" if ok else "FAIL", kind, name, detail[:90]))
    unload()

    print()
    print("%d control(s): %s."
          % (len(reg), ", ".join("%d %s" % (counts[k], k) for k in sorted(counts))))
    n_mutants = None
    if mutants:
        import plan_memo_selftest_mutants as mm
        import plan_memo_selftest_mutants_pr510  # noqa: F401 -- appends R1-R16's mutants to MUTANTS
        import plan_memo_selftest_mutants_inline  # noqa: F401 -- appends R17-on's mutants to MUTANTS
        fails += mm.run(reg)
        n_mutants = len(mm.MUTANTS)
    fails += empty_registry_fails(len(reg), n_mutants)
    if fails:
        print()
        for f in fails:
            print("FAIL: %s" % f)
        return 1
    print("all controls behaved as declared.")
    return 0
