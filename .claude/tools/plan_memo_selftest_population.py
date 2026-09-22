#!/usr/bin/env python3
"""Controls over the MODULE POPULATION and the two ROW REGISTRIES -- the
partner controls of every mechanism `plan_memo_selftest_harness` (the one
population) and `plan_memo_selftest_registry` (the one collection step) hold.

WHY A MODULE OF ITS OWN (the sixth attestation).  Three passes in a row added a
mechanism here -- a glob, a file-name partition, an import order, a collection
step, a seal -- and each pass left some of them without a control that could
go red, so the next attestation's probes found them silent.  The rule is now
that a mechanism arrives WITH its partner here and its killing row in
`plan_memo_selftest_mutants_population.py`.  Every partner works on a
TEMPORARY directory or a PLANTED module, never on the real tree, because the
real tree is exactly the input on which a wrong rule and a right one agree.

Every harness / registry function is reached through a CALL-TIME import, so a
row that patches either module (both are leaves: they own no controls) is
seen by the control it names.
"""

import ast
import importlib
import pathlib
import sys
import tempfile

from plan_memo_selftest_properties import _swept_sources


def _h():
    return importlib.import_module("plan_memo_selftest_harness")


def _tree(d, files):
    for name, text in files.items():
        (d / name).write_text(text, encoding="utf-8")


# -- the population ----------------------------------------------------------

_ENTRY_SRC = "import plan_memo_a\n"
_DIR = {
    "plan_memo.py": "",                    # the glob is `plan_memo*.py`, not `plan_memo_*.py`
    "plan_memoize.py": "",
    "plan_memo_a.py": "import plan_memo_b\n",   # an `ast.Import` edge: b loads first
    "plan_memo_b.py": "",
    "plan_memo_selftest_mutants.py": "MUTANTS = []\n",
    "plan_memo_selftest_mut_r26.py": "MUTANTS = []\n",     # renamed out of any name prefix
    "plan_memo_selftest_mutants_zz.py": "X = 1\n",         # named like one, holds no list
    "plan_memo_umbrella_selftest.py": "",
    "memo_extra.py": "",                   # outside the glob
    "other.py": "",
}


def population_partner_control(M):
    """PROPERTY: the harness DERIVES the population from a directory -- the
    glob (`plan_memo*.py` plus the entry point), the self-test partition by
    file name, the registry partition by CONTENT (a module holding its own
    `MUTANTS`), and the checker load order from the imports, with a cycle an
    error -- each asked of a temporary directory the real tree does not look
    like."""
    H = _h()
    bad = []
    with tempfile.TemporaryDirectory() as d:
        d = pathlib.Path(d)
        _tree(d, dict(_DIR, **{H.ENTRY: _ENTRY_SRC}))
        want = sorted(f for f in _DIR if f.startswith("plan_memo")) + [H.ENTRY]
        if H.files(d) != want:
            bad.append("files() = %s, want %s" % (H.files(d), want))
        part = {f: H.is_selftest(f) for f in ("plan_memo_umbrella_selftest.py",
                                              "plan_memo_selftest_mut_r26.py", "plan_memo_a.py")}
        if part != {"plan_memo_umbrella_selftest.py": True, "plan_memo_selftest_mut_r26.py": True,
                    "plan_memo_a.py": False}:
            bad.append("is_selftest partition %s" % part)
        reg = H.registry_modules("MUTANTS", d)
        if reg != ["plan_memo_selftest_mut_r26", "plan_memo_selftest_mutants"]:
            bad.append("registry_modules('MUTANTS') = %s (want the two that HOLD a list)" % reg)
        order = [n for n, _f in H._import_order(d)]
        if order.index("plan_memo_b") > order.index("plan_memo_a"):
            bad.append("import order %s puts an importer before its `import`ed module" % order)
    with tempfile.TemporaryDirectory() as d:
        d = pathlib.Path(d)
        _tree(d, {"plan_memo_a.py": "import plan_memo_b\n", "plan_memo_b.py": "import plan_memo_a\n",
                  H.ENTRY: ""})
        try:
            H._import_order(d)
            bad.append("a top-level import cycle was NOT an error")
        except RuntimeError:
            pass
    return not bad, "; ".join(bad) or "glob, partition, registry-by-content, order and cycle all hold"


# -- the registries ----------------------------------------------------------

