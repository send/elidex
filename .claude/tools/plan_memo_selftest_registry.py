#!/usr/bin/env python3
"""THE ONE COLLECTION STEP of the self-test's two row registries -- the case
records (`CASES`) and the mutation rows (`MUTANTS`).

⚠ WHY IT EXISTS (the fifth and sixth attestations).  Both registries were a
base module's list that other modules EXTENDED at import time, gathered by
whoever imported them: the runner by a hand list, the loop ratchet by a glob,
the controls module by five `import` lines.  So which rows existed depended on
what had been imported, and in what order: a cases module on disk that nobody
imported never ran (rc 0), dropping one import lost 71 controls in silence,
and the append-to-base refusal sampled the base list too late to see a module
imported earlier.  The rules are now structural:

  * a registry module is one that holds its OWN list, read off its top level
    (`plan_memo_selftest_harness.registry_modules`, by content, not by name);
  * the BASE module's list is SEALED -- a tuple -- the moment the base module
    finishes importing, so appending to it fails at the appender's own import
    whatever the order (for `CASES`, the base's spellings refuse too);
  * `collect` is the only reader: base rows, then each module's own list; a
    module holding no list, an EMPTY list, or the SAME list as another module
    (the base's included) is refused.

Every rule has a partner control (`plan_memo_selftest_population.
registry_step_control`) and a killing row (`plan_memo_selftest_mutants_
population.py`).  A LEAF: it owns no controls, and every caller imports it at
call time, so a row against it is reached.

THE THREAT MODEL -- the ONE home of it (the harness points here).  Decided by
the user on 2026-09-23: the registry guards against ACCIDENTAL DRIFT and
against GAPS IN THE PROOF (a check whose change would go undetected); it does
NOT guard against deliberate tampering by the self-test's own code, which is
visible in a diff and in review.  For each class below, the question asked
first was whether an ACCIDENTAL edit reaches it; a class an accident reaches
is in the model, whatever it looks like.

IN THE MODEL (each has a partner arm and a killing row):
  * a registry module nothing imports, or one renamed -- content partition +
    `registry_membership_control` (probes e2g, e2i);
  * the old append-to-base pattern -- the seal (probes E2-11, e2d);
  * an ALIAS of another module's list: `MUTANTS = sib.MUTANTS`, or `from sib
    import MUTANTS` then `MUTANTS += [...]`, which extends `sib`'s list in place
    -- the alias refusal above (probe E2c, 510 rows at rc 0);
  * rows bound to a list nobody collects -- `spellings([])` instead of
    `spellings(CASES)` is one wrong argument, so it IS accidental: refused at
    the caller's import (`plan_memo_selftest_cases.spellings`; probes E2b /
    E2b');
  * a whole registry list emptied, or a name collision holding an empty
    `MUTANTS` -- the empty-list refusal above (probes E3d' / E3h / E2a');
  * a row naming no control, a label or an edit repeated
    (`plan_memo_selftest_mutants.row_problems`); a control name shadowed, or
    a fragment nobody merges (`plan_memo_selftest_harness.Registry`,
    `registry_merge_control`, `fragment_totality_control`; probes N-1, N-4).

OUT OF THE MODEL (deliberate only -- with the reasoning that no accident
reaches it, and the probe that demonstrates the class):
  * REASSIGNING another module's list attribute -- `_b.MUTANTS =
    _b.MUTANTS[:-50]` (probes E3a' 422 rows / E3f', rc 0).  An accident cannot
    write it: rebinding one's OWN `MUTANTS`, even one imported by name, only
    rebinds the local name; changing another module's needs an explicit
    attribute assignment on its module object;
  * DELETING PART of another module's list in place -- `del _i.MUTANTS[:10]`.
    Deleting ALL of it is in the model (empty-list refusal), and the accidental
    precursor -- holding another module's list under one's own name -- is the
    alias refusal; a partial delete through a module handle is written on
    purpose;
  * mutating another module's ROWS -- `for r in _b.MUTANTS: r[4].clear()`
    (probe E3c').  The accidental form, a row written with no control, is in
    the model (`row_problems`); reaching into another module's rows is not;
  * mutating BUILTINS or the STANDARD LIBRARY at import -- `builtins.sorted =
    ...`, `ast.dump = ...`.  A module-level `def sorted` or `list = [...]`
    shadows only that module's own namespace; a global change needs an
    explicit `import builtins` / attribute assignment.  The one accidental
    global state the self-test itself creates -- `sys.path` / `sys.modules`
    entries from a planted module -- is restored in a `finally` by every
    partner that plants.
"""


def collect(listname, base, names=None):
    """Every row of `listname`: the base module's sealed rows, then each
    registry module's OWN list, in population order.  `names` (default: the
    harness's `registry_modules(listname)`) lets the partner plant a module."""
    import importlib
    harness = importlib.import_module("plan_memo_selftest_harness")
    base_rows = getattr(importlib.import_module(base), listname)
    rows = list(base_rows)
    held = {id(base_rows): base}
    for name in (harness.registry_modules(listname) if names is None else names):
        if name == base:
            continue
        own = getattr(importlib.import_module(name), listname, None)
        if not own:
            # no list at all, or an EMPTY one (emptied, or a name collision):
            # one refusal, since `None` and `[]` are both nothing to collect
            raise RuntimeError("registry module %s holds no %s rows -- no list, or an EMPTY one "
                               "(emptied, or a name collision)" % (name, listname))
        if id(own) in held:
            # an ALIAS: `MUTANTS = sibling.MUTANTS`, or `from sibling import
            # MUTANTS` followed by `MUTANTS += [...]` (which extends the
            # sibling's list in place) -- the accidental path to duplicated
            # rows (the sixth-pass attestation's MIN-6: 510 rows, rc 0)
            # (the base's list is in `held` from the start, so a module whose
            # list IS the base's is this refusal too -- a separate `is` test
            # stood here and was subsumed, found unkillable by its own row)
            raise RuntimeError("registry module %s holds the SAME %s list as %s"
                               % (name, listname, held[id(own)]))
        held[id(own)] = name
        rows += own
    return rows
