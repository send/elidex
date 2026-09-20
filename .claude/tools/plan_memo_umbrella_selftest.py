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
it.

⚠ THE PER-MODULE INVENTORY THAT STOOD HERE IS GONE (PR #510 R32).  It was a
SECOND list of the self-test's modules, hand-maintained, read by nothing -- the
entry point's `MODULES` section is the one home and it IS checked, in both
directions, by `module_map_completeness_control` / `module_map_existence_control`.
The copy here had already drifted: it opened "the self-test is four modules"
while listing eight, and the same commit that carved `plan_memo_selftest_
pipeline.py` out extended this list without adding it.  Two homes for one
record, one of which silently stopped being maintained -- so the record has one
home now, and what stays here is the only thing this file can say that the map
cannot:

  IMPORT DIRECTION, one way and no cycles: this runner imports the controls
  module; the controls module imports the RECORDS module (not the properties or
  invariants module -- those two reach the table through `records.registry()`,
  which merges `properties.registry()`, which merges the invariants module's),
  the work module, the case registry and, at three function-local sites, the
  conformance module; the work module imports the pipeline and growth modules.
  Every module of the set but the conformance module imports the harness, and
  the harness imports none of them.  A registry fragment is merged UPWARDS
  along that chain into the one name -> (kind, control) table the runner reads.

  ⚠ THIS PARAGRAPH REPLACED A STALE INVENTORY AT R32 AND WAS ITSELF FALSE until
  R38's design re-gate: it said "the controls module imports the property
  moduleS", plural, and the invariants module is not among its imports.  Written
  in the same commit that built `import_seam_control` for exactly this class --
  and that control cannot reach it, because it checks SYMBOL-level seams ("the
  only importer of X") and this is a MODULE-DIRECTION claim.  The tree states
  fourteen of those; a re-gate tested all fourteen and this one, the only one
  that commit authored, was the only false one.

  MUTANTS follow the cases modules' review-round seams and append to one
  `MUTANTS` list, read at one import site (the runner).

Run:  python3 .claude/tools/plan-memo-umbrella-check.py --self-test [--mutants]
"""

from plan_memo_selftest_controls import empty_registry_fails, registry
from plan_memo_selftest_harness import load, unload


def run(mutants=False):
    fails = []
    printable = None
    counts = {}
    print("=" * 74)
    print("plan-memo-umbrella-check  --  self-test")
    print("=" * 74)
    M = load()
    printable = M.printable    # the ONE escape, owned by the report boundary
    reg = registry()
    for name, (kind, control) in reg.items():
        counts[kind] = counts.get(kind, 0) + 1
        ok, detail = control(M)
        if kind == "KNOWN-MISS":
            # `ok` means "reported 0": the site IS wrong, so the control stays red
            print(printable("  RED  [KNOWN-MISS] %s -- %s (this site IS wrong)" % (name, detail)))
            if not ok:
                fails.append(printable("KNOWN-MISS %s now reports; update the declared miss class" % name))
            continue
        if not ok:
            fails.append(printable("%s %s :: %s" % (kind, name, detail)))
        print(printable("  %-4s [%s] %s (%s)" % ("ok" if ok else "FAIL", kind, name, detail[:90])))
    unload()

    print()
    print(printable("%d control(s): %s."
                    % (len(reg), ", ".join("%d %s" % (counts[k], k) for k in sorted(counts)))))
    n_mutants = None
    if mutants:
        import plan_memo_selftest_mutants as mm
        import plan_memo_selftest_mutants_pr510  # noqa: F401 -- appends R1-R16's mutants to MUTANTS
        import plan_memo_selftest_mutants_inline  # noqa: F401 -- appends R17-R25's mutants to MUTANTS
        import plan_memo_selftest_mutants_r26  # noqa: F401 -- appends R26-R29's mutants to MUTANTS
        import plan_memo_selftest_mutants_r30  # noqa: F401 -- appends R30-on's mutants to MUTANTS
        fails += mm.run(reg)
        n_mutants = len(mm.MUTANTS)
    fails += empty_registry_fails(len(reg), n_mutants)
    if fails:
        print()
        for f in fails:
            print(printable("FAIL: %s" % f))
        return 1
    print("all controls behaved as declared.")
    return 0
