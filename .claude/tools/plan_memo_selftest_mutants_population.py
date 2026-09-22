#!/usr/bin/env python3
"""The mutation-proof rows against the MODULE POPULATION's rules -- carved on a
SUBJECT, like `plan_memo_selftest_mutants_ratchets.py`: the rules that the one
population (`plan_memo_selftest_harness.files`) must satisfy and that are
checked as rules rather than listed (the fifth attestation).  Today that is
the IMPORT DIRECTION rule (`plan_memo_selftest_records.import_direction_control`);
each row disables one of its clauses, against the control's probe arms.

`MUTANTS` here is this module's OWN list; `plan_memo_selftest_mutants.mutants()`
gathers every mutants module's list (the harness's file-name rule decides which
modules those are) in one explicit step.
"""

from plan_memo_selftest_mutants import CONTROLS, RECORDS

MUTANTS = []

DIRECTION = ("PROPERTY: the self-test's IMPORT DIRECTION rule holds of the imports (no cycle, the "
             "harness imports nothing of the set, no checker module imports the self-test but the "
             "entry point's dispatch) -- a rule over the AST, not an inventory of edges")

FAMILY = "the entries named `PROPERTY: ...` are exactly the property family's fragments (`_FAMILY`), both directions"
HARNESS, REGISTRY, CASES_BASE, MUTANTS_BASE = (
    "plan_memo_selftest_harness.py", "plan_memo_selftest_registry.py",
    "plan_memo_selftest_cases.py", "plan_memo_selftest_mutants.py")
POPULATION_CTL = ("PROPERTY: the harness DERIVES the population from a directory (glob, self-test "
                  "partition, registry-by-content, import order, cycle) -- asked of a planted directory")
STEP = ("PROPERTY: each row registry is gathered in ONE step and no module can extend the base list "
        "in any import order (planted: legit, ownless, appenders imported first)")
MEMBERSHIP = ("PROPERTY: no module can leave a run unnoticed -- every file related to the population "
              "is in it, and every module in it is reached from a root")

