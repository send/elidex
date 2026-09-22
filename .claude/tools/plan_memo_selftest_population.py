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
    "plan_memo_selftest_mut_ann.py": "MUTANTS: list = []\n",        # an AnnAssign binds it
    "plan_memo_selftest_mut_nested.py": "def f():\n    MUTANTS = []\n",  # not at top level
    "plan_memo_c.py": "from plan_memo_d import x\n",   # a `from X import` edge: d loads first
    "plan_memo_d.py": "x = 1\nMUTANTS = []\n",        # holds a list, but is no SELF-TEST module
    "plan_memo_umbrella_selftest.py": "",
    "memo_extra.py": "",                   # outside the glob
    "other.py": "",
}


def population_partner_control(M):
    """(See `_population_partner`; an exception from the harness is this
    control going red, not a crash.)"""
    try:
        return _population_partner()
    except Exception as e:
        return False, "the harness RAISED on the planted directory: %s: %s" % (type(e).__name__, e)


def _population_partner():
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
        if reg != ["plan_memo_selftest_mut_ann", "plan_memo_selftest_mut_r26",
                   "plan_memo_selftest_mutants"]:
            bad.append("registry_modules('MUTANTS') = %s (want the two that HOLD a list)" % reg)
        order = [n for n, _f in H._import_order(d)]
        if order.index("plan_memo_b") > order.index("plan_memo_a"):
            bad.append("import order %s puts an importer before its `import`ed module" % order)
        if order.index("plan_memo_d") > order.index("plan_memo_c"):
            bad.append("import order %s puts an importer before its `from`-imported module" % order)
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
_C_THROWAWAY = ("from plan_memo_selftest_cases import build, spellings\nCASES = []\n"
                "case, acase, rcase = spellings([])\n"
                "case('POSITIVE', 'planted into a throwaway list', build(), '', 0)\n")
_M_ALIAS = "import plan_memo_selftest_mutants_r26 as _r\nMUTANTS = _r.MUTANTS\n"
_M_LISTLESS = "X = 1\n"
_M_EMPTY = "MUTANTS = []\n"


