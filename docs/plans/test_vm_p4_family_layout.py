"""The VM-P4 umbrella family's layout -- a pin on THIS family, not a checker rule.

The plan-memo umbrella checker (`.claude/tools/plan-memo-umbrella-check.py`)
reads a schema as present iff >= 1 memo of the population holds a table
matching it, and allows >= 2 (rule C, #562).  This test does NOT restate or
tighten that rule.  It pins two facts this family asserts about its own layout
(`docs/plans/2026-10-vm-p4-memo-seam-split.md` §1.5), which the checker
deliberately does not enforce:

  (i)  the population is exactly the nine family memos -- so a child the entry
       stops linking is red here, though the checker exits 0 without it;
  (ii) each of the four schemas has its tables in the pinned memos -- today
       exactly ONE table, in ONE memo, per schema.  The lists are compared
       sorted, so first-link (BFS) order never moves this test.

What the pins guarantee, by layer.  While a schema's pin has length one, a
header that drifts off that schema leaves the schema in no memo: the checker
itself exits 2 (presence miss), and every test here is red through `setUp`.
Updating a pin to length >= 2 does NOT make that drift silent in CI: the
drifted table drops out of `homes`, which then differs from the pin, so (ii)
stays red and the `tools` job with it.  What it gives up is the checker's own
signal: under rule C the checker run by hand can exit 0 (the drifted table's
rows leave the census unless one of them carries a kind phrase), so a manual
run no longer shows the drift.

When a structural change moves a schema table to another memo, adds a
carve-out, or otherwise changes the layout, update this test in the same
commit -- that update is the intended change, not a workaround.  When (i) is
red because a member links a `.md` outside the family, the default remedy is
to drop the link (a backticked path adds no edge); widening the family instead
means updating FAMILY here and the `docs/plans/2026-07-vm-p4-*.md` glob in
CLAUDE.md in the same commit.

It lives beside the family, not beside the checker: it is a consumer of the
checker's library, not a module of the checker, so it is outside the checker
self-test's membership rule (`registry_membership_control`), and it runs as
its own root of `scripts/python-suites.sh`.
"""

import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
TOOLS = os.path.normpath(os.path.join(HERE, "..", "..", ".claude", "tools"))
if TOOLS not in sys.path:
    sys.path.insert(0, TOOLS)

from plan_memo_population import Population  # noqa: E402
from plan_memo_tables import SCHEMAS  # noqa: E402

X = "2026-07-vm-p4-"
ENTRY = X + "es-language-completeness.md"
FAMILY = {ENTRY} | {X + s + ".md" for s in (
    "design-invariants", "slice-plan", "touch-time-split-record", "slot-ledger", "open-decisions",
    "slice-0a-landing-record", "slice-1a-1b-call-spread-detail", "umbrella-review-rounds")}
HOMES = {
    "citation": [ENTRY],
    "stub": [ENTRY],
    "slice": [X + "slice-plan.md"],
    "slot": [X + "slot-ledger.md"],
}


class VmP4FamilyLayout(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.p = Population(os.path.join(HERE, ENTRY))

    def setUp(self):
        # a population with misses is unscanned: both sets below would mean
        # nothing.  Any miss turns every test red -- a content defect (a row
        # of the wrong width, an unresolved reference) included.
        self.assertEqual(self.p.misses, [])

    def test_population_is_the_family(self):
        got = {self.p.display(m.path) for m in self.p.memos}
        self.assertEqual(got, FAMILY)

    def test_each_schema_has_one_home(self):
        self.assertEqual(set(HOMES), {s.name for s in SCHEMAS})
        homes = {}
        for m in self.p.memos:
            for t in m.tables:
                if t.schema is not None:
                    homes.setdefault(t.schema.name, []).append(self.p.display(m.path))
        self.assertEqual({k: sorted(v) for k, v in homes.items()},
                         {k: sorted(v) for k, v in HOMES.items()})


if __name__ == "__main__":
    unittest.main()