_M_LEGIT = "MUTANTS = [('planted', 'plan_memo_ids.py', 'x', 'y', [])]\n"
_M_OWNLESS = "from plan_memo_selftest_mutants import MUTANTS\n"
_M_APPENDER = ("MUTANTS = []\nimport plan_memo_selftest_mutants as _b\n"
               "_b.MUTANTS.append(('planted', 'plan_memo_ids.py', 'x', 'y', []))\n")
_C_LEGIT = ("from plan_memo_selftest_cases import build, spellings\nCASES = []\n"
            "case, acase, rcase = spellings(CASES)\n"
            "case('POSITIVE', 'planted legit', build(), '', 0)\n")
_C_OWNLESS = "from plan_memo_selftest_cases import CASES\n"
_C_SPELLER = ("from plan_memo_selftest_cases import build, case\n"
              "case('POSITIVE', 'planted via the base spelling', build(), '', 0)\n")
_C_LISTER = ("CASES = []\nimport plan_memo_selftest_cases as _b\n"
             "_b.CASES.append(_b.CASES[0])\n")


def _plant(d, name, body):
    (d / (name + ".py")).write_text(body, encoding="utf-8")
    return name


def _raises(fn):
    try:
        fn()
    except Exception:
        return True
    return False


def registry_step_control(M):
    """PROPERTY: each row registry is gathered in ONE step, and no module can
    extend the base list in ANY import order -- planted, for both registries:
    a legitimate module (collected), a module whose list IS the base's
    (refused), and modules that extend the base (refused at their OWN import,
    so importing them first -- before any collection -- is refused too; the
    sixth attestation's order-dependent case).  The unplanted collection
    succeeds, with the base rows once."""
    mm = importlib.import_module("plan_memo_selftest_mutants")
    cm = importlib.import_module("plan_memo_selftest_cases")
    arms = {}
    planted = []
    with tempfile.TemporaryDirectory() as d:
        d = pathlib.Path(d)
        sys.path.insert(0, str(d))
        try:
            p = _plant(d, "plan_memo_selftest_plant_m_legit", _M_LEGIT)
            planted.append(p)
            arms["mutants: a legit module is collected"] = (
                ("planted", "plan_memo_ids.py", "x", "y", []) in mm.mutants([p]))
            p = _plant(d, "plan_memo_selftest_plant_m_ownless", _M_OWNLESS)
            planted.append(p)
            arms["mutants: a module whose list IS the base's is refused"] = _raises(
                lambda: mm.mutants([p]))
            p = _plant(d, "plan_memo_selftest_plant_m_appender", _M_APPENDER)
            planted.append(p)
            arms["mutants: an appender fails at its own import (imported FIRST)"] = _raises(
                lambda: importlib.import_module(p))
            sys.modules.pop(p, None)
            arms["mutants: ... and through the collection step"] = _raises(lambda: mm.mutants([p]))
            p = _plant(d, "plan_memo_selftest_plant_c_legit", _C_LEGIT)
            planted.append(p)
            arms["cases: a legit module is collected"] = any(
                c.name == "planted legit" for c in cm.cases([p]))
            p = _plant(d, "plan_memo_selftest_plant_c_ownless", _C_OWNLESS)
            planted.append(p)
            arms["cases: a module whose list IS the base's is refused"] = _raises(lambda: cm.cases([p]))
            for tag, body in (("speller", _C_SPELLER), ("lister", _C_LISTER)):
                p = _plant(d, "plan_memo_selftest_plant_c_" + tag, body)
                planted.append(p)
                arms["cases: a %s fails at its own import" % tag] = _raises(
                    lambda: importlib.import_module(p))
        finally:
            sys.path.remove(str(d))
            for p in planted:
                sys.modules.pop(p, None)
    # an exception from the unplanted step is the arm going red, not a crash:
    # it is exactly what a step that refuses the base's own list does
    try:
        rows = mm.mutants()
        base = len(mm.MUTANTS)
        arms["the unplanted collection holds the base rows exactly once"] = (
            rows[:base] == list(mm.MUTANTS) and not any(r is mm.MUTANTS[0] for r in rows[base:]))
    except RuntimeError:
        arms["the unplanted collection holds the base rows exactly once"] = False
    try:
        cs = cm.cases()
        arms["the unplanted case collection holds the base cases exactly once"] = (
            cs[:len(cm.CASES)] == list(cm.CASES) and len({c.name for c in cs}) == len(cs))
    except RuntimeError:
        arms["the unplanted case collection holds the base cases exactly once"] = False
    failed = [k for k, v in arms.items() if not v]
    return not failed, ("%d of %d arm(s) hold%s" % (len(arms) - len(failed), len(arms),
                                                      ("; FAILED: " + "; ".join(failed)) if failed else ""))