def _refused(fn):
    """True when `fn` raises a RuntimeError -- a REFUSAL, as distinct from any
    other exception (a `TypeError` from adding `None` to a list is a crash)."""
    try:
        fn()
    except RuntimeError:
        return True
    except Exception:
        return False
    return False


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
            arms["mutants: a module whose list IS the base's is refused"] = _refused(
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
            arms["cases: a module whose list IS the base's is refused"] = _refused(lambda: cm.cases([p]))
            p = _plant(d, "plan_memo_selftest_plant_c_throwaway", _C_THROWAWAY)
            planted.append(p)
            arms["cases: spellings bound to a throwaway list fail at the import"] = _refused(
                lambda: importlib.import_module(p))
            p = _plant(d, "plan_memo_selftest_plant_m_alias", _M_ALIAS)
            planted.append(p)
            arms["mutants: a module aliasing a sibling's list is refused"] = _refused(
                lambda: mm.mutants(["plan_memo_selftest_mutants_r26", p]))
            p = _plant(d, "plan_memo_selftest_plant_m_empty", _M_EMPTY)
            planted.append(p)
            arms["mutants: a module holding an EMPTY list is refused"] = _refused(
                lambda: mm.mutants([p]))
            p = _plant(d, "plan_memo_selftest_plant_m_listless", _M_LISTLESS)
            planted.append(p)
            arms["mutants: a named module holding no list is REFUSED, not a crash"] = _refused(
                lambda: mm.mutants([p]))
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
    """Every module name `src` imports, at any depth.  (A relative `from .
    import x` yields `None`, which no file stem equals, so the caller's
    intersection drops it -- a `node.module` guard stood here and was deleted
    as unkillable by the clause-drop sweep.)"""
    got = set()
    for node in ast.walk(ast.parse(src, filename=file)):
        if isinstance(node, ast.ImportFrom):
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
    # REACHED, as a bounded fixed point (the sixth-pass attestation's N-3: a
    # worklist's `if u in seen` guard was unkillable -- dropping it changed
    # nothing but termination on a cycle).  A root outside the population
    # reaches nothing, which the partner's ghost-root arm pins.
    reached = {r for r in roots if r in inside}
    for _round in range(len(inside)):
        reached |= {v for u in reached for v in graph.get(u, set()) & inside}
    bad += ["%s is in the population and reached from no root -- on disk, never run" % stem[n]
            for n in sorted(inside - reached)]
    return sorted(set(bad))


def _roots(H, here=None):
    return ([H.ENTRY_NAME] + H.registry_modules("MUTANTS", here)
            + H.registry_modules("CASES", here))


def _membership_of(here, extra_roots=()):
    """THE membership verdict of a directory -- read off its disk exactly as
    the real control reads it, so the partner (a planted directory) and the
    real control call the SAME function (the sixth-pass attestation's IMP-2:
    the partner had driven a hand dict, and narrowing the disk read to the
    population glob left both green)."""
    H = _h()
    disk = {q.name: q.read_text(encoding="utf-8") for q in sorted(here.glob("*.py"))}
    return _membership_verdict(disk, H.files(here), _roots(H, here) + list(extra_roots))


_M_DIR = {
    "plan_memo_roles.py": "import memo_extra\n",
    "memo_extra.py": "",                                  # outside the glob, imported by it
    "stranger.py": "import plan_memo_roles\nimport plan_memo_zz\n",   # outside, importing it
    "plan_memo_zz.py": "",                                # on disk, reached from nothing
    "plan_memo_selftest_cases_zz.py": "CASES = []\n",   # a registry module: a root
}


def registry_membership_control(M):
    """PROPERTY: no module can leave a run unnoticed -- every `.py` file beside
    the checker that the population imports, or that imports it, IS in the
    population; and every module of the population is REACHED from a root
    (the entry point, and each registry module the collection step gathers).

    ⚠ The sixth attestation's probes, each of which was rc 0 before this:
    a helper outside the glob imported by `plan_memo_roles.py` (its calls
    were never read); a cases module on disk that nothing imported (its case
    never ran); a new empty checker file, and a self-test-named helper that
    nothing imports.  The partner arms plant each IN A DIRECTORY and ask the
    same `_membership_of` the real control asks.  The ghost-root arm: a root
    outside the population (`stranger`) reaches nothing."""
    H = _h()
    bad = _membership_of(H.HERE)
    with tempfile.TemporaryDirectory() as d:
        d = pathlib.Path(d)
        _tree(d, dict(_M_DIR, **{H.ENTRY: "import plan_memo_roles\n"}))
        got = _membership_of(d, extra_roots=["stranger"])
    arms = [any("memo_extra.py is imported by the population" in b for b in got),
            any("stranger.py imports the population" in b for b in got),
            any(b.startswith("plan_memo_zz.py is in the population and reached from no root")
                for b in got),
            not any("plan_memo_selftest_cases_zz.py" in b for b in got)]
    return not bad and all(arms), ("%d violation(s)%s; planted-directory arms %s"
                                   % (len(bad), ("; " + "; ".join(bad[:3])) if bad else "", arms))


# -- the tables themselves: rows, merges, fragments --------------------------

def row_shape_control(M):
    """PROPERTY: the mutant rows carry no accidental defect of their own -- a
    row naming no control, a label used twice, an edit repeated
    (`plan_memo_selftest_mutants.row_problems`, which the runner applies before
    any row runs) -- over the real registry and over planted rows, one per
    check (the sixth-pass attestation's MIN-5 / MIN-6)."""
    mm = importlib.import_module("plan_memo_selftest_mutants")
    real = mm.row_problems(mm.mutants())
    got = mm.row_problems([("e", "f.py", "p", "q", []), ("a", "f.py", "x", "y", ["c"]),
                           ("a", "f.py", "x2", "y2", ["c"]), ("b", "f.py", "x", "y", ["c"])])
    arms = [any("'e' names no control" in g for g in got),
            any("label 'a' is used twice" in g for g in got),
            any("'b' repeats another row's edit" in g for g in got)]
    return not real and all(arms), ("%d problem(s) in the real rows%s; planted arms %s"
                                    % (len(real), ("; " + "; ".join(real[:3])) if real else "", arms))


def _merge_problems(src, file):
    """A `registry()` function that builds a PLAIN dict (`dict(...)`), or holds
    a dict literal repeating a key: either lets one control shadow another in
    silence (the sixth-pass attestation's N-1)."""
    bad = []
    for fn in ast.walk(ast.parse(src, filename=file)):
        if not (isinstance(fn, ast.FunctionDef) and fn.name == "registry"):
            continue
        for node in ast.walk(fn):
            if isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id == "dict":
                bad.append("%s:%d `registry()` builds a plain dict -- merge through `Registry`"
                           % (file, node.lineno))
            if isinstance(node, ast.Dict):
                keys = [k.value for k in node.keys if isinstance(k, ast.Constant)]
                dup = sorted({k for k in keys if keys.count(k) > 1})
                bad += ["%s:%d `registry()` repeats the key %r" % (file, node.lineno, k[:60])
                        for k in dup]
    return bad


def registry_merge_control(M):
    """PROPERTY: no control name can shadow another -- every `registry()` of the
    self-test merges through `plan_memo_selftest_harness.Registry` (which
    refuses a name it holds) and no fragment's literal repeats a key; asked of
    every self-test module, of a planted source, and of `Registry` itself."""
    H = _h()
    bad = []
    for f in H.files():
        if H.is_selftest(f):
            bad += _merge_problems((H.HERE / f).read_text(encoding="utf-8"), f)
    got = _merge_problems("def registry():\n    reg = dict(a())\n    return {'k': 1, 'k': 2}\n",
                          "planted.py")
    arms = [any("plain dict" in g for g in got), any("repeats the key 'k'" in g for g in got)]
    for label, fn in (("merge", lambda: H.merge({"k": 1}, {"k": 2})),
                      ("item", lambda: H.merge({"k": 1}).__setitem__("k", 2)),
                      ("update", lambda: H.merge({"k": 1}).update({"k": 2}))):
        arms.append(_raises(fn))
    return not bad and all(arms), ("%d problem(s)%s; planted and `Registry` arms %s"
                                   % (len(bad), ("; " + "; ".join(bad[:3])) if bad else "", arms))


def _unmerged(fragments, final):
    """{module: keys} of every registry fragment, and the final table's keys:
    the fragments whose controls the final table does not carry."""
    return ["%s: %d control(s) never merged, e.g. %r" % (m, len(k - final), sorted(k - final)[0][:60])
            for m, k in sorted(fragments.items()) if k - final]


def fragment_totality_control(M):
    """PROPERTY: every self-test module that defines a `registry()` fragment has
    ALL of its controls in the one table the runner reads -- a fragment nobody
    merges drops its controls at rc 0 (the sixth-pass attestation's N-4: 740
    controls, "all controls behaved").  The fragments are found by the AST of
    the one population, not listed."""
    H = _h()
    final = set(importlib.import_module("plan_memo_selftest_controls").registry())
    fragments = {}
    for f in H.files():
        if not H.is_selftest(f):
            continue
        tree = ast.parse((H.HERE / f).read_text(encoding="utf-8"), filename=f)
        if any(isinstance(n, ast.FunctionDef) and n.name == "registry" for n in tree.body):
            fragments[H.import_name(f)] = set(importlib.import_module(H.import_name(f)).registry())
    bad = _unmerged(fragments, final)
    planted = _unmerged({"planted": {"k"}}, set())
    return not bad and bool(planted) and len(fragments) > 1, (
        "%d fragment(s), %d not merged%s; a planted fragment is %s"
        % (len(fragments), len(bad), ("; " + "; ".join(bad[:3])) if bad else "",
           "reported" if planted else "SILENT"))


def registry():
    """name -> (kind, control), this module's fragment of the one table."""
    return {
        "PROPERTY: the mutant rows carry no accidental defect of their own (a row naming no control, a label used twice, an edit repeated)":
            ("CONTROL", row_shape_control),
        "PROPERTY: no control name can shadow another -- every `registry()` merges through `Registry` and no fragment literal repeats a key":
            ("CONTROL", registry_merge_control),
        "PROPERTY: every self-test module's `registry()` fragment is merged into the one table (a fragment nobody merges drops its controls at rc 0)":
            ("CONTROL", fragment_totality_control),
        "PROPERTY: the harness DERIVES the population from a directory (glob, self-test partition, registry-by-content, import order, cycle) -- asked of a planted directory":
            ("CONTROL", population_partner_control),
        "PROPERTY: each row registry is gathered in ONE step and no module can extend the base list in any import order (planted: legit, ownless, appenders imported first)":
            ("CONTROL", registry_step_control),
        "PROPERTY: no module can leave a run unnoticed -- every file related to the population is in it, and every module in it is reached from a root":
            ("CONTROL", registry_membership_control),
    }
