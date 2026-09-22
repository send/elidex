#!/usr/bin/env python3
"""THE ONE COLLECTION STEP of the self-test's two row registries -- the case
records (`CASES`) and the mutation rows (`MUTANTS`).

A registry module is one that holds its OWN list, read off its top level
(`plan_memo_selftest_harness.registry_modules`, by content, not by name).
`collect` is the only reader: the base module's rows, then each registry
module's list, in population order.  It FREEZES what it reads -- every row a
tuple all the way down, and each module's list attribute replaced by that
frozen tuple -- so nothing that runs after collection can change a row in
place (a control's `.pop()` raises).

THE THREAT MODEL -- its ONE home (the harness points here).  Decided by the
user on 2026-09-23 and instrumented by the golden manifest
(`plan_memo_selftest_manifest`, user-approved the same day):

  * ANY accidental change to the row set or to row content -- a row, case or
    control added, dropped, duplicated, renamed, shadowed or re-pointed, by
    whatever spelling -- is caught by the manifest diff that every
    `--self-test` run takes against the value it then executes;
  * the ONE out-of-model case is a deliberate edit that ALSO regenerates the
    manifest: that is not silent, because the manifest's own diff is in the
    commit, which is the review surface.

A LEAF: it owns no controls, and every caller imports it at call time, so a
row against it is reached.
"""


def _freeze(x):
    """`x` with every list, tuple, dict and record inside it made a tuple (a
    dict becomes sorted pairs), so it cannot be changed in place."""
    if hasattr(x, "_fields"):      # a record (only a namedtuple carries `_fields`)
        return type(x)(*(_freeze(v) for v in x))
    if isinstance(x, (list, tuple)):
        return tuple(_freeze(v) for v in x)
    if isinstance(x, dict):
        return tuple(sorted((k, _freeze(v)) for k, v in x.items()))
    return x


def collect(listname, base):
    """Every row of `listname`: the base module's rows, then each registry
    module's own list, in population order -- frozen, and each module's list
    attribute replaced by its frozen copy."""
    import importlib
    harness = importlib.import_module("plan_memo_selftest_harness")
    rows = []
    for name in [base] + [n for n in harness.registry_modules(listname) if n != base]:
        mod = importlib.import_module(name)
        frozen = _freeze(getattr(mod, listname))
        setattr(mod, listname, frozen)
        rows += frozen
    return tuple(rows)