# -- membership: nothing related to the population is outside it, and nothing
# in it is unreached --------------------------------------------------------

def _imports(src, file):
    got = set()
    for node in ast.walk(ast.parse(src, filename=file)):
        if isinstance(node, ast.ImportFrom) and node.module:
            got.add(node.module)
        elif isinstance(node, ast.Import):
            got |= {a.name for a in node.names}
    return got


def _membership_verdict(disk, population, roots):
    """The rule's violations over {file: text} of a directory, its population
    and the roots a run starts from: (1) a file OUTSIDE the population that
    imports it or that it imports; (2) a population module reached from no
    root along the import graph -- on disk, and never run."""
    H = _h()
    stem = {H.import_name(f): f for f in disk}
    inside = {H.import_name(f) for f in population}
    graph = {H.import_name(f): _imports(src, f) & set(stem) for f, src in disk.items()}
    bad = []
    for name, deps in sorted(graph.items()):
        if name in inside:
            bad += ["%s is imported by the population but is not in it" % stem[x]
                    for x in sorted(deps - inside)]
        elif deps & inside:
            bad.append("%s imports the population but is not in it" % stem[name])
    seen, todo = set(), [r for r in roots if r in inside]
    while todo:
        u = todo.pop()
        if u in seen:
            continue
        seen.add(u)
        todo += sorted(graph.get(u, set()) & inside)
    bad += ["%s is in the population and reached from no root -- on disk, never run" % stem[n]
            for n in sorted(inside - seen)]
    return sorted(set(bad))


def _roots(H, here=None):
    return ([H.ENTRY_NAME] + H.registry_modules("MUTANTS", here)
            + H.registry_modules("CASES", here))


def registry_membership_control(M):
    """PROPERTY: no module can leave a run unnoticed -- every `.py` file beside
    the checker that the population imports, or that imports it, IS in the
    population; and every module of the population is REACHED from a root
    (the entry point, and each registry module the collection step gathers).

    ⚠ The sixth attestation's probes, each of which was rc 0 before this:
    a helper outside the glob imported by `plan_memo_roles.py` (its calls
    were never read); a cases module on disk that nothing imported (its case
    never ran); a new empty checker file, and a self-test-named helper that
    nothing imports.  The partner arms plant each."""
    H = _h()
    here = H.HERE
    disk = {p.name: p.read_text(encoding="utf-8") for p in sorted(here.glob("*.py"))}
    disk.update(dict(_swept_sources()))
    bad = _membership_verdict(disk, H.files(), _roots(H))
    probe = {
        "plan-memo-umbrella-check.py": "import plan_memo_roles\n",
        "plan_memo_roles.py": "import memo_extra\n",
        "memo_extra.py": "",
        "plan_memo_zz.py": "",
        "plan_memo_selftest_cases_zz.py": "CASES = []\n",
        "stranger.py": "import plan_memo_roles\n",
    }
    got = _membership_verdict(probe, [f for f in probe if f.startswith("plan") and "memo" in f],
                              [H.ENTRY_NAME, "plan_memo_selftest_cases_zz"])
    arms = [any("memo_extra.py is imported by the population" in b for b in got),
            any("stranger.py imports the population" in b for b in got),
            any(b.startswith("plan_memo_zz.py is in the population and reached from no root")
                for b in got),
            not any("plan_memo_selftest_cases_zz.py" in b for b in got)]
    return not bad and all(arms), ("%d file(s) on disk, %d violation(s)%s; probe arms %s"
                                   % (len(disk), len(bad), ("; " + "; ".join(bad[:3])) if bad else "",
                                      arms))


def registry():
    """name -> (kind, control), this module's fragment of the one table."""
    return {
        "PROPERTY: the harness DERIVES the population from a directory (glob, self-test partition, registry-by-content, import order, cycle) -- asked of a planted directory":
            ("CONTROL", population_partner_control),
        "PROPERTY: each row registry is gathered in ONE step and no module can extend the base list in any import order (planted: legit, ownless, appenders imported first)":
            ("CONTROL", registry_step_control),
        "PROPERTY: no module can leave a run unnoticed -- every file related to the population is in it, and every module in it is reached from a root":
            ("CONTROL", registry_membership_control),
    }