MUTANTS += [
    ("family: the ratchets module IS in the property family (re-inject the three-module family the "
     "docstrings stated -- every ratchet entry is then a PROPERTY name from outside it)", CONTROLS,
     '_FAMILY = ("plan_memo_selftest_records", "plan_memo_selftest_ratchets",',
     '_FAMILY = ("plan_memo_selftest_records",',
     [FAMILY]),
    ("family: the STRAY direction is reported (drop it -- a PROPERTY name from outside the family "
     "passes)", CONTROLS,
     '    return sorted(set(family) - named), sorted(named - set(family))',
     '    return sorted(set(family) - named), []',
     [FAMILY]),
    # -- the population (the harness) --
    ("population: the entry point is in it (drop it from `files()`)", HARNESS,
     '    return sorted(p.name for p in here.glob(GLOB)) + [ENTRY]',
     '    return sorted(p.name for p in here.glob(GLOB))',
     [POPULATION_CTL]),
    ("population: the glob is `plan_memo*.py` (re-inject the narrowed `plan_memo_*.py` -- "
     "`plan_memo.py` leaves in silence)", HARNESS,
     'GLOB = "plan_memo*.py"',
     'GLOB = "plan_memo_*.py"',
     [POPULATION_CTL]),
    ("population: the self-test partition reads the whole name (read a prefix -- the runner "
     "`plan_memo_umbrella_selftest.py` becomes a checker module)", HARNESS,
     '    return "selftest" in file',
     '    return file.startswith("plan_memo_selftest")',
     [POPULATION_CTL]),
    ("population: a registry module is one that HOLDS the list (re-inject the name rule -- a "
     "renamed module's rows leave in silence)", HARNESS,
     "    return [import_name(f) for f in files(here) if is_selftest(f) and _assigns(here, f, name)]",
     '    return [import_name(f) for f in files(here)\n'
     '            if f.startswith("plan_memo_selftest_" + name.lower())]',
     [POPULATION_CTL]),
    ("population: an `import X` is an order edge (read `from X import` alone)", HARNESS,
     "            elif isinstance(node, ast.Import):\n                got |= {a.name for a in node.names if a.name in by_name}",
     "            elif False:\n                got |= {a.name for a in node.names if a.name in by_name}",
     [POPULATION_CTL]),
    ("population: an import cycle is an error (break out of it -- a misordered load in silence)",
     HARNESS,
     "        if not ready:\n            raise RuntimeError(",
     "        if not ready:\n            break\n            raise RuntimeError(",
     [POPULATION_CTL]),
    # -- the registries (the one collection step, and the seals) --
    ("registry: the base module is skipped by the step (drop the skip -- the base's own list is "
     "then refused as a module holding the base's list)", REGISTRY,
     "        if name == base:\n            continue\n",
     "",
     [STEP]),
    ("registry: a module whose list IS the base's is refused (drop the check)", REGISTRY,
     "        if own is None or own is base_rows:",
     "        if own is None:",
     [STEP]),
    ("registry: `names` plants a module (ignore it -- the partner cannot plant)", REGISTRY,
     "    for name in (harness.registry_modules(listname) if names is None else names):",
     "    for name in harness.registry_modules(listname):",
     [STEP]),
    ("registry: the base MUTANTS is sealed (leave it a list -- an appender extends it at import, "
     "in whatever order)", MUTANTS_BASE,
     "MUTANTS = tuple(MUTANTS)\n",
     "MUTANTS = list(MUTANTS)\n",
     [STEP]),
    ("registry: the base CASES is sealed (leave it a list)", CASES_BASE,
     "CASES = tuple(_OWN)\n",
     "CASES = _OWN\n",
     [STEP]),
    ("registry: the base spellings refuse once sealed (never seal them -- a module calling the "
     "base `case` loses its rows)", CASES_BASE,
     "_SEALED[0] = True\n",
     "_SEALED[0] = False\n",
     [STEP]),
    # -- membership --
    ("membership: a file the population imports must be in it (drop the clause)", "plan_memo_selftest_population.py",
     '            bad += ["%s is imported by the population but is not in it" % stem[x]\n'
     '                    for x in sorted(deps - inside)]',
     "            pass",
     [MEMBERSHIP]),
    ("membership: a file importing the population must be in it (drop the clause)",
     "plan_memo_selftest_population.py",
     '            bad.append("%s imports the population but is not in it" % stem[name])',
     "            pass",
     [MEMBERSHIP]),
    ("membership: every module is reached from a root (drop the clause -- a module on disk never "
     "runs)", "plan_memo_selftest_population.py",
     '    bad += ["%s is in the population and reached from no root -- on disk, never run" % stem[n]\n'
     '            for n in sorted(inside - seen)]',
     "    pass",
     [MEMBERSHIP]),
    ("direction: the harness imports nothing of the population (drop the clause)", RECORDS,
     "    if graph.get(harness):",
     "    if False:",
     [DIRECTION]),
    ("direction: no checker module imports the self-test (drop the clause)", RECORDS,
     "            if (mod, dep) != (import_name(ENTRY), RUNNER_NAME):",
     "            if False:",
     [DIRECTION]),
    ("direction: the entry point's dispatch is the ONE exception (drop the exception -- the rule "
     "then forbids the runner's own entry)", RECORDS,
     "            if (mod, dep) != (import_name(ENTRY), RUNNER_NAME):",
     "            if True:",
     [DIRECTION]),
    ("direction: a cycle is a violation (drop the report)", RECORDS,
     '                bad.append("cycle: %s" % " -> ".join(path[path.index(v):] + [v]))',
     "                pass",
     [DIRECTION]),
    ("direction: a FUNCTION-LOCAL import is an edge (read module-level statements only -- the "
     "form a hidden cycle takes)", RECORDS,
     "        for node in ast.walk(ast.parse(src, filename=file)):\n            if isinstance(node, ast.ImportFrom) and node.module in names:",
     "        for node in ast.parse(src, filename=file).body:\n            if isinstance(node, ast.ImportFrom) and node.module in names:",
     [DIRECTION]),
]
