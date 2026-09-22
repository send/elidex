#!/usr/bin/env python3
"""The mutation-proof rows against the MODULE POPULATION's rules -- carved on a
SUBJECT, like `plan_memo_selftest_mutants_ratchets.py`: the rules that the one
population (`plan_memo_selftest_harness.files`) must satisfy and that are
checked as rules rather than listed (the fifth attestation).  Today that is
the IMPORT DIRECTION rule (`plan_memo_selftest_records.import_direction_control`);
each row disables one of its clauses, against the control's probe arms.

`MUTANTS` here is this module's OWN list; `plan_memo_selftest_mutants.mutants()`
gathers every mutants module's list (`plan_memo_selftest_harness.registry_modules`
decides which modules those are, by CONTENT: a module holding its own list) in one explicit step.
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
     '            for n in sorted(inside - reached)]',
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

ROWS_CTL = ("PROPERTY: the mutant rows carry no accidental defect of their own (a row naming no "
            "control, a label used twice, an edit repeated)")
MERGE_CTL = ("PROPERTY: no control name can shadow another -- every `registry()` merges through "
             "`Registry` and no fragment literal repeats a key")
TOTAL_CTL = ("PROPERTY: every self-test module's `registry()` fragment is merged into the one table "
             "(a fragment nobody merges drops its controls at rc 0)")
POPMOD = "plan_memo_selftest_population.py"

# -- the sixth-pass attestation: every mechanism that pass added or found
# unpinned, each with the partner arm that kills it.
MUTANTS += [
    ("population: a registry module binds its list at TOP LEVEL (read every assignment -- a list "
     "bound inside a function makes a module a registry)", HARNESS,
     "    for node in tree.body:\n        targets = (node.targets",
     "    for node in ast.walk(tree):\n        targets = (node.targets",
     [POPULATION_CTL]),
    ("population: an ANNOTATED assignment binds the list (read `Assign` alone)", HARNESS,
     "                   else [node.target] if isinstance(node, ast.AnnAssign) else [])",
     "                   else [])",
     [POPULATION_CTL]),
    ("population: a registry module is a SELF-TEST module (drop it -- a checker module holding "
     "a `MUTANTS` becomes one)", HARNESS,
     "if is_selftest(f) and _assigns(here, f, name)]",
     "if _assigns(here, f, name)]",
     [POPULATION_CTL]),
    ("population: a `from X import` is an order edge (drop it)", HARNESS,
     "            if isinstance(node, ast.ImportFrom) and node.module in by_name:\n"
     "                got.add(node.module)",
     "            if False:\n                got.add(node.module)",
     [POPULATION_CTL]),
    ("merge: `Registry` refuses a name it holds (drop the refusal)", HARNESS,
     "        if key in self:\n            raise KeyError(",
     "        if False:\n            raise KeyError(",
     [MERGE_CTL]),
    ("merge: `Registry.update` goes through the refusal (bypass it)", HARNESS,
     "        for key, value in dict(*args, **kw).items():\n            self[key] = value",
     "        dict.update(self, *args, **kw)",
     [MERGE_CTL]),
    ("merge: a `registry()` building a plain dict is reported (drop the clause)", POPMOD,
     '            if isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id == "dict":',
     "            if False:",
     [MERGE_CTL]),
    ("merge: a fragment literal repeating a key is reported (drop the clause)", POPMOD,
     "                dup = sorted({k for k in keys if keys.count(k) > 1})",
     "                dup = []",
     [MERGE_CTL]),
    ("totality: an unmerged fragment is reported (drop the report)", POPMOD,
     "            for m, k in sorted(fragments.items()) if k - final]",
     "            for m, k in sorted(fragments.items()) if False]",
     [TOTAL_CTL]),
    ("registry: a module holding an EMPTY list is refused (drop the check -- a list emptied by "
     "another module, or a name collision, passes)", REGISTRY,
     "        if not own:\n",
     "        if False:\n",
     [STEP]),
    ("registry: a module ALIASING another's list is refused (drop the check -- its rows are "
     "counted twice)", REGISTRY,
     "        if id(own) in held:",
     "        if False:",
     [STEP]),
    ("registry: `spellings` binds only the caller's own CASES (drop the check -- cases written "
     "to a throwaway list never run)", CASES_BASE,
     '    if caller is not globals() and caller.get("CASES") is not into:',
     "    if False:",
     [STEP]),
    ("rows: a row naming NO control is a problem (drop the check -- it counts as killed)",
     MUTANTS_BASE,
     "        if not controls:",
     "        if False:",
     [ROWS_CTL]),
    ("rows: a label used twice is a problem (drop the check)", MUTANTS_BASE,
     "        if name in labels:",
     "        if False:",
     [ROWS_CTL]),
    ("rows: an edit repeated is a problem (drop the check -- an aliased list's rows run twice)",
     MUTANTS_BASE,
     "        if (file, find, replace) in edits:",
     "        if False:",
     [ROWS_CTL]),
    ("membership: the disk is EVERY `.py` beside the checker (read the population glob alone -- "
     "a helper outside it is never seen)", POPMOD,
     '    disk = {q.name: q.read_text(encoding="utf-8") for q in sorted(here.glob("*.py"))}',
     '    disk = {q.name: q.read_text(encoding="utf-8") for q in sorted(here.glob(H.GLOB))}',
     [MEMBERSHIP]),
    ("membership: a FUNCTION-LOCAL import is an edge (read top-level statements only -- the "
     "runner, imported inside the entry point's dispatch, is then reached by nothing)", POPMOD,
     "    for node in ast.walk(ast.parse(src, filename=file)):\n        if isinstance(node, ast.ImportFrom):",
     "    for node in ast.parse(src, filename=file).body:\n        if isinstance(node, ast.ImportFrom):",
     [MEMBERSHIP]),
    ("membership: a root outside the population reaches nothing (take every root)", POPMOD,
     "    reached = {r for r in roots if r in inside}",
     "    reached = set(roots)",
     [MEMBERSHIP]),
    ("membership: reachability follows the import graph (stop after the roots)", POPMOD,
     "    for _round in range(len(inside)):",
     "    for _round in range(0):",
     [MEMBERSHIP]),
]
