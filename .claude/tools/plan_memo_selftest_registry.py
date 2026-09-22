#!/usr/bin/env python3
"""THE ONE COLLECTION STEP of the self-test's two row registries -- the case
records (`CASES`) and the mutation rows (`MUTANTS`).

A registry module is one that holds its OWN list, read off its top level
(`plan_memo_selftest_harness.registry_modules`, by content, not by name).
`collect` is the only reader: the base module's rows, then each registry
module's list, in population order.  An EMPTY list is refused (nothing to
compare, so the manifest cannot see it).  It FREEZES what it reads -- every row a
tuple all the way down, and each module's list attribute replaced by that
frozen tuple -- so nothing that runs after collection can change a row in
place (a control's `.pop()` raises).

THE THREAT MODEL -- its ONE home (the harness points here).  Decided by the
user on 2026-09-23 and instrumented by the golden manifest
(`plan_memo_selftest_manifest`, user-approved the same day):

  * ANY accidental change to the row set or to row content -- a row, case or
    control added, dropped, duplicated, renamed, shadowed, re-pointed or
    re-bodied, by whatever spelling -- is caught by the manifest diff that
    every `--self-test` run takes against the value it then executes, and the
    verified value is the ONLY one the runner can execute
    (`plan_memo_selftest_manifest.take`);
  * what a comparison cannot see is a row or case that was never COLLECTED, so
    those two paths are closed here instead: the case constructors resolve the
    calling module's `CASES` at call time (nothing can be written to a list
    the collection does not read), and a registry module holding an EMPTY list
    is refused;
  * the ONE out-of-model case is a deliberate edit that ALSO regenerates the
    manifest: that is not silent, because the manifest's own diff is in the
    commit, which is the review surface;
  * WHAT IT DOES NOT PROMISE: that a control can go RED.  The manifest pins a
    control's identity, kind, defining function and source digest; 254 of the
    747 are named by no mutation row, so nothing proves they would fail if
    their subject broke.  The workflow rule and this boundary have ONE home,
    `plan_memo_selftest_manifest`'s docstring.

A LEAF: it owns no controls, and every caller imports it at call time, so a
row against it is reached.
"""


CALLS = []
"""The LAST few `collect` calls, by list name, in order -- the instrument the
manifest partner reads to hold "one collection call per registry, and no
second enumeration" (I3).  Bounded, because an unbounded global that grows
with every call is a leak, not an instrument."""
_CALLS_KEPT = 8


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
    import sys
    harness = importlib.import_module("plan_memo_selftest_harness")
    import plan_memo_selftest_manifest as manifest
    ManifestError = manifest.ManifestError
    CALLS.append(listname)
    del CALLS[:-_CALLS_KEPT]
    # THE POPULATION IS WHAT THE MODULES HOLD AT RUNTIME.  The file-name and
    # assignment-shape rules only say which modules to IMPORT; membership is
    # then decided by the attribute each imported module actually has -- the
    # same object the constructors appended to.  ⚠ The AST rule alone was a
    # SECOND, NARROWER population (the CI attestation's CRIT-2): `globals()
    # ["CASES"] = []`, `CASES, _X = [], 1` and `for CASES in ([],)` all bind a
    # list the constructors write to and `ast` does not see as an assignment,
    # so those cases were collected by nothing and no manifest line was made.
    for name in harness.registry_modules(listname):
        importlib.import_module(name)
    holders = sorted(n for n, m in list(sys.modules.items())
                     if n.startswith("plan_memo") and "selftest" in n and n != base
                     and isinstance(getattr(m, listname, None), (list, tuple)))
    rows = []
    for name in [base] + holders:
        mod = importlib.import_module(name)
        frozen = _freeze(getattr(mod, listname))
        if not frozen:
            # EMPTY: a list emptied by something else, or a name collision.  The
            # manifest cannot see it (it compares what WAS collected, and an
            # empty module contributes nothing to compare), so it is refused
            # here -- the manifest attestation's I1, a regression this restores.
            raise ManifestError(manifest.printable(
                "registry module %s holds an EMPTY %s list" % (name, listname)))
        setattr(mod, listname, frozen)
        rows += frozen
    return tuple(rows)
