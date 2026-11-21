"""Shared test helper for the `_webref` suites: `mutate()` re-execs a module's
source with edits so a test can prove its assertion bites. Not a test module."""
from __future__ import annotations

import functools
import inspect
import types
import warnings


def mutate(mod, needle: str, replacement: str, *more: str):
    """Re-exec `mod`'s source with one edit: a scratch copy carrying the
    mutant, never written to disk. The needle must be present."""
    src = inspect.getsource(mod)
    pairs = [(needle, replacement), *zip(more[::2], more[1::2])]
    for n, _ in pairs:
        assert n in src, n
    m = types.ModuleType(mod.__name__ + "_mutant")
    m.__package__ = mod.__package__
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", DeprecationWarning)
        exec(compile(functools.reduce(lambda a, p: a.replace(*p), pairs, src),
                     "mutant", "exec"), m.__dict__)
    return m
