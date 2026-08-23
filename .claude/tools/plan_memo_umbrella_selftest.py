#!/usr/bin/env python3
"""Firing proofs and controls for `plan-memo-umbrella-check.py`.

A checker that has never been shown to fire is a checker that reports `0`
for a class it cannot see.  Every check in the companion has at least one
control here, and the controls come in four kinds:

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
it.  The registries live in `plan_memo_selftest_cases.py`; the mutants (a
re-executable proof that each control can go red) in
`plan_memo_selftest_mutants.py`.

Run:  python3 .claude/tools/plan-memo-umbrella-check.py --self-test [--mutants]
"""

import importlib.util
import pathlib
import sys
import tempfile

from plan_memo_selftest_cases import CASES, build

HERE = pathlib.Path(__file__).resolve().parent

# Import name -> file, in dependency order.  The checker's file name is not an
# import name, so it is loaded under a fixed one.
MODULES = [
    ("plan_memo_lexer", "plan_memo_lexer.py"),
    ("plan_memo_tables", "plan_memo_tables.py"),
    ("plan_memo_roles", "plan_memo_roles.py"),
    ("plan_memo_umbrella_check", "plan-memo-umbrella-check.py"),
]


_CODE = {}      # file name -> code object of the UNPATCHED source (immutable)


def load(patches=None):
    """A FRESH module set, exec'd from source text (`patches` = {file name:
    source} overrides), installed in `sys.modules` in dependency order so the
    checker's own imports resolve to the patched modules.  Returns the checker
    module.  `unload()` removes the set again.  Unpatched sources are compiled
    once; every call still execs into fresh module dicts."""
    patches = patches or {}
    unload()
    mod = None
    for name, file in MODULES:
        src = patches.get(file)
        if src is not None:
            code = compile(src, str(HERE / file), "exec")
        elif file in _CODE:
            code = _CODE[file]
        else:
            code = _CODE[file] = compile((HERE / file).read_text(), str(HERE / file), "exec")
        spec = importlib.util.spec_from_loader(name, loader=None, origin=str(HERE / file))
        mod = importlib.util.module_from_spec(spec)
        mod.__file__ = str(HERE / file)
        sys.modules[name] = mod
        exec(code, mod.__dict__)
    return mod


def unload():
    for name, _ in MODULES:
        sys.modules.pop(name, None)


def run_on(M, text, prose="", sibling=None, files=None):
    """Write the fixture and its siblings to a scratch dir and run `check()`.
    Returns (Result, unlicensed mentions)."""
    with tempfile.TemporaryDirectory() as d:
        p = pathlib.Path(d) / "fixture.md"
        p.write_text(text + "\n" + prose + "\n")
        # Every fixture link resolves to this file (an absent target is rc 2);
        # its name carries an id so the destination-masking control keeps its
        # subject.
        (pathlib.Path(d) / "slice-9z-sib.md").write_text((sibling or "") + "\n")
        for name, content in (files or {}).items():
            f = pathlib.Path(d) / name
            f.parent.mkdir(parents=True, exist_ok=True)
            f.write_text(content)
        res = M.check(str(p))
        return res, [m for m in res.mentions if not m.licensed]


# ----------------------------------------------------------------------------
# Each control is a function of the checker module returning (ok, detail).
# The mutant runner re-runs controls BY NAME against a patched module set, so
# the registry is name -> callable and every control is reachable that way.
# ----------------------------------------------------------------------------


def measure(res, reported, m):
    """The value a `Case.measure` takes on one run -> (got, detail)."""
    if m == "sites":
        return len(reported), "reported :: %s" % [x.context()[:80] for x in reported]
    if m == "rc":
        return res.rc, "rc :: %s" % [f[0] + " " + f[3][:60] for f in res.mechanical][:3]
    what, arg = m
    if what == "finding":
        return sum(1 for f in res.findings if f[0] == arg), arg
    if what == "note":
        return sum(1 for n in res.notes if arg in n), "note %r" % arg
    if what == "schema":
        # the one measure that reads a schema-miss run: SCHEMA findings carrying `arg`
        return (sum(1 for f in res.findings if f[0] == "SCHEMA" and arg in f[3]),
                "SCHEMA %r (rc must be 2 iff any)" % arg)
    if what == "id":
        return int(arg in res.population.ids), "id %r declared" % arg
    raise ValueError(m)


