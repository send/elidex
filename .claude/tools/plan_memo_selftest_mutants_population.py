#!/usr/bin/env python3
"""The mutation-proof rows against the MODULE POPULATION's rules -- carved on a
SUBJECT, like `plan_memo_selftest_mutants_ratchets.py`: the rules that the one
population (`plan_memo_selftest_harness.files`) must satisfy and that are
checked as rules rather than listed (the fifth attestation): the population's
derivation, membership, the IMPORT DIRECTION rule, the property family and the
golden manifest (`plan_memo_selftest_manifest`).  Each row disables one clause,
against the partner arm that kills it.

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
HARNESS, REGISTRY = "plan_memo_selftest_harness.py", "plan_memo_selftest_registry.py"
POPULATION_CTL = ("PROPERTY: the harness DERIVES the population from a directory (glob, self-test "
                  "partition, registry-by-content, import order, cycle) -- asked of a planted directory")
MEMBERSHIP = ("PROPERTY: no module can leave a run unnoticed -- every file related to the population "
              "is in it, and every module in it is reached from a root")

POPMOD = "plan_memo_selftest_population.py"

MANIFEST_CTL = ("PROPERTY: the live collection is exactly the committed golden manifest, and the "
                "manifest mechanism holds (comparison, one source, deep immutability, "
                "generation-time validations)")
MANIFEST_MOD = "plan_memo_selftest_manifest.py"

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
     [MANIFEST_CTL]),
    ("merge: `Registry.update` goes through the refusal (bypass it)", HARNESS,
     "        for key, value in dict(*args, **kw).items():\n            self[key] = value",
     "        dict.update(self, *args, **kw)",
     [MANIFEST_CTL]),
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

# -- the golden manifest: each part of the mechanism, against the arm of
# `manifest_control` that kills it.
MUTANTS += [
    ("manifest: a difference is reported (skip the comparison)", MANIFEST_MOD,
     "    if removed or added or changed:",
     "    if False:",
     [MANIFEST_CTL]),
    ("manifest: a REMOVED line is a difference (drop that half)", MANIFEST_MOD,
     "    removed = sorted((w - g).elements())",
     "    removed = []",
     [MANIFEST_CTL]),
    ("manifest: an ADDED line alone is a difference (read removals and changes only)", MANIFEST_MOD,
     "    if removed or added or changed:",
     "    if removed or changed:",
     [MANIFEST_CTL]),
    ("manifest: a CHANGED line is reported once, as changed (leave it in removed and added too)",
     MANIFEST_MOD,
     "    return ([x for x in removed if x not in gone], [a for a in added if a not in came], changed)",
     "    return (removed, added, changed)",
     [MANIFEST_CTL]),
    ("manifest: `read` reads the file it is given (read the committed one always)", MANIFEST_MOD,
     '    text = (PATH if path is None else path).read_text(encoding="utf-8")',
     '    text = PATH.read_text(encoding="utf-8")',
     [MANIFEST_CTL]),
    ("manifest: the generator writes the RUNNER's collection (write a different one -- the rows "
     "dropped)", MANIFEST_MOD,
     "    snap = snapshot()\n    problems = validate(snap)",
     "    snap = snapshot()._replace(rows=())\n    problems = validate(snap)",
     [MANIFEST_CTL]),
    ("manifest: the collection FREEZES what it reads (return it as it is)", REGISTRY,
     "        frozen = _freeze(getattr(mod, listname))",
     "        frozen = getattr(mod, listname)",
     [MANIFEST_CTL]),
    ("manifest: the collection replaces each module's list by its frozen copy (leave the list)",
     REGISTRY,
     "        setattr(mod, listname, frozen)",
     "        pass",
     [MANIFEST_CTL]),
    ("manifest: a dict is frozen into sorted pairs (leave it a dict -- a case's `files`)", REGISTRY,
     "    if isinstance(x, dict):\n        return tuple(",
     "    if False:\n        return tuple(",
     [MANIFEST_CTL]),
    ("manifest: a row naming NO control is refused at generation (drop the check)", MANIFEST_MOD,
     "        if not controls:",
     "        if False:",
     [MANIFEST_CTL]),
    ("manifest: a row naming a MISSING control is refused at generation (drop the check)",
     MANIFEST_MOD,
     "            if c not in snap.table:",
     "            if False:",
     [MANIFEST_CTL]),
    ("manifest: a mutation label used twice is refused at generation (drop the check)", MANIFEST_MOD,
     '    bad += ["mutation label %r is used %d times"',
     '    bad += [] and ["mutation label %r is used %d times"',
     [MANIFEST_CTL]),
    ("manifest: a case name used twice is refused at generation (drop the check)", MANIFEST_MOD,
     '    bad += ["case name %r is used %d times"',
     '    bad += [] and ["case name %r is used %d times"',
     [MANIFEST_CTL]),
]
