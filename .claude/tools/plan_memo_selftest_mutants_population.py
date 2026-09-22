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

FAMILY = ("the entries named `PROPERTY: ...` are exactly the property family's fragments (records "
          "and ratchets), both directions")

MUTANTS += [
    ("family: the ratchets module IS in the property family (re-inject the three-module family the "
     "docstrings stated -- every ratchet entry is then a PROPERTY name from outside it)", CONTROLS,
     '    family = set(property_registry()) | set(ratchet_registry())',
     '    family = set(property_registry())',
     [FAMILY]),
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