def control(c):
    """The one factory: run the fixture, take the measure, compare EXACTLY.
    Exact, not `>=`: every fixture carries exactly one intended site, so a
    scanner that reports one site twice must turn a control red rather than
    inflate the production census behind a green self-test.  Every measure
    but `rc` and `schema` also requires the run not to be a schema miss;
    `schema` requires rc 2 exactly when it counts one."""
    def run(M):
        res, reported = run_on(M, c.text, c.prose, c.sibling, c.files)
        got, detail = measure(res, reported, c.measure)
        if c.measure == "rc":
            ok = got == c.expect
        elif c.measure[0] == "schema":
            ok = got == c.expect and (res.rc == 2) == (got > 0)
        else:
            ok = got == c.expect and res.rc != 2
        return ok, "%s = %d (expected %d), rc %d" % (detail, got, c.expect, res.rc)
    return run


def attribution_control(M):
    """A pointer slot whose cell opens `Slice **9z** -- **UMBRELLA, ...**` is
    declaring 9z's kind, not its own.  §5: a pointer slot "carries no marker of
    its own".  The count must not move when such a row is added."""
    base = build()
    ptr = build(wb="**(carved at PR-B)** Slice **9z** — **UMBRELLA, not a terminal unit** — points into §5.")
    n = [len(run_on(M, t)[0].population.no_owner_ids()) for t in (base, ptr)]
    return n[0] == n[1], "a marker naming another row does not enter the count (%d -> %d)" % tuple(n)


def degenerate_control(M):
    """A whole-line grep for the marker CANNOT disagree with the marker count;
    the declaring-field parse can.  This proves the two are different programs
    rather than one program written twice."""
    import plan_memo_tables   # the FRESHLY loaded module, not the import-time one
    text = build(d7z="**UMBRELLA, not a terminal unit** stray")
    umb = run_on(M, text)[0].population.no_owner_ids()
    by_grep = sum(1 for l in text.split("\n") if l.startswith("|") and plan_memo_tables.MARKER in l)
    return len(umb) != by_grep, "declaring-field parse=%d vs whole-line marker grep=%d (must differ)" % (
        len(umb), by_grep)


def pipe_shape_control(M):
    """The same table written with and without leading/trailing pipes yields
    the same ids and the same reported sites (I-C cell shape)."""
    piped = ("| Slot | Why deferred | Trigger | Re-eval |\n|---|---|---|---|\n"
             "| `#11-zz-gamma` | **UMBRELLA, not a terminal unit.** Slice 9z lands first. | now | 2026-12-31 |")
    bare = "\n".join(l.strip("|") for l in piped.split("\n"))
    out = []
    for extra in (piped, bare):
        res, reported = run_on(M, build(extra=extra))
        out.append((res.rc, sorted(res.population.no_owner_ids()), sorted((m.id, m.source, m.text[m.start:m.end]) for m in reported)))
    return out[0] == out[1] and out[0][0] != 2, "ids %s, sites %s" % (out[0][1], out[0][2])


def raw_offset_control(M):
    """A site after a `\\|` in its cell is reported at its RAW column."""
    _, reported = run_on(M, build(c1=r"x \| Slice 9z owns it"))
    ok = len(reported) == 1 and reported[0].line[reported[0].col:reported[0].col + 2] == "9z"
    return ok, "col lands on %r" % (reported[0].line[reported[0].col:reported[0].col + 2] if reported else None)


def registry():
    """name -> (kind, control)."""
    reg = {}
    for c in CASES:
        assert c.name not in reg, "duplicate control name %r" % c.name
        reg[c.name] = (c.kind, control(c))
    reg["a marker naming another row does not enter the count"] = ("CONTROL", attribution_control)
    reg["declaring-field parse and whole-line marker grep differ"] = ("CONTROL", degenerate_control)
    reg["a table with and without edge pipes reads the same"] = ("CONTROL", pipe_shape_control)
    reg["a site after an escaped pipe is reported at its raw column"] = ("CONTROL", raw_offset_control)
    return reg


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
    if mutants:
        import plan_memo_selftest_mutants as mm
        fails += mm.run(reg)
    if fails:
        print()
        for f in fails:
            print("FAIL: %s" % f)
        return 1
    print("all controls behaved as declared.")
    return 0
