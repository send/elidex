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
  * `collect` is the only reader: base rows, then each module's own list, and a
    module whose list IS the base's is refused.

Every rule has a partner control (`plan_memo_selftest_population.
registry_step_control`) and a killing row (`plan_memo_selftest_mutants_
population.py`).  A LEAF: it owns no controls, and every caller imports it at
call time, so a row against it is reached.
"""


def collect(listname, base, names=None):
    """Every row of `listname`: the base module's sealed rows, then each
    registry module's OWN list, in population order.  `names` (default: the
    harness's `registry_modules(listname)`) lets the partner plant a module."""
    import importlib
    harness = importlib.import_module("plan_memo_selftest_harness")
    base_rows = getattr(importlib.import_module(base), listname)
    rows = list(base_rows)
    for name in (harness.registry_modules(listname) if names is None else names):
        if name == base:
            continue
        own = getattr(importlib.import_module(name), listname, None)
        if own is None or own is base_rows:
            raise RuntimeError("registry module %s holds no %s list of its own" % (name, listname))
        rows += own
    return rows
