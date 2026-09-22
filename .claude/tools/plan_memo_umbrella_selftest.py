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

  IMPORT DIRECTION, as a RULE over the imports (module-level and
  function-local alike), checked by `plan_memo_selftest_records.
  import_direction_control` rather than listed: the import graph of the one
  population has NO CYCLE; the harness imports nothing of it; and no checker
  module imports a self-test module except the entry point's `--self-test`
  dispatch into this runner.  A registry fragment is merged UPWARDS along the
  graph into the one name -> (kind, control) table the runner reads.
  ⚠ Until the fifth attestation this was an INVENTORY of edges, and false: it
  said the controls module imports the conformance module at three
  function-local sites (four), omitted its import of the ratchets module, and
  said every self-test module but the conformance module imports the harness
  (ten others do not).

  ⚠ THIS PARAGRAPH REPLACED A STALE INVENTORY AT R32 AND WAS ITSELF FALSE until
  R38's design re-gate: it said "the controls module imports the property
  moduleS", plural, and the invariants module is not among its imports.  Written
  in the same commit that built `import_seam_control` for exactly this class --
  and that control cannot reach it, because it checks SYMBOL-level seams ("the
  only importer of X") and this is a MODULE-DIRECTION claim.  The tree stated
  fourteen of those at R38; a re-gate tested all fourteen and this one, the
  only one that commit authored, was the only false one.

  CASES and MUTANTS modules are carved at a review round or a subject; each
  holds its OWN list, and the one collection step
  (`plan_memo_selftest_registry.collect`) gathers them -- no list of modules is
  spelled anywhere (⚠ this said "append to one `MUTANTS` list, read at one
  import site" after that mechanism was replaced).

Run:  python3 .claude/tools/plan-memo-umbrella-check.py --self-test [--mutants]
"""

from plan_memo_selftest_controls import empty_registry_fails, registry  # noqa: F401 -- `registry` is what makes this module patchable by a mutation row (`plan_memo_selftest_mutants.run`)
from plan_memo_selftest_harness import load, unload
import plan_memo_selftest_manifest as manifest


def run(mutants=False):
    fails = []
    counts = {}
    # ⚠ THE SET IS LOADED BEFORE THE BANNER, and that ordering is the escape's
    # (PR #510 R42-8).  `printable` belongs to the report boundary, which is the
    # entry point, so the runner takes it off the LOADED module -- and the two
    # banner lines used to print before that binding existed.  Ordering the load
    # first is what lets the rule be "every non-literal line is escaped" with no
    # exemption for the two that happen to carry no memo text: an exemption list
    # is where the next unescaped site hides
    # (`memory/feedback_enumerated-exemptions-leave-the-next-class-authoritative.md`).
    M = load()
    printable = M.printable    # the ONE escape, owned by the report boundary
    print(printable("=" * 74))
    print("plan-memo-umbrella-check  --  self-test")
    print(printable("=" * 74))
    # THE ONE COLLECTION, compared against the committed golden manifest BEFORE
    # anything runs, and then executed as it was compared: the control table
    # and the mutation rows below are this snapshot's, never a second call.
    # WORKFLOW RULE: adding, removing or changing a row, a control or a case
    # requires `--write-manifest` and committing the manifest's diff.
    snap = manifest.snapshot()
    for line in manifest.verify(snap):
        print(printable(line))
        fails.append(printable(line))
    reg = snap.table
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
        # the registry is gathered by `mutants()`, the ONE step that imports its
        # modules -- no list of them is spelled here (it was, and it was inert)
        import plan_memo_selftest_mutants as mm
        fails += mm.run(reg, snap.rows)
        n_mutants = len(snap.rows)
    fails += empty_registry_fails(len(reg), n_mutants)
    if fails:
        print()
        for f in fails:
            print(printable("FAIL: %s" % f))
        return 1
    print("all controls behaved as declared.")
    return 0


def write_manifest():
    """`--write-manifest`: regenerate the golden manifest from the ONE
    collection (`plan_memo_selftest_manifest.write`), printed through the
    report boundary's escape."""
    printable = load().printable
    unload()
    rc, messages = manifest.write()
    for m in messages:
        print(printable(m))
    return rc
