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

from plan_memo_selftest_cases import ASSERT_CASES, CASES, RC_CASES, build
from plan_memo_tables import MARKER

HERE = pathlib.Path(__file__).resolve().parent

# Import name -> file, in dependency order.  The checker's file name is not an
# import name, so it is loaded under a fixed one.
MODULES = [
    ("plan_memo_lexer", "plan_memo_lexer.py"),
    ("plan_memo_tables", "plan_memo_tables.py"),
    ("plan_memo_roles", "plan_memo_roles.py"),
    ("plan_memo_umbrella_check", "plan-memo-umbrella-check.py"),
]


def load(patches=None):
    """A FRESH module set, exec'd from source text (`patches` = {file name:
    source} overrides), installed in `sys.modules` in dependency order so the
    checker's own imports resolve to the patched modules.  Returns the checker
    module.  `unload()` removes the set again."""
    patches = patches or {}
    unload()
    mod = None
    for name, file in MODULES:
        src = patches.get(file)
        if src is None:
            src = (HERE / file).read_text()
        spec = importlib.util.spec_from_loader(name, loader=None, origin=str(HERE / file))
        mod = importlib.util.module_from_spec(spec)
        mod.__file__ = str(HERE / file)
        sys.modules[name] = mod
        exec(compile(src, str(HERE / file), "exec"), mod.__dict__)
    return mod


def unload():
    for name, _ in MODULES:
        sys.modules.pop(name, None)


def run_on(M, text, prose="", sibling=None, files=None):
    """Write the fixture and its siblings to a scratch dir and run `check()`.
    Returns (no-owner ids, unlicensed mentions, findings, rc)."""
    with tempfile.TemporaryDirectory() as d:
        p = pathlib.Path(d) / "fixture.md"
        p.write_text(text + "\n" + prose + "\n")
        # Every fixture link resolves to this file (an absent target is rc 2);
        # its name carries an id so the destination-masking control keeps its
        # subject.
        (pathlib.Path(d) / "slice-9z-sib.md").write_text((sibling or "") + "\n")
        for name, content in (files or {}).items():
            (pathlib.Path(d) / name).write_text(content)
        res = M.check(str(p))
        umb = res.population.no_owner_ids()
        return umb, [m for m in res.mentions if not m.licensed], res.findings, res.rc


# ----------------------------------------------------------------------------
# Each control is a function of the checker module returning (ok, detail).
# The mutant runner re-runs controls BY NAME against a patched module set, so
# the registry is name -> callable and every control is reachable that way.
# ----------------------------------------------------------------------------


def naming_control(kind, text, prose, expect, sibling, files):
    def run(M):
        _, reported, _, rc = run_on(M, text, prose, sibling, files)
        got = len(reported)
        # Exact, not `>=`: every fixture carries exactly one intended site, so a
        # scanner that reports one site twice must turn a control red rather
        # than inflate the production census behind a green self-test.
        ok = got == expect and rc != 2
        return ok, "%d reported (expected %d), rc %d :: %s" % (
            got, expect, rc, [m.context()[:80] for m in reported])
    return run


def assert_control(text, code, expect, prose, sibling):
    def run(M):
        _, _, findings, rc = run_on(M, text, prose, sibling)
        got = sum(1 for c, _, _, _ in findings if c == code)
        return got == expect and rc != 2, "%s x%d (expected %d), rc %d" % (code, got, expect, rc)
    return run


def rc_control(text, prose, sibling, files, expect):
    def run(M):
        _, _, findings, rc = run_on(M, text, prose, sibling, files)
        return rc == expect, "rc %d (expected %d) :: %s" % (
            rc, expect, [f[0] + " " + f[3][:60] for f in findings if not f[0].endswith("?")][:3])
    return run


def attribution_control(M):
    """A pointer slot whose cell opens `Slice **9z** -- **UMBRELLA, ...**` is
    declaring 9z's kind, not its own.  §5: a pointer slot "carries no marker of
    its own".  The count must not move when such a row is added."""
    base = build()
    ptr = build(wb="**(carved at PR-B)** Slice **9z** — **UMBRELLA, not a terminal unit** — points into §5.")
    n = [len(run_on(M, t)[0]) for t in (base, ptr)]
    return n[0] == n[1], "a marker naming another row does not enter the count (%d -> %d)" % tuple(n)


def degenerate_control(M):
    """A whole-line grep for the marker CANNOT disagree with the marker count;
    the declaring-field parse can.  This proves the two are different programs
    rather than one program written twice."""
    text = build(d7z="**UMBRELLA, not a terminal unit** stray")
    umb = run_on(M, text)[0]
    by_grep = sum(1 for l in text.split("\n") if l.startswith("|") and MARKER in l)
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
        umb, reported, _, rc = run_on(M, build(extra=extra))
        out.append((rc, sorted(umb), sorted((m.id, m.source, m.line[m.start:m.end]) for m in reported)))
    return out[0] == out[1] and out[0][0] != 2, "ids %s, sites %s" % (out[0][1], out[0][2])


def raw_offset_control(M):
    """A site after a `\\|` in its cell is reported at its RAW column."""
    _, reported, _, _ = run_on(M, build(c1=r"x \| Slice 9z owns it"))
    ok = len(reported) == 1 and reported[0].line[reported[0].idpos:reported[0].idpos + 2] == "9z"
    return ok, "idpos lands on %r" % (reported[0].line[reported[0].idpos:reported[0].idpos + 2] if reported else None)


def registry():
    """name -> (kind, control)."""
    reg = {}
    for kind, name, text, prose, expect, sibling, files in CASES:
        reg[name] = (kind, naming_control(kind, text, prose, expect, sibling, files))
    for kind, name, text, code, expect, prose, sibling in ASSERT_CASES:
        reg[name] = (kind, assert_control(text, code, expect, prose, sibling))
    for kind, name, text, prose, sibling, files, rc in RC_CASES:
        reg[name] = (kind, rc_control(text, prose, sibling, files, rc))
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
